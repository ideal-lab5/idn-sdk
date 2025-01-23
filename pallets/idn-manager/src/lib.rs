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

//! # IDN Manager Pallet

#![cfg_attr(not(feature = "std"), no_std)]

pub mod traits;

use crate::traits::FeeCalculator;
use codec::{Decode, Encode};
// use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	storage::StorageMap,
	traits::{
		fungible::{hold, Inspect, Mutate},
		Get,
	},
	weights::Weight,
};
use frame_system::{
	ensure_signed,
	pallet_prelude::{BlockNumberFor, OriginFor},
};
use scale_info::TypeInfo;
use sp_core::{MaxEncodedLen, H256};
use sp_io::hashing::blake2_256;
use sp_std::vec::Vec;
use xcm::{latest::prelude::*, opaque::v3::MultiLocation};

pub use pallet::*;

pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

// pub type BlockNumberFor<T> = <T as frame_system::Config>::BlockNumber;

#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct Subscription<AccountId, BlockNumber> {
	details: SubscriptionDetails<AccountId, BlockNumber>,
	status: SubscriptionStatus,
}
pub struct SubscriptionDetails<AccountId, BlockNumber> {
	subscriber: AccountId,
	para_id: ParaId,
	start_block: BlockNumber,
	end_block: BlockNumber,
	frequency: BlockNumber,
	target: MultiLocation,
	filter: Option<Filter>, // ?
}

type ParaId = u32; // todo import type

impl<AccountId, BlockNumber> Subscription<AccountId, BlockNumber> {
	pub fn id(&self) -> SubscriptionId {
		// Encode the struct using SCALE codec
		let details = self.details.encode();
		// Hash the encoded bytes using blake2_256
		H256::from_slice(&blake2_256(&details))
	}
}

type SubscriptionId = H256;
type Filter = Vec<u8>;

#[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen, Debug)]
pub enum SubscriptionStatus {
	Inactive,
	Active,
	Paused,
	Expired,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcm::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency type for handling subscription payments
		type Currency: Inspect<Self::AccountId> + Mutate<Self::AccountId>;

		/// Maximum subscription duration
		#[pallet::constant]
		type MaxSubscriptionDuration: Get<BlockNumberFor<Self>>;

		/// Fee calculator implementation
		type FeeCalculator: FeeCalculator<BalanceOf<Self>>;

		/// The type for the randomness
		type Rnd;
	}

	#[pallet::storage]
	pub type Subscriptions<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SubscriptionId,
		Subscription<T::AccountId, BlockNumberFor<T>>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new subscription was created (includes single-block subscriptions)
		SubscriptionCreated {
			para_id: ParaId,
			subscriber: T::AccountId,
			duration: BlockNumberFor<T>,
			target: MultiLocation,
		},
		/// A subscription was deactivated
		SubscriptionDeactivated { sub_id: SubscriptionId },
		///
		SubscriptionPaused { sub_id: SubscriptionId },
		/// Notify of low budget
		SubscriptionLowBudget { sub_id: SubscriptionId, budget: BalanceOf<T> },
		/// Randomness was successfully distributed
		RandomnessDistributed { sub_id: SubscriptionId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// An active subscription already exists
		ActiveSubscriptionExists,
		/// The subscription duration is invalid
		InvalidSubscriptionDuration,
		/// The parachain ID is invalid
		InvalidParaId,
		/// Insufficient balance for subscription
		InsufficientBalance,
		/// XCM execution failed
		XcmExecutionFailed,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut reads = 0u64;
			let mut writes = 0u64;

			// Filter active subscriptions that have expired
			for (sub_id) in Subscriptions::<T>::iter()
				.filter(|(_, sub)| sub.status == SubscriptionStatus::Active && sub.end_block <= n)
			{
				reads += 1;

				// Use update instead of insert for atomic operation
				Subscriptions::<T>::try_mutate(sub_id, |maybe_sub| -> DispatchResult {
					if let Some(sub) = maybe_sub {
						sub.status = SubscriptionStatus::Expired;
						writes += 1;
						Self::deposit_event(Event::SubscriptionDeactivated { sub_id });
					}
					Ok(())
				})?;
			}

			T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a subscription for one or multiple blocks
		// To keep this pallet low level `request_single_randomness` is not implemented and rather
		// let it up to the pallet on the client side to handle the single block subscription
		#[pallet::call_index(0)]
		#[pallet::weight(T::BaseWeight::get())]
		pub fn create_subscription(
			origin: OriginFor<T>,
			para_id: ParaId,
			duration: BlockNumberFor<T>,
			target: MultiLocation,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;
			// Calculate and hold the subscription fee
			let fee = T::FeeCalculator::calculate_subscription_fee(duration);

			Self::create_subscription_internal(subscriber, para_id, duration, target, fee)
		}

		/// Calculate te subscription fee for a given duration
		#[pallet::call_index(1)]
		#[pallet::weight(T::BaseWeight::get())]
		pub fn calculate_subscription_fee(
			origin: OriginFor<T>,
			duration: BlockNumberFor<T>,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;
			let fee = T::FeeCalculator::calculate_subscription_fee(duration);

			Ok(fee)
		}
	}

	// /// Distribute randomness to subscribers
	// /// Returns a weight based on the number of storage reads and writes performed
	// fn distribute(rnd: T::Rnd) -> Result<Weight, DispatchError> {

	// }

	// impl<T: Config, R: T::Rnd, O: Result<Weight, DispatchError>> idn_traits::rand::Consumer<R, O>
	// for Pallet<T> { 	fn consume(rnd: R) -> O {
	// 		let mut reads = 0u64;
	// 		let mut writes = 0u64;

	// 		let randomness = T::RandomnessSource::get_randomness();

	// 		// Filter for active subscriptions only
	// 		for (sub_id, sub) in
	// 			Subscriptions::<T>::iter().filter(|(_, sub)| sub.status == SubscriptionStatus::Active)
	// 		{
	// 			reads += 1;

	// 			if let Ok(msg) = Self::construct_randomness_xcm(sub.details.target, randomness.clone())
	// 			{
	// 				let _ = pallet_xcm::Pallet::<T>::send_xcm(Here, sub.details.target, msg);
	// 				writes += 1;

	// 				Self::deposit_event(Event::RandomnessDistributed { sub_id });
	// 			}
	// 		}

	// 		T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
	// 	}
	// }
}

impl<T: Config> Pallet<T> {
	/// Distribute randomness to subscribers
	/// Returns a weight based on the number of storage reads and writes performed
	fn distribute(rnd: T::Rnd) -> Result<Weight, DispatchError> {
		let mut reads = 0u64;
		let mut writes = 0u64;

		let randomness = T::RandomnessSource::get_randomness();

		// Filter for active subscriptions only
		for (sub_id, sub) in
			Subscriptions::<T>::iter().filter(|(_, sub)| sub.status == SubscriptionStatus::Active)
		{
			reads += 1;

			if let Ok(msg) = Self::construct_randomness_xcm(sub.details.target, randomness.clone())
			{
				let _ = pallet_xcm::Pallet::<T>::send_xcm(Here, sub.details.target, msg);
				writes += 1;

				Self::deposit_event(Event::RandomnessDistributed { sub_id });
			}
		}

		T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
	}

	/// Check if there's an active subscription for the given para_id
	fn has_active_subscription(para_id: &ParaId) -> bool {
		Subscriptions::<T>::get(para_id)
			.map(|sub| sub.status == SubscriptionStatus::Active)
			.unwrap_or(false)
	}

	/// Internal function to handle subscription creation
	fn create_subscription_internal(
		subscriber: T::AccountId,
		para_id: ParaId,
		duration: BlockNumberFor<T>,
		target: MultiLocation,
		fee: Balance,
	) -> DispatchResult {
		ensure!(
			duration <= T::MaxSubscriptionDuration::get(),
			Error::<T>::InvalidSubscriptionDuration
		);

		ensure!(!Self::has_active_subscription(&para_id), Error::<T>::ActiveSubscriptionExists);

		T::Currency::hold(&HoldReason::fee(), &subscriber, fee)
			.map_err(|_| Error::<T>::InsufficientBalance)?;

		let current_block = frame_system::Pallet::<T>::block_number();
		let subscription = Subscription {
			subscriber: subscriber.clone(),
			para_id,
			status: SubscriptionStatus::Active,
			start_block: current_block,
			end_block: current_block + duration,
			target: target.clone(),
			filter: None,
		};

		Subscriptions::<T>::insert(para_id, subscription);

		Self::deposit_event(Event::SubscriptionCreated { para_id, subscriber, duration, target });

		Ok(())
	}

	/// Helper function to construct XCM message for randomness distribution
	fn construct_randomness_xcm(
		target: MultiLocation,
		randomness: Vec<u8>,
	) -> Result<Xcm<()>, Error<T>> {
		Ok(Xcm(vec![
			WithdrawAsset((Here, 0u128).into()),
			BuyExecution { fees: (Here, 0u128).into(), weight_limit: Unlimited },
			Transact {
				origin_kind: OriginKind::Native,
				require_weight_at_most: Weight::from_ref_time(1_000_000),
				call: randomness.into(),
			},
			RefundSurplus,
			DepositAsset { assets: All.into(), beneficiary: target.into() },
		]))
	}
}

impl<T: Config, R: T::Rnd, O: Result<Weight, DispatchError>> idn_traits::rand::Consumer<R, O>
	for Pallet<T>
{
	fn consume(rnd: R) -> O {
		// look for active subscriptions
		// deliver rand to them
		Pallet::<T>::distribute(rnd);
		Ok(())
	}
}
