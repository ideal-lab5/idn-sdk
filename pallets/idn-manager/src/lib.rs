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
//! This pallet manages subscriptions for pulse distribution.
//!
//! ## Subscription State Lifecycle
#![doc = simple_mermaid::mermaid!("../docs/mermaid/sub_state_lifecycle.mmd")]
//! ### States
//!
//! 1. Active (Default)
//!    - Randomness is actively distributed
//!    - IDN delivers pulses according to subscription parameters
//!
//! 2. Paused
//!    - Randomness distribution temporarily suspended
//!    - Subscription remains in storage
//!    - Can be reactivated
//!
//! ### Termination
//! Subscription ends when:
//! - All allocated pulses are consumed
//! - Manually killed by the original caller
//!
//! Upon termination:
//! - Removed from storage
//! - Storage deposit returned
//! - Unused credits refunded to the origin
//!
//! ## Pulse Filtering Security
// [SRLABS]
//! ### Overview
//! Subscribers can specify filters to receive only pulses that match specific criteria. For
//! security reasons, we only allow filtering by round numbers and prohibit filtering by
//! randomness values.
//!
//! ### Security Concern
//! If filtering by pulses were allowed, a subscriber could potentially:
//! - Set up filters to only receive specific randomness patterns
//! - Game the system by cherry-picking favorable pulses
//! - Undermine the fairness and unpredictability of the randomness distribution
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod impls;
pub mod primitives;
pub mod traits;
pub mod weights;

use crate::{
	primitives::{CallIndex, CreateSubParams, PulseFilter, SubscriptionMetadata},
	traits::{
		BalanceDirection, DepositCalculator, DiffBalance, FeesError, FeesManager,
		Subscription as SubscriptionTrait,
	},
};
use codec::{Codec, Decode, Encode, EncodeLike, MaxEncodedLen};
use frame_support::{
	pallet_prelude::{
		ensure, Blake2_128Concat, DispatchError, DispatchResult, DispatchResultWithPostInfo, Hooks,
		IsType, OptionQuery, StorageMap, StorageValue, ValueQuery, Weight, Zero,
	},
	sp_runtime::traits::AccountIdConversion,
	traits::{
		fungible::{hold::Mutate as HoldMutate, Inspect},
		tokens::Precision,
		Get,
	},
};
use frame_system::{
	ensure_signed,
	pallet_prelude::{BlockNumberFor, OriginFor},
};
use scale_info::TypeInfo;
use sp_arithmetic::traits::Unsigned;
use sp_core::H256;
use sp_idn_traits::pulse::{Dispatcher, Pulse, PulseMatch};
use sp_runtime::traits::Saturating;
use sp_std::{boxed::Box, fmt::Debug, vec, vec::Vec};
use xcm::{
	v5::{
		prelude::{
			ExpectTransactStatus, MaybeErrorCode, OriginKind, Transact, Unlimited, UnpaidExecution,
			Xcm,
		},
		Location,
	},
	VersionedLocation, VersionedXcm,
};
use xcm_builder::SendController;

pub use pallet::*;
pub use weights::WeightInfo;

const LOG_TARGET: &str = "pallet-idn-manager";

/// The balance type used in the pallet, derived from the currency type in the configuration.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// The metadata type used in the pallet, represented as a bounded vector of bytes.
pub type MetadataOf<T> = SubscriptionMetadata<<T as Config>::MaxMetadataLen>;

/// The subscription type used in the pallet, containing various details about the subscription.
pub type SubscriptionOf<T> = Subscription<
	<T as frame_system::Config>::AccountId,
	BlockNumberFor<T>,
	<T as pallet::Config>::Credits,
	MetadataOf<T>,
	PulseFilterOf<T>,
	<T as pallet::Config>::SubscriptionId,
>;

/// A filter that controls which pulses are delivered to a subscription
///
/// See [`PulseFilter`] for more details.
type PulseFilterOf<T> = PulseFilter<<T as pallet::Config>::Pulse, <T as Config>::MaxPulseFilterLen>;

/// Represents a subscription in the system.
#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct Subscription<AccountId, BlockNumber, Credits, Metadata, PulseFilter, SubscriptionId> {
	// The subscription ID
	id: SubscriptionId,
	// The immutable params of the subscription
	details: SubscriptionDetails<AccountId>,
	// Number of random values left to distribute
	credits_left: Credits,
	// The state of the subscription
	state: SubscriptionState,
	// A timestamp for creation
	created_at: BlockNumber,
	// A timestamp for update
	updated_at: BlockNumber,
	// Credits subscribed for
	credits: Credits,
	// How often to receive pulses
	frequency: BlockNumber,
	// Optional metadata that can be used by the subscriber
	metadata: Option<Metadata>,
	// Last block in which a pulse was received
	last_delivered: Option<BlockNumber>,
	// A custom filter for receiving pulses
	pulse_filter: Option<PulseFilter>,
}

/// Details specific to a subscription for pulse delivery
///
/// This struct contains information about the subscription owner, where randomness should be
/// delivered, how to deliver it via XCM, and any additional metadata for the subscription.
#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct SubscriptionDetails<AccountId> {
	// The account that created and pays for the subscription
	subscriber: AccountId,
	// The XCM location where randomness should be delivered
	target: Location,
	// Identifier for dispatching the XCM call, see [`crate::CallIndex`]
	call_index: CallIndex,
}

/// The subscription details type used in the pallet, containing information about the subscription
/// owner and target.
pub type SubscriptionDetailsOf<T> = SubscriptionDetails<<T as frame_system::Config>::AccountId>;

/// The parameters for creating a new subscription, containing various details about the
/// subscription.
pub type CreateSubParamsOf<T> = CreateSubParams<
	<T as pallet::Config>::Credits,
	BlockNumberFor<T>,
	MetadataOf<T>,
	PulseFilterOf<T>,
	<T as pallet::Config>::SubscriptionId,
>;

/// Parameters for updating an existing subscription.
///
/// When the parameter is `None`, the field is not updated.
#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq)]
pub struct UpdateSubParams<SubId, Credits, BlockNumber, PulseFilter, Metadata> {
	// The Subscription Id
	pub sub_id: SubId,
	// New number of credits
	pub credits: Option<Credits>,
	// New distribution interval
	pub frequency: Option<BlockNumber>,
	// Bounded vector for additional data
	pub metadata: Option<Option<Metadata>>,
	// New Pulse Filter
	pub pulse_filter: Option<Option<PulseFilter>>,
}

/// The parameters for updating an existing subscription, containing various details about the
/// subscription.
pub type UpdateSubParamsOf<T> = UpdateSubParams<
	<T as Config>::SubscriptionId,
	<T as Config>::Credits,
	BlockNumberFor<T>,
	PulseFilterOf<T>,
	MetadataOf<T>,
>;

/// Represents the state of a subscription.
///
/// This enum defines the possible states of a subscription, which can be either active or paused.
///
/// # Variants
/// * [`Active`](SubscriptionState::Active) - The subscription is active and randomness is being
///   distributed.
/// * [`Paused`](SubscriptionState::Paused) - The subscription is paused and randomness distribution
///   is temporarily suspended.
///
/// See [Subscription State Lifecycle](./index.html#subscription-state-lifecycle) for more details.
#[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen, Debug)]
pub enum SubscriptionState {
	/// The subscription is active and randomness is being distributed.
	Active,
	/// The subscription is paused and randomness distribution is temporarily suspended.
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
			Self::DiffBalance,
		>;

		/// Storage deposit calculator implementation
		type DepositCalculator: DepositCalculator<
			BalanceOf<Self>,
			SubscriptionOf<Self>,
			Self::DiffBalance,
		>;

		/// The type for the randomness pulse
		type Pulse: Pulse + Encode + Decode + Debug + Clone + TypeInfo + PartialEq + Default;

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
		type MaxMetadataLen: Get<u32>;

		/// Maximum Pulse Filter size
		type MaxPulseFilterLen: Get<u32>;

		/// A type to define the amount of credits in a subscription
		type Credits: Unsigned + Codec + TypeInfo + MaxEncodedLen + Debug + Saturating + Copy;

		/// Maximum number of subscriptions allowed
		type MaxSubscriptions: Get<u32>;

		/// Subscription ID type
		type SubscriptionId: From<H256>
			+ Codec
			+ Copy
			+ PartialEq
			+ TypeInfo
			+ EncodeLike
			+ MaxEncodedLen
			+ Debug;

		/// Type for indicating balance movements across subscribers and IDN
		type DiffBalance: DiffBalance<BalanceOf<Self>>;
	}

	/// The subscription storage. It maps a subscription ID to a subscription.
	#[pallet::storage]
	pub(crate) type Subscriptions<T: Config> =
		StorageMap<_, Blake2_128Concat, T::SubscriptionId, SubscriptionOf<T>, OptionQuery>;

	/// Storage for the subscription counter. It keeps track of how many subscriptions are in
	/// storage
	#[pallet::storage]
	pub(crate) type SubCounter<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new subscription was created (includes single-block subscriptions)
		SubscriptionCreated { sub_id: T::SubscriptionId },
		/// A subscription has finished
		SubscriptionRemoved { sub_id: T::SubscriptionId },
		/// A subscription was paused
		SubscriptionPaused { sub_id: T::SubscriptionId },
		/// A subscription was updated
		SubscriptionUpdated { sub_id: T::SubscriptionId },
		/// A subscription was reactivated
		SubscriptionReactivated { sub_id: T::SubscriptionId },
		/// Randomness was successfully distributed
		RandomnessDistributed { sub_id: T::SubscriptionId },
		/// Fees collected
		FeesCollected { sub_id: T::SubscriptionId, fees: BalanceOf<T> },
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
		/// Can't filter out based on pulses
		FilterRandNotPermitted,
		/// Too many subscriptions in storage
		TooManySubscriptions,
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
		/// Creates a subscription in the pallet.
		///
		/// This function allows a user to create a new subscription for receiving pulses.
		/// The subscription includes details such as the number of credits, target location, call
		/// index, frequency, metadata, and an optional pulse filter.
		///
		/// # Errors
		/// * [`TooManySubscriptions`](Error::TooManySubscriptions) - If the maximum number of
		///   subscriptions has been reached.
		/// * [`FilterRandNotPermitted`](Error::FilterRandNotPermitted) - If the pulse filter
		///   contains pulses.
		/// * [`SubscriptionAlreadyExists`](Error::SubscriptionAlreadyExists) - If a subscription
		///   with the specified ID already exists.
		///
		/// # Events
		/// * [`SubscriptionCreated`](Event::SubscriptionCreated) - A new subscription has been
		///   created.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_subscription(T::MaxPulseFilterLen::get()))]
		#[allow(clippy::useless_conversion)]
		pub fn create_subscription(
			origin: OriginFor<T>,
			params: CreateSubParamsOf<T>,
		) -> DispatchResultWithPostInfo {
			let subscriber = ensure_signed(origin)?;

			ensure!(
				SubCounter::<T>::get() < T::MaxSubscriptions::get(),
				Error::<T>::TooManySubscriptions
			);

			let current_block = frame_system::Pallet::<T>::block_number();

			let details = SubscriptionDetails {
				subscriber: subscriber.clone(),
				target: params.target.clone(),
				call_index: params.call_index,
			};

			let sub_id = params.sub_id.unwrap_or(Self::generate_sub_id(
				&subscriber,
				&params,
				&current_block,
			));

			ensure!(
				!Subscriptions::<T>::contains_key(sub_id),
				Error::<T>::SubscriptionAlreadyExists
			);

			let subscription = Subscription {
				id: sub_id,
				state: SubscriptionState::Active,
				credits_left: params.credits,
				details,
				created_at: current_block,
				updated_at: current_block,
				credits: params.credits,
				frequency: params.frequency,
				metadata: params.metadata,
				last_delivered: None,
				pulse_filter: params.pulse_filter.clone(),
			};

			// Calculate and hold the subscription fees
			let fees = Self::calculate_subscription_fees(&params.credits);

			Self::hold_fees(&subscriber, fees)?;

			Self::hold_deposit(
				&subscriber,
				T::DepositCalculator::calculate_storage_deposit(&subscription),
			)?;

			Subscriptions::<T>::insert(sub_id, subscription);

			// Increase the subscription counter
			SubCounter::<T>::mutate(|c| c.saturating_inc());

			Self::deposit_event(Event::SubscriptionCreated { sub_id });

			Ok(Some(T::WeightInfo::create_subscription(if let Some(pf) = params.pulse_filter {
				pf.len().try_into().unwrap_or(T::MaxPulseFilterLen::get())
			} else {
				0
			}))
			.into())
		}

		/// Temporarily halts randomness distribution for a subscription.
		///
		/// This function allows the subscriber to pause their subscription,
		/// which temporarily halts the distribution of pulses.
		///
		/// # Errors
		/// * [`SubscriptionDoesNotExist`](Error::SubscriptionDoesNotExist) - If a subscription with
		///   the specified ID does not exist.
		/// * [`NotSubscriber`](Error::NotSubscriber) - If the origin is not the subscriber of the
		///   specified subscription.
		/// * [`SubscriptionAlreadyPaused`](Error::SubscriptionAlreadyPaused) - If the subscription
		///   is already paused.
		///
		/// # Events
		/// * [`SubscriptionPaused`](Event::SubscriptionPaused) - A subscription has been paused.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::pause_subscription())]
		pub fn pause_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			sub_id: T::SubscriptionId,
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

		/// Ends the subscription before its natural conclusion.
		///
		/// This function allows the subscriber to terminate their subscription early,
		/// which stops the distribution of pulses and refunds any unused credits.
		///
		/// # Errors
		/// * [`SubscriptionDoesNotExist`](Error::SubscriptionDoesNotExist) - If a subscription with
		///   the specified ID does not exist.
		/// * [`NotSubscriber`](Error::NotSubscriber) - If the origin is not the subscriber of the
		///   specified subscription.
		///
		/// # Events
		/// * [`SubscriptionRemoved`](Event::SubscriptionRemoved) - A subscription has been
		///   terminated.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::kill_subscription())]
		pub fn kill_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			sub_id: T::SubscriptionId,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;

			let sub =
				Self::get_subscription(&sub_id).ok_or(Error::<T>::SubscriptionDoesNotExist)?;
			ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);

			Self::finish_subscription(&sub, sub_id)
		}

		/// Updates a subscription.
		///
		/// This function allows the subscriber to modify their subscription parameters,
		/// such as the number of credits, distribution interval, and pulse filter.
		///
		/// # Errors
		/// * [`SubscriptionDoesNotExist`](Error::SubscriptionDoesNotExist) - If a subscription with
		///   the specified ID does not exist.
		/// * [`NotSubscriber`](Error::NotSubscriber) - If the origin is not the subscriber of the
		///   specified subscription.
		/// * [`FilterRandNotPermitted`](Error::FilterRandNotPermitted) - If the pulse filter
		///   contains random values.
		///
		/// # Events
		/// * [`SubscriptionUpdated`](Event::SubscriptionUpdated) - A subscription has been updated.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::update_subscription(
			T::MaxPulseFilterLen::get(),
			T::MaxMetadataLen::get()
		))]
		#[allow(clippy::useless_conversion)]
		pub fn update_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			params: UpdateSubParamsOf<T>,
		) -> DispatchResultWithPostInfo {
			let subscriber = ensure_signed(origin)?;

			let inner_pulse_filter = params.pulse_filter.clone().unwrap_or_default();

			Subscriptions::<T>::try_mutate(params.sub_id, |maybe_sub| {
				let sub = maybe_sub.as_mut().ok_or(Error::<T>::SubscriptionDoesNotExist)?;
				ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);

				let mut fees_diff = T::DiffBalance::new(Zero::zero(), BalanceDirection::None);

				if let Some(credits) = params.credits {
					fees_diff = T::FeesManager::calculate_diff_fees(&sub.credits, &credits);
					sub.credits = credits;
				}
				if let Some(frequency) = params.frequency {
					sub.frequency = frequency;
				}
				if let Some(pulse_filter) = params.pulse_filter {
					sub.pulse_filter = pulse_filter;
				}
				if let Some(metadata) = params.metadata.clone() {
					sub.metadata = metadata;
				}
				sub.updated_at = frame_system::Pallet::<T>::block_number();

				let old_sub = sub.clone();

				let deposit_diff = T::DepositCalculator::calculate_diff_deposit(sub, &old_sub);

				// Hold or refund diff fees
				Self::manage_diff_fees(&subscriber, &fees_diff)?;
				// Hold or refund diff deposit
				Self::manage_diff_deposit(&subscriber, &deposit_diff)?;

				Self::deposit_event(Event::SubscriptionUpdated { sub_id: params.sub_id });
				Ok::<(), DispatchError>(())
			})?;

			Ok(Some(T::WeightInfo::update_subscription(
				if let Some(pf) = inner_pulse_filter {
					pf.len().try_into().unwrap_or(T::MaxPulseFilterLen::get())
				} else {
					0
				},
				if let Some(Some(md)) = params.metadata {
					md.len().try_into().unwrap_or(T::MaxMetadataLen::get())
				} else {
					0
				},
			))
			.into())
		}

		/// Reactivates a paused subscription.
		///
		/// This function allows the subscriber to resume a paused subscription,
		/// which restarts the distribution of pulses.
		///
		/// # Events
		/// * [`SubscriptionReactivated`](Event::SubscriptionReactivated) - A subscription has been
		///   reactivated.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::reactivate_subscription())]
		pub fn reactivate_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			sub_id: T::SubscriptionId,
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
		sub_id: T::SubscriptionId,
	) -> DispatchResult {
		// fees left and deposit to refund
		let fees_diff = T::FeesManager::calculate_diff_fees(&sub.credits_left, &Zero::zero());
		let sd = T::DepositCalculator::calculate_storage_deposit(sub);

		Self::manage_diff_fees(&sub.details.subscriber, &fees_diff)?;
		Self::release_deposit(&sub.details.subscriber, sd)?;

		Self::deposit_event(Event::SubscriptionRemoved { sub_id });

		Subscriptions::<T>::remove(sub_id);

		// Decrease the subscription counter
		SubCounter::<T>::mutate(|c| c.saturating_dec());

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
			|(sub_id, mut sub): (T::SubscriptionId, SubscriptionOf<T>)| -> DispatchResult {
				// Filter for active subscriptions that are eligible for delivery based on frequency
				// and custom filter criteria
				if
				// Subscription must be active
				sub.state ==  SubscriptionState::Active   &&
					// And either never delivered before, or enough blocks have passed since last delivery
					(sub.last_delivered.is_none() ||
					current_block >= sub.last_delivered.unwrap() + sub.frequency) &&
 					// The pulse passes the custom filter
					// see the [Pulse Filtering Security](#pulse-filtering-security) section
					Self::custom_filter(&sub.pulse_filter, &pulse)
				{
					let msg = Self::construct_xcm_msg(sub.details.call_index, &pulse, &sub_id);
					let versioned_target: Box<VersionedLocation> =
						Box::new(sub.details.target.clone().into());
					let versioned_msg: Box<VersionedXcm<()>> =
						Box::new(xcm::VersionedXcm::V5(msg.into()));
					let origin = frame_system::RawOrigin::Signed(Self::pallet_account_id());

					// Make sure credits are consumed before sending the XCM message
					let consume_credits = T::FeesManager::get_consume_credits(&sub);

					// [SRLabs]: If this line throws an error then the entire set of subscriptions
					// will fail to be distributed for the pulse. Recommendations on handling this?
					// Our initial idea is to simply log an event and continue, then pause the subscription.
					Self::collect_fees(&sub, consume_credits)?;

					// Update subscription with consumed credits and last_delivered block number
					sub.credits_left = sub.credits_left.saturating_sub(consume_credits);
					sub.last_delivered = Some(current_block);

					// Store the updated subscription
					Subscriptions::<T>::insert(sub_id, &sub);

					// Send the XCM message
					// [SRLabs]: If this line throws an error then the entire set of subscriptions
					// will fail to be distributed for the pulse. Recommendations on handling this?
					// Our initial idea is to simply log an event and continue, then pause the subscription.
					T::Xcm::send(origin.into(), versioned_target, versioned_msg)?;

					Self::deposit_event(Event::RandomnessDistributed { sub_id });
				} else {
					let idle_credits = T::FeesManager::get_idle_credits(&sub);
					// Collect fees for idle block
					Self::collect_fees(&sub, idle_credits)?;
					// Update subscription with consumed credits
					sub.credits_left = sub.credits_left.saturating_sub(idle_credits);
					// Store the updated subscription
					Subscriptions::<T>::insert(sub_id, &sub);
				}
				Ok(())
			},
		)?;

		Ok(())
	}

	/// Collects fees for a subscription based on credits consumed.
	///
	/// This internal helper function calculates and collects fees from a subscription based on
	/// the number of credits consumed. If no credits are consumed, no fees are collected.
	///
	/// It calculates the fees to collect using
	/// [`T::FeesManager::calculate_diff_fees`](FeesManager::calculate_diff_fees), which determines
	/// the difference between the current credits left and the credits left after consumption.
	/// It collects the fees using [`T::FeesManager::collect_fees`](FeesManager::collect_fees),
	/// transferring the fees from the subscriber's account to the treasury.
	///
	/// # Events
	/// * [`FeesCollected`](Event::FeesCollected) - Emitted when fees are successfully collected
	///   from the subscription.
	fn collect_fees(sub: &SubscriptionOf<T>, credits_consumed: T::Credits) -> DispatchResult {
		if credits_consumed.is_zero() {
			return Ok(());
		}
		let fees_to_collect = T::FeesManager::calculate_diff_fees(
			&sub.credits_left,
			&sub.credits_left.saturating_sub(credits_consumed),
		)
		.balance();
		let fees = T::FeesManager::collect_fees(&fees_to_collect, sub).map_err(|e| match e {
			FeesError::NotEnoughBalance { .. } => DispatchError::Other("NotEnoughBalance"),
			FeesError::Other(de) => de,
		})?;
		Self::deposit_event(Event::FeesCollected { sub_id: sub.id, fees });
		Ok(())
	}

	/// Determines whether a pulse should pass through a filter
	///
	/// This function checks whether a given pulse meets the filter criteria specified
	/// in a subscription. If no filter is present, all pulses pass through.
	///
	/// # How Filtering Works
	/// 1. If `pulse_filter` is `None`, returns `true` (no filtering)
	/// 2. Otherwise, checks if ANY of the properties in the filter match the pulse using the
	///    PulseMatch trait's match_prop method
	///
	/// This enables subscriptions to receive randomness only when the pulse has specific
	/// properties, such as coming from a particular round number.
	fn custom_filter(pulse_filter: &Option<PulseFilterOf<T>>, pulse: &T::Pulse) -> bool {
		match pulse_filter {
			Some(filter) => filter.iter().any(|prop| pulse.match_prop(prop.clone())),
			None => true,
		}
	}

	/// Generates a unique subscription ID.
	///
	/// This internal helper function generates a unique subscription ID based on the
	/// create subscription parameters and the current block number.
	fn generate_sub_id(
		subscriber: &T::AccountId,
		params: &CreateSubParamsOf<T>,
		current_block: &BlockNumberFor<T>,
	) -> T::SubscriptionId {
		let mut salt = current_block.encode();
		salt.extend(subscriber.encode());
		params.hash(salt)
	}

	/// Holds fees for a subscription.
	///
	/// This internal helper function places a hold on the specified fees in the subscriber's
	/// account.
	fn hold_fees(subscriber: &T::AccountId, fees: BalanceOf<T>) -> DispatchResult {
		T::Currency::hold(&HoldReason::Fees.into(), subscriber, fees)
	}

	/// Releases fees for a subscription.
	///
	/// This internal helper function releases the specified fees from the subscriber's account.
	fn release_fees(subscriber: &T::AccountId, fees: BalanceOf<T>) -> DispatchResult {
		let _ = T::Currency::release(&HoldReason::Fees.into(), subscriber, fees, Precision::Exact)?;
		Ok(())
	}

	/// Manages the difference in fees for a subscription.
	///
	/// This internal helper function manages the difference in fees for a subscription, either
	/// collecting additional fees or releasing excess fees.
	fn manage_diff_fees(subscriber: &T::AccountId, diff: &T::DiffBalance) -> DispatchResult {
		match diff.direction() {
			BalanceDirection::Collect => Self::hold_fees(subscriber, diff.balance()),
			BalanceDirection::Release => Self::release_fees(subscriber, diff.balance()),
			BalanceDirection::None => Ok(()),
		}
	}

	/// Holds a storage deposit for a subscription.
	///
	/// This internal helper function places a hold on the specified storage deposit in the
	/// subscriber's account.
	fn hold_deposit(subscriber: &T::AccountId, deposit: BalanceOf<T>) -> DispatchResult {
		T::Currency::hold(&HoldReason::StorageDeposit.into(), subscriber, deposit)
	}

	/// Releases a storage deposit for a subscription.
	///
	/// This internal helper function releases the specified storage deposit from the subscriber's
	/// account.
	fn release_deposit(subscriber: &T::AccountId, deposit: BalanceOf<T>) -> DispatchResult {
		let _ = T::Currency::release(
			&HoldReason::StorageDeposit.into(),
			subscriber,
			deposit,
			Precision::BestEffort,
		)?;
		Ok(())
	}

	/// Manages the difference in storage deposit for a subscription.
	///
	/// This internal helper function manages the difference in storage deposit for a subscription,
	/// either collecting additional deposit or releasing excess deposit.
	fn manage_diff_deposit(subscriber: &T::AccountId, diff: &T::DiffBalance) -> DispatchResult {
		match diff.direction() {
			BalanceDirection::Collect => Self::hold_deposit(subscriber, diff.balance()),
			BalanceDirection::Release => Self::release_deposit(subscriber, diff.balance()),
			BalanceDirection::None => Ok(()),
		}
	}

	/// Constructs an XCM message for delivering randomness to a subscriber
	///
	/// This function creates a complete XCM message that will:
	/// 1. Execute without payment at the destination chain ([`UnpaidExecution`])
	/// 2. Execute a call on the destination chain using the provided call index and randomness data
	///    ([`Transact`])
	/// 3. Expect successful execution and check status code (ExpectTransactStatus)
	fn construct_xcm_msg(
		call_index: CallIndex,
		pulse: &T::Pulse,
		sub_id: &T::SubscriptionId,
	) -> Xcm<()> {
		Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact {
				origin_kind: OriginKind::Xcm,
				fallback_max_weight: None,
				// Create a tuple of call_index and pulse, encode it using SCALE codec, then convert
				// to Vec<u8> The encoded data will be used by the receiving chain to:
				// 1. Find the target pallet using first byte of call_index
				// 2. Find the target function using second byte of call_index
				// 3. Pass the parameters to that function
				call: (call_index, pulse, sub_id).encode().into(),
			},
			ExpectTransactStatus(MaybeErrorCode::Success),
		])
	}

	/// Computes the fee for a given number of credits.
	///
	/// This function calculates the subscription fee based on the specified number of credits
	/// using the [`FeesManager`] implementation.
	pub fn calculate_subscription_fees(credits: &T::Credits) -> BalanceOf<T> {
		T::FeesManager::calculate_subscription_fees(credits)
	}

	/// Retrieves a specific subscription by its ID.
	///
	/// This function retrieves a subscription from storage based on the provided subscription ID.
	pub fn get_subscription(sub_id: &T::SubscriptionId) -> Option<SubscriptionOf<T>> {
		Subscriptions::<T>::get(sub_id)
	}

	/// Retrieves all subscriptions for a specific subscriber.
	///
	/// This function retrieves all subscriptions from storage that are associated with the
	/// provided subscriber account.
	pub fn get_subscriptions_for_subscriber(subscriber: &T::AccountId) -> Vec<SubscriptionOf<T>> {
		Subscriptions::<T>::iter()
			.filter(|(_, sub)| &sub.details.subscriber == subscriber)
			.map(|(_, sub)| sub)
			.collect()
	}
}

impl<T: Config> Dispatcher<T::Pulse, DispatchResult> for Pallet<T> {
	/// Dispatches a collection of pulses by distributing it to eligible subscriptions.
	///
	/// This function serves as the entry point for distributing randomness pulses
	/// to active subscriptions. It calls the `distribute` function to handle the
	/// actual distribution logic.
	/// TODO: https://github.com/ideal-lab5/idn-sdk/issues/195
	fn dispatch(pulses: Vec<T::Pulse>) -> DispatchResult {
		for pulse in pulses {
			let round = pulse.round().clone();
			if let Err(e) = Pallet::<T>::distribute(pulse) {
				log::warn!(target: LOG_TARGET, "Distribution of pulse # {:?} failed: {:?}", round, e);
			}
		}

		Ok(())
	}

	fn dispatch_weight(pulses: usize) -> Weight {
		T::WeightInfo::dispatch_pulse(T::MaxPulseFilterLen::get(), T::MaxSubscriptions::get())
			.saturating_mul(pulses as u64)
	}
}

sp_api::decl_runtime_apis! {
	#[api_version(1)]
	pub trait IdnManagerApi<Balance, Credits, AccountId, Subscription, SubscriptionId> where
		Balance: Codec,
		Credits: Codec,
		AccountId: Codec,
		Subscription: Codec,
		SubscriptionId: Codec
	{
		/// Computes the fee for a given credits
		///
		/// See [`crate::Pallet::calculate_subscription_fees`]
		fn calculate_subscription_fees(
			// Number of pulses to receive
			credits: Credits
		) -> Balance;

		/// Retrieves a specific subscription
		///
		/// See [`crate::Pallet::get_subscription`]
		fn get_subscription(
			// Subscription ID
			sub_id: SubscriptionId
		) -> Option<Subscription>;

		/// Retrieves all subscriptions for a specific subscriber
		///
		/// See [`crate::Pallet::get_subscriptions_for_subscriber`]
		fn get_subscriptions_for_subscriber(
			// subscriber account ID
			subscriber: AccountId
		) -> Vec<Subscription>;
	}
}
