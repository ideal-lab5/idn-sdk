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
use sp_idn_crypto::verifier::{OpaqueAccumulation, SignatureVerifier};
use sp_idn_traits::pulse::{Dispatcher, Pulse as TPulse};

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
	use frame_support::ensure;
	use frame_system::pallet_prelude::*;
	use sp_consensus_randomness_beacon::digest::ConsensusLog;
	use sp_runtime::{generic::DigestItem, traits::Debug, Saturating};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The public key type
	type PubkeyOf<T> = <<T as pallet::Config>::Pulse as TPulse>::Pubkey;
	/// The round number type
	type RoundOf<T> = <<T as pallet::Config>::Pulse as TPulse>::Round;
	/// The beacon configuration type
	type BeaconConfigurationOf<T> = BeaconConfiguration<PubkeyOf<T>, RoundOf<T>>;

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
		/// The number of historical missed blocks that we store.
		/// Once the limit is reached, historical missed fblocks are pruned as a FIFO queue.
		type MissedBlocksHistoryDepth: Get<u32>;
		/// The pulse type
		type Pulse: TPulse + Encode + Decode + Debug + Clone + TypeInfo + PartialEq;
		// /// Something that can dispatch pulses
		type Dispatcher: Dispatcher<Self::Pulse, DispatchResult>;
		// #[cfg(feature = "runtime-benchmarks")]
		// type DispatcherWeightInfo: ....
	}

	/// The round when we start consuming pulses
	#[pallet::storage]
	pub type BeaconConfig<T: Config> = StorageValue<_, BeaconConfigurationOf<T>, OptionQuery>;

	/// The latest observed round
	#[pallet::storage]
	pub type LatestRound<T: Config> = StorageValue<_, RoundOf<T>, OptionQuery>;

	/// The aggregated signature and aggregated public key (identifier) of all observed pulses
	#[pallet::storage]
	pub type SparseAccumulation<T: Config> = StorageValue<_, Accumulation, OptionQuery>;

	/// The collection of blocks for which collators could not report an aggregated signature
	#[pallet::storage]
	pub type MissedBlocks<T: Config> =
		StorageValue<_, BoundedVec<BlockNumberFor<T>, T::MissedBlocksHistoryDepth>, ValueQuery>;

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
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = MakeFatalError<()>;

		const INHERENT_IDENTIFIER: [u8; 8] =
			sp_consensus_randomness_beacon::inherents::INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			if let Some(config) = BeaconConfig::<T>::get() {
				if let Ok(Some(raw_pulses)) =
					data.get_data::<Vec<Vec<u8>>>(&Self::INHERENT_IDENTIFIER)
				{
					// ignores non-deserializable messages and pulses with invalid signature lengths
					//  ignores rounds less than the genesis round
					let pulses = raw_pulses
						.iter()
						.filter_map(|rp| T::Pulse::decode(&mut rp.as_slice()).ok())
						.filter(|op| op.round().into() >= config.genesis_round.clone().into())
						.collect::<Vec<_>>();

					return Some(Call::try_submit_asig { pulses });
				}
			}

			None
		}

		fn check_inherent(call: &Self::Call, _data: &InherentData) -> Result<(), Self::Error> {
			match call {
				Call::try_submit_asig { .. } => Ok(()),
				_ => unreachable!("other calls are not inherents"),
			}
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::try_submit_asig { .. })
		}
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
		/// to `false`, then either:
		/// (1) the collator refused to provide data for the inherent
		/// (2) drand was unavailable while the block was being built
		/// in which case simply log an error.
		///
		/// ## Complexity
		/// - `O(1)`
		fn on_finalize(n: BlockNumberFor<T>) {
			if !DidUpdate::<T>::take() && BeaconConfig::<T>::get().is_some() {
				// this implies we did not ingest randomness from drand during this block
				log::error!(target: LOG_TARGET, "Failed to ingest pulses during lifetime of block {:?}", n);
				// we simply notify the runtime - we ingested nothing during this block
				MissedBlocks::<T>::mutate(|blocks| {
					// remove old missed blocks if the history depth is reached
					if blocks.len() as u32 == T::MissedBlocksHistoryDepth::get() {
						blocks.remove(0);
					}

					let _ = blocks.try_push(n).map_err(|e| {
						log::error!(target: LOG_TARGET, "Failed to update historic missed blocks for block number {:?} due to {:?}", n, e)
					});
				});
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Write a set of pulses to the runtime
		///
		/// * `origin`: An unsigned origin.
		/// * `pulses`: A list of drand pulses.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as pallet::Config>::WeightInfo::try_submit_asig(T::MaxSigsPerBlock::get().into()))]
		// + <T as >::...::dispatcher(T::MaxSigs...)  )]
		#[allow(clippy::useless_conversion)]
		pub fn try_submit_asig(
			origin: OriginFor<T>,
			pulses: Vec<T::Pulse>,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			// the extrinsic can only be successfully executed once per block
			ensure!(!DidUpdate::<T>::exists(), Error::<T>::SignatureAlreadyVerified);

			let config = BeaconConfig::<T>::get().ok_or(Error::<T>::BeaconConfigNotSet)?;
			// 0 < num_sigs <= MaxSigsPerBlock
			let height: u64 = pulses.len() as u64;
			ensure!(height > 0, Error::<T>::ZeroHeightProvided);
			ensure!(
				height <= T::MaxSigsPerBlock::get() as u64,
				Error::<T>::ExcessiveHeightProvided
			);

			let latest_round: RoundOf<T> =
				LatestRound::<T>::get().expect("The latest round must be set; qed");
			// aggregate old asig/apk with the new one and verify the aggregation
			let mut sigs: Vec<Vec<u8>> = Vec::new();
			for p in &pulses {
				let s: Vec<u8> = p.sig().as_ref().to_vec();
				sigs.push(s);
			}

			let prev_acc: Option<OpaqueAccumulation> =
				SparseAccumulation::<T>::get().map(|a| a.into());

			let acc = T::SignatureVerifier::verify(
				config.public_key.as_ref().to_vec(),
				sigs.clone(),
				latest_round.clone().into(),
				prev_acc,
			)
			.map_err(|_| Error::<T>::VerificationFailed)?;

			let new_latest_round = latest_round.saturating_add(height.into());
			LatestRound::<T>::set(Some(new_latest_round.clone()));
			// TODO handle error
			let sacc = Accumulation::try_from(acc).map_err(|_| Error::<T>::VerificationFailed)?;
			SparseAccumulation::<T>::set(Some(sacc));
			DidUpdate::<T>::put(true);

			// dispatch pulses to subscribers
			T::Dispatcher::dispatch(pulses)?;

			Self::deposit_event(Event::<T>::SignatureVerificationSuccess);
			// Insert the latest round into the header digest
			let digest_item: DigestItem = ConsensusLog::LatestRoundNumber(new_latest_round).into();
			<frame_system::Pallet<T>>::deposit_log(digest_item);
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
			config: BeaconConfigurationOf<T>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			ensure!(BeaconConfig::<T>::get().is_none(), Error::<T>::BeaconConfigAlreadySet);

			BeaconConfig::<T>::set(Some(config.clone()));

			let genesis = config.genesis_round;
			LatestRound::<T>::set(Some(genesis.clone()));
			// set the genesis round as the default digest log for the initial valid round number
			let digest_item: DigestItem =
				ConsensusLog::<RoundOf<T>>::LatestRoundNumber(genesis).into();

			<frame_system::Pallet<T>>::deposit_log(digest_item);
			Self::deposit_event(Event::<T>::BeaconConfigSet);

			Ok(Pays::No.into())
		}
	}
}
