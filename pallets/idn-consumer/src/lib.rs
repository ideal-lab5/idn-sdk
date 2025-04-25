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

extern crate alloc;

use alloc::vec;
use codec::{Codec, Decode, DecodeWithMemTracking, Encode, EncodeLike, MaxEncodedLen};
use cumulus_primitives_core::ParaId;
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	pallet_prelude::{Encode, EnsureOrigin, Get, IsType, Pays, Weight},
	sp_runtime::traits::AccountIdConversion,
};
use frame_system::{
	pallet_prelude::{BlockNumberFor, OriginFor},
	RawOrigin,
};
use idn_runtime::primitives::{
	types::Credits as IdnCredits, Call as IdnRuntimeCall, CreateSubParamsOf, IdnManagerCall,
	MetadataOf, PulseFilterOf,
};
use scale_info::prelude::{boxed::Box, sync::Arc, vec};
use sp_idn_traits::pulse::Pulse as PulseTrait;
use xcm::{
	v5::{
		prelude::{BuyExecution, OriginKind, Transact, Xcm},
		Asset, Junction, Junctions, Location,
		WeightLimit::Unlimited,
	},
	VersionedLocation, VersionedXcm,
};

use xcm_builder::SendController;

pub use idn_runtime::primitives::types::{
	OpaquePulse as IdnPulse, SubscriptionId as IdnSubscriptionId,
};
pub use pallet::*;
pub use sp_idn_traits::pulse::Consumer as ConsumerTrait;

#[cfg(test)]
mod tests;

type CreateSubParams = CreateSubParamsOf<idn_runtime::Runtime>;
type IdnBlockNumber = BlockNumberFor<idn_runtime::Runtime>;
type IdnMetadata = MetadataOf<idn_runtime::Runtime>;
type IdnPulseFilter = PulseFilterOf<idn_runtime::Runtime>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// An implementation of the [`ConsumerTrait`] trait, which defines how to consume a pulse
		type Consumer: ConsumerTrait<IdnPulse, IdnSubscriptionId, DispatchResultWithPostInfo>;

		/// The location of the sibling IDN chain
		///
		/// **Example definition**
		/// ```nocompile
		/// pub SiblingIdnLocation: Location = Location::new(1, Parachain(IDN_PARACHAIN_ID));
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
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::error]
	#[derive(PartialEq)]
	pub enum Error<T> {
		/// An error occurred while consuming the pulse
		ConsumeError,
		/// An error occurred while converting the pallet index to a u8
		PalletIndexConversionError,
		/// An error occurred while sending the XCM message
		XcmSendError,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A random value was successfully consumed.
		RandomnessConsumed { round: <IdnPulse as PulseTrait>::Round, sub_id: IdnSubscriptionId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a subscription.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		#[allow(clippy::useless_conversion)]
		pub fn consume(
			origin: OriginFor<T>,
			pulse: IdnPulse,
			sub_id: IdnSubscriptionId,
		) -> DispatchResultWithPostInfo {
			// ensure origin is coming from IDN
			let _ = T::IdnOrigin::ensure_origin(origin)?;

			let round = pulse.round().clone();

			T::Consumer::consume(pulse, sub_id)?;

			Self::deposit_event(Event::RandomnessConsumed { round, sub_id });

			Ok(Pays::No.into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Creates a subscription.
	pub fn create_subscription(
		// Number of random values to receive
		credits: IdnCredits,
		// Distribution interval for pulses
		frequency: IdnBlockNumber,
		// Bounded vector for additional data
		metadata: Option<IdnMetadata>,
		// Optional Pulse Filter
		pulse_filter: Option<IdnPulseFilter>,
		// Optional Subscription Id, if None, a new one will be generated
		sub_id: Option<IdnSubscriptionId>,
	) -> Result<IdnSubscriptionId, Error<T>> {
		let mut params = CreateSubParams {
			credits,
			target: Self::pallet_location()?,
			// the `0` on the second element is the call index for the `consume` call
			call_index: [Self::pallet_index()?, 0],
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
				let sub_id = params.hash(salt);
				params.sub_id = Some(sub_id);
				sub_id
			},
		};

		let call = IdnRuntimeCall::IdnManager(IdnManagerCall::create_subscription { params });

		let asset_hub_fee_asset: Asset = (Location::parent(), T::AssetHubFee::get()).into();

		let xcm_call: Xcm<IdnRuntimeCall> = Xcm(vec![
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

		Ok(sub_id)
	}

	/// Pauses a subscription.
	pub fn pause_subscription() -> Result<(), Error<T>> {
		// TODO: finish implementation https://github.com/ideal-lab5/idn-sdk/issues/83
		Ok(())
	}

	/// Kills a subscription.
	pub fn kill_subscription() -> Result<(), Error<T>> {
		// TODO: finish implementation https://github.com/ideal-lab5/idn-sdk/issues/84
		Ok(())
	}

	/// Updates a subscription.
	pub fn update_subscription() -> Result<(), Error<T>> {
		// TODO: finish implementation https://github.com/ideal-lab5/idn-sdk/issues/82
		Ok(())
	}

	/// Reactivates a subscription.
	pub fn reactivate_subscription() -> Result<(), Error<T>> {
		// TODO: finish implementation https://github.com/ideal-lab5/idn-sdk/issues/83
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
}
