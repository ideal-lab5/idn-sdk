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

//! # IDN Manager Pallet
//!
//! This pallet manages subscriptions for random value distribution.
//!
//! ## Subscription State Lifecycle
//! ```mermaid
//! stateDiagram-v2
//!     [*] --> Active
//!     Active --> Paused
//!     Paused --> Active
//!     Paused --> [*]
//!     Active --> [*]
//! ```
//! ### States
//!
//! 1. Active (Default)
//!    - Randomness is actively distributed
//!    - IDN delivers random values according to subscription parameters
//!
//! 2. Paused
//!    - Randomness distribution temporarily suspended
//!    - Subscription remains in storage
//!    - Can be reactivated
//!
//! ### Termination
//! Subscription ends when:
//! - All allocated random values are consumed
//! - Manually killed by the original caller
//!
//! Upon termination:
//! - Removed from storage
//! - Storage deposit returned
//! - Unused credits refunded to the origin

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

pub mod impls;
pub mod traits;
pub mod weights;

use crate::traits::FeesError;
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::{
	pallet_prelude::{
		ensure, Blake2_128Concat, DispatchError, DispatchResult, Hooks, IsType, OptionQuery,
		StorageMap, Zero,
	},
	sp_runtime::traits::AccountIdConversion,
	traits::{
		fungible::{hold::Mutate as HoldMutate, Inspect},
		tokens::Precision,
		Get,
	},
	BoundedVec,
};
use frame_system::{
	ensure_signed,
	pallet_prelude::{BlockNumberFor, OriginFor},
};
use scale_info::TypeInfo;
use sp_arithmetic::traits::Unsigned;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::{traits::One, Saturating};
use sp_std::fmt::Debug;
use traits::{
	BalanceDirection, DepositCalculator, DiffBalance, FeesManager,
	Subscription as SubscriptionTrait,
};
use xcm::{
	v5::{prelude::*, Location},
	VersionedLocation, VersionedXcm,
};
use xcm_builder::SendController;

pub use pallet::*;
pub use weights::WeightInfo;

pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

pub type MetadataOf<T> = BoundedVec<u8, <T as Config>::SubMetadataLen>;

pub type SubscriptionOf<T> =
	Subscription<<T as frame_system::Config>::AccountId, BlockNumberFor<T>, MetadataOf<T>>;

#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct Subscription<AccountId, BlockNumber: Unsigned, Metadata> {
	details: SubscriptionDetails<AccountId, Metadata>,
	// Number of random values left to distribute
	credits_left: BlockNumber,
	state: SubscriptionState,
	created_at: BlockNumber,
	updated_at: BlockNumber,
	credits: BlockNumber,
	frequency: BlockNumber,
}

// TODO: details should be immutable, they are what make the subscription unique
// https://github.com/ideal-lab5/idn-sdk/issues/114
#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct SubscriptionDetails<AccountId, Metadata> {
	subscriber: AccountId,
	target: Location,
	metadata: Metadata,
}

impl<AccountId, BlockNumber, Metadata> Subscription<AccountId, BlockNumber, Metadata>
where
	AccountId: Encode,
	BlockNumber: Encode + Copy + Unsigned,
	Metadata: Encode + Clone,
{
	pub fn id(&self) -> SubscriptionId {
		let id_tuple = (
			self.created_at,
			&self.details.subscriber,
			self.details.target.clone(),
			self.details.metadata.clone(),
		);
		// Encode the tuple using SCALE codec.
		let encoded = id_tuple.encode();
		// Hash the encoded bytes using blake2_256.
		H256::from_slice(&blake2_256(&encoded))
	}
}

type SubscriptionId = H256;

#[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen, Debug)]
pub enum SubscriptionState {
	Active,
	Paused,
}

#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub enum Call {
	#[codec(index = 1)]
	DistributeRnd,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency type for handling subscription payments
		type Currency: Inspect<<Self as frame_system::pallet::Config>::AccountId>
			+ HoldMutate<
				<Self as frame_system::pallet::Config>::AccountId,
				Reason = Self::RuntimeHoldReason,
			>;

		/// Overarching hold reason.
		type RuntimeHoldReason: From<HoldReason>;

		/// Fees calculator implementation
		type FeesManager: FeesManager<
			BalanceOf<Self>,
			BlockNumberFor<Self>,
			SubscriptionOf<Self>,
			DispatchError,
			<Self as frame_system::pallet::Config>::AccountId,
		>;

		/// Storage deposit calculator implementation
		type DepositCalculator: DepositCalculator<BalanceOf<Self>, SubscriptionOf<Self>>;

		/// The type for the randomness
		type Rnd;

		// The weight information for this pallet.
		type WeightInfo: WeightInfo;

		/// A type that exposes XCM APIs, allowing contracts to interact with other parachains, and
		/// execute XCM programs.
		type Xcm: xcm_builder::Controller<
			OriginFor<Self>,
			<Self as frame_system::Config>::RuntimeCall,
			BlockNumberFor<Self>,
		>;

		/// The IDN Manager pallet id.
		#[pallet::constant]
		type PalletId: Get<frame_support::PalletId>;

		/// Maximum metadata size
		type SubMetadataLen: Get<u32> + TypeInfo;
	}

	#[pallet::storage]
	pub type Subscriptions<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SubscriptionId,
		Subscription<T::AccountId, BlockNumberFor<T>, MetadataOf<T>>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new subscription was created (includes single-block subscriptions)
		SubscriptionCreated { sub_id: SubscriptionId },
		/// A subscription has finished
		SubscriptionRemoved { sub_id: SubscriptionId },
		/// A subscription was paused
		SubscriptionPaused { sub_id: SubscriptionId },
		/// A subscription was updated
		SubscriptionUpdated { sub_id: SubscriptionId },
		/// A subscription was reactivated
		SubscriptionReactivated { sub_id: SubscriptionId },
		/// Randomness was successfully distributed
		RandomnessDistributed { sub_id: SubscriptionId },
		/// Fees collected
		FeesCollected { sub_id: SubscriptionId, fees: BalanceOf<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// A Subscription already exists
		SubscriptionAlreadyExists,
		/// A Subscription does not exist
		SubscriptionDoesNotExist,
		/// Subscription is already active
		SubscriptionAlreadyActive,
		/// Subscription is already paused
		SubscriptionAlreadyPaused,
		/// The origin isn't the subscriber
		NotSubscriber,
	}

	/// A reason for the IDN Manager Pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		/// The IDN Manager Pallet holds balance for future charges.
		#[codec(index = 0)]
		Fees,
		/// Storage deposit
		#[codec(index = 1)]
		StorageDeposit,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: BlockNumberFor<T>) {
			// Look for subscriptions that should be finished
			for (sub_id, sub) in
				Subscriptions::<T>::iter().filter(|(_, sub)| sub.credits_left == Zero::zero())
			{
				// finish the subscription
				let _ = Self::finish_subscription(&sub, sub_id);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a subscription for one or multiple blocks
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_subscription())]
		pub fn create_subscription(
			origin: OriginFor<T>,
			// Number of random values to receive
			credits: BlockNumberFor<T>,
			// XCM multilocation for random value delivery
			target: Location,
			// Distribution interval for random values
			frequency: BlockNumberFor<T>,
			// Bounded vector for additional data
			metadata: Option<MetadataOf<T>>,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;
			Self::create_subscription_internal(subscriber, credits, target, frequency, metadata)
		}

		/// Temporarily halts randomness distribution
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::pause_subscription())]
		pub fn pause_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			sub_id: SubscriptionId,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;
			Subscriptions::<T>::try_mutate(sub_id, |maybe_sub| {
				let sub = maybe_sub.as_mut().ok_or(Error::<T>::SubscriptionDoesNotExist)?;
				ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);
				ensure!(
					sub.state != SubscriptionState::Paused,
					Error::<T>::SubscriptionAlreadyPaused
				);
				sub.state = SubscriptionState::Paused;
				Self::deposit_event(Event::SubscriptionPaused { sub_id });
				Ok(())
			})
		}

		/// Ends the subscription before its natural conclusion
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::kill_subscription())]
		pub fn kill_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			sub_id: SubscriptionId,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;

			let sub =
				Subscriptions::<T>::get(sub_id).ok_or(Error::<T>::SubscriptionDoesNotExist)?;
			ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);

			Self::finish_subscription(&sub, sub_id)
		}

		/// Updates a subscription
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::update_subscription())]
		pub fn update_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			sub_id: SubscriptionId,
			// New number of random values
			credits: BlockNumberFor<T>,
			// New distribution interval
			frequency: BlockNumberFor<T>,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;
			Subscriptions::<T>::try_mutate(sub_id, |maybe_sub| {
				let sub = maybe_sub.as_mut().ok_or(Error::<T>::SubscriptionDoesNotExist)?;
				ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);

				let fees_diff = T::FeesManager::calculate_diff_fees(&sub.credits, &credits);
				let deposit_diff = T::DepositCalculator::calculate_diff_deposit(
					sub,
					&Subscription { credits, frequency, ..sub.clone() },
				);

				sub.credits = credits;
				sub.frequency = frequency;
				sub.updated_at = frame_system::Pallet::<T>::block_number();

				// Hold or refund diff fees
				Self::manage_diff_fees(&subscriber, &fees_diff)?;
				// Hold or refund diff deposit
				Self::manage_diff_deposit(&subscriber, &deposit_diff)?;
				Self::deposit_event(Event::SubscriptionUpdated { sub_id });
				Ok(())
			})
		}

		/// Reactivates a paused subscription
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::reactivate_subscription())]
		pub fn reactivate_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			sub_id: SubscriptionId,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;
			Subscriptions::<T>::try_mutate(sub_id, |maybe_sub| {
				let sub = maybe_sub.as_mut().ok_or(Error::<T>::SubscriptionDoesNotExist)?;
				ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);
				ensure!(
					sub.state != SubscriptionState::Active,
					Error::<T>::SubscriptionAlreadyActive
				);
				sub.state = SubscriptionState::Active;
				Self::deposit_event(Event::SubscriptionReactivated { sub_id });
				Ok(())
			})
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Finishes a subscription by removing it from storage and emitting a finish event.
	pub(crate) fn finish_subscription(
		sub: &SubscriptionOf<T>,
		sub_id: SubscriptionId,
	) -> DispatchResult {
		// fees left and deposit to refund
		let fees_diff = T::FeesManager::calculate_diff_fees(&sub.credits_left, &Zero::zero());
		let sd = T::DepositCalculator::calculate_storage_deposit(sub);

		Self::manage_diff_fees(&sub.details.subscriber, &fees_diff)?;
		Self::release_deposit(&sub.details.subscriber, sd)?;

		Self::deposit_event(Event::SubscriptionRemoved { sub_id });

		Subscriptions::<T>::remove(sub_id);
		Self::deposit_event(Event::SubscriptionRemoved { sub_id });
		Ok(())
	}

	fn pallet_account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	/// Distribute randomness to subscribers
	/// Returns a weight based on the number of storage reads and writes performed
	// TODO: finish off this as part of https://github.com/ideal-lab5/idn-sdk/issues/77
	fn distribute(rnd: T::Rnd) -> DispatchResult {
		// Filter for active subscriptions only
		for (sub_id, sub) in
			Subscriptions::<T>::iter().filter(|(_, sub)| sub.state == SubscriptionState::Active)
		{
			if let Ok(msg) = Self::construct_randomness_xcm(sub.details.target.clone(), &rnd) {
				let versioned_target: Box<VersionedLocation> =
					Box::new(sub.details.target.clone().into());
				let versioned_msg: Box<VersionedXcm<()>> =
					Box::new(xcm::VersionedXcm::V5(msg.into()));
				let origin = frame_system::RawOrigin::Signed(Self::pallet_account_id());
				if T::Xcm::send(origin.into(), versioned_target, versioned_msg).is_ok() {
					// We consume a fixed one credit per distribution
					let credits_consumed = One::one();
					Self::collect_fees(&sub, credits_consumed)?;
					Self::consume_credits(&sub_id, sub.clone(), credits_consumed);
					Self::deposit_event(Event::RandomnessDistributed { sub_id });
				}
			}
		}

		Ok(())
	}

	fn consume_credits(
		sub_id: &SubscriptionId,
		mut sub: SubscriptionOf<T>,
		credits_consumed: BlockNumberFor<T>,
	) {
		// Decrease credits_left by `credits_consumed` using saturating_sub
		sub.credits_left = sub.credits_left.saturating_sub(credits_consumed);
		// Update the subscription in storage
		Subscriptions::<T>::insert(sub_id, sub)
	}

	fn collect_fees(
		sub: &SubscriptionOf<T>,
		credits_consumed: BlockNumberFor<T>,
	) -> DispatchResult {
		let fees_to_collect = T::FeesManager::calculate_diff_fees(
			&sub.credits_left,
			&sub.credits_left.saturating_sub(credits_consumed),
		)
		.balance;
		let fees = T::FeesManager::collect_fees(&fees_to_collect, sub).map_err(|e| match e {
			FeesError::NotEnoughBalance { .. } => DispatchError::Other("NotEnoughBalance"),
			FeesError::Other(de) => de,
		})?;
		Self::deposit_event(Event::FeesCollected { sub_id: sub.id(), fees });
		Ok(())
	}

	/// Internal function to handle subscription creation
	fn create_subscription_internal(
		subscriber: T::AccountId,
		credits: BlockNumberFor<T>,
		target: Location,
		frequency: BlockNumberFor<T>,
		metadata: Option<MetadataOf<T>>,
	) -> DispatchResult {
		// Calculate and hold the subscription fees
		let fees = T::FeesManager::calculate_subscription_fees(&credits);

		Self::hold_fees(&subscriber, fees)?;

		let current_block = frame_system::Pallet::<T>::block_number();
		let details = SubscriptionDetails {
			subscriber: subscriber.clone(),
			target: target.clone(),
			metadata: metadata.unwrap_or_default(),
		};
		let subscription = Subscription {
			state: SubscriptionState::Active,
			credits_left: credits,
			details,
			created_at: current_block,
			updated_at: current_block,
			credits,
			frequency,
		};

		Self::hold_deposit(
			&subscriber,
			T::DepositCalculator::calculate_storage_deposit(&subscription),
		)?;

		let sub_id = subscription.id();

		ensure!(!Subscriptions::<T>::contains_key(sub_id), Error::<T>::SubscriptionAlreadyExists);

		Subscriptions::<T>::insert(sub_id, subscription);

		Self::deposit_event(Event::SubscriptionCreated { sub_id });

		Ok(())
	}

	fn hold_fees(subscriber: &T::AccountId, fees: BalanceOf<T>) -> DispatchResult {
		T::Currency::hold(&HoldReason::Fees.into(), subscriber, fees)
	}

	fn release_fees(subscriber: &T::AccountId, fees: BalanceOf<T>) -> DispatchResult {
		let _ = T::Currency::release(&HoldReason::Fees.into(), subscriber, fees, Precision::Exact)?;
		Ok(())
	}

	fn manage_diff_fees(
		subscriber: &T::AccountId,
		diff: &DiffBalance<BalanceOf<T>>,
	) -> DispatchResult {
		match diff.direction {
			BalanceDirection::Collect => Self::hold_fees(subscriber, diff.balance),
			BalanceDirection::Release => Self::release_fees(subscriber, diff.balance),
			BalanceDirection::None => Ok(()),
		}
	}

	fn hold_deposit(subscriber: &T::AccountId, deposit: BalanceOf<T>) -> DispatchResult {
		T::Currency::hold(&HoldReason::StorageDeposit.into(), subscriber, deposit)
	}

	fn release_deposit(subscriber: &T::AccountId, deposit: BalanceOf<T>) -> DispatchResult {
		let _ = T::Currency::release(
			&HoldReason::StorageDeposit.into(),
			subscriber,
			deposit,
			Precision::BestEffort,
		)?;
		Ok(())
	}

	fn manage_diff_deposit(
		subscriber: &T::AccountId,
		diff: &DiffBalance<BalanceOf<T>>,
	) -> DispatchResult {
		match diff.direction {
			BalanceDirection::Collect => Self::hold_deposit(subscriber, diff.balance),
			BalanceDirection::Release => Self::release_deposit(subscriber, diff.balance),
			BalanceDirection::None => Ok(()),
		}
	}

	/// Helper function to construct XCM message for randomness distribution
	// TODO: finish this off as part of https://github.com/ideal-lab5/idn-sdk/issues/77
	fn construct_randomness_xcm(_target: Location, _rnd: &T::Rnd) -> Result<Xcm<()>, Error<T>> {
		Ok(Xcm(vec![]))
	}
}

impl<T: Config> idn_traits::rand::Dispatcher<T::Rnd, DispatchResult> for Pallet<T> {
	fn dispatch(rnd: T::Rnd) -> DispatchResult {
		Pallet::<T>::distribute(rnd)
	}
}

sp_api::decl_runtime_apis! {
	#[api_version(1)]
	pub trait IdnManagerApi<Balance, BlockNumber, Metadata, AccountId> where
		Balance: Codec,
		BlockNumber: Codec + Unsigned,
		Metadata: Codec,
		AccountId: Codec,
	{
		/// Computes the fee for a given credits
		fn calculate_subscription_fees(
			// Number of random values to receive
			credits: BlockNumber
		) -> Balance;

		/// Retrieves a specific subscription
		fn get_subscription(
			// Subscription ID
			sub_id: H256
		) -> Option<Subscription<
				AccountId,
				BlockNumber,
				Metadata
			>>;

		/// Retrieves all subscriptions for a specific origin
		fn get_subscriptions_for_origin(
			// Origin account ID
			origin: AccountId
		) -> Vec<Subscription<
				AccountId,
				BlockNumber,
				Metadata
			>>;
	}
}
