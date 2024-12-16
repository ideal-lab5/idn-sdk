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

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use cumulus_primitives_core::ParaId;
	use frame_support::{
		pallet_prelude::*,
		storage::StorageMap,
		traits::{
			fungible::{Hold, Inspect, Mutate},
			Get,
		},
		weights::Weight,
	};
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;
	use xcm::latest::prelude::*;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

	pub type BlockNumberFor<T> = <T as frame_system::Config>::BlockNumber;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen, Debug)]
	pub enum SubscriptionStatus {
		Active,
		Inactive,
		Expired,
	}

	#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
	pub struct Subscription<AccountId, BlockNumber> {
		subscriber: AccountId,
		para_id: ParaId,
		status: SubscriptionStatus,
		start_block: BlockNumber,
		end_block: BlockNumber,
		target: MultiLocation,
	}

	/// Trait for fee calculation implementations
	pub trait FeeCalculator<Balance> {
		/// Calculate the total fee for a subscription of given duration
		fn calculate_subscription_fee(duration: BlockNumberFor<Self>) -> Balance;
	}

	pub trait RandomnessSource {
		fn get_randomness() -> Vec<u8>;
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_xcm::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency type for handling subscription payments
		type Currency: Inspect<Self::AccountId> + Mutate<Self::AccountId> + Hold<Self::AccountId>;

		/// Maximum subscription duration
		#[pallet::constant]
		type MaxSubscriptionDuration: Get<BlockNumberFor<Self>>;

		/// Fee calculator implementation
		type FeeCalculator: FeeCalculator<BalanceOf<Self>>;

		/// Discount rate per block for longer subscriptions (in percentage basis points)
		/// For example, 10 means 0.1% discount per block
		#[pallet::constant]
		type DiscountRatePerBlock: Get<u32>;

		/// Base weight for subscription operations
		#[pallet::constant]
		type BaseWeight: Get<Weight>;

		/// The pallet that provides randomness
		type RandomnessSource: RandomnessSource;
	}

	#[pallet::storage]
	pub type Subscriptions<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		ParaId,
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
		SubscriptionDeactivated { para_id: ParaId },
		/// Randomness was successfully distributed
		RandomnessDistributed { para_id: ParaId, target_block: BlockNumberFor<T> },
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
			for (para_id, subscription) in Subscriptions::<T>::iter()
				.filter(|(_, sub)| sub.status == SubscriptionStatus::Active && sub.end_block <= n)
			{
				reads += 1;

				// Use update instead of insert for atomic operation
				Subscriptions::<T>::try_mutate(para_id, |maybe_sub| -> DispatchResult {
					if let Some(sub) = maybe_sub {
						sub.status = SubscriptionStatus::Expired;
						writes += 1;
						Self::deposit_event(Event::SubscriptionDeactivated { para_id });
					}
					Ok(())
				})?;
			}

			T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
		}

		fn on_finalize(_n: BlockNumberFor<T>) -> Weight {
			let (reads, writes) = Self::distribute_randomness();
			T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a subscription for multiple blocks
		#[pallet::call_index(0)]
		#[pallet::weight(T::BaseWeight::get())]
		pub fn create_subscription(
			origin: OriginFor<T>,
			para_id: ParaId,
			duration: BlockNumberFor<T>,
			target: MultiLocation,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;

			Self::create_subscription_internal(subscriber, para_id, duration, target)
		}

		/// Create a single-block subscription
		#[pallet::call_index(1)]
		#[pallet::weight(T::BaseWeight::get())]
		pub fn request_single_randomness(
			origin: OriginFor<T>,
			para_id: ParaId,
			target: MultiLocation,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;

			Self::create_subscription_internal(subscriber, para_id, One::one(), target)
		}
	}

	impl<T: Config> Pallet<T> {
		/// Calculate the subscription fee for a given duration, applying duration-based discounts
		fn calculate_subscription_fee(duration: BlockNumberFor<T>) -> BalanceOf<T> {
			let base_fee = T::BaseSubscriptionFeePerBlock::get();
			let duration_num: u32 = duration.saturated_into();

			// Calculate discount percentage (in basis points)
			// The longer the duration, the higher the discount
			let discount = duration_num.saturating_mul(T::DiscountRatePerBlock::get());

			// Convert basis points to a multiplier (10000 basis points = 100%)
			let discount_multiplier = (10000u32.saturating_sub(discount)) as u128;

			// Apply discount to base fee
			let discounted_fee_per_block = base_fee
				.saturating_mul(discount_multiplier.into())
				.saturating_div(10000u32.into());

			// Calculate total fee for the duration
			discounted_fee_per_block.saturating_mul(duration.saturated_into())
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
		) -> DispatchResult {
			ensure!(
				duration <= T::MaxSubscriptionDuration::get(),
				Error::<T>::InvalidSubscriptionDuration
			);

			ensure!(!Self::has_active_subscription(&para_id), Error::<T>::ActiveSubscriptionExists);

			// Calculate and hold the subscription fee
			let fee = T::FeeCalculator::calculate_subscription_fee(duration);

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
			};

			Subscriptions::<T>::insert(para_id, subscription);

			Self::deposit_event(Event::SubscriptionCreated {
				para_id,
				subscriber,
				duration,
				target,
			});

			Ok(())
		}

		/// Internal function to distribute randomness to subscribers
		/// Returns the number of storage reads and writes performed
		fn distribute_randomness() -> (u64, u64) {
			let mut reads = 0u64;
			let mut writes = 0u64;

			let current_block = frame_system::Pallet::<T>::block_number();
			let randomness = T::RandomnessSource::get_randomness();

			// Filter for active subscriptions only
			for (para_id, subscription) in Subscriptions::<T>::iter()
				.filter(|(_, sub)| sub.status == SubscriptionStatus::Active)
			{
				reads += 1;

				if let Ok(msg) =
					Self::construct_randomness_xcm(subscription.target, randomness.clone())
				{
					let _ = pallet_xcm::Pallet::<T>::send_xcm(Here, subscription.target, msg);
					writes += 1;

					Self::deposit_event(Event::RandomnessDistributed {
						para_id,
						target_block: current_block,
					});
				}
			}

			(reads, writes)
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
}
