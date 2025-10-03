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
//! - Aggregates and verifies pulses using the [`SignatureVerifier`] trait.
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
//! The pallet relies on a [`SignatureVerifier`] implementation to aggregate and verify randomness
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
//! - `Accumulation`: Stores the latest aggregated signature for verification purposes.
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

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::pallet_prelude::*;

use frame_support::traits::Randomness;
use frame_system::pallet_prelude::BlockNumberFor;
use sp_core::H256;
use sp_idn_crypto::verifier::SignatureVerifier;
use sp_idn_traits::{
	pulse::{Dispatcher, Pulse as TPulse},
	Hashable,
};
use sp_std::fmt::Debug;

extern crate alloc;
use alloc::vec::Vec;

pub mod types;
pub mod weights;
pub use weights::*;

pub use types::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

const LOG_TARGET: &str = "pallet-randomness-beacon";

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use ark_serialize::CanonicalSerialize;
	use frame_support::ensure;
	use frame_system::pallet_prelude::*;
	use sp_consensus_randomness_beacon::types::{OpaqueSignature, RoundNumber};
	use sp_idn_crypto::{bls12_381::zero_on_g1, drand::compute_round_on_g1};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The public key type
	type PubkeyOf<T> = <<T as pallet::Config>::Pulse as TPulse>::Pubkey;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching runtime event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// A type representing the weights required by the dispatchables of this pallet.
		type WeightInfo: WeightInfo;
		/// something that knows how to aggregate and verify beacon pulses.
		type SignatureVerifier: SignatureVerifier;
		/// The number of signatures per block.
		type MaxSigsPerBlock: Get<u8>;
		/// The pulse type
		type Pulse: TPulse
			+ Encode
			+ Decode
			+ Debug
			+ Clone
			+ TypeInfo
			+ PartialEq
			+ From<Accumulation>;
		/// Something that can dispatch pulses
		type Dispatcher: Dispatcher<Self::Pulse>;
		/// The fallback randomness source
		type FallbackRandomness: Randomness<Self::Hash, BlockNumberFor<Self>>;
	}

	/// The round when we start consuming pulses
	#[pallet::storage]
	pub type BeaconConfig<T: Config> = StorageValue<_, PubkeyOf<T>, OptionQuery>;

	/// The latest observed round
	#[pallet::storage]
	pub type LatestRound<T: Config> = StorageValue<_, RoundNumber, ValueQuery>;

	/// The aggregated signature and (start, end) rounds
	#[pallet::storage]
	pub type SparseAccumulation<T: Config> = StorageValue<_, Accumulation, OptionQuery>;

	/// Whether the asig has been updated in this block.
	///
	/// This value is updated to `true` upon successful submission of an asig by a node.
	/// It is then checked at the end of each block execution in the `on_finalize` hook.
	#[pallet::storage]
	pub(super) type DidUpdate<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The beacon config has been set by a root address
		BeaconConfigSet,
		/// Signature verification succeeded for the provided rounds.
		SignatureVerificationSuccess,
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The beacon config is already set
		BeaconConfigAlreadySet,
		/// The beacon config is not set
		BeaconConfigNotSet,
		/// The pulse could not be verified
		VerificationFailed,
		/// There must be at least one signature to construct an asig
		ZeroHeightProvided,
		/// The height exceeds the maximum allowed signatures per block
		ExcessiveHeightProvided,
		/// Only one aggregated signature can be provided per block
		SignatureAlreadyVerified,
		/// A critical error occurred where serialization failed
		SerializationFailed,
		/// The round of this pulse has already happened
		StartExpired,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// A dummy `on_initialize` to return the amount of weight that `on_finalize` requires to
		/// execute.
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			// weight of `on_finalize`
			<T as pallet::Config>::WeightInfo::on_finalize()
		}

		/// At the end of block execution, the `on_finalize` hook checks that the asig was
		/// updated. Upon success, it removes the boolean value from storage. If the value resolves
		/// to `false`, then the runtime did  **not** receive any valid pulses from drand and we log an error.
		/// If the value resolves to `true`, then process subscriptions.
		///
		fn on_finalize(n: BlockNumberFor<T>) {
			if !DidUpdate::<T>::take() && BeaconConfig::<T>::get().is_some() {
				// this implies we did not ingest randomness from drand during this block
				log::error!(target: LOG_TARGET, "Failed to ingest pulses during lifetime of block {:?}", n);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Write a set of pulses to the runtime
		///
		/// * `origin`: An unsigned origin.
		/// * `asig`: An aggregated signature as bytes
		/// * `height`: The number of signatures aggregated in asig
		#[pallet::call_index(0)]
		#[pallet::weight((<T as pallet::Config>::WeightInfo::try_submit_asig(
			T::MaxSigsPerBlock::get().into())
				.saturating_add(
					T::Dispatcher::dispatch_weight()),
			DispatchClass::Operational
		))]
		#[allow(clippy::useless_conversion)]
		pub fn try_submit_asig(
			origin: OriginFor<T>,
			asig: OpaqueSignature,
			start: RoundNumber,
			end: RoundNumber,
		) -> DispatchResultWithPostInfo {
			ensure_signed(origin)?;
			// the extrinsic can only be successfully executed once per block
			ensure!(!DidUpdate::<T>::exists(), Error::<T>::SignatureAlreadyVerified);

			let pk = BeaconConfig::<T>::get().ok_or(Error::<T>::BeaconConfigNotSet)?;
			// 0 < num_sigs <= MaxSigsPerBlock
			let height = end - start;
			ensure!(height > 0, Error::<T>::ZeroHeightProvided);
			ensure!(
				height <= T::MaxSigsPerBlock::get() as u64,
				Error::<T>::ExcessiveHeightProvided
			);

			let latest_round: RoundNumber = LatestRound::<T>::get();
			// start must be greater than last known latest round
			ensure!(start >= latest_round, Error::<T>::StartExpired);

			// build the message
			let mut amsg = zero_on_g1();
			for r in start..end + 1 {
				let msg = compute_round_on_g1(r).map_err(|_| Error::<T>::SerializationFailed)?;
				amsg = (amsg + msg).into();
			}
			// convert to bytes
			let mut amsg_bytes = Vec::new();
			amsg.serialize_compressed(&mut amsg_bytes)
				.map_err(|_| Error::<T>::SerializationFailed)?;
			// verify the signature
			T::SignatureVerifier::verify(
				pk.as_ref().to_vec(),
				asig.clone().as_ref().to_vec(),
				amsg_bytes,
			)
			.map_err(|_| Error::<T>::VerificationFailed)?;

			// update storage
			LatestRound::<T>::set(end + 1);
			let sacc = Accumulation::new(asig, start, end);
			SparseAccumulation::<T>::set(Some(sacc.clone()));
			DidUpdate::<T>::put(true);

			// handle vraas subs
			let runtime_pulse = T::Pulse::from(sacc);
			T::Dispatcher::dispatch(runtime_pulse);

			// events
			Self::deposit_event(Event::<T>::SignatureVerificationSuccess);
			// successful verification is beneficial to the network, so we do not charge when the
			// signature is correct
			Ok(Pays::No.into())
		}

		/// Set the genesis round exactly once if you are root
		///
		/// * `origin`: A root origin
		/// * `config`: The randomness beacon configuration (genesis round and public key).
		#[pallet::call_index(1)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::set_beacon_config())]
		#[allow(clippy::useless_conversion)]
		pub fn set_beacon_config(
			origin: OriginFor<T>,
			pk: PubkeyOf<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			BeaconConfig::<T>::set(Some(pk));
			Self::deposit_event(Event::<T>::BeaconConfigSet);
			Ok(Pays::No.into())
		}
	}
}

impl<T: Config> Randomness<T::Hash, BlockNumberFor<T>> for Pallet<T>
where
	T::Hash: From<H256>,
{
	fn random(subject: &[u8]) -> (T::Hash, BlockNumberFor<T>) {
		match SparseAccumulation::<T>::get() {
			Some(accumulation) => {
				// Hash the aggregated signature directly as suggested by the colleague
				let randomness_hash = accumulation.signature.hash(subject).into();
				(randomness_hash, frame_system::Pallet::<T>::block_number())
			},
			None => {
				log::warn!(
					target: LOG_TARGET,
					"Randomness requested but no sparse accumulation available. Returning fallback values."
				);
				T::FallbackRandomness::random(subject)
			},
		}
	}
}

sp_api::decl_runtime_apis! {
    pub trait ExtrinsicBuilderApi<AccountId, RuntimeCall, Signature, TxExtension, Nonce> 
	where
		AccountId: Encode + Decode,
		RuntimeCall: Encode + Decode,
		Signature: Encode + Decode,
		TxExtension : Encode + Decode,
		Nonce: Encode + Decode,
	{
        fn construct_pulse_payload(
            asig: Signature,
            start: u64,
            end: u64,
            nonce: Nonce,
        ) -> (Vec<u8>, RuntimeCall, TxExtension);
    }
}
