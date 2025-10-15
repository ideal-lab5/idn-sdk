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

use ark_serialize::CanonicalSerialize;
use frame_support::pallet_prelude::*;

use frame_support::{
	traits::{FindAuthor, Randomness},
	BoundedSlice, BoundedVec,
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_consensus_randomness_beacon::types::OpaqueSignature;
use sp_consensus_randomness_beacon::types::{OpaquePublicKey, RoundNumber};
use sp_core::H256;
use sp_idn_crypto::verifier::SignatureVerifier;
use sp_idn_crypto::{bls12_381::zero_on_g1, drand::compute_round_on_g1};
use sp_idn_traits::{
	pulse::{Dispatcher, Pulse as TPulse},
	Hashable,
};
use sp_runtime::{traits::Verify, RuntimeAppPublic};
use sp_std::fmt::Debug;

extern crate alloc;
use alloc::{vec, vec::Vec};

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

/// The public key type
type PubkeyOf<T> = <<T as pallet::Config>::Pulse as TPulse>::Pubkey;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::ensure;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{Convert, IdentifyAccount, Verify};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_session::Config {
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

		/// Signature type that the extension of this pallet can verify.
		type Signature: Verify<Signer = Self::AccountIdentifier>
			+ Parameter
			+ Encode
			+ Decode
			+ Send
			+ Sync;

		/// The account identifier used by this pallet's signature type.
		type AccountIdentifier: IdentifyAccount<AccountId = Self::AccountId>;

		///
		type FindAuthor: FindAuthor<Self::AccountId>;
	}

	/// The beacon public key
	#[pallet::storage]
	pub type BeaconConfig<T: Config> = StorageValue<_, OpaquePublicKey, OptionQuery>;

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

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		/// The randomness beacon public key
		pub beacon_pubkey_hex: Vec<u8>,
		_phantom: core::marker::PhantomData<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_beacon_pubkey(&self.beacon_pubkey_hex)
		}
	}

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
		/// The beacon config is already set.
		BeaconConfigAlreadySet,
		/// The beacon config is not set.
		BeaconConfigNotSet,
		/// The height exceeds the maximum allowed signatures per block.
		ExcessiveHeightProvided,
		/// The caller was not a network authority.
		InvalidAuthority,
		/// The signature could not be verified (authority sig, not from the beacon).
		InvalidSignature,
		/// The Authorities vec is empty
		NoAvailableAuthorities,
		/// Only one aggregated signature can be provided per block.
		SignatureAlreadyVerified,
		/// A critical error occurred where serialization failed.
		SerializationFailed,
		/// The first round provided has already happened.
		StartExpired,
		/// The pulse could not be verified.
		VerificationFailed,
		/// There must be at least one signature to construct an asig.
		ZeroHeightProvided,
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		/// It restricts calls to `try_submit_asig` to local calls (i.e. extrinsics generated
		/// on this node) or that already in a block. This guarantees that only block authors can include
		/// unsigned equivocation reports.
		fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// reject if not from local
			if !matches!(source, TransactionSource::Local | TransactionSource::InBlock) {
				return InvalidTransaction::Call.into();
			}

			match call {
				Call::try_submit_asig { asig, start, end, .. } => {
					// invalidate early if start < latest_round since it will fail anyway
					let latest_round = LatestRound::<T>::get();
					if *start < latest_round {
						return InvalidTransaction::Call.into();
					}

					ValidTransaction::with_tag_prefix("RandomnessBeacon")
						// prioritize execution
						.priority(TransactionPriority::MAX)
						// unique tag per call
						.and_provides(vec![(b"beacon_pulse", asig, start, end).encode()])
						// how long?
						.longevity(5)
						// do not propagate to other nodes
						.propagate(false)
						.build()
				},
				_ => InvalidTransaction::Call.into(),
			}
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
		/// to `false`, then the runtime did  **not** receive any valid pulses from drand and we log
		/// an error. If the value resolves to `true`, then process subscriptions.
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
		/// * `start`: The round number where sig aggregation began
		/// * `end`: The round number where sig aggregation stopped
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
			signature: T::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;

			// verify that that the block author signed this tx
			let payload = (asig.to_vec().clone(), start, end).encode();
			Self::verify_signature(payload, signature)?;

			// the extrinsic can only be successfully executed once per block
			ensure!(!DidUpdate::<T>::exists(), Error::<T>::SignatureAlreadyVerified);

			let pk = BeaconConfig::<T>::get().ok_or(Error::<T>::BeaconConfigNotSet)?;
			// 0 < num_sigs <= MaxSigsPerBlock
			let height = end.saturating_sub(start);
			// must allow start = end if start > 0
			if height == 0 {
				// we allow height == 0 iff there is a single pulse being ingested (start == end)
				ensure!(start == end, Error::<T>::ZeroHeightProvided);
			}
			ensure!(
				height <= T::MaxSigsPerBlock::get() as u64,
				Error::<T>::ExcessiveHeightProvided
			);

			let latest_round: RoundNumber = LatestRound::<T>::get();
			// we accept any *new* pulses, a somewhat weaker condition than expecting
			// a monotonically increasing sequence of pulses.
			// This will be strengthened in: https://github.com/ideal-lab5/idn-sdk/issues/392
			if latest_round > 0 {
				ensure!(start >= latest_round, Error::<T>::StartExpired);
			}

			Self::verify_beacon_signature(pk, asig, start, end)?;

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
			Ok(())
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
			pk: OpaquePublicKey,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			BeaconConfig::<T>::set(Some(pk));
			Self::deposit_event(Event::<T>::BeaconConfigSet);
			Ok(Pays::No.into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Initial the beacon public key.
	///
	/// The storage will be applied immediately.
	///
	/// The beacon_pubkey_hex must be 96  bytes.
	pub fn initialize_beacon_pubkey(beacon_pubkey_hex: &[u8]) {
		if !beacon_pubkey_hex.is_empty() {
			assert!(<BeaconConfig<T>>::get().is_none(), "Beacon config is already initialized!");
			let bytes = hex::decode(beacon_pubkey_hex)
				.expect("The beacon public key must be hex-encoded and 96 bytes.");
			let bpk: OpaquePublicKey =
				bytes.try_into().expect("The beacon public key must be exactly 96 bytes.");
			BeaconConfig::<T>::set(Some(bpk));
		}
	}

	/// Verify that asig is a BLS signature on the message $\sum_{i = start}^{end} Sha256(i)$
	///
	/// *`pk`: The beacon public key
	/// * `asig`: The signature to verify
	/// * `start`: The first round to use when constructing the message
	/// * `end`: The last round to use when constructing the message
	///
	fn verify_beacon_signature(
		pk: OpaquePublicKey,
		asig: OpaqueSignature,
		start: RoundNumber,
		end: RoundNumber,
	) -> DispatchResult {
		// build the message
		let mut amsg = zero_on_g1();
		for r in start..=end {
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
		Ok(())
	}

	/// Verify that the `signature` is a valid signature on the `payload` 
	/// under the current block author's public key
	fn verify_signature(payload: Vec<u8>, signature: T::Signature) -> DispatchResult {
		let digest = <frame_system::Pallet<T>>::digest();
		let pre_runtime_digests = digest.logs.iter().filter_map(|d| d.as_pre_runtime());
		let author_id = T::FindAuthor::find_author(pre_runtime_digests)
			.ok_or(DispatchError::Other("No block author found"))?;
		// verify sig
		ensure!(signature.verify(&payload[..], &author_id), Error::<T>::InvalidSignature);
		Ok(())
	}

	/// get the latest round from the runtime
	pub fn latest_round() -> RoundNumber {
		LatestRound::<T>::get()
	}

	/// get the max number of pulses we can hold in a block
	pub fn max_rounds() -> u8 {
		T::MaxSigsPerBlock::get()
	}
}

impl<T: Config> Randomness<T::Hash, BlockNumberFor<T>> for Pallet<T>
where
	T::Hash: From<H256>,
{
	fn random(subject: &[u8]) -> (T::Hash, BlockNumberFor<T>) {
		match SparseAccumulation::<T>::get() {
			Some(accumulation) => {
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
	pub trait RandomnessBeaconApi {
		/// Get the latest round finalized on-chain
		fn latest_round() -> sp_consensus_randomness_beacon::types::RoundNumber;
		/// Get the maximum number of outputs from the beacon we can verify simultaneously onchain
		fn max_rounds() -> u8;
		/// Build an unsigned extrinsic with signed payload
		fn build_extrinsic(
			asig: Vec<u8>,
			start: u64,
			end: u64,
			signature: Vec<u8>,
		) -> Block::Extrinsic;
	}
}
