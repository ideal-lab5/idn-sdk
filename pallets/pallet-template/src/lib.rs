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

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResultWithPostInfo, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
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
		pub fn create_subscription(
			origin: OriginFor<T>,
			something: u32,
		) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			Ok(().into())
		}

		/// Pauses a subscription.
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn pause_subscription(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			Ok(().into())
		}

		/// Kills a subscription.
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn kill_subscription(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			Ok(().into())
		}

		/// Updates a subscription.
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn update_subscription(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			Ok(().into())
		}

		/// Reactivates a subscription.
		#[pallet::call_index(4)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		pub fn reactivate_subscription(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let _who = ensure_signed(origin)?;

			Ok(().into())
		}
	}
}
