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

//! # Randomness Beacon Aggregation and Verification Pallet
//!
//! This pallet facilitates the aggregation and verification of randomness pulses from an external
//! verifiable randomness beacon, such as [drand](https://drand.love)'s Quicknet. It enables
//! runtime access to externally sourced, cryptographically secure randomness while ensuring that
//! only properly signed pulses are accepted.
//!
//! ## Overview
//!
//! - Provides a mechanism to ingest randomness pulses from an external randomness beacon.
//! - Aggregates and verifies pulses using the [`SignatureAggregator`] trait.
//! - Ensures that the runtime only uses verified randomness for security-critical applications.
//! - Stores the latest aggregated signature to enable efficient verification within the runtime.
//!
//! This pallet is particularly useful for use cases that require externally verifiable randomness,
//! such as fair lotteries, gaming applications, and leader election mechanisms.
//!
//! ## Terminology
//!
//! - **Randomness Pulse**: A cryptographically signed value representing a random output from an
//!   external randomness beacon.
//! - **Round Number**: A sequential identifier corresponding to each randomness pulse.
//! - **Aggregated Signature**: A combined (aggregated) cryptographic signature that ensures all
//!   observed pulses originate from the trusted randomness beacon.
//!
//! ## Implementation Details
//!
//! The pallet relies on a [`SignatureAggregator`] implementation to aggregate and verify randomness
//! pulses. It maintains the latest observed rounds, validates incoming pulses, and aggregates valid
//! signatures before storing them in runtime storage. It expects a monotonically increasing
//! sequence of beacon pulses delivered in packets of size `T::SignatureToBlockRatio`, beginning at
//! the genesis round.
//!
//! To be more specific, if the randomness beacon incrementally outputs pulses A -> B -> C -> D,
//! the genesis round expects pulse A first, and the SignatureToBlockRatio is 2, then this pallet
//! would first expect the 'aggregated' pulse AB = A + B, which produces both an aggregated
//! *signature* (asig) and an aggregated *public key* (apk). Subsequently, it would expected the
//! next value to be CD = C + D. On-chain, this results in the aggregated signature, ABCD = AB + CD,
//! which we can use to prove we have observed all pulses between A and D.
//!
//! ### Storage Items
//!
//! - `BeaconConfig`: Stores the beacon configuration details.
//! - `GenesisRound`: The first round number from which randomness pulses are considered valid.
//! - `LatestRound`: Tracks the latest verified round number.
//! - `AggregatedSignature`: Stores the latest aggregated signature for verification purposes.
//!
//! ## Usage
//!
//! This pallet is designed to securely ingest verifiable randomness into the runtime.
//! It exposes an inherent provider that automatically processes randomness pulses included
//! in block production.
//!
//! ## Interface
//!
//! - **Extrinsics**
//!   - `try_submit_asig`: Submit an aggregated signature for verification. This is an unsigned
//!     extrinsic, intended to be called from the inherent.
//!
//! - **Inherent Implementation**
//!   - This pallet provides an inherent that automatically submits aggregated randomness pulses
//!     during block execution.
//!
//! Run `cargo doc --package pallet-randomness-beacon --open` to view this pallet's documentation.

// We make sure this pallet uses `no_std` for compiling to Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

extern crate alloc;

use alloc::{vec, vec::Vec};
use ark_serialize::CanonicalSerialize;
use frame_support::pallet_prelude::*;
use sc_consensus_randomness_beacon::types::OpaquePulse;

pub mod aggregator;
pub mod bls12_381;
pub mod types;
pub mod weights;
pub use weights::*;

use aggregator::{zero_on_g1, SignatureAggregator};
use types::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

/// The buffer size required to represent an element of the signature group
const SERIALIZED_SIG_SIZE: usize = 48;

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
		/// The beacon configuration for which this pallet is defined.
		type BeaconConfig: Get<BeaconConfiguration>;
		/// something that knows how to aggregate and verify beacon pulses.
		type SignatureAggregator: SignatureAggregator;
		/// The number of pulses per block.
		type SignatureToBlockRatio: Get<u8>;
	}

	/// A first round number for which a pulse was observed
	#[pallet::storage]
	pub type GenesisRound<T: Config> = StorageValue<_, RoundNumber, ValueQuery>;

	/// The latest observed round
	#[pallet::storage]
	pub type LatestRound<T: Config> = StorageValue<_, RoundNumber, ValueQuery>;

	/// The aggregated signature and aggregated public key (identifier) of all observed pulses of
	/// randomness
	#[pallet::storage]
	pub type AggregatedSignature<T: Config> = StorageValue<_, Aggregate, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The genesis round has been changed by a root address
		GenesisRoundChanged,
		/// Siganture verification succeeded for signatures associated with the given rounds.
		SignatureVerificationSuccess,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The input data could not be decoded or was empty
		InvalidInput,
		/// The pulse could not be verified
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

				let mut asig_bytes = Vec::with_capacity(SERIALIZED_SIG_SIZE);
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

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Write a set of pulses to the runtime
		///
		/// * `origin`: A None origin
		/// * `asig`: An aggregated signature
		/// * `round`: An optional genesis round number. It can only be set if the existing genesis
		///   round is 0.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::try_submit_asig())]
		pub fn try_submit_asig(
			origin: OriginFor<T>,
			asig: OpaqueSignature,
			round: Option<RoundNumber>,
		) -> DispatchResult {
			// In the future, this will expected a signed payload
			// https://github.com/ideal-lab5/idn-sdk/issues/117
			ensure_none(origin)?;
			let config = T::BeaconConfig::get();
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
