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

#[cfg(test)]
mod tests;

pub mod traits;
pub mod weights;

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{
	pallet_prelude::{
		ensure, Blake2_128Concat, DispatchResult, Hooks, IsType, OptionQuery, StorageMap, Zero,
	},
	sp_runtime::traits::AccountIdConversion,
	traits::{
		fungible::{hold::Mutate as HoldMutate, Inspect},
		Get,
	},
	BoundedVec,
};
use frame_system::{
	ensure_signed,
	pallet_prelude::{BlockNumberFor, OriginFor},
};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use traits::*;
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

#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct Subscription<AccountId, BlockNumber, Metadata> {
	details: SubscriptionDetails<AccountId, BlockNumber, Metadata>,
	credits_left: BlockNumber,
	state: SubscriptionState,
}

#[derive(Encode, Decode, Clone, TypeInfo, MaxEncodedLen, Debug)]
pub struct SubscriptionDetails<AccountId, BlockNumber, Metadata> {
	subscriber: AccountId,
	para_id: ParaId,
	created_at: BlockNumber,
	updated_at: BlockNumber,
	amount: BlockNumber,
	frequency: BlockNumber,
	target: Location,
	metadata: Metadata,
}

type ParaId = u32; // todo import type

impl<AccountId, BlockNumber, Metadata> Subscription<AccountId, BlockNumber, Metadata>
where
	AccountId: Encode,
	BlockNumber: Encode + Copy,
	Metadata: Encode + Clone,
{
	pub fn id(&self) -> SubscriptionId {
		// Create a tuple with the fields to include. Adjust fields as necessary.
		let id_tuple = (
			&self.details.subscriber,
			self.details.para_id,
			self.details.created_at,
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
		type FeesCalculator: FeesCalculator<BalanceOf<Self>, BlockNumberFor<Self>>;

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
		/// A subscription has naturally finished
		SubscriptionFinished { sub_id: SubscriptionId },
		/// A subscription was paused
		SubscriptionPaused { sub_id: SubscriptionId },
		/// A subscription has been manually finished
		SubscriptionKilled { sub_id: SubscriptionId },
		/// Randomness was successfully distributed
		RandomnessDistributed { sub_id: SubscriptionId },
		/// WARNING Subscription credit went bellow 0. This should never happen
		SubscriptionCreditBelowZero { sub_id: SubscriptionId, credits: BlockNumberFor<T> },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// An active subscription already exists
		ActiveSubscriptionExists,
		// /// The parachain ID is invalid
		// InvalidParaId,
		/// Insufficient balance for subscription
		InsufficientBalance,
		// /// XCM execution failed
		// XcmExecutionFailed,
	}

	/// A reason for the IDN Manager Pallet placing a hold on funds.
	#[pallet::composite_enum]
	pub enum HoldReason {
		/// The IDN Manager Pallet holds balance for future charges.
		#[codec(index = 0)]
		Fees,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_n: BlockNumberFor<T>) {
			// let mut reads = 0u64;
			// let mut writes = 0u64;

			// Look for subscriptions that should be finished
			for (sub_id, sub) in
				Subscriptions::<T>::iter().filter(|(_, sub)| sub.credits_left <= Zero::zero())
			{
				// reads += 1

				if sub.credits_left < Zero::zero() {
					// throw  warning
					Self::deposit_event(Event::SubscriptionCreditBelowZero {
						sub_id,
						credits: sub.credits_left,
					});
				}

				Self::finish_subscription(sub_id);
				// writes += 1;
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
			amount: BlockNumberFor<T>,
			target: Location,
			frequency: BlockNumberFor<T>,
			metadata: Option<MetadataOf<T>>,
		) -> DispatchResult {
			let subscriber = ensure_signed(origin)?;
			// TODO how to allow for fees preview?
			Self::create_subscription_internal(
				subscriber, para_id, amount, target, frequency, metadata,
			)
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Finishes a subscription by removing it from storage and emitting a finish event.
	pub(crate) fn finish_subscription(sub_id: SubscriptionId) {
		Subscriptions::<T>::remove(sub_id);
		Self::deposit_event(Event::SubscriptionFinished { sub_id });
	}

	fn pallet_account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	/// Distribute randomness to subscribers
	/// Returns a weight based on the number of storage reads and writes performed
	fn distribute(rnd: T::Rnd) -> DispatchResult {
		// let mut reads = 0u64;
		// let mut writes = 0u64;

		// let randomness = T::RandomnessSource::get_randomness();

		// Filter for active subscriptions only
		for (sub_id, sub) in
			Subscriptions::<T>::iter().filter(|(_, sub)| sub.state == SubscriptionState::Active)
		{
			// reads += 1;

			if let Ok(msg) = Self::construct_randomness_xcm(sub.details.target.clone(), &rnd) {
				let versioned_target: Box<VersionedLocation> =
					Box::new(sub.details.target.clone().into());
				let versioned_msg: Box<VersionedXcm<()>> =
					Box::new(xcm::VersionedXcm::V5(msg.into()));
				let origin = frame_system::RawOrigin::Signed(Self::pallet_account_id());
				let _ = T::Xcm::send(origin.into(), versioned_target, versioned_msg);
				// writes += 1;

				// TODO: decrease credits left !!!!

				Self::deposit_event(Event::RandomnessDistributed { sub_id });
			}
		}

		// T::DbWeight::get().reads(reads) + T::DbWeight::get().writes(writes)
		Ok(())
	}

	/// Check if there's an active subscription for the given para_id
	fn has_active_subscription(sub_id: &SubscriptionId) -> bool {
		Subscriptions::<T>::get(sub_id)
			.map(|sub| sub.state == SubscriptionState::Active)
			.unwrap_or(false)
	}

	/// Internal function to handle subscription creation
	fn create_subscription_internal(
		subscriber: T::AccountId,
		para_id: ParaId,
		amount: BlockNumberFor<T>,
		target: Location,
		frequency: BlockNumberFor<T>,
		metadata: Option<MetadataOf<T>>,
	) -> DispatchResult {
		// Calculate and hold the subscription fees
		let fees = T::FeesCalculator::calculate_subscription_fees(amount);
		T::Currency::hold(&HoldReason::Fees.into(), &subscriber, fees)
			.map_err(|_| Error::<T>::InsufficientBalance)?;

		let current_block = frame_system::Pallet::<T>::block_number();
		let details = SubscriptionDetails {
			subscriber: subscriber.clone(),
			para_id,
			created_at: current_block,
			updated_at: current_block,
			amount,
			target: target.clone(),
			frequency,
			metadata: metadata.unwrap_or_default(),
		};
		let subscription =
			Subscription { state: SubscriptionState::Active, credits_left: amount, details };
		let sub_id = subscription.id();

		ensure!(!Self::has_active_subscription(&sub_id), Error::<T>::ActiveSubscriptionExists);

		Subscriptions::<T>::insert(&sub_id, subscription);

		Self::deposit_event(Event::SubscriptionCreated { sub_id });

		Ok(())
	}

	/// Helper function to construct XCM message for randomness distribution
	fn construct_randomness_xcm(target: Location, _rnd: &T::Rnd) -> Result<Xcm<()>, Error<T>> {
		Ok(Xcm(vec![
			WithdrawAsset((Here, 0u128).into()),
			BuyExecution { fees: (Here, 0u128).into(), weight_limit: Unlimited },
			Transact {
				origin_kind: OriginKind::Native,
				fallback_max_weight: None,
				call: Call::DistributeRnd.encode().into(), // TODO
			},
			RefundSurplus,
			DepositAsset { assets: All.into(), beneficiary: target.into() },
		]))
	}
}

impl<T: Config> idn_traits::rand::Consumer<T::Rnd, DispatchResult> for Pallet<T> {
	fn consume(rnd: T::Rnd) -> DispatchResult {
		// look for active subscriptions
		// deliver rand to them
		Pallet::<T>::distribute(rnd)
	}
}
