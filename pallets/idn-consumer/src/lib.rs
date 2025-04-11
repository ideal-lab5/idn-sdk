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

use codec::{Codec, Decode, Encode, EncodeLike, MaxEncodedLen};
use frame_support::{
	dispatch::DispatchResultWithPostInfo,
	pallet_prelude::{DispatchResult, EnsureOrigin, Get, IsType, Pays, TypeInfo, Weight},
};
use frame_system::pallet_prelude::{BlockNumberFor, OriginFor};
use pallet::*;
use scale_info::prelude::fmt::Debug;
use sp_arithmetic::traits::Unsigned;
use sp_core::H256;
use sp_idn_traits::pulse::{Consumer, Pulse};
use pallet_idn_manager::primitives::{CreateSubParams, IdnManagerCall, PulseFilter, SubscriptionMetadata};
use xcm::v5::{
	prelude::{OriginKind, Transact, Xcm},
	Location,
};

/// The metadata type used in the pallet, represented as a bounded vector of bytes.
type MetadataOf<T> = SubscriptionMetadata<<T as Config>::MaxMetadataLen>;

/// A filter that controls which pulses are delivered to a subscription
///
/// See [`PulseFilter`] for more details.
type PulseFilterOf<T> = PulseFilter<<T as pallet::Config>::Pulse, <T as Config>::MaxPulseFilterLen>;
/// The parameters for creating a new subscription, containing various details about the
/// subscription.
pub type CreateSubParamsOf<T> = CreateSubParams<
	<T as pallet::Config>::Credits,
	BlockNumberFor<T>,
	MetadataOf<T>,
	PulseFilterOf<T>,
	<T as pallet::Config>::SubscriptionId,
>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The type for the randomness pulse
		type Pulse: Pulse + Encode + Debug + Decode + Clone + TypeInfo + PartialEq;

		/// An implementation of the [`Consumer`] trait, which defines how to consume a pulse
		type Consumer: Consumer<Self::Pulse, Self::SubscriptionId, DispatchResult>;

		/// The location of the sibling IDN chain
		///
		/// **Example definition**
		/// ```nocompile
		/// pub SiblingIdnParaId: u32 = IDN_PARACHAIN_ID;
		/// pub SiblingIdnLocation: Location = Location::new(1, Parachain(SiblingIDNParaId::get()));
		/// ```
		type SiblingIdnLocation: Get<Location>;

		/// The origin of the IDN chain
		///
		/// **Example definition**
		/// ```nocompile
		/// pub IdnOrigin: EnsureXcm<Equals<Self::SiblingIdnLocation>>
		/// ```
		type IdnOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Location>;

		/// A type to define the amount of credits in a subscription
		type Credits: Unsigned + Encode;

		/// Maximum metadata size
		type MaxMetadataLen: Get<u32>;

		/// Subscription ID type
		type SubscriptionId: From<H256>
			+ Codec
			+ Copy
			+ PartialEq
			+ TypeInfo
			+ EncodeLike
			+ MaxEncodedLen
			+ Debug;

		/// Maximum Pulse Filter size
		type MaxPulseFilterLen: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {
		/// An error occurred while consuming the pulse
		ConsumeError,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A random value was successfully consumed.
		RandomnessConsumed { round: <T::Pulse as Pulse>::Round, sub_id: T::SubscriptionId },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates a subscription.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::from_parts(0, 0))]
		#[allow(clippy::useless_conversion)]
		pub fn consume(
			origin: OriginFor<T>,
			pulse: T::Pulse,
			sub_id: T::SubscriptionId,
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
		credits: T::Credits,
		// Distribution interval for pulses
		frequency: BlockNumberFor<T>,
		// Bounded vector for additional data
		metadata: Option<MetadataOf<T>>,
		// Optional Pulse Filter
		pulse_filter: Option<PulseFilterOf<T>>,
		// Optional Subscription Id, if None, a new one will be generated
		sub_id: Option<T::SubscriptionId>,
	) -> Result<T::SubscriptionId, Error<T>> {
		// TODO: finish implementation https://github.com/ideal-lab5/idn-sdk/issues/81

		let mut params = CreateSubParamsOf::<T> {
			credits,
			target: T::SiblingIdnLocation::get(),
			call_index: [0, 0],
			frequency,
			metadata,
			pulse_filter,
			sub_id,
		};

		// If sub_id is not provided, generate a new one and asign it to the params
		let sub_id = match sub_id {
			Some(sub_id) => sub_id,
			None => {
				let salt = frame_system::Pallet::<T>::block_number().encode();
				let sub_id = params.hash(salt);
				params.sub_id = Some(sub_id);
				sub_id
			},
		};

		// TODO: should this be under the runtime?
		// https://github.com/ideal-lab5/idn-sdk/issues/81
		#[derive(Encode, Decode, Debug, PartialEq, Clone, scale_info::TypeInfo)]
		enum Call<CreateSubParams> {
			#[codec(index = 57)]
			IdnManager(IdnManagerCall<CreateSubParams>),
		}

		let _call: Xcm<()> = Xcm(vec![Transact {
			origin_kind: OriginKind::Xcm,
			fallback_max_weight: None,
			call: (Call::<CreateSubParamsOf<T>>::IdnManager(IdnManagerCall::<
				CreateSubParamsOf<T>,
			>::create_subscription {
				params,
			}))
			.encode()
			.into(),
		}]);

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
}
