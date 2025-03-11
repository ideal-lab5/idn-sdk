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
use idn_traits::rand::{Dispatcher, Pulse};
use scale_info::TypeInfo;
use sp_arithmetic::traits::Unsigned;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::{traits::One, Saturating};
use sp_std::fmt::Debug;
use traits::{
	BalanceDirection, DepositCalculator, DiffBalance, FeesError, FeesManager,
	Subscription as SubscriptionTrait,
};
use xcm::{
	v5::{prelude::*, Location},
	VersionedLocation, VersionedXcm,
};
use xcm_builder::SendController;

pub use pallet::*;
pub use weights::WeightInfo;

/// Two-byte identifier for dispatching XCM calls
///
/// This type represents a compact encoding of pallet and function identifiers:
/// - The first byte represents the pallet index in the destination runtime
/// - The second byte represents the call index within that pallet
///
/// # Example
/// ```rust
/// # use pallet_idn_manager::CallIndex;
///
/// let call_index: CallIndex = [42, 3];  // Target the 42nd pallet, 3rd function
/// ```
///
/// This identifier is used in XCM messages to ensure randomness is delivered
/// to the appropriate function in the destination pallet.
pub type CallIndex = [u8; 2];

pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

pub type MetadataOf<T> = BoundedVec<u8, <T as Config>::SubMetadataLen>;

pub type SubscriptionOf<T> = Subscription<
	<T as frame_system::Config>::AccountId,
	BlockNumberFor<T>,
	<T as pallet::Config>::Credits,
	MetadataOf<T>,
>;

#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct Subscription<AccountId, BlockNumber, Credits, Metadata> {
	details: SubscriptionDetails<AccountId, Metadata>,
	// Number of random values left to distribute
	credits_left: Credits,
	state: SubscriptionState,
	created_at: BlockNumber,
	updated_at: BlockNumber,
	credits: Credits,
	frequency: BlockNumber,
	last_delivered: Option<BlockNumber>,
}

/// Details specific to a subscription for random value delivery
///
/// This struct contains information about the subscription owner, where randomness should be
/// delivered, how to deliver it via XCM, and any additional metadata for the subscription.
///
/// # Fields
/// * `subscriber` - The account that created and pays for the subscription
/// * `target` - The XCM location where randomness should be delivered
/// * `call_index` - Identifier for dispatching the XCM call, see [`crate::CallIndex`]
/// * `metadata` - Optional bounded metadata that can be used by the subscriber for identification
///   or application-specific data
///
/// # Example
/// ```rust
/// # use xcm::v5::{Junction, Location};
/// # use sp_runtime::AccountId32;
/// # use pallet_idn_manager::SubscriptionDetails;
/// # const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
///
/// let details = SubscriptionDetails {
///     subscriber: ALICE,
///     target: Location::new(1, [Junction::PalletInstance(42)]),
///     call_index: [42, 3],  // Target the 42nd pallet, 3rd function
///     metadata: b"My randomness subscription".to_vec(),
/// };
/// ```
#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct SubscriptionDetails<AccountId, Metadata> {
	/// The account that created and pays for the subscription
	pub subscriber: AccountId,
	/// The XCM location where randomness should be delivered
	pub target: Location,
	/// Identifier for dispatching the XCM call, see [`crate::CallIndex`]
	pub call_index: CallIndex,
	/// Optional metadata that can be used by the subscriber
	pub metadata: Metadata,
}

impl<AccountId, BlockNumber, Credits, Metadata>
	Subscription<AccountId, BlockNumber, Credits, Metadata>
where
	AccountId: Encode,
	BlockNumber: Encode + Copy,
	Metadata: Encode + Clone,
{
	pub fn id(&self) -> SubscriptionId {
		let id_tuple = (
			self.created_at,
			&self.details.subscriber,
			self.details.target.clone(),
			self.details.call_index,
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
			Self::Credits,
			SubscriptionOf<Self>,
			DispatchError,
			<Self as frame_system::pallet::Config>::AccountId,
		>;

		/// Storage deposit calculator implementation
		type DepositCalculator: DepositCalculator<BalanceOf<Self>, SubscriptionOf<Self>>;

		/// The type for the randomness pulse
		type Pulse: Pulse + Encode;

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

		/// A type to define the amount of credits in a subscription
		type Credits: Unsigned + Codec + TypeInfo + MaxEncodedLen + Debug + Saturating + Copy;
	}

	#[pallet::storage]
	pub type Subscriptions<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SubscriptionId,
		Subscription<T::AccountId, BlockNumberFor<T>, T::Credits, MetadataOf<T>>,
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
			credits: T::Credits,
			// XCM multilocation for random value delivery
			target: Location,
			// Call index for XCM message
			call_index: CallIndex,
			// Distribution interval for random values
			frequency: BlockNumberFor<T>,
			// Bounded vector for additional data
			metadata: Option<MetadataOf<T>>,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;
			Self::create_subscription_internal(
				subscriber, credits, target, call_index, frequency, metadata,
			)
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
				Self::get_subscription(&sub_id).ok_or(Error::<T>::SubscriptionDoesNotExist)?;
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
			credits: T::Credits,
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
	fn distribute(pulse: T::Pulse) -> DispatchResult {
		// Get the current block number once for comparison
		let current_block = frame_system::Pallet::<T>::block_number();

		Subscriptions::<T>::iter().try_for_each(
			|(sub_id, mut sub): (SubscriptionId, SubscriptionOf<T>)| -> DispatchResult {
				// Filter for active subscriptions that are eligible for delivery based on frequency
				if !(
					// Subscription must be active
					sub.state == SubscriptionState::Active  &&
					// And either never delivered before, or enough blocks have passed since last delivery
					(sub.last_delivered.is_none() ||
					current_block >= sub.last_delivered.unwrap() + sub.frequency)
				) {
					return Ok(()); // Skip this subscription
				}

				let msg = Self::construct_randomness_xcm(&rnd, sub.details.call_index);
				let versioned_target: Box<VersionedLocation> =
					Box::new(sub.details.target.clone().into());
				let versioned_msg: Box<VersionedXcm<()>> =
					Box::new(xcm::VersionedXcm::V5(msg.into()));
				let origin = frame_system::RawOrigin::Signed(Self::pallet_account_id());

				if T::Xcm::send(origin.into(), versioned_target, versioned_msg).is_ok() {
					// We consume a fixed one credit per distribution
					let credits_consumed = One::one();
					Self::collect_fees(&sub, credits_consumed)?;

					// Update subscription with consumed credits and last_delivered block number
					sub.credits_left = sub.credits_left.saturating_sub(credits_consumed);
					sub.last_delivered = Some(current_block);

					// Store the updated subscription
					Subscriptions::<T>::insert(sub_id, sub);

					Self::deposit_event(Event::RandomnessDistributed { sub_id });
				}
				Ok(())
			},
		)?;

		Ok(())
	}

	fn collect_fees(sub: &SubscriptionOf<T>, credits_consumed: T::Credits) -> DispatchResult {
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
		credits: T::Credits,
		target: Location,
		call_index: CallIndex,
		frequency: BlockNumberFor<T>,
		metadata: Option<MetadataOf<T>>,
	) -> DispatchResult {
		// Calculate and hold the subscription fees
		let fees = Self::calculate_subscription_fees(&credits);

		Self::hold_fees(&subscriber, fees)?;

		let current_block = frame_system::Pallet::<T>::block_number();
		let details = SubscriptionDetails {
			subscriber: subscriber.clone(),
			target: target.clone(),
			call_index,
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
			last_delivered: None,
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

	/// Constructs an XCM message for delivering randomness to a subscriber
	///
	/// This function creates a complete XCM message that will:
	/// 1. Execute without payment at the destination chain (UnpaidExecution)
	/// 2. Execute a call on the destination chain using the provided call index and randomness data
	///    (Transact)
	/// 3. Expect successful execution and check status code (ExpectTransactStatus)
	///
	/// # Parameters
	/// * `pulse` - The random value to deliver to the subscriber
	/// * `call_index` - Identifier for dispatching the XCM call, see [`crate::CallIndex`]
	///
	/// # Returns
	/// * `Xcm` - A complete XCM message ready to be sent
	fn construct_randomness_xcm(pulse: &T::Pulse, call_index: CallIndex) -> Xcm<()> {
		Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact {
				origin_kind: OriginKind::Xcm,
				fallback_max_weight: None,
				// Create a tuple of call_index and pulse, encode it using SCALE codec, then convert
				// to Vec<u8> The encoded data will be used by the receiving chain to:
				// 1. Find the target pallet using first byte of call_index
				// 2. Find the target function using second byte of call_index
				// 3. Pass the random value to that function
				call: (call_index, pulse).encode().into(),
			},
			ExpectTransactStatus(MaybeErrorCode::Success),
		])
	}

	/// Computes the fee for a given credits
	pub fn calculate_subscription_fees(credits: &T::Credits) -> BalanceOf<T> {
		T::FeesManager::calculate_subscription_fees(credits)
	}

	/// Retrieves a specific subscription
	pub fn get_subscription(sub_id: &SubscriptionId) -> Option<SubscriptionOf<T>> {
		Subscriptions::<T>::get(sub_id)
	}

	/// Retrieves all subscriptions for a specific subscriber
	pub fn get_subscriptions_for_subscriber(subscriber: &T::AccountId) -> Vec<SubscriptionOf<T>> {
		Subscriptions::<T>::iter()
			.filter(|(_, sub)| &sub.details.subscriber == subscriber)
			.map(|(_, sub)| sub)
			.collect()
	}
}

impl<T: Config> Dispatcher<T::Pulse, DispatchResult> for Pallet<T> {
	fn dispatch(pulse: T::Pulse) -> DispatchResult {
		Pallet::<T>::distribute(pulse)
	}
}

sp_api::decl_runtime_apis! {
	#[api_version(1)]
	pub trait IdnManagerApi<Balance, BlockNumber, Credits, Metadata, AccountId> where
		Balance: Codec,
		BlockNumber: Codec,
		Credits: Codec,
		Metadata: Codec,
		AccountId: Codec,
	{
		/// Computes the fee for a given credits
		///
		/// See [`crate::Pallet::calculate_subscription_fees`]
		fn calculate_subscription_fees(
			// Number of random values to receive
			credits: BlockNumber
		) -> Balance;

		/// Retrieves a specific subscription
		///
		/// See [`crate::Pallet::get_subscription`]
		fn get_subscription(
			// Subscription ID
			sub_id: H256
		) -> Option<Subscription<
				AccountId,
				BlockNumber,
				Credits,
				Metadata
			>>;

		/// Retrieves all subscriptions for a specific subscriber
		///
		/// See [`crate::Pallet::get_subscriptions_for_subscriber`]
		fn get_subscriptions_for_subscriber(
			// subscriber account ID
			subscriber: AccountId
		) -> Vec<Subscription<
				AccountId,
				BlockNumber,
				Credits,
				Metadata
			>>;
	}
}
