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

use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
use frame_system::pallet_prelude::*;
use pallet::*;
use scale_info::prelude::fmt::Debug;
use sp_idn_traits::pulse::Pulse;

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
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The type for the randomness pulse
		type Pulse: Pulse + Encode + Debug + Decode + Clone + TypeInfo + PartialEq;
		type SubscriptionId: Encode + Debug + Decode + Clone + TypeInfo + PartialEq;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A random value was received.
		RandomnessReceived,
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
			_pulse: T::Pulse,
			_sub_id: T::SubscriptionId,
		) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Creates a subscription.
	pub fn create_subscription() -> Result<(), ()> {
		todo!()
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
}
