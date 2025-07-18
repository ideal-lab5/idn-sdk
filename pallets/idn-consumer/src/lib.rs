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
//!
//! ## Overview
//!
//! The `pallet-idn-consumer` provides functionality for interacting with the Ideal Network (IDN) as
//! a consumer. It allows parachains to subscribe to randomness pulses, request subscription quotes,
//! and manage subscription states via XCM.
//!
//! ## Key Features
//!
//! - **Subscription Management:** Create, update, pause, reactivate, and terminate subscriptions.
//! - **Randomness Consumption:** Consume randomness pulses delivered by the IDN.
//! - **Quote Requests:** Request and consume subscription fee quotes.
//! - **Subscription Info:** Retrieve and consume subscription details.
//! - **XCM Integration:** Seamlessly interact with the IDN Manager pallet on the Ideal Network.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod traits;
pub mod weights;

use bp_idn::{
	types::{
		BlockNumber as IdnBlockNumber, CallIndex, CreateSubParams, Credits, Metadata, QuoteRequest,
		QuoteSubParams, RequestReference, SubInfoRequest, UpdateSubParams,
	},
	Call as RuntimeCall, IdnManagerCall,
};
use cumulus_primitives_core::{Instruction::WithdrawAsset, ParaId};
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	pallet_prelude::{Decode, DecodeWithMemTracking, Encode, EnsureOrigin, Get, IsType, Pays},
	sp_runtime::traits::AccountIdConversion,
};
use frame_system::{ensure_root, pallet_prelude::OriginFor, RawOrigin};
use scale_info::{
	prelude::{boxed::Box, sync::Arc, vec},
	TypeInfo,
};
use sp_idn_traits::Hashable;
use traits::{PulseConsumer, QuoteConsumer, SubInfoConsumer};
use xcm::{
	v5::{
		prelude::{BuyExecution, DepositAsset, OriginKind, RefundSurplus, Transact, Xcm},
		Asset,
		AssetFilter::Wild,
		AssetId, Junction, Junctions, Location,
		WeightLimit::Unlimited,
		WildAsset::AllOf,
		WildFungibility,
	},
	VersionedLocation, VersionedXcm,
};
use xcm_builder::SendController;

pub use bp_idn::types::{Quote, RuntimePulse as Pulse, SubInfoResponse, SubscriptionId};
pub use pallet::*;
pub use weights::WeightInfo;

#[derive(Clone, PartialEq, Debug, Encode, Decode, TypeInfo, DecodeWithMemTracking)]
struct SubFeesQuote {
	quote_id: u8,
	fees: Credits,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	/// Configuration trait for the pallet.
	///
	/// This trait defines the types and constants required to configure the pallet.
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// An implementation of the [`PulseConsumer`] trait, which defines how to consume a pulse.
		type PulseConsumer: PulseConsumer<Pulse, SubscriptionId, (), ()>;

		/// An implementation of the [`QuoteConsumer`] trait, which defines how to consume a quote.
		type QuoteConsumer: QuoteConsumer<Quote, (), ()>;

		/// An implementation of the [`SubInfoConsumer`] trait, which defines how to consume
		/// subscription info.
		type SubInfoConsumer: SubInfoConsumer<SubInfoResponse, (), ()>;

		/// The location of the sibling IDN chain.
		///
		/// Example definition:
		/// ```nocompile
		/// parameter_types! {
		/// 	pub SiblingIdnLocation: Location = Location::new(1, Parachain(IDN_PARACHAIN_ID));
		/// }
		/// impl pallet_idn_consumer::Config for Runtime {
		/// 	pub type SiblingIdnLocation = SiblingIdnLocation;
		/// }
		/// ```
		#[pallet::constant]
		type SiblingIdnLocation: Get<Location>;

		/// The origin of the IDN chain.
		///
		/// Example definition:
		/// ```nocompile
		/// type IdnOrigin = EnsureXcm<Equals<Self::SiblingIdnLocation>>
		/// ```
		type IdnOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Location>;

		/// A type that exposes XCM APIs, allowing contracts to interact with other parachains and
		/// execute XCM programs.
		type Xcm: xcm_builder::SendController<OriginFor<Self>>;

		/// This pallet's unique identifier.
		#[pallet::constant]
		type PalletId: Get<frame_support::PalletId>;

		/// The parachain ID of this chain.
		///
		/// Example definition:
		/// ```nocompile
		/// pub ParaId: ParaId = ParachainInfo::parachain_id();
		/// ```
		#[pallet::constant]
		type ParaId: Get<ParaId>;

		/// The maximum amount of fees to pay for the execution of a single XCM message sent to the
		/// IDN chain, expressed in the IDN asset.
		#[pallet::constant]
		type MaxIdnXcmFees: Get<u128>;

		/// The weight information for this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	/// Events emitted by this pallet.
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A randomness pulse was successfully consumed.
		///
		/// - `sub_id`: The subscription ID associated with the pulse.
		RandomnessConsumed { sub_id: SubscriptionId },
		/// A subscription quote was successfully consumed.
		///
		/// - `quote`: The subscription quote that was consumed.
		QuoteConsumed { quote: Quote },

		/// Subscription info was successfully consumed.
		///
		/// - `sub_id`: The subscription ID associated with the consumed info.
		SubInfoConsumed { sub_id: SubscriptionId },
	}

	#[pallet::error]
	/// Errors that may occur in this pallet.
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// An error occurred while consuming the pulse.
		ConsumePulseError,

		/// An error occurred while consuming the quote.
		ConsumeQuoteError,

		/// An error occurred while consuming the subscription info.
		ConsumeSubInfoError,

		/// An error occurred while converting the pallet index to a `u8`.
		PalletIndexConversionError,

		/// An error occurred while sending the XCM message.
		XcmSendError,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Consumes a randomness pulse from the IDN chain.
		///
		/// This function processes randomness pulses delivered by the IDN chain. The logic for
		/// handling the pulse is defined in the [`PulseConsumer`] trait implementation.
		///
		/// # Parameters
		/// - `origin`: The origin of the call. Must be from the IDN chain, verified using the
		///   [`Config::IdnOrigin`] type.
		/// - `pulse`: The randomness pulse to be consumed.
		/// - `sub_id`: The subscription ID associated with the pulse.
		///
		/// # Errors
		/// - [`Error::ConsumePulseError`]: If the pulse cannot be consumed.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::consume_pulse())]
		#[allow(clippy::useless_conversion)]
		pub fn consume_pulse(
			origin: OriginFor<T>,
			pulse: Pulse,
			sub_id: SubscriptionId,
		) -> DispatchResultWithPostInfo {
			// ensure origin is coming from IDN
			let _ = T::IdnOrigin::ensure_origin(origin)?;

			T::PulseConsumer::consume_pulse(pulse, sub_id)
				.map_err(|_| Error::<T>::ConsumePulseError)?;

			Self::deposit_event(Event::RandomnessConsumed { sub_id });

			Ok(Pays::No.into())
		}

		/// Consumes a subscription quote from the IDN chain.
		///
		/// This function processes subscription fee quotes received from the IDN chain. The
		/// behavior for handling the quote is defined in the [`QuoteConsumer`] trait
		/// implementation.
		///
		/// # Parameters
		/// - `origin`: The origin of the call. Must be from the IDN chain, verified using the
		///   [`Config::IdnOrigin`] type.
		/// - `quote`: The subscription quote to be consumed.
		///
		/// # Errors
		/// - [`Error::ConsumeQuoteError`]: If the quote cannot be consumed.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::consume_quote())]
		#[allow(clippy::useless_conversion)]
		pub fn consume_quote(origin: OriginFor<T>, quote: Quote) -> DispatchResultWithPostInfo {
			// ensure origin is coming from IDN
			let _ = T::IdnOrigin::ensure_origin(origin)?;

			T::QuoteConsumer::consume_quote(quote.clone())
				.map_err(|_| Error::<T>::ConsumeQuoteError)?;

			Self::deposit_event(Event::QuoteConsumed { quote });

			Ok(Pays::No.into())
		}

		/// Consumes subscription info from the IDN chain.
		///
		/// This function processes subscription information received from the IDN chain. The
		/// behavior for handling the subscription info is defined in the [`SubInfoConsumer`] trait
		/// implementation.
		///
		/// # Parameters
		/// - `origin`: The origin of the call. Must be from the IDN chain, verified using the
		///   [`Config::IdnOrigin`] type.
		/// - `sub_info`: The subscription information to be consumed.
		///
		/// # Errors
		/// - [`Error::ConsumeSubInfoError`]: If the subscription info cannot be consumed.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::consume_sub_info())]
		#[allow(clippy::useless_conversion)]
		pub fn consume_sub_info(
			origin: OriginFor<T>,
			sub_info: SubInfoResponse,
		) -> DispatchResultWithPostInfo {
			// ensure origin is coming from IDN
			let _ = T::IdnOrigin::ensure_origin(origin)?;

			T::SubInfoConsumer::consume_sub_info(sub_info.clone())
				.map_err(|_| Error::<T>::ConsumeSubInfoError)?;

			Self::deposit_event(Event::SubInfoConsumed { sub_id: sub_info.sub.id });

			Ok(Pays::No.into())
		}

		/// Dispatchable function to create a subscription on the consumer chain with sudo
		/// privileges.
		///
		/// # Parameters
		/// - `origin`: Must be the root (sudo) origin.
		/// - `credits`: Number of random values to receive.
		/// - `frequency`: Distribution interval for pulses.
		/// - `metadata`: Optional additional data for the subscription.
		/// - `sub_id`: Optional subscription ID. If `None`, a new one will be generated.
		///
		/// # Returns
		/// - [`DispatchResultWithPostInfo`]: Returns `Ok(Pays::No)` if successful.
		///
		/// # Errors
		/// - Fails if the origin is not root.
		/// - Fails if subscription creation fails (see [`Self::create_subscription`]).
		///
		/// # Usage
		/// This function is intended for privileged (sudo) operations, such as testing or
		/// administrative tasks, to create a subscription directly on the consumer chain.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::sudo_create_subscription())]
		#[allow(clippy::useless_conversion)]
		pub fn sudo_create_subscription(
			origin: OriginFor<T>,
			credits: Credits,
			frequency: IdnBlockNumber,
			metadata: Option<Metadata>,
			sub_id: Option<SubscriptionId>,
		) -> DispatchResultWithPostInfo {
			// Ensure the origin is a sudo origin
			ensure_root(origin)?;

			Self::create_subscription(credits, frequency, metadata, sub_id)?;
			Ok(Pays::No.into())
		}

		/// Dispatchable function to pause a subscription on the consumer chain with sudo
		/// privileges.
		///
		/// # Parameters
		/// - `origin`: Must be the root (sudo) origin.
		/// - `sub_id`: The subscription ID to pause.
		///
		/// # Returns
		/// - [`DispatchResultWithPostInfo`]: Returns `Ok(Pays::No)` if successful.
		///
		/// # Errors
		/// - Fails if the origin is not root.
		/// - Fails if pausing the subscription fails.
		///
		/// # Usage
		/// This function is intended for privileged (sudo) operations, such as testing or
		/// administrative tasks, to create a subscription directly on the consumer chain.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::sudo_pause_subscription())]
		#[allow(clippy::useless_conversion)]
		pub fn sudo_pause_subscription(
			origin: OriginFor<T>,
			sub_id: SubscriptionId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::pause_subscription(sub_id)?;
			Ok(Pays::No.into())
		}

		/// Dispatchable function to kill a subscription on the consumer chain with sudo privileges.
		///
		/// # Parameters
		/// - `origin`: Must be the root (sudo) origin.
		/// - `sub_id`: The subscription ID to kill.
		///
		/// # Returns
		/// - [`DispatchResultWithPostInfo`]: Returns `Ok(Pays::No)` if successful.
		///
		/// # Errors
		/// - Fails if the origin is not root.
		/// - Fails if killing the subscription fails.
		///
		/// # Usage
		/// This function is intended for privileged (sudo) operations, such as testing or
		/// administrative tasks, to create a subscription directly on the consumer chain.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::sudo_kill_subscription())]
		#[allow(clippy::useless_conversion)]
		pub fn sudo_kill_subscription(
			origin: OriginFor<T>,
			sub_id: SubscriptionId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::kill_subscription(sub_id)?;
			Ok(Pays::No.into())
		}

		/// Dispatchable function to update a subscription on the consumer chain with sudo
		/// privileges.
		///
		/// # Parameters
		/// - `origin`: Must be the root (sudo) origin.
		/// - `sub_id`: The subscription ID to update.
		/// - `credits`: Optional new number of random values to receive.
		/// - `frequency`: Optional new distribution interval for pulses.
		/// - `metadata`: Optional new metadata for the subscription.
		///
		/// # Returns
		/// - [`DispatchResultWithPostInfo`]: Returns `Ok(Pays::No)` if successful.
		///
		/// # Errors
		/// - Fails if the origin is not root.
		/// - Fails if updating the subscription fails.
		///
		/// # Usage
		/// This function is intended for privileged (sudo) operations, such as testing or
		/// administrative tasks, to create a subscription directly on the consumer chain.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::sudo_update_subscription())]
		#[allow(clippy::useless_conversion)]
		pub fn sudo_update_subscription(
			origin: OriginFor<T>,
			sub_id: SubscriptionId,
			credits: Option<Credits>,
			frequency: Option<IdnBlockNumber>,
			metadata: Option<Option<Metadata>>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::update_subscription(sub_id, credits, frequency, metadata)?;
			Ok(Pays::No.into())
		}

		/// Dispatchable function to reactivate a subscription on the consumer chain with sudo
		/// privileges.
		///
		/// # Parameters
		/// - `origin`: Must be the root (sudo) origin.
		/// - `sub_id`: The subscription ID to reactivate.
		///
		/// # Returns
		/// - [`DispatchResultWithPostInfo`]: Returns `Ok(Pays::No)` if successful.
		///
		/// # Errors
		/// - Fails if the origin is not root.
		/// - Fails if reactivating the subscription fails.
		///
		/// # Usage
		/// This function is intended for privileged (sudo) operations, such as testing or
		/// administrative tasks, to create a subscription directly on the consumer chain.
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::sudo_reactivate_subscription())]
		#[allow(clippy::useless_conversion)]
		pub fn sudo_reactivate_subscription(
			origin: OriginFor<T>,
			sub_id: SubscriptionId,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::reactivate_subscription(sub_id)?;
			Ok(Pays::No.into())
		}

		/// Dispatchable function to request a subscription quote from the consumer chain with sudo
		/// privileges.
		///
		/// # Parameters
		/// - `origin`: Must be the root (sudo) origin.
		/// - `credits`: Number of random values to receive.
		/// - `frequency`: Distribution interval for pulses.
		/// - `metadata`: Optional additional data for the subscription.
		/// - `sub_id`: Optional subscription ID.
		/// - `req_ref`: Optional quote request reference.
		///
		/// # Returns
		/// - [`DispatchResultWithPostInfo`]: Returns `Ok(Pays::No)` if successful.
		///
		/// # Errors
		/// - Fails if the origin is not root.
		/// - Fails if requesting the quote fails.
		///
		/// # Usage
		/// This function is intended for privileged (sudo) operations, such as testing or
		/// administrative tasks, to create a subscription directly on the consumer chain.
		#[pallet::call_index(8)]
		#[pallet::weight(T::WeightInfo::sudo_request_quote())]
		#[allow(clippy::useless_conversion)]
		pub fn sudo_request_quote(
			origin: OriginFor<T>,
			credits: Credits,
			frequency: IdnBlockNumber,
			metadata: Option<Metadata>,
			sub_id: Option<SubscriptionId>,
			req_ref: Option<RequestReference>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::request_quote(credits, frequency, metadata, sub_id, req_ref)?;
			Ok(Pays::No.into())
		}

		/// Dispatchable function to request subscription info from the consumer chain with sudo
		/// privileges.
		///
		/// # Parameters
		/// - `origin`: Must be the root (sudo) origin.
		/// - `sub_id`: The subscription ID to request info for.
		/// - `req_ref`: Optional quote request reference.
		///
		/// # Returns
		/// - [`DispatchResultWithPostInfo`]: Returns `Ok(Pays::No)` if successful.
		///
		/// # Errors
		/// - Fails if the origin is not root.
		/// - Fails if requesting the subscription info fails.
		///
		/// # Usage
		/// This function is intended for privileged (sudo) operations, such as testing or
		/// administrative tasks, to create a subscription directly on the consumer chain.
		#[pallet::call_index(9)]
		#[pallet::weight(T::WeightInfo::sudo_request_sub_info())]
		#[allow(clippy::useless_conversion)]
		pub fn sudo_request_sub_info(
			origin: OriginFor<T>,
			sub_id: SubscriptionId,
			req_ref: Option<RequestReference>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Self::request_sub_info(sub_id, req_ref)?;
			Ok(Pays::No.into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Creates a subscription.
	pub fn create_subscription(
		// Number of random values to receive
		credits: Credits,
		// Distribution interval for pulses
		frequency: IdnBlockNumber,
		// Bounded vector for additional data
		metadata: Option<Metadata>,
		// Optional Subscription Id, if None, a new one will be generated
		sub_id: Option<SubscriptionId>,
	) -> Result<SubscriptionId, Error<T>> {
		let mut params = CreateSubParams {
			credits,
			target: Self::pallet_location()?,
			// the `0` on the second element is the call index for the `consume` call
			call_index: Self::pulse_callback_index()?,
			frequency,
			metadata,
			sub_id,
		};

		// If `sub_id` is not provided, generate a new one and assign it to the params
		let sub_id = match sub_id {
			Some(sub_id) => sub_id,
			None => {
				let salt = frame_system::Pallet::<T>::block_number().encode();
				let sub_id = params.hash(&salt).into();
				params.sub_id = Some(sub_id);
				sub_id
			},
		};

		let call = RuntimeCall::IdnManager(IdnManagerCall::create_subscription { params });

		Self::xcm_send(call)?;

		Ok(sub_id)
	}

	/// Pauses a subscription.
	pub fn pause_subscription(sub_id: SubscriptionId) -> Result<(), Error<T>> {
		let call = RuntimeCall::IdnManager(IdnManagerCall::pause_subscription { sub_id });
		Self::xcm_send(call)
	}

	/// Kills a subscription.
	pub fn kill_subscription(sub_id: SubscriptionId) -> Result<(), Error<T>> {
		let call = RuntimeCall::IdnManager(IdnManagerCall::kill_subscription { sub_id });
		Self::xcm_send(call)
	}

	/// Updates a subscription.
	pub fn update_subscription(
		sub_id: SubscriptionId,
		credits: Option<Credits>,
		frequency: Option<IdnBlockNumber>,
		metadata: Option<Option<Metadata>>,
	) -> Result<(), Error<T>> {
		let params = UpdateSubParams { sub_id, credits, frequency, metadata };

		let call = RuntimeCall::IdnManager(IdnManagerCall::update_subscription { params });

		Self::xcm_send(call)
	}

	/// Reactivates a subscription.
	pub fn reactivate_subscription(sub_id: SubscriptionId) -> Result<(), Error<T>> {
		let call = RuntimeCall::IdnManager(IdnManagerCall::reactivate_subscription { sub_id });
		Self::xcm_send(call)
	}

	/// Request a subscription quote for a given number of credits and frequency.
	///
	/// This function dispatches an XCM message to the IDN chain to get the fee quote.
	/// The request should then be handled by the IDN chain and return the fee quote to the
	/// [`Pallet::consume_quote`] function along with the `req_ref`.
	/// The `req_ref` is generated by this function and used to identify the request when returned.
	pub fn request_quote(
		// Number of random values to receive
		credits: Credits,
		// Distribution interval for pulses
		frequency: IdnBlockNumber,
		// Bounded vector for additional data
		metadata: Option<Metadata>,
		// Optional Subscription Id
		sub_id: Option<SubscriptionId>,
		// Optional quote request reference, if None, a new one will be generated
		req_ref: Option<RequestReference>,
	) -> Result<RequestReference, Error<T>> {
		let create_sub_params = CreateSubParams {
			credits,
			target: Self::pallet_location()?,
			// the `0` on the second element is the call index for the `consume` call
			call_index: Self::pulse_callback_index()?,
			frequency,
			metadata,
			sub_id,
		};

		// If `req_ref` is not provided, generate a new one and assign it to the params
		let req_ref = match req_ref {
			Some(req_ref) => req_ref,
			None => {
				let salt = frame_system::Pallet::<T>::block_number().encode();
				create_sub_params.hash(&salt).into()
			},
		};

		let quote_request = QuoteRequest { req_ref, create_sub_params };

		let params = QuoteSubParams { quote_request, call_index: Self::quote_callback_index()? };

		let call = RuntimeCall::IdnManager(IdnManagerCall::quote_subscription { params });

		Self::xcm_send(call)?;

		Ok(req_ref)
	}

	/// Request subscription info for a given subscription ID.
	///
	/// This function dispatches an XCM message to the IDN chain to get the subscription info.
	/// The request should then be handled by the IDN chain and return the subscription info to the
	/// [`Pallet::consume_sub_info`] function along with the `req_ref`.
	/// The `req_ref` is generated by this function and used to identify the request when returned.
	pub fn request_sub_info(
		sub_id: SubscriptionId,
		// Optional quote request reference, if None, a new one will be generated
		req_ref: Option<RequestReference>,
	) -> Result<RequestReference, Error<T>> {
		// If `req_ref` is not provided, generate a new one and assign it to the params
		let req_ref = match req_ref {
			Some(req_ref) => req_ref,
			None => {
				let salt = frame_system::Pallet::<T>::block_number().encode();
				sub_id.hash(&salt).into()
			},
		};
		let req = SubInfoRequest { sub_id, req_ref, call_index: Self::sub_info_callback_index()? };
		let call = RuntimeCall::IdnManager(IdnManagerCall::get_subscription_info { req });

		Self::xcm_send(call)?;

		Ok(req_ref)
	}

	/// Sends an XCM message to the Ideal Network (IDN) chain.
	///
	/// This function constructs and dispatches an XCM message using the provided `RuntimeCall`.
	/// The message includes the following instructions:
	/// - `WithdrawAsset`: Withdraws the specified asset (IDN fee) from the sender's account.
	/// - `BuyExecution`: Pays for the execution of the XCM message with the withdrawn asset.
	/// - `Transact`: Executes the provided `RuntimeCall` on the target chain.
	/// - `RefundSurplus`: Refunds any surplus fees to the sender.
	/// - `DepositAsset`: Deposits the refunded asset back into the sender's account.
	///
	/// The function ensures that the XCM message is properly versioned and sent to the target
	/// location (`SiblingIdnLocation`). If the message fails to send, an `XcmSendError` is
	/// returned.
	///
	/// # Parameters
	/// - `call`: The `RuntimeCall` to be executed on the target chain.
	///
	/// # Returns
	/// - `Ok(())` if the message is successfully sent.
	/// - `Err(Error<T>)` if the message fails to send.
	fn xcm_send(call: RuntimeCall) -> Result<(), Error<T>> {
		let idn_fee_asset = Asset {
			id: AssetId(Location { parents: 1, interior: Junctions::Here }),
			fun: T::MaxIdnXcmFees::get().into(),
		};

		let xcm_call: Xcm<RuntimeCall> = Xcm(vec![
			WithdrawAsset(idn_fee_asset.clone().into()),
			BuyExecution { weight_limit: Unlimited, fees: idn_fee_asset.clone() },
			Transact {
				origin_kind: OriginKind::Xcm,
				fallback_max_weight: None,
				call: call.encode().into(),
			},
			RefundSurplus,
			DepositAsset {
				assets: Wild(AllOf { id: idn_fee_asset.id, fun: WildFungibility::Fungible }),
				beneficiary: Location::here(),
			},
		]);

		let versioned_target: Box<VersionedLocation> =
			Box::new(T::SiblingIdnLocation::get().into());

		let versioned_msg: Box<VersionedXcm<()>> = Box::new(xcm::VersionedXcm::V5(xcm_call.into()));

		T::Xcm::send(Self::pallet_origin().into(), versioned_target, versioned_msg)
			.map_err(|_err| Error::<T>::XcmSendError)?;

		Ok(())
	}

	/// Get the index of this pallet in the runtime
	fn pallet_index() -> Result<u8, Error<T>> {
		<Self as frame_support::traits::PalletInfoAccess>::index()
			.try_into()
			.map_err(|_| Error::<T>::PalletIndexConversionError)
	}

	/// Get the account id of this pallet
	fn pallet_account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	/// Get the signed origin of this pallet
	fn pallet_origin() -> RawOrigin<T::AccountId> {
		RawOrigin::Signed(Self::pallet_account_id())
	}

	/// Get this pallet's xcm Location
	fn pallet_location() -> Result<Location, Error<T>> {
		Ok(Location {
			parents: 1,
			interior: Junctions::X2(Arc::new([
				Junction::Parachain(T::ParaId::get().into()),
				Junction::PalletInstance(Self::pallet_index()?),
			])),
		})
	}

	/// Get the call index for the [`Pallet::consume_pulse`] call
	fn pulse_callback_index() -> Result<CallIndex, Error<T>> {
		// IMPORTANT: The second element of the call index MUST MATCH the index of the
		// `consume_pulse` dispatchable.
		Ok([Self::pallet_index()?, 0])
	}

	/// Get the call index for the [`Pallet::consume_quote`] call
	fn quote_callback_index() -> Result<CallIndex, Error<T>> {
		// IMPORTANT: The second element of the call index MUST MATCH the index of the
		// `consume_quote` dispatchable.
		Ok([Self::pallet_index()?, 1])
	}

	/// Get the call index for the [`Pallet::consume_sub_info`] call
	fn sub_info_callback_index() -> Result<CallIndex, Error<T>> {
		// IMPORTANT: The second element of the call index MUST MATCH the index of the
		// `consume_sub_info` dispatchable.
		Ok([Self::pallet_index()?, 2])
	}
}
