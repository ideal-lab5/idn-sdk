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
//!
//! ### Implementation Approach
//! The [`ensure_filter_no_rand`](Pallet::ensure_filter_no_rand) function validates that
//! filters don't contain any pulse criteria. This validation happens when the filter
//! is first created or updated, rather than during distribution:
//!
//! - **Benefits**: The filter validation cost is paid by the subscriber creating the filter, not
//!   distributed across all subscribers during the randomness delivery process
//! - **Calls**: Implemented in [`create_subscription`](Pallet::create_subscription) and
//!   [`update_subscription`](Pallet::update_subscription)
//!
//! ### Developer Warning
//! **Important**: When adding or modifying any dispatchable that creates or updates filters,
//! always include a call to [`ensure_filter_no_rand`](Pallet::ensure_filter_no_rand).
//! Failure to do this could create security vulnerabilities by allowing random-value filtering.
//! Test suites should catch issues with existing dispatchables, but new functions need
//! careful attention.
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod impls;
pub mod traits;
pub mod weights;

use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::{
	pallet_prelude::{
		ensure, Blake2_128Concat, DispatchError, DispatchResult, DispatchResultWithPostInfo, Hooks,
		IsType, OptionQuery, StorageMap, StorageValue, ValueQuery, Zero,
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
use idn_traits::pulse::{Dispatcher, Pulse, PulseMatch, PulseProperty};
use scale_info::TypeInfo;
use sp_arithmetic::traits::Unsigned;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::traits::Saturating;
use sp_std::{boxed::Box, fmt::Debug, vec, vec::Vec};
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

/// The balance type used in the pallet, derived from the currency type in the configuration.
pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// The metadata type used in the pallet, represented as a bounded vector of bytes.
pub type MetadataOf<T> = BoundedVec<u8, <T as Config>::SubMetadataLen>;

/// The subscription type used in the pallet, containing various details about the subscription.
pub type SubscriptionOf<T> = Subscription<
	<T as frame_system::Config>::AccountId,
	BlockNumberFor<T>,
	<T as pallet::Config>::Credits,
	MetadataOf<T>,
	PulseFilterOf<T>,
>;

/// The pulse property type used in the pallet, representing various properties of a pulse.
pub type PulsePropertyOf<T> = PulseProperty<
	<<T as pallet::Config>::Pulse as Pulse>::Rand,
	<<T as pallet::Config>::Pulse as Pulse>::Round,
	<<T as pallet::Config>::Pulse as Pulse>::Sig,
>;

/// A filter that controls which pulses are delivered to a subscription
///
/// This type allows subscribers to define specific criteria for which pulses they want to receive.
/// For example, a subscriber might want to receive only pulses from specific round numbers.
///
/// # Implementation
/// - Uses a bounded vector to limit the maximum number of filter conditions
/// - Each element is a [`PulseProperty`] that can match against `round` (but not `rand` values)
/// - A pulse passes the filter if it matches ANY of the properties in the filter
///
/// # Usage
/// ```rust
/// use idn_traits::pulse::PulseProperty as PulsePropertyTrait;
/// type PulseProperty = PulsePropertyTrait<[u8; 32], u64, [u8; 48]>;
/// // Create a filter for even-numbered rounds only
/// let filter = vec![
///     PulseProperty::Round(2),
///     PulseProperty::Round(4),
///     PulseProperty::Round(6),
/// ];
/// ```
///
/// # Security
/// For security reasons, filtering on `rand` values is explicitly prohibited.
/// See the [Pulse Filtering Security](#pulse-filtering-security) section for more details.
pub type PulseFilterOf<T> = BoundedVec<PulsePropertyOf<T>, <T as Config>::PulseFilterLen>;

/// Represents a subscription in the system.
#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct Subscription<AccountId, BlockNumber, Credits, Metadata, PulseFilter> {
	id: SubscriptionId,
	details: SubscriptionDetails<AccountId, Metadata>,
	// Number of pulses left to distribute
	credits_left: Credits,
	state: SubscriptionState,
	created_at: BlockNumber,
	updated_at: BlockNumber,
	credits: Credits,
	frequency: BlockNumber,
	last_delivered: Option<BlockNumber>,
	pulse_filter: Option<PulseFilter>,
}

/// Details specific to a subscription for pulse delivery
///
/// This struct contains information about the subscription owner, where randomness should be
/// delivered, how to deliver it via XCM, and any additional metadata for the subscription.
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

/// The subscription details type used in the pallet, containing information about the subscription
/// owner and target.
pub type SubscriptionDetailsOf<T> =
	SubscriptionDetails<<T as frame_system::Config>::AccountId, MetadataOf<T>>;

/// The subscription ID type used in the pallet, represented as a 32-byte array.
pub type SubscriptionId = [u8; 32];

/// Parameters for creating a new subscription
#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq)]
pub struct CreateSubParams<Credits, BlockNumber, Metadata, PulseFilter> {
	// Number of pulses to receive
	pub credits: Credits,
	// XCM multilocation for pulse delivery
	pub target: Location,
	// Call index for XCM message
	pub call_index: CallIndex,
	// Distribution interval for pulses
	pub frequency: BlockNumber,
	// Bounded vector for additional data
	pub metadata: Option<Metadata>,
	// Optional Pulse Filter
	pub pulse_filter: Option<PulseFilter>,
	// Optional Subscription Id, if None, a new one will be generated
	pub sub_id: Option<SubscriptionId>,
}

/// The parameters for creating a new subscription, containing various details about the
/// subscription.
pub type CreateSubParamsOf<T> = CreateSubParams<
	<T as pallet::Config>::Credits,
	BlockNumberFor<T>,
	MetadataOf<T>,
	PulseFilterOf<T>,
>;

/// Parameters for updating an existing subscription
#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq)]
pub struct UpdateSubParams<SubId, Credits, Frequency, PulseFilter> {
	// The Subscription Id
	pub sub_id: SubId,
	// New number of credits
	pub credits: Credits,
	// New distribution interval
	pub frequency: Frequency,
	// New Pulse Filter
	pub pulse_filter: Option<PulseFilter>,
}

/// The parameters for updating an existing subscription, containing various details about the
/// subscription.
pub type UpdateSubParamsOf<T> =
	UpdateSubParams<SubscriptionId, <T as Config>::Credits, BlockNumberFor<T>, PulseFilterOf<T>>;

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
		type SubMetadataLen: Get<u32>;

		/// Maximum Pulse Filter size
		type PulseFilterLen: Get<u32>;

		/// A type to define the amount of credits in a subscription
		type Credits: Unsigned + Codec + TypeInfo + MaxEncodedLen + Debug + Saturating + Copy;

		/// Maximum number of subscriptions allowed
		type MaxSubscriptions: Get<u32>;
	}

	/// The subscription storage. It maps a subscription ID to a subscription.
	#[pallet::storage]
	pub(crate) type Subscriptions<T: Config> =
		StorageMap<_, Blake2_128Concat, SubscriptionId, SubscriptionOf<T>, OptionQuery>;

	/// Storage for the subscription counter. It keeps track of how many subscriptions are in
	/// storage
	#[pallet::storage]
	pub(crate) type SubCounter<T: Config> = StorageValue<_, u32, ValueQuery>;

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
		#[pallet::weight(T::WeightInfo::create_subscription(T::PulseFilterLen::get()))]
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

			// make sure the filter does not filter on pulses
			// see the [Pulse Filtering Security](#pulse-filtering-security) section
			Self::ensure_filter_no_rand(&params.pulse_filter)?;

			let current_block = frame_system::Pallet::<T>::block_number();

			let details = SubscriptionDetails {
				subscriber: subscriber.clone(),
				target: params.target.clone(),
				call_index: params.call_index,
				metadata: params.metadata.unwrap_or_default(),
			};

			let sub_id = params.sub_id.unwrap_or(Self::generate_sub_id(&details, &current_block));

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
				pf.len().try_into().unwrap_or(T::PulseFilterLen::get())
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
			sub_id: SubscriptionId,
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
		#[pallet::weight(T::WeightInfo::update_subscription(T::PulseFilterLen::get()))]
		#[allow(clippy::useless_conversion)]
		pub fn update_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			params: UpdateSubParamsOf<T>,
		) -> DispatchResultWithPostInfo {
			let subscriber = ensure_signed(origin)?;

			// make sure the filter does not filter on pulses
			// see the [Pulse Filtering Security](#pulse-filtering-security) section
			Self::ensure_filter_no_rand(&params.pulse_filter)?;

			Subscriptions::<T>::try_mutate(params.sub_id, |maybe_sub| {
				let sub = maybe_sub.as_mut().ok_or(Error::<T>::SubscriptionDoesNotExist)?;
				ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);

				let fees_diff = T::FeesManager::calculate_diff_fees(&sub.credits, &params.credits);
				let old_sub = sub.clone();

				sub.credits = params.credits;
				sub.frequency = params.frequency;
				sub.pulse_filter = params.pulse_filter.clone();
				sub.updated_at = frame_system::Pallet::<T>::block_number();

				let deposit_diff = T::DepositCalculator::calculate_diff_deposit(sub, &old_sub);

				// Hold or refund diff fees
				Self::manage_diff_fees(&subscriber, &fees_diff)?;
				// Hold or refund diff deposit
				Self::manage_diff_deposit(&subscriber, &deposit_diff)?;

				Self::deposit_event(Event::SubscriptionUpdated { sub_id: params.sub_id });
				Ok::<(), DispatchError>(())
			})?;

			Ok(Some(T::WeightInfo::update_subscription(if let Some(pf) = params.pulse_filter {
				pf.len().try_into().unwrap_or(T::PulseFilterLen::get())
			} else {
				0
			}))
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
			|(sub_id, mut sub): (SubscriptionId, SubscriptionOf<T>)| -> DispatchResult {
				// Filter for active subscriptions that are eligible for delivery based on frequency
				// and custom filter criteria
				if
				// Subscription must be active
				sub.state ==   SubscriptionState::Active   &&
					// And either never delivered before, or enough blocks have passed since last delivery
					(sub.last_delivered.is_none() ||
					current_block >= sub.last_delivered.unwrap() + sub.frequency) &&
 					// The pulse passes the custom filter
					// see the [Pulse Filtering Security](#pulse-filtering-security) section
					Self::custom_filter(&sub.pulse_filter, &pulse)
				{
					let msg = Self::construct_xcm_msg(&pulse, sub.details.call_index);
					let versioned_target: Box<VersionedLocation> =
						Box::new(sub.details.target.clone().into());
					let versioned_msg: Box<VersionedXcm<()>> =
						Box::new(xcm::VersionedXcm::V5(msg.into()));
					let origin = frame_system::RawOrigin::Signed(Self::pallet_account_id());

					// Make sure credits are consumed before sending the XCM message
					let consume_credits = T::FeesManager::get_consume_credits(&sub);
					Self::collect_fees(&sub, T::FeesManager::get_consume_credits(&sub))?;

					// Update subscription with consumed credits and last_delivered block number
					sub.credits_left = sub.credits_left.saturating_sub(consume_credits);
					sub.last_delivered = Some(current_block);

					// Store the updated subscription
					Subscriptions::<T>::insert(sub_id, &sub);

					// Send the XCM message
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
		.balance;
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
	/// subscription details and the current block number.
	///
	/// The subscription ID is generated using a combination of the subscription details and the
	/// current block number to ensure uniqueness and prevent collisions.
	fn generate_sub_id(
		sub_details: &SubscriptionDetailsOf<T>,
		current_block: &BlockNumberFor<T>,
	) -> SubscriptionId {
		let id_tuple = (
			current_block,
			&sub_details.subscriber,
			sub_details.target.clone(),
			sub_details.call_index,
			sub_details.metadata.clone(),
		);
		// Encode the tuple using SCALE codec.
		let encoded = id_tuple.encode();
		// Hash the encoded bytes using blake2_256.
		H256::from_slice(&blake2_256(&encoded)).into()
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

	/// Ensures that a pulse filter doesn't attempt to filter on pulses
	///
	/// This function checks that the filter doesn't use the [`PulseProperty::Rand`] or
	/// [`PulseProperty::Sig`] variant for filtering. Filtering based on randomness is not permitted
	/// because it would contradict the purpose of randomness distribution - clients shouldn't be
	/// able to selectively receive only certain pulses.
	///
	/// # Security Notes
	/// This function is an important security check that prevents subscribers from
	/// potentially gaming the system by receiving only certain pulses that
	/// match specific patterns.
	pub fn ensure_filter_no_rand(pulse_filter: &Option<PulseFilterOf<T>>) -> DispatchResult {
		if let Some(filter) = pulse_filter {
			// Check if any item in the filter is a Rand or Sig variant
			if filter.iter().any(|prop| {
				matches!(prop, PulseProperty::Rand(_)) || matches!(prop, PulseProperty::Sig(_))
			}) {
				return Err(Error::<T>::FilterRandNotPermitted.into());
			}
		}
		Ok(())
	}

	/// Manages the difference in storage deposit for a subscription.
	///
	/// This internal helper function manages the difference in storage deposit for a subscription,
	/// either collecting additional deposit or releasing excess deposit.
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
	/// 1. Execute without payment at the destination chain ([`UnpaidExecution`])
	/// 2. Execute a call on the destination chain using the provided call index and randomness data
	///    ([`Transact`])
	/// 3. Expect successful execution and check status code (ExpectTransactStatus)
	fn construct_xcm_msg(pulse: &T::Pulse, call_index: CallIndex) -> Xcm<()> {
		Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact {
				origin_kind: OriginKind::Xcm,
				fallback_max_weight: None,
				// Create a tuple of call_index and pulse, encode it using SCALE codec, then convert
				// to Vec<u8> The encoded data will be used by the receiving chain to:
				// 1. Find the target pallet using first byte of call_index
				// 2. Find the target function using second byte of call_index
				// 3. Pass the pulse to that function
				call: (call_index, pulse).encode().into(),
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
	pub fn get_subscription(sub_id: &SubscriptionId) -> Option<SubscriptionOf<T>> {
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
	/// Dispatches a given pulse by distributing it to eligible subscriptions.
	///
	/// This function serves as the entry point for distributing randomness pulses
	/// to active subscriptions. It calls the `distribute` function to handle the
	/// actual distribution logic.
	fn dispatch(pulse: T::Pulse) -> DispatchResult {
		Pallet::<T>::distribute(pulse)
	}
}

sp_api::decl_runtime_apis! {
	#[api_version(1)]
	pub trait IdnManagerApi<Balance, Credits, AccountId, Subscription> where
		Balance: Codec,
		Credits: Codec,
		AccountId: Codec,
		Subscription: Codec,
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
