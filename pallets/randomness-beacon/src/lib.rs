/*
 * Copyright 2025 by Ideal Labs, LLC
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

use alloc::{vec, vec::Vec};
use ark_serialize::CanonicalSerialize;
use frame_support::{pallet_prelude::*, storage::bounded_vec::BoundedVec};
use sp_consensus_randomness_beacon::types::OpaquePulse;

pub mod aggregator;
pub mod bls12_381;
pub mod types;

use aggregator::SignatureAggregator;
use types::*;

use crate::{
	aggregator::zero_on_g1,
	types::{BoundedStorage, Metadata, OpaquePublicKey, OpaqueSignature, RoundNumber},
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

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
		/// something that knows how to aggregate and verify beacon pulses
		type SignatureAggregator: SignatureAggregator;
		/// The number of pulses per block
		type SignatureToBlockRatio: Get<u8>;
	}

	/// the drand beacon configuration
	#[pallet::storage]
	pub type BeaconConfig<T: Config> = StorageValue<_, BeaconConfiguration, OptionQuery>;

	/// A first round number for which a pulse was observed
	#[pallet::storage]
	pub type GenesisRound<T: Config> = StorageValue<_, RoundNumber, ValueQuery>;

	/// The latest observed round
	#[pallet::storage]
	pub type LatestRound<T: Config> = StorageValue<_, RoundNumber, ValueQuery>;

	/// The aggregated signature and aggregated public key (identifier) of all observed pulses of
	/// randomness
	#[pallet::storage]
	pub type AggregatedSignature<T: Config> =
		StorageValue<_, (OpaqueSignature, OpaqueSignature), OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The beacon configuration has been updated by a root address
		BeaconConfigChanged,
		/// The genesis round has been changed by a root address
		GenesisRoundChanged,
		/// Siganture verification succeeded for signatures associated with the given rounds.
		SignatureVerificationSuccess,
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
		/// The network is at block 0.
		NetworkTooEarly,
		/// There must be at least one pulse provided.
		NonPositiveHeight,
		/// The genesis round is zero.
		GenesisRoundNotSet,
		/// The genesis is already set.
		GenesisRoundAlreadySet,
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = MakeFatalError<()>;

		const INHERENT_IDENTIFIER: [u8; 8] =
			sp_consensus_randomness_beacon::inherents::INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			// if we do not find any pulse data, then do nothing
			if let Ok(Some(raw_pulses)) = data.get_data::<Vec<Vec<u8>>>(&Self::INHERENT_IDENTIFIER)
			{
				let asig = raw_pulses
					.iter()
					.filter_map(|rp| OpaquePulse::deserialize_from_vec(rp).ok())
					.filter_map(|pulse| pulse.signature_point().ok())
					.fold(zero_on_g1(), |acc, sig| (acc + sig).into());

				let mut asig_bytes = Vec::with_capacity(48);
				if asig.serialize_compressed(&mut asig_bytes).is_err() {
					log::error!("Failed to serialize the aggregated signature.");
					return None;
				}

				return Some(Call::try_submit_asig {
					asig: OpaqueSignature::truncate_from(asig_bytes),
					round: None,
				});
			} else {
				log::info!("The node provided empty pulse data to the inherent!");
			}

			None
		}

		fn check_inherent(_call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::try_submit_asig { .. })
		}
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		/// The randomness beacon config
		pub config: BeaconConfiguration,
		/// The first round where we should start consuming from the beacon
		pub genesis_round: RoundNumber,
		/// Phantom config
		#[serde(skip)]
		pub _phantom: core::marker::PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		/// The default configuration is Drand Quicknet
		/// https://api.drand.sh/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971/info
		fn default() -> Self {
			Self { config: drand_quicknet_config(), genesis_round: 0, _phantom: Default::default() }
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
		/// * `origin`: The root user
		/// * `config`: The new beacon configuration
		/// TODO: weights generation and benchmarking: https://github.com/ideal-lab5/idn-sdk/issues/56
		#[pallet::call_index(0)]
		#[pallet::weight(1_000)]
		pub fn set_beacon_config(
			origin: OriginFor<T>,
			config: BeaconConfiguration,
		) -> DispatchResult {
			ensure_root(origin)?;
			BeaconConfig::<T>::put(config);
			Self::deposit_event(Event::BeaconConfigChanged {});
			Ok(())
		}

		/// Set the genesis round if the origin has root privileges
		///
		/// * `origin`: The root user
		/// * `round`: The new genesis round
		/// TODO: weights generation and benchmarking: https://github.com/ideal-lab5/idn-sdk/issues/56
		#[pallet::call_index(1)]
		#[pallet::weight(1_000)]
		pub fn set_genesis_round(origin: OriginFor<T>, round: RoundNumber) -> DispatchResult {
			ensure_root(origin)?;
			GenesisRound::<T>::put(round);
			Self::deposit_event(Event::GenesisRoundChanged {});
			Ok(())
		}

		/// Write a set of pulses to the runtime
		///
		/// * `origin`: A None origin
		/// * `asig`: An aggregated signature
		/// * `round`: An optional genesis round number. It can only be set if the existing genesis
		///   round is 0.
		/// TODO: weights generation and benchmarking: https://github.com/ideal-lab5/idn-sdk/issues/56
		#[pallet::call_index(2)]
		#[pallet::weight(1_000)]
		pub fn try_submit_asig(
			origin: OriginFor<T>,
			asig: OpaqueSignature,
			round: Option<RoundNumber>,
		) -> DispatchResult {
			ensure_none(origin)?;
			// the beacon config must exist
			let config = BeaconConfig::<T>::get().ok_or(Error::<T>::MissingBeaconConfig)?;

			let mut genesis_round = GenesisRound::<T>::get();
			let mut latest_round = LatestRound::<T>::get();

			if let Some(r) = round {
				// if a round is provided and the genesis round is not set
				frame_support::ensure!(genesis_round == 0, Error::<T>::GenesisRoundAlreadySet);
				GenesisRound::<T>::set(r);
				genesis_round = r;
				latest_round = genesis_round;
			} else {
				//  if the genesis round is not set and a round is not provided
				frame_support::ensure!(
					GenesisRound::<T>::get() > 0,
					Error::<T>::GenesisRoundNotSet
				);
			}

			// aggregate old asig/apk with the new one and verify the aggregation
			let (new_asig, new_apk) = T::SignatureAggregator::aggregate_and_verify(
				config.public_key,
				asig,
				latest_round,
				T::SignatureToBlockRatio::get() as u64,
				AggregatedSignature::<T>::get(),
			)
			.map_err(|_| Error::<T>::VerificationFailed)?;

			LatestRound::<T>::set(
				latest_round.saturating_add(T::SignatureToBlockRatio::get() as u64),
			);

			AggregatedSignature::<T>::set(Some((new_asig, new_apk)));

			Self::deposit_event(Event::<T>::SignatureVerificationSuccess);

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Set the beacon configuration
	fn initialize(config: BeaconConfiguration) -> Result<(), ()> {
		BeaconConfig::<T>::set(Some(config));
		Ok(())
	}
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

/// build a beacon configuration struct
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
