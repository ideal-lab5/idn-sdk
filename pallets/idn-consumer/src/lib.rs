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
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod traits;
pub mod weights;

use bp_idn::{
	types::{
		BlockNumber as IdnBlockNumber, CallIndex, CreateSubParams, Credits, Metadata, PulseFilter,
		QuoteRequest, QuoteSubParams, RequestReference, SubInfoRequest, UpdateSubParams,
	},
	Call as RuntimeCall, IdnManagerCall,
};
use cumulus_primitives_core::ParaId;
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	pallet_prelude::{
		Decode, DecodeWithMemTracking, Encode, EnsureOrigin, Get, IsType, Pays, Weight,
	},
	sp_runtime::traits::AccountIdConversion,
};
use frame_system::{pallet_prelude::OriginFor, RawOrigin};
use scale_info::{
	prelude::{boxed::Box, sync::Arc, vec},
	TypeInfo,
};
use sp_idn_traits::{pulse::Pulse as PulseTrait, Hashable};
use traits::{PulseConsumer, QuoteConsumer, SubInfoConsumer};
use xcm::{
	v5::{
		prelude::{BuyExecution, OriginKind, Transact, Xcm},
		Asset, Junction, Junctions, Location,
		WeightLimit::Unlimited,
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
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// An implementation of the [`PulseConsumer`] trait, which defines how to consume a pulse
		type PulseConsumer: PulseConsumer<Pulse, SubscriptionId, (), ()>;

		/// An implementation of the [`QuoteConsumer`] trait, which defines how to consume a
		/// quote
		type QuoteConsumer: QuoteConsumer<Quote, (), ()>;

		/// An implementation of the [`SubInfoConsumer`] trait, which defines how to consume a
		/// subscription
		type SubInfoConsumer: SubInfoConsumer<SubInfoResponse, (), ()>;

		/// The location of the sibling IDN chain
		///
		/// **Example definition**
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

		/// The origin of the IDN chain
		///
		/// **Example definition**
		/// ```nocompile
		/// type IdnOrigin = EnsureXcm<Equals<Self::SiblingIdnLocation>>
		/// ```
		type IdnOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Location>;

		/// A type that exposes XCM APIs, allowing contracts to interact with other parachains, and
		/// execute XCM programs.
		type Xcm: xcm_builder::SendController<OriginFor<Self>>;

		/// The IDN Manager pallet id.
		#[pallet::constant]
		type PalletId: Get<frame_support::PalletId>;

		/// The parachain ID of this chain
		///
		/// **Example definition**
		/// ```nocompile
		/// pub ParaId: ParaId = ParachainInfo::parachain_id();
		/// ```
		#[pallet::constant]
		type ParaId: Get<ParaId>;

		/// The asset hub asset ID for paying the IDN fees.
		///
		/// E.g. DOT, USDC, etc.
		#[pallet::constant]
		type AssetHubFee: Get<u128>;

		// The weight information for this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// An error occurred while consuming the pulse
		ConsumePulseError,
		/// An error occurred while consuming the quote
		ConsumeQuoteError,
		/// An error occurred while consuming the subscription info
		ConsumeSubInfoError,
		/// An error occurred while converting the pallet index to a u8
		PalletIndexConversionError,
		/// An error occurred while sending the XCM message
		XcmSendError,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A random value was successfully consumed.
		RandomnessConsumed { round: <Pulse as PulseTrait>::Round, sub_id: SubscriptionId },
		/// A subscription quote was successfully consumed.
		QuoteConsumed { quote: Quote },
		/// A subscription info was successfully consumed.
		SubInfoConsumed { sub_id: SubscriptionId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Consumes a pulse from the IDN chain.
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

			let round = pulse.round();

			T::PulseConsumer::consume_pulse(pulse, sub_id)
				.map_err(|_| Error::<T>::ConsumePulseError)?;

			Self::deposit_event(Event::RandomnessConsumed { round, sub_id });

			Ok(Pays::No.into())
		}

		/// Consumes a subscription quote from the IDN chain.
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

		/// Consumes a subscription info response from the IDN chain.
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
		// Optional Pulse Filter
		pulse_filter: Option<PulseFilter>,
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
			pulse_filter,
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
		pulse_filter: Option<Option<PulseFilter>>,
	) -> Result<(), Error<T>> {
		let params = UpdateSubParams { sub_id, credits, frequency, metadata, pulse_filter };

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
		// Optional Pulse Filter
		pulse_filter: Option<PulseFilter>,
		// Optional Subscription Id
		sub_id: Option<SubscriptionId>,
		// Optional quote request reference, if None, a new one will be generated
		req_ref: Option<RequestReference>,
	) -> Result<SubscriptionId, Error<T>> {
		let create_sub_params = CreateSubParams {
			credits,
			target: Self::pallet_location()?,
			// the `0` on the second element is the call index for the `consume` call
			call_index: Self::pulse_callback_index()?,
			frequency,
			metadata,
			pulse_filter,
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
	) -> Result<(), Error<T>> {
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

		Ok(())
	}

	fn xcm_send(call: RuntimeCall) -> Result<(), Error<T>> {
		let asset_hub_fee_asset: Asset = (Location::parent(), T::AssetHubFee::get()).into();

		let xcm_call: Xcm<RuntimeCall> = Xcm(vec![
			BuyExecution { weight_limit: Unlimited, fees: asset_hub_fee_asset },
			Transact {
				origin_kind: OriginKind::Xcm,
				fallback_max_weight: None,
				call: call.encode().into(),
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
