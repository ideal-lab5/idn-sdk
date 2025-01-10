/*
 * Copyright 2024 by Ideal Labs, LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! # Drand Bridge Pallet
//!
//! A pallet to bridge to [drand](drand.love)'s Quicknet, injecting publicly verifiable randomness
//! into the runtime
//!
//! ## Overview
//!
//! Quicknet chain runs in an 'unchained' mode, producing a fresh pulse of randomness every 3s
//! This pallet implements an offchain worker that consumes pulses from quicket and then sends a
//! signed transaction to encode them in the runtime. The runtime uses the optimized arkworks host
//! functions to efficiently verify the pulse.
//!
//! Run `cargo doc --package pallet-drand --open` to view this pallet's documentation.

// We make sure this pallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

extern crate alloc;

use alloc::{
	format,
	string::{String, ToString},
	vec,
	vec::Vec,
};
use ark_serialize::CanonicalDeserialize;
use codec::Encode;
use frame_support::{
	pallet_prelude::*,
	storage::bounded_vec::BoundedVec,
	traits::{ConstU32, Randomness},
};
use frame_system::{
	offchain::{
		AppCrypto, CreateSignedTransaction, SendUnsignedTransaction, SignedPayload, Signer,
		SigningTypes,
	},
	pallet_prelude::BlockNumberFor,
};
use log;
use sha2::{Digest, Sha256};
use sp_consensus_randomness_beacon::types::OpaquePulse;
use sp_runtime::{
	offchain::{http, Duration},
	traits::{Hash, One, Zero},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	KeyTypeId,
};
use timelock::{curves::drand::TinyBLS381, tlock::EngineBLS};

pub mod bls12_381;
pub mod types;
pub mod verifier;

use types::*;
use verifier::Verifier;

use crate::types::{BoundedStorage, Metadata, OpaquePublicKey, RoundNumber};

pub type RandomValue = [u8; 32];
pub type OpaqueSignature = [u8; 48];

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: WeightInfo;
		/// something that knows how to verify beacon pulses
		type Verifier: Verifier;
	}

	/// the drand beacon configuration
	#[pallet::storage]
	pub type BeaconConfig<T: Config> = StorageValue<_, BeaconConfiguration, OptionQuery>;

	/// A map of round number to pulses from the randomness beacon
	#[pallet::storage]
	pub type Pulses<T: Config> =
		StorageMap<_, Blake2_128Concat, RoundNumber, OpaqueSignature, OptionQuery>;

	#[pallet::storage]
	pub type LatestRound<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		BeaconConfigChanged,
		/// A user has successfully set a new value.
		PulseVerificationSuccess {
			/// The new value set.
			rounds: Vec<RoundNumber>,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The value retrieved was `None` as no value was previously set.
		NoneValue,
		/// There is no valid beacon config
		MissingBeaconConfig,
		/// There was an attempt to increment the value in storage over `u32::MAX`.
		StorageOverflow,
		/// The input data could not be decoded or was empty
		InvalidInput,
		/// the pulse could not be verified
		VerificationFailed,
	}

	/// Allows the pallet to provide some inherent. See [`frame_support::inherent::ProvideInherent`]
	/// for more info.
	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = MakeFatalError<()>;

		const INHERENT_IDENTIFIER: [u8; 8] =
			sp_consensus_randomness_beacon::inherents::INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			let data: Option<Vec<Vec<u8>>> = data
				.get_data::<Vec<Vec<u8>>>(&Self::INHERENT_IDENTIFIER)
				.expect("The inherent data should be well formatted.");

			if let Some(pulses) = data {
				return Some(Call::write_pulses { data: pulses });
			}

			None
		}

		fn check_inherent(_call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
			// TODO: it should be signed by the expected block author
			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::write_pulses { .. })
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// The randomness beacon config
		pub config: BeaconConfiguration,
		/// Phantom config
		#[serde(skip)]
		pub _phantom: core::marker::PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		/// The default configuration is Drand Quicknet
		/// https://api.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/info
		fn default() -> Self {
			Self {
				config: build_beacon_configuration(
					"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a",
					3,
					1692803367,
					"52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971",
					"f477d5c89f21a17c863a7f937c6a6d15859414d2be09cd448d4279af331c5d3e",
					"bls-unchained-g1-rfc9380",
					"quicknet"
				),
				_phantom: Default::default(),
			}
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize(self.config.clone())
				.expect("The genesis config should be correct.");
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// allows the root user to set the beacon configuration
		/// generally this would be called from an offchain worker context.
		/// there is no verification of configurations, so be careful with this.
		///
		/// * `origin`: the root user
		/// * `config`: the beacon configuration
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::set_beacon_config())]
		pub fn set_beacon_config(
			origin: OriginFor<T>,
			config: BeaconConfiguration,
		) -> DispatchResult {
			ensure_root(origin)?;
			BeaconConfig::<T>::put(config);
			Self::deposit_event(Event::BeaconConfigChanged {});
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(1_000)]
		pub fn write_pulses(origin: OriginFor<T>, data: Vec<Vec<u8>>) -> DispatchResult {
			ensure_none(origin)?;

			let config = BeaconConfig::<T>::get();
			ensure!(config.is_some(), Error::<T>::MissingBeaconConfig);
			// .expect("The beacon configuration exists.")
			let config = config.unwrap();

			let mut rounds = vec![];
			let pulses: Vec<OpaquePulse> = data
				.iter()
				.map(|d| {
					let pulse = OpaquePulse::deserialize_from_vec(&d);
					rounds.push(pulse.round);
					pulse
				})
				.collect::<Vec<_>>();

			let validity =
				T::Verifier::verify(config, pulses).map_err(|reason| Error::<T>::InvalidInput)?;

			frame_support::ensure!(validity, Error::<T>::VerificationFailed);

			LatestRound::<T>::set(*rounds.get(rounds.len() - 1).unwrap());

			Self::deposit_event(Event::PulseVerificationSuccess { rounds });

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn initialize(config: BeaconConfiguration) -> Result<(), ()> {
		BeaconConfig::<T>::set(Some(config));
		Ok(())
	}

	/// get the randomness at a specific block height
	/// returns None if it is invalid or does not exist
	pub fn random_at(block_number: BlockNumberFor<T>) -> Option<RandomValue> {
		// if let Some(pulse) = Pulses::<T>::get(block_number) {
		// 	pulse.randomness.into_inner().try_into().ok()
		// } else {
		// 	None
		// }
		None
	}
}

/// construct a message (e.g. signed by drand)
pub fn message(current_round: RoundNumber, prev_sig: &[u8]) -> Vec<u8> {
	let mut hasher = Sha256::default();
	hasher.update(prev_sig);
	hasher.update(current_round.to_be_bytes());
	hasher.finalize().to_vec()
}

pub(crate) fn drand_quicknet_config() -> BeaconConfiguration {
	build_beacon_configuration(
		"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a",
		3,
		1692803367,
		"52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971",
		"f477d5c89f21a17c863a7f937c6a6d15859414d2be09cd448d4279af331c5d3e",
		"bls-unchained-g1-rfc9380",
		"quicknet"
	)
}

fn build_beacon_configuration(
	pk_hex: &str,
	period: u32,
	genesis_time: u32,
	hash_hex: &str,
	group_hash_hex: &str,
	scheme_id: &str,
	beacon_id: &str,
) -> BeaconConfiguration {
	let pk = hex::decode(pk_hex).expect("Valid hex");
	let hash = hex::decode(hash_hex).expect("Valid hex");
	let group_hash = hex::decode(group_hash_hex).expect("Valid hex");

	let public_key: OpaquePublicKey = BoundedVec::try_from(pk).expect("Public key within bounds");
	let hash: BoundedStorage = BoundedVec::try_from(hash).expect("Hash within bounds");
	let group_hash: BoundedStorage =
		BoundedVec::try_from(group_hash).expect("Group hash within bounds");
	let scheme_id: BoundedStorage =
		BoundedVec::try_from(scheme_id.as_bytes().to_vec()).expect("Scheme ID within bounds");
	let beacon_id: BoundedStorage =
		BoundedVec::try_from(beacon_id.as_bytes().to_vec()).expect("Scheme ID within bounds");

	let metadata = Metadata { beacon_id };

	BeaconConfiguration { public_key, period, genesis_time, hash, group_hash, scheme_id, metadata }
}

impl<T: Config> Randomness<T::Hash, BlockNumberFor<T>> for Pallet<T> {
	// this function hashes together the subject with the latest known randomness from quicknet
	fn random(subject: &[u8]) -> (T::Hash, BlockNumberFor<T>) {
		let block_number_minus_one = <frame_system::Pallet<T>>::block_number() - One::one();

		let mut entropy = T::Hash::default();
		// if let Some(pulse) = Pulses::<T>::get(block_number_minus_one) {
		// 	entropy = (subject, block_number_minus_one, pulse.randomness.clone())
		// 		.using_encoded(T::Hashing::hash);
		// }

		(entropy, block_number_minus_one)
	}
}
