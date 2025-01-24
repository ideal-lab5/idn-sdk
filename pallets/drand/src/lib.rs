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
//! into the runtime.
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
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
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
// use sc_consensus_randomness_beacon::timelock::{TimelockEncryptionProvider, TimelockError};
use ark_ec::AffineRepr;
use sha2::{Digest, Sha256};
use sp_consensus_randomness_beacon::types::OpaquePulse;
use sp_runtime::{
	offchain::{http, Duration},
	traits::{Hash, One, Zero},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	KeyTypeId,
};
use timelock::{
	curves::drand::TinyBLS381,
	tlock::{tld, EngineBLS, TLECiphertext},
};

pub mod bls12_381;
pub mod types;
pub mod verifier;

use types::*;
use verifier::Verifier;

#[cfg(not(feature = "host-arkworks"))]
use ark_bls12_381::G1Affine as G1AffineOpt;
#[cfg(feature = "host-arkworks")]
use sp_ark_bls12_381::G1Affine as G1AffineOpt;

use crate::types::{BoundedStorage, Metadata, OpaquePublicKey, OpaqueSignature, RoundNumber};

pub type RandomValue = [u8; 32];

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
	pub type AggregatedSignatures<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		(OpaqueSignature, RoundNumber, RoundNumber), // (asig, init round, height)
		OptionQuery,
	>;

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
		/// The next round number is invalid (either too high or too low)
		InvalidNextRound,
		NetworkTooEarly,
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = MakeFatalError<()>;

		const INHERENT_IDENTIFIER: [u8; 8] =
			sp_consensus_randomness_beacon::inherents::INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			// the pulse data
			let data: Option<Vec<Vec<u8>>> = data
				.get_data::<Vec<Vec<u8>>>(&Self::INHERENT_IDENTIFIER)
				.expect("The inherent data should be well formatted.");

			if let Some(raw_pulses) = data {
				// convert to pulses and compute asig while extracts rounds
				let mut start_round = 0;
				let height = raw_pulses.len();
				let mut asig = G1AffineOpt::zero();
				let size = raw_pulses.len();
				raw_pulses.iter().for_each(|rp| {
					let pulse = OpaquePulse::deserialize_from_vec(rp);
					if start_round == 0 {
						start_round = pulse.round;
					}
					// TODO: handle unwrap
					let sig = pulse.signature_point().unwrap();
					asig = (asig + sig).into();
				});
				// todo: with capacity
				let mut asig_bytes = Vec::new();
				asig.serialize_compressed(&mut asig_bytes).unwrap();

				let bounded_asig_bytes = OpaqueSignature::truncate_from(asig_bytes);
				return Some(Call::write_pulses {
					asig: bounded_asig_bytes,
					start_round,
					height: height as u64,
				});
			}

			None
		}

		fn check_inherent(_call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
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
			Self { config: drand_quicknet_config(), _phantom: Default::default() }
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
		/// there is no verification of configurations, so be careful with this.
		///
		/// * `origin`: the root user
		/// * `config`: the beacon configuration
		///
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

		/// Write a set of pulses to the runtime
		///
		/// * `origin`: A None origin
		/// * `asig`: An aggregated signature
		/// * `start_round`: The round that coincides with the first signature added to asig
		/// * `height`: The number of signature aggregated to get asig. i.e. if asig = a1 + a2 + a3, then height = 3
		///
		#[pallet::call_index(1)]
		#[pallet::weight(1_000)]
		pub fn write_pulses(
			origin: OriginFor<T>,
			asig: OpaqueSignature,
			start_round: RoundNumber,
			height: RoundNumber,
		) -> DispatchResult {
			ensure_none(origin)?;
			let config = BeaconConfig::<T>::get().ok_or(Error::<T>::MissingBeaconConfig)?;
			let latest_block = <frame_system::Pallet<T>>::block_number();
			// unlikely, but to let's be safe
			frame_support::ensure!(!latest_block.is_zero(), Error::<T>::NetworkTooEarly);

			let prev_block = latest_block - 1u32.into();
			// fail early on missing beacon config
			// If start_round <= latest_round => fail
			if let Some(latest) = AggregatedSignatures::<T>::get(prev_block) {
				// latest = last init round + #{pulses}
				let latest_round: RoundNumber = latest.1 + latest.2;
				frame_support::ensure!(start_round == latest_round + 1, Error::<T>::InvalidNextRound);
			}
			// note: if there is not latest round, we can assume it is the genesis round (for now..)
				
			let rounds = (start_round..start_round + height).collect::<Vec<_>>();

			let validity = T::Verifier::verify(config.public_key, asig.clone(), &rounds)
				.map_err(|reason| Error::<T>::InvalidInput)?;

			frame_support::ensure!(validity, Error::<T>::VerificationFailed);

			AggregatedSignatures::<T>::insert(latest_block, &(asig, start_round, height));
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
}

/// build a beacon config for drand quicknet
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
	/// This function hashes together the subject with the latest known randomness from quicknet
	/// we should replace this with the merkle root later on...
	fn random(subject: &[u8]) -> (T::Hash, BlockNumberFor<T>) {
		let latest_block = <frame_system::Pallet<T>>::block_number();
		let mut entropy = T::Hash::default();
		if let Some((asig, start_round, height)) = AggregatedSignatures::<T>::get(latest_block) {
			entropy = (subject, start_round, height, asig).using_encoded(T::Hashing::hash);
		}

		(entropy, latest_block)
	}
}

// impl<T: Config> TimelockEncryptionProvider<RoundNumber> for Pallet<T> {
// 	fn decrypt_at(
// 		ciphertext_bytes: &[u8],
// 		round_number: RoundNumber,
// 	) -> Result<Vec<u8>, TimelockError> {
// 		if let Some(secret) = Pulses::<T>::get(round_number) {
// 			// TODO: replace with optimized arkworks types?
// 			let ciphertext: TLECiphertext<TinyBLS377> =
// 				TLECiphertext::deserialize_compressed(ciphertext_bytes)
// 					.map_err(|_| TimelockError::DecodeFailure)?;

// 			let sig: <TinyBLS377 as EngineBLS>::SignatureGroup =
// 				<TinyBLS377 as EngineBLS>::SignatureGroup::deserialize_compressed(
// 					&secret.body.signature.to_vec()[..],
// 				)
// 				.map_err(|_| TimelockError::DecodeFailure)?;

// 			let plaintext = ciphertext.tld(sig).map_err(|_| TimelockError::DecryptionFailed)?;

// 			return Ok(plaintext);
// 		}
// 		Err(TimelockError::MissingSecret)
// 	}

// 	fn latest() -> BlockNumberFor<T> {
// 		return Height::<T>::get();
// 	}
// }
