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
//! 3. Finalized
//!   - Subscription is finalized for good
//!   - Cannot be reactivated
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
	primitives::{
		CallIndex, CreateSubParams, Quote, QuoteSubParams, SubInfoRequest, SubInfoResponse,
		SubscriptionMetadata,
	},
	traits::{
		BalanceDirection, DepositCalculator, DiffBalance, FeesError, FeesManager,
		Subscription as SubscriptionTrait,
	},
};
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, EncodeLike, MaxEncodedLen};
use frame_support::{
	pallet_prelude::{
		ensure, Blake2_128Concat, DispatchError, DispatchResult, DispatchResultWithPostInfo,
		EnsureOrigin, Hooks, IsType, OptionQuery, StorageMap, StorageValue, ValueQuery, Weight,
		Zero,
	},
	traits::{
		fungible::{hold::Mutate as HoldMutate, Inspect},
		tokens::Precision,
		Get, OriginTrait,
	},
};
use frame_system::{ensure_signed, pallet_prelude::OriginFor};
use scale_info::TypeInfo;
use sp_arithmetic::traits::Unsigned;
use sp_core::H256;
use sp_idn_traits::{
	pulse::{Dispatcher, Pulse},
	Hashable,
};
use sp_runtime::traits::Saturating;
use sp_std::{boxed::Box, fmt::Debug, vec, vec::Vec};
use xcm::{
	prelude::{Location, OriginKind, Transact, Unlimited, UnpaidExecution, Xcm},
	DoubleEncoded, VersionedLocation, VersionedXcm,
};
use xcm_builder::SendController;
use xcm_executor::traits::ConvertLocation;

pub use frame_system::pallet_prelude::BlockNumberFor;
pub use pallet::*;
pub use weights::WeightInfo;

const LOG_TARGET: &str = "pallet-idn-manager";

/// The balance type used in the pallet, derived from the currency type in the configuration.
pub type BalanceOf<T> = <<T as Config>::Currency as Inspect<AccountIdOf<T>>>::Balance;

/// The metadata type used in the pallet, represented as a bounded vector of bytes.
pub type MetadataOf<T> = SubscriptionMetadata<<T as Config>::MaxMetadataLen>;

/// The quote type used in the pallet, representing a quote for a subscription.
pub type QuoteOf<T> = Quote<BalanceOf<T>>;

/// The quote request type used in the pallet, containing details about the subscription to be
/// quoted.
pub type QuoteSubParamsOf<T> = QuoteSubParams<CreateSubParamsOf<T>>;

pub type SubInfoRequestOf<T> = SubInfoRequest<SubscriptionIdOf<T>>;

/// The subscription ID type used in the pallet, derived from the configuration.
pub type SubscriptionIdOf<T> = <T as pallet::Config>::SubscriptionId;

/// The subscription type used in the pallet, containing various details about the subscription.
pub type SubscriptionOf<T> = Subscription<
	AccountIdOf<T>,
	BlockNumberFor<T>,
	<T as pallet::Config>::Credits,
	MetadataOf<T>,
	SubscriptionIdOf<T>,
>;

/// Represents a subscription in the system.
#[derive(
	Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq, DecodeWithMemTracking,
)]
pub struct Subscription<AccountId, BlockNumber, Credits, Metadata, SubscriptionId> {
	// The subscription ID
	pub id: SubscriptionId,
	// The immutable params of the subscription
	pub details: SubscriptionDetails<AccountId>,
	// Number of random values left to distribute
	pub credits_left: Credits,
	// The state of the subscription
	pub state: SubscriptionState,
	// A timestamp for creation
	pub created_at: BlockNumber,
	// A timestamp for update
	pub updated_at: BlockNumber,
	// Credits subscribed for
	pub credits: Credits,
	// How often to receive pulses
	pub frequency: BlockNumber,
	// Optional metadata that can be used by the subscriber
	pub metadata: Option<Metadata>,
	// Last block in which a pulse was received
	pub last_delivered: Option<BlockNumber>,
}

/// Details specific to a subscription for pulse delivery
///
/// This struct contains information about the subscription owner, where randomness should be
/// delivered, how to deliver it via XCM, and any additional metadata for the subscription.
#[derive(
	Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq, DecodeWithMemTracking,
)]
pub struct SubscriptionDetails<AccountId> {
	// The account that created and pays for the subscription
	pub subscriber: AccountId,
	// The XCM location where randomness should be delivered
	pub target: Location,
	// Identifier for dispatching the XCM call, see [`crate::CallIndex`]
	pub call_index: CallIndex,
}

/// The AccountId type used in the pallet, derived from the configuration.
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// The subscription details type used in the pallet, containing information about the subscription
/// owner and target.
pub type SubscriptionDetailsOf<T> = SubscriptionDetails<AccountIdOf<T>>;

/// The parameters for creating a new subscription, containing various details about the
/// subscription.
pub type CreateSubParamsOf<T> = CreateSubParams<
	<T as pallet::Config>::Credits,
	BlockNumberFor<T>,
	MetadataOf<T>,
	SubscriptionIdOf<T>,
>;

/// Parameters for updating an existing subscription.
///
/// When the parameter is `None`, the field is not updated.
#[derive(
	Encode, Decode, DecodeWithMemTracking, Clone, TypeInfo, MaxEncodedLen, Debug, PartialEq,
)]
pub struct UpdateSubParams<SubId, Credits, BlockNumber, Metadata> {
	// The Subscription Id
	pub sub_id: SubId,
	// New number of credits
	pub credits: Option<Credits>,
	// New distribution interval
	pub frequency: Option<BlockNumber>,
	// Bounded vector for additional data
	pub metadata: Option<Option<Metadata>>,
}

/// The parameters for updating an existing subscription, containing various details about the
/// subscription.
pub type UpdateSubParamsOf<T> = UpdateSubParams<
	<T as Config>::SubscriptionId,
	<T as Config>::Credits,
	BlockNumberFor<T>,
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
/// * [`Finalized`](SubscriptionState::Finalized) - The subscription is finalized and cannot be
///   resumed.
///
/// See [Subscription State Lifecycle](./index.html#subscription-state-lifecycle) for more details.
#[derive(
	Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen, Debug, DecodeWithMemTracking,
)]
pub enum SubscriptionState {
	/// The subscription is active and randomness is being distributed.
	Active,
	/// The subscription is paused and randomness distribution is temporarily suspended.
	Paused,
	/// The subscription is finalized and cannot be resumed.
	Finalized,
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
			Self::Credits,
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
		#[pallet::constant]
		type MaxMetadataLen: Get<u32>;

		/// A type to define the amount of credits in a subscription
		type Credits: Unsigned
			+ Codec
			+ TypeInfo
			+ MaxEncodedLen
			+ Debug
			+ Saturating
			+ Copy
			+ PartialOrd;

		/// Maximum number of subscriptions allowed
		///
		/// This and [`MaxTerminatableSubs`] should be set to a number that keeps the estimated
		/// `dispatch_pulse` Proof size in combinations with the `on_finalize` Proof size under
		/// the relay chain's [`MAX_POV_SIZE`](https://github.com/paritytech/polkadot-sdk/blob/da8c374871cc97807935230e7c398876d5adce62/polkadot/primitives/src/v8/mod.rs#L441)
		#[pallet::constant]
		type MaxSubscriptions: Get<u32>;

		/// Maximum number of subscriptions that can be terminated in a `on_finalize` execution
		///
		/// This and [`MaxSubscriptions`] should be set to a number that keeps the estimated Proof
		/// Size in the weights under the relay chain's [`MAX_POV_SIZE`](https://github.com/paritytech/polkadot-sdk/blob/da8c374871cc97807935230e7c398876d5adce62/polkadot/primitives/src/v8/mod.rs#L441)
		#[pallet::constant]
		type MaxTerminatableSubs: Get<u32>;

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

		/// Origin check for XCM locations that can interact with the IDN Manager.
		///
		/// **Example definition**
		/// ```rust
		/// use pallet_idn_manager::primitives::AllowSiblingsOnly;
		/// type SiblingOrigin = pallet_xcm::EnsureXcm<AllowSiblingsOnly>;
		/// ```
		type SiblingOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Location>;

		/// A type that converts XCM locations to account identifiers.
		type XcmLocationToAccountId: ConvertLocation<Self::AccountId>;
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
		/// A subscription has terminated
		SubscriptionTerminated { sub_id: T::SubscriptionId },
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
		/// Subscription Fees quoted
		SubQuoted { requester: Location, quote: QuoteOf<T> },
		/// Subscription Distributed
		SubscriptionDistributed { sub_id: T::SubscriptionId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// A Subscription already exists
		SubscriptionAlreadyExists,
		/// A Subscription does not exist
		SubscriptionDoesNotExist,
		/// Subscription is already active
		SubscriptionAlreadyActive,
		/// Subscription's state transition is not possible
		SubscriptionInvalidTransition,
		/// Subscription can't be updated
		SubscriptionNotUpdatable,
		/// The origin isn't the subscriber
		NotSubscriber,
		/// Too many subscriptions in storage
		TooManySubscriptions,
		/// Invalid subscriber
		InvalidSubscriber,
		/// Invalid parameters
		InvalidParams,
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
		/// A dummy `on_initialize` to return the amount of weight that `on_finalize` requires to
		/// execute. See [`on_finalize`](https://paritytech.github.io/polkadot-sdk/master/frame_support/traits/trait.Hooks.html#method.on_finalize).
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			<T as pallet::Config>::WeightInfo::on_finalize()
		}

		/// It iterates over all [`Finalized`](SubscriptionState::Finalized) subscriptions calls
		/// `terminate_subscription`. `terminate_subscription` involves refunding deposits,
		/// removing the subscription from storage and dispatching events, among other actions.
		fn on_finalize(_n: BlockNumberFor<T>) {
			// Look for subscriptions that should be terminated
			for (sub_id, sub) in Subscriptions::<T>::iter()
				.filter(|(_, sub)| sub.state == SubscriptionState::Finalized)
				.take(T::MaxTerminatableSubs::get() as usize)
			{
				// terminate the subscription
				let _ = Self::terminate_subscription(&sub, sub_id);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a subscription in the pallet.
		///
		/// This function allows a user to create a new subscription for receiving pulses.
		/// The subscription includes details such as the number of credits, target location, call
		/// index, frequency and metadata.
		///
		/// # Errors
		/// * [`TooManySubscriptions`](Error::TooManySubscriptions) - If the maximum number of
		///   subscriptions has been reached.
		/// * [`SubscriptionAlreadyExists`](Error::SubscriptionAlreadyExists) - If a subscription
		///   with the specified ID already exists.
		///
		/// # Events
		/// * [`SubscriptionCreated`](Event::SubscriptionCreated) - A new subscription has been
		///   created.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_subscription())]
		#[allow(clippy::useless_conversion)]
		pub fn create_subscription(
			origin: OriginFor<T>,
			params: CreateSubParamsOf<T>,
		) -> DispatchResultWithPostInfo {
			log::trace!(target: LOG_TARGET, "Create subscription request received: {:?}", params);
			let subscriber = Self::ensure_signed_or_xcm_sibling(origin)?;

			ensure!(params.credits != Zero::zero(), Error::<T>::InvalidParams);

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

			Ok(Some(T::WeightInfo::create_subscription()).into())
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
		/// * [`SubscriptionInvalidTransition`](Error::SubscriptionInvalidTransition) - If the
		///   subscription cannot transition to the [`Paused`](SubscriptionState::Paused) state.
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
			log::trace!(target: LOG_TARGET, "Pause subscription request received: {:?}", sub_id);
			let subscriber = Self::ensure_signed_or_xcm_sibling(origin)?;
			Subscriptions::<T>::try_mutate(sub_id, |maybe_sub| {
				let sub = maybe_sub.as_mut().ok_or(Error::<T>::SubscriptionDoesNotExist)?;
				ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);
				ensure!(
					sub.state == SubscriptionState::Active,
					Error::<T>::SubscriptionInvalidTransition
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
		/// * [`SubscriptionTerminated`](Event::SubscriptionTerminated) - A subscription has been
		///   terminated.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::kill_subscription())]
		pub fn kill_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			sub_id: T::SubscriptionId,
		) -> DispatchResult {
			log::trace!(target: LOG_TARGET, "Kill subscription request received: {:?}", sub_id);
			let subscriber = Self::ensure_signed_or_xcm_sibling(origin)?;

			let sub =
				Self::get_subscription(&sub_id).ok_or(Error::<T>::SubscriptionDoesNotExist)?;
			ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);

			Self::terminate_subscription(&sub, sub_id)
		}

		/// Updates a subscription.
		///
		/// This function allows the subscriber to modify their subscription parameters,
		/// such as the number of credits and distribution interval.
		///
		/// # Errors
		/// * [`SubscriptionDoesNotExist`](Error::SubscriptionDoesNotExist) - If a subscription with
		///   the specified ID does not exist.
		/// * [`NotSubscriber`](Error::NotSubscriber) - If the origin is not the subscriber of the
		///   specified subscription.
		///
		/// # Events
		/// * [`SubscriptionUpdated`](Event::SubscriptionUpdated) - A subscription has been updated.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::update_subscription(T::MaxMetadataLen::get()))]
		#[allow(clippy::useless_conversion)]
		pub fn update_subscription(
			// Must match the subscription's original caller
			origin: OriginFor<T>,
			params: UpdateSubParamsOf<T>,
		) -> DispatchResultWithPostInfo {
			log::trace!(target: LOG_TARGET, "Update subscription request received: {:?}", params);
			let subscriber = Self::ensure_signed_or_xcm_sibling(origin)?;

			Subscriptions::<T>::try_mutate(params.sub_id, |maybe_sub| {
				let sub = maybe_sub.as_mut().ok_or(Error::<T>::SubscriptionDoesNotExist)?;
				ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);
				ensure!(
					sub.state == SubscriptionState::Active ||
						sub.state == SubscriptionState::Paused,
					Error::<T>::SubscriptionNotUpdatable
				);

				let mut fees_diff = T::DiffBalance::new(Zero::zero(), BalanceDirection::None);

				if let Some(credits) = params.credits {
					fees_diff = T::FeesManager::calculate_diff_fees(&sub.credits, &credits);
					sub.credits = credits;
				}

				if let Some(frequency) = params.frequency {
					sub.frequency = frequency;
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

			Ok(Some(T::WeightInfo::update_subscription(if let Some(Some(md)) = params.metadata {
				md.len().try_into().unwrap_or(T::MaxMetadataLen::get())
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
			sub_id: T::SubscriptionId,
		) -> DispatchResult {
			log::trace!(target: LOG_TARGET, "Reactivating subscription: {:?}", sub_id);
			let subscriber = Self::ensure_signed_or_xcm_sibling(origin)?;
			Subscriptions::<T>::try_mutate(sub_id, |maybe_sub| {
				let sub = maybe_sub.as_mut().ok_or(Error::<T>::SubscriptionDoesNotExist)?;
				ensure!(sub.details.subscriber == subscriber, Error::<T>::NotSubscriber);
				ensure!(
					sub.state == SubscriptionState::Paused,
					Error::<T>::SubscriptionInvalidTransition
				);
				sub.state = SubscriptionState::Active;
				Self::deposit_event(Event::SubscriptionReactivated { sub_id });
				Ok(())
			})
		}

		/// Calculates the subscription fees and deposit and sends the result back to the caller
		/// specified function via XCM.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::quote_subscription())]
		#[allow(clippy::useless_conversion)]
		pub fn quote_subscription(
			origin: OriginFor<T>,
			params: QuoteSubParamsOf<T>,
		) -> DispatchResultWithPostInfo {
			log::trace!(target: LOG_TARGET, "Quote subscription request received: {:?}", params);
			// Ensure the origin is a sibling, and get the location
			let requester: Location = T::SiblingOrigin::ensure_origin(origin.clone())?;

			let deposit  = Self::calculate_storage_deposit_from_create_params(
				&T::XcmLocationToAccountId::convert_location(&requester)
					.ok_or_else(|| {
						log::warn!(target: LOG_TARGET, "InvalidSubscriber: failed to convert XCM location to AccountId");
						Error::<T>::InvalidSubscriber
					})?,
				&params.quote_request.create_sub_params,
			);
			let fees =
				Self::calculate_subscription_fees(&params.quote_request.create_sub_params.credits);

			let quote = Quote { req_ref: params.quote_request.req_ref, fees, deposit };

			Self::xcm_send(&requester, (params.call_index, quote.clone()).encode().into())?;
			Self::deposit_event(Event::SubQuoted { requester, quote });

			Ok(Some(T::WeightInfo::quote_subscription()).into())
		}

		/// Gets a subscription by its ID and sends the result back to the caller
		/// specified function via XCM.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::get_subscription_info())]
		pub fn get_subscription_info(
			origin: OriginFor<T>,
			req: SubInfoRequestOf<T>,
		) -> DispatchResult {
			log::trace!(target: LOG_TARGET, "Get subscription info request received: {:?}", req);
			// Ensure the origin is a sibling, and get the location
			let requester: Location = T::SiblingOrigin::ensure_origin(origin.clone())?;

			let sub =
				Self::get_subscription(&req.sub_id).ok_or(Error::<T>::SubscriptionDoesNotExist)?;

			let response = SubInfoResponse { sub, req_ref: req.req_ref };
			Self::xcm_send(&requester, (req.call_index, response).encode().into())?;
			Self::deposit_event(Event::SubscriptionDistributed { sub_id: req.sub_id });

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
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

	/// Calculates the storage deposit for a subscription based on its creation parameters.
	///
	/// It creates an ephimeral dummy subscription to calculate the fees.
	pub(crate) fn calculate_storage_deposit_from_create_params(
		subscriber: &T::AccountId,
		params: &CreateSubParamsOf<T>,
	) -> BalanceOf<T> {
		let current_block = frame_system::Pallet::<T>::block_number();

		let details = SubscriptionDetails {
			subscriber: subscriber.clone(),
			target: params.target.clone(),
			call_index: params.call_index,
		};

		let subscription = Subscription {
			id: SubscriptionIdOf::<T>::from(H256::default()),
			state: SubscriptionState::Active,
			credits_left: params.credits,
			details,
			created_at: current_block,
			updated_at: current_block,
			credits: params.credits,
			frequency: params.frequency,
			metadata: params.metadata.clone(),
			last_delivered: None,
		};
		T::DepositCalculator::calculate_storage_deposit(&subscription)
	}

	/// Terminates a subscription by removing it from storage and emitting a terminate event.
	pub(crate) fn terminate_subscription(
		sub: &SubscriptionOf<T>,
		sub_id: T::SubscriptionId,
	) -> DispatchResult {
		log::trace!(target: LOG_TARGET, "Terminating subscription: {:?}", sub_id);
		// fees left and deposit to refund
		let fees_diff = T::FeesManager::calculate_diff_fees(&sub.credits_left, &Zero::zero());
		let sd = T::DepositCalculator::calculate_storage_deposit(sub);

		Self::manage_diff_fees(&sub.details.subscriber, &fees_diff)?;
		Self::release_deposit(&sub.details.subscriber, sd)?;

		Subscriptions::<T>::remove(sub_id);

		// Decrease the subscription counter
		SubCounter::<T>::mutate(|c| c.saturating_dec());

		Self::deposit_event(Event::SubscriptionTerminated { sub_id });
		Ok(())
	}

	pub(crate) fn get_min_credits(sub: &SubscriptionOf<T>) -> T::Credits {
		T::FeesManager::get_idle_credits(sub)
	}

	/// Distribute randomness to subscribers
	/// Returns a weight based on the number of storage reads and writes performed
	fn distribute(pulse: T::Pulse) {
		// Get the current block number once for comparison
		let current_block = frame_system::Pallet::<T>::block_number();

		for (sub_id, mut sub) in Subscriptions::<T>::iter() {
			// Filter for active subscriptions that are eligible for delivery based on frequency
			if
			// Subscription must be active
			sub.state ==  SubscriptionState::Active   &&
					// And either never delivered before, or enough blocks have passed since last delivery
					(sub.last_delivered.is_none() ||
					current_block >= sub.last_delivered.unwrap() + sub.frequency)
			{
				// Make sure credits are consumed before sending the XCM message
				let consume_credits = T::FeesManager::get_consume_credits(&sub);

				if let Err(e) = Self::collect_fees(&sub, consume_credits) {
					Self::pause_subscription_on_error(sub_id, sub, "Failed to collect fees", e);
					continue;
				}

				// Update subscription with consumed credits and last_delivered block number
				sub.credits_left = sub.credits_left.saturating_sub(consume_credits);
				sub.last_delivered = Some(current_block);

				// Send the XCM message
				if let Err(e) = Self::xcm_send(
					&sub.details.target,
					// Create a tuple of call_index and pulse, encode it using SCALE codec,
					// then convert to Vec<u8> The encoded data will be used by the
					// receiving chain to:
					// 1. Find the target pallet using first byte of call_index
					// 2. Find the target function using second byte of call_index
					// 3. Pass the parameters to that function
					(sub.details.call_index, &pulse, &sub_id).encode().into(),
				) {
					Self::pause_subscription_on_error(sub_id, sub, "Failed to dispatch XCM", e);
					continue;
				}

				Self::deposit_event(Event::RandomnessDistributed { sub_id });
			} else {
				let idle_credits = T::FeesManager::get_idle_credits(&sub);
				// Collect fees for idle block
				if let Err(e) = Self::collect_fees(&sub, idle_credits) {
					log::warn!(
						target: LOG_TARGET,
						"Failed to collect fees for idle subscription id = {:?}: {:?}", sub_id, e);
				}
				// Update subscription with consumed credits
				sub.credits_left = sub.credits_left.saturating_sub(idle_credits);
			}
			// Finalize the subscription if there are not enough credits left
			if sub.state != SubscriptionState::Finalized &&
				sub.credits_left < Self::get_min_credits(&sub)
			{
				sub.state = SubscriptionState::Finalized;
			}

			// Store the updated subscription
			Subscriptions::<T>::insert(sub_id, &sub);
		}
	}

	/// Gracefully handles the pausing of a subscription in the case an error occurs
	fn pause_subscription_on_error(
		sub_id: SubscriptionIdOf<T>,
		mut sub: SubscriptionOf<T>,
		message: &str,
		err: DispatchError,
	) {
		log::warn!(
			target: LOG_TARGET,
			"{}: subscription id = {:?}. Pausing subscription due to: {:?}",
			message,
			sub_id,
			err
		);
		sub.state = SubscriptionState::Paused;
		Subscriptions::<T>::insert(sub_id, &sub);
		Self::deposit_event(Event::SubscriptionPaused { sub_id });
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
		params.hash(&salt).into()
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

	/// Dispatches the call to the target
	/// WARNING: Possible attack vector, as the `Xcm::send` call's origin is different to the
	/// account that created the subscription that feeds the target and the transact msg.
	/// Also, this is requesting `UnpaidExecution`, which is expected to be honored by the
	/// target chain.
	/// A potential attacker could try to get "almost" free execution (only paying the pulse
	/// consumption fees) to any target on a chain that honors `UnpaidExecution` requests from IDN.
	/// Though the attacker would not be able to manipulate the call's parameters.
	// TODO: Solve this issue https://github.com/ideal-lab5/idn-sdk/issues/290
	fn xcm_send(target: &Location, call: DoubleEncoded<()>) -> DispatchResult {
		let msg = Xcm(vec![
			UnpaidExecution { weight_limit: Unlimited, check_origin: None },
			Transact { origin_kind: OriginKind::Xcm, fallback_max_weight: None, call },
		]);
		let versioned_target: Box<VersionedLocation> =
			Box::new(VersionedLocation::V5(target.clone()));
		let versioned_msg: Box<VersionedXcm<()>> = Box::new(VersionedXcm::V5(msg.into()));
		// TODO: do not use root origin, instead bring down the origin all the way from
		// `try_submit_asig` to here https://github.com/ideal-lab5/idn-sdk/issues/289
		let origin = T::RuntimeOrigin::root();
		let xcm_hash =
			T::Xcm::send(origin.clone(), versioned_target.clone(), versioned_msg.clone())?;
		log::trace!(
			target: LOG_TARGET,
			"XCM message sent with hash: {:?}. Target: {:?}. Msg: {:?}",
			xcm_hash,
			versioned_target,
			versioned_msg
		);
		Ok(())
	}

	/// Validates the origin as either a signed origin or an XCM sibling origin.
	///
	/// # Parameters
	/// - `origin`: The origin to validate, which can be either a signed origin or an XCM sibling
	///   origin.
	///
	/// # Returns
	/// - `Ok(T::AccountId)`: The account ID of the signed origin or the account ID derived from the
	///   XCM sibling origin.
	/// - `Err(DispatchError)`: An error if the origin is invalid or cannot be converted to an
	///   account ID.
	fn ensure_signed_or_xcm_sibling(origin: OriginFor<T>) -> Result<T::AccountId, DispatchError> {
		match T::SiblingOrigin::ensure_origin(origin.clone()) {
			Ok(location) => T::XcmLocationToAccountId::convert_location(&location)
				.ok_or(DispatchError::from(Error::<T>::InvalidSubscriber)),
			Err(_) => ensure_signed(origin).map_err(|e| e.into()),
		}
	}

	/// Return the existential deposit of [`Config::Currency`].
	#[cfg(any(test, feature = "runtime-benchmarks"))]
	fn min_balance() -> BalanceOf<T> {
		// Get the minimum balance for the currency used in the pallet
		<T::Currency as Inspect<AccountIdOf<T>>>::minimum_balance()
	}
}

impl<T: Config> Dispatcher<T::Pulse> for Pallet<T> {
	/// Dispatches a pulse by distributing it to eligible subscriptions.
	///
	/// This function serves as the entry point for distributing randomness pulses
	/// to active subscriptions. It calls the `distribute` function to handle the
	/// actual distribution logic.
	fn dispatch(pulse: T::Pulse) {
		Pallet::<T>::distribute(pulse);
	}

	fn dispatch_weight() -> Weight {
		T::WeightInfo::dispatch_pulse(SubCounter::<T>::get())
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
