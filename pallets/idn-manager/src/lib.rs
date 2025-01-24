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

pub mod traits;
pub mod weights;

use codec::{Decode, Encode, MaxEncodedLen};
use traits::*;
// use cumulus_primitives_core::ParaId;
use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::AccountIdConversion,
	// storage::StorageMap,
	traits::{
		fungible::{hold::Mutate as HoldMutate, Inspect},
		Get,
	},
	weights::Weight,
};
use frame_system::{
	ensure_signed,
	pallet_prelude::{BlockNumberFor, OriginFor},
};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use xcm::{latest::prelude::*, opaque::v4::Location, VersionedLocation, VersionedXcm};
use xcm_builder::SendController;

pub use pallet::*;
pub use weights::*;

pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct Subscription<AccountId, BlockNumber> {
	details: SubscriptionDetails<AccountId, BlockNumber>,
	status: SubscriptionStatus,
}

#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct SubscriptionDetails<AccountId, BlockNumber> {
	subscriber: AccountId,
	para_id: ParaId,
	start_block: BlockNumber,
	end_block: BlockNumber,
	frequency: BlockNumber,
	target: Location,
	filter: Option<Filter>, // ?
}

type ParaId = u32; // todo import type

impl<AccountId: Encode, BlockNumber: Encode> Subscription<AccountId, BlockNumber> {
	pub fn id(&self) -> SubscriptionId {
		// Encode the struct using SCALE codec
		let details = self.details.encode();
		// Hash the encoded bytes using blake2_256
		H256::from_slice(&blake2_256(&details))
	}
}

type SubscriptionId = H256;
type Filter = u32; // to update

#[derive(Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen, Debug)]
pub enum SubscriptionStatus {
	Inactive,
	Active,
	Paused,
	Expired,
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
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The currency type for handling subscription payments
		type Currency: Inspect<<Self as frame_system::pallet::Config>::AccountId>
			// + Mutate<<Self as frame_system::pallet::Config>::AccountId>
			+ HoldMutate<
				<Self as frame_system::pallet::Config>::AccountId,
				Reason = Self::RuntimeHoldReason,
			>;

		/// Overarching hold reason.
		type RuntimeHoldReason: From<HoldReason>;

		/// Maximum subscription duration
		#[pallet::constant]
		type MaxSubscriptionDuration: Get<BlockNumberFor<Self>>;

		/// Fee calculator implementation
		type FeeCalculator: FeeCalculator<BalanceOf<Self>, BlockNumberFor<Self>>;

		/// The type for the randomness
		type Rnd;

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
	}

	#[pallet::storage]
	pub type Subscriptions<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		SubscriptionId,
		Subscription<T::AccountId, BlockNumberFor<T>>,
		OptionQuery,
	>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A new subscription was created (includes single-block subscriptions)
		SubscriptionCreated {
			sub_id: SubscriptionId,
			para_id: ParaId,
			subscriber: T::AccountId,
			duration: BlockNumberFor<T>,
			target: Location,
		},
		/// A subscription was deactivated
		SubscriptionDeactivated { sub_id: SubscriptionId },
		///
		SubscriptionPaused { sub_id: SubscriptionId },
		/// Notify of low budget
		SubscriptionLowBudget { sub_id: SubscriptionId, budget: BalanceOf<T> },
		/// Randomness was successfully distributed
		RandomnessDistributed { sub_id: SubscriptionId },
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

	/// A reason for the IDN Manager Pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		/// The IDN Manager Pallet holds balance for future charges.
		#[codec(index = 0)]
		Fee,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(n: BlockNumberFor<T>) {
			// let mut reads = 0u64;
			// let mut writes = 0u64;

			// Filter active subscriptions that have expired
			for (sub_id, _sub) in Subscriptions::<T>::iter().filter(|(_, sub)| {
				sub.status == SubscriptionStatus::Active && sub.details.end_block <= n
			}) {
				// reads += 1;

				// Use update instead of insert for atomic operation
				let result =
					Subscriptions::<T>::try_mutate(sub_id, |maybe_sub| -> DispatchResult {
						if let Some(sub) = maybe_sub {
							sub.status = SubscriptionStatus::Expired;
							// writes += 1;
							Self::deposit_event(Event::SubscriptionDeactivated { sub_id });
						}
						Ok(())
					});

				if let Err(_) = result {
					// TODO handle error
				}
			}

			// T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Create a subscription for one or multiple blocks
		// To keep this pallet low level `request_single_randomness` is not implemented and rather
		// let it up to the pallet on the client side to handle the single block subscription
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::create_subscription())]
		pub fn create_subscription(
			origin: OriginFor<T>,
			para_id: ParaId,
			duration: BlockNumberFor<T>,
			target: Location,
			frequency: BlockNumberFor<T>,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;
			// Calculate and hold the subscription fee
			let fee = T::FeeCalculator::calculate_subscription_fee(duration);
			// TODO how to allow for fees preview?
			Self::create_subscription_internal(
				subscriber, para_id, duration, target, fee, frequency,
			)
		}
	}
}

impl<T: Config> Pallet<T> {
	fn pallet_account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}
	/// Distribute randomness to subscribers
	/// Returns a weight based on the number of storage reads and writes performed
	fn distribute(rnd: T::Rnd) -> Result<(), DispatchError> {
		// let mut reads = 0u64;
		// let mut writes = 0u64;

		// let randomness = T::RandomnessSource::get_randomness();

		// Filter for active subscriptions only
		for (sub_id, sub) in
			Subscriptions::<T>::iter().filter(|(_, sub)| sub.status == SubscriptionStatus::Active)
		{
			// reads += 1;

			if let Ok(msg) = Self::construct_randomness_xcm(sub.details.target.clone(), &rnd) {
				let versioned_target: Box<VersionedLocation> =
					Box::new(sub.details.target.clone().into());
				let versioned_msg: Box<VersionedXcm<()>> =
					Box::new(xcm::VersionedXcm::V4(msg.into()));
				let origin = frame_system::RawOrigin::Signed(Self::pallet_account_id());
				let _ = T::Xcm::send(origin.into(), versioned_target, versioned_msg);
				// writes += 1;

				Self::deposit_event(Event::RandomnessDistributed { sub_id });
			}
		}

		// T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
		Ok(())
	}

	/// Check if there's an active subscription for the given para_id
	fn has_active_subscription(sub_id: &SubscriptionId) -> bool {
		Subscriptions::<T>::get(sub_id)
			.map(|sub| sub.status == SubscriptionStatus::Active)
			.unwrap_or(false)
	}

	/// Internal function to handle subscription creation
	fn create_subscription_internal(
		subscriber: T::AccountId,
		para_id: ParaId,
		duration: BlockNumberFor<T>,
		target: Location,
		fee: BalanceOf<T>,
		frequency: BlockNumberFor<T>,
	) -> DispatchResult {
		ensure!(
			duration <= T::MaxSubscriptionDuration::get(),
			Error::<T>::InvalidSubscriptionDuration
		);

		T::Currency::hold(&HoldReason::Fee.into(), &subscriber, fee)
			.map_err(|_| Error::<T>::InsufficientBalance)?;

		let current_block = frame_system::Pallet::<T>::block_number();
		let details = SubscriptionDetails {
			subscriber: subscriber.clone(),
			para_id,
			start_block: current_block,
			end_block: current_block + duration,
			target: target.clone(),
			frequency,
			filter: None, // TODO define this
		};
		let subscription = Subscription { status: SubscriptionStatus::Active, details };
		let sub_id = subscription.id();

		ensure!(!Self::has_active_subscription(&sub_id), Error::<T>::ActiveSubscriptionExists);

		Subscriptions::<T>::insert(&sub_id, subscription);

		Self::deposit_event(Event::SubscriptionCreated {
			sub_id,
			para_id,
			subscriber,
			duration,
			target,
		});

		Ok(())
	}

	/// Helper function to construct XCM message for randomness distribution
	fn construct_randomness_xcm(target: Location, _rnd: &T::Rnd) -> Result<Xcm<()>, Error<T>> {
		Ok(Xcm(vec![
			WithdrawAsset((Here, 0u128).into()),
			BuyExecution { fees: (Here, 0u128).into(), weight_limit: Unlimited },
			Transact {
				origin_kind: OriginKind::Native,
				require_weight_at_most: Weight::from_parts(1_000_000, 0),
				call: Call::DistributeRnd.encode().into(), // TODO
			},
			RefundSurplus,
			DepositAsset { assets: All.into(), beneficiary: target.into() },
		]))
	}
}

impl<T: Config> idn_traits::rand::Consumer<T::Rnd, Result<(), DispatchError>> for Pallet<T> {
	fn consume(rnd: T::Rnd) -> Result<(), DispatchError> {
		// look for active subscriptions
		// deliver rand to them
		Pallet::<T>::distribute(rnd)
	}
}
