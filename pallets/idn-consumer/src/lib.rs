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

//! Pallet IDN Consumer

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	pallet_prelude::{
		Decode, DispatchResult, Encode, EnsureOrigin, Get, IsType, Pays, TypeInfo, Weight,
	},
};
use frame_system::pallet_prelude::{BlockNumberFor, OriginFor};
use pallet::*;
use scale_info::prelude::fmt::Debug;
use sp_arithmetic::traits::Unsigned;
use sp_idn_traits::pulse::{Consumer, Pulse};
use sp_idn_types::SubscriptionMetadata;
use xcm::v5::Location;

/// The metadata type used in the pallet, represented as a bounded vector of bytes.
type MetadataOf<T> = SubscriptionMetadata<<T as Config>::MaxMetadataLen>;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The type for the randomness pulse
		type Pulse: Pulse + Encode + Debug + Decode + Clone + TypeInfo + PartialEq;
		/// The type for the subscription ID
		type SubscriptionId: Encode + Debug + Decode + Clone + TypeInfo + PartialEq;
		type Consumer: Consumer<Self::Pulse, Self::SubscriptionId, DispatchResult>;
		// pub SiblingIDNParaId: u32 = IDN_PARACHAIN_ID;
		// pub SiblingIDN: Location = Location::new(1, Parachain(SiblingIDNParaId::get()));
		type SiblingIdnLocation: Get<Location>;
		// pub IDNOrigin: EnsureXcm<Equals<Self::SiblingIDNLocation>>
		type IdnOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Location>;
		type Credits: Unsigned + Encode;
		/// Maximum metadata size
		type MaxMetadataLen: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A random value was successfully consumed.
		RandomnessConsumed { round: <T::Pulse as Pulse>::Round, sub_id: T::SubscriptionId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a subscription.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn consume(
			origin: OriginFor<T>,
			pulse: T::Pulse,
			sub_id: T::SubscriptionId,
		) -> DispatchResultWithPostInfo {
			// ensure origin is coming from IDN
			let _ = T::IdnOrigin::ensure_origin(origin)?;

			let round = pulse.round().clone();

			T::Consumer::consume(pulse, sub_id.clone())?;

			Self::deposit_event(Event::RandomnessConsumed { round, sub_id });

			Ok(Pays::No.into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Creates a subscription.
	pub fn create_subscription(
		// Number of random values to receive
		credits: T::Credits,
		// Distribution interval for pulses
		frequency: BlockNumberFor<T>,
		// Bounded vector for additional data
		metadata: Option<MetadataOf<T>>,
		// Optional Pulse Filter
		// pulse_filter: Option<PulseFilter>,
		// // Optional Subscription Id, if None, a new one will be generated
		// sub_id: Option<SubscriptionId>,
	) -> Result<(), ()> {
		todo!()
		// let msg = Self::construct_xcm_msg(sub.details.call_index, &pulse, &sub_id);
		// let versioned_target: Box<VersionedLocation> =
		// 	Box::new(sub.details.target.clone().into());
		// let versioned_msg: Box<VersionedXcm<()>> =
		// 	Box::new(xcm::VersionedXcm::V5(msg.into()));
		// let origin = frame_system::RawOrigin::Signed(Self::pallet_account_id());

		// // Make sure credits are consumed before sending the XCM message
		// let consume_credits = T::FeesManager::get_consume_credits(&sub);
		// Self::collect_fees(&sub, T::FeesManager::get_consume_credits(&sub))?;

		// // Update subscription with consumed credits and last_delivered block number
		// sub.credits_left = sub.credits_left.saturating_sub(consume_credits);
		// sub.last_delivered = Some(current_block);

		// // Store the updated subscription
		// Subscriptions::<T>::insert(sub_id, &sub);

		// // Send the XCM message
		// T::Xcm::send(origin.into(), versioned_target, versioned_msg)?;
	}

	/// Pauses a subscription.
	pub fn pause_subscription() -> Result<(), ()> {
		todo!()
	}

	/// Kills a subscription.
	pub fn kill_subscription() -> Result<(), ()> {
		todo!()
	}

	/// Updates a subscription.
	pub fn update_subscription() -> Result<(), ()> {
		todo!()
	}

	/// Reactivates a subscription.
	pub fn reactivate_subscription() -> Result<(), ()> {
		todo!()
	}

	// fn construct_xcm_msg(
	// 	call_index: CallIndex,
	// 	pulse: &T::Pulse,
	// 	sub_id: &T::SubscriptionId,
	// ) -> Xcm<()> {
	// 	Xcm(vec![
	// 		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
	// 		Transact {
	// 			origin_kind: OriginKind::Xcm,
	// 			fallback_max_weight: None,
	// 			// Create a tuple of call_index and pulse, encode it using SCALE codec, then convert
	// 			// to Vec<u8> The encoded data will be used by the receiving chain to:
	// 			// 1. Find the target pallet using first byte of call_index
	// 			// 2. Find the target function using second byte of call_index
	// 			// 3. Pass the parameters to that function
	// 			call: (call_index, pulse, sub_id).encode().into(),
	// 		},
	// 		ExpectTransactStatus(MaybeErrorCode::Success),
	// 	])
	// }
}
