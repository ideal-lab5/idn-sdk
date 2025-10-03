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

//! Benchmarking setup for pallet-template

use super::*;
use crate::{
	pallet::Pallet as IdnManager,
	primitives::{OriginKind, QuoteRequest},
	CreateSubParamsOf, SubInfoRequestOf, UpdateSubParamsOf,
};
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{
		fungible::{Inspect, Mutate},
		OriginTrait,
	},
	BoundedVec,
};
use frame_system::{Pallet as System, RawOrigin};
use sp_core::H256;
use sp_idn_traits::Hashable;
use xcm::prelude::Junction;

pub fn create_subscriber<T: Config>(
	sub_id: Option<T::AccountId>,
) -> (T::AccountId, RawOrigin<T::AccountId>) {
	match sub_id {
		Some(sub_id) => {
			let origin = RawOrigin::Signed(sub_id.clone());
			(sub_id, origin)
		},
		None => {
			let subscriber: T::AccountId = whitelisted_caller();
			let origin = RawOrigin::Signed(subscriber.clone());
			(subscriber, origin)
		},
	}
}

#[benchmarks(
    where
        T::Credits: From<u64>,
		T::Currency: Mutate<T::AccountId>,
		<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance: From<u64>,
		<T as frame_system::Config>::RuntimeEvent: From<Event<T>>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_subscription() {
		let (subscriber, origin) = create_subscriber::<T>(None);
		let credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id = None;

		T::Currency::set_balance(
			&subscriber,
			IdnManager::<T>::min_balance().saturating_mul(100_000u64.into()),
		);

		let params = CreateSubParamsOf::<T> {
			credits,
			target: target.clone(),
			call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
			frequency,
			metadata,
			sub_id,
			origin_kind: OriginKind::Xcm,
		};

		#[extrinsic_call]
		_(origin, params);

		// assert that the subscription details are correct
		let (_, sub) = Subscriptions::<T>::iter().next().expect("Subscription should exist");
		assert_eq!(sub.details.subscriber, subscriber);
		assert_eq!(sub.details.target, target);
		assert_eq!(sub.credits, credits);
		assert_eq!(sub.frequency, frequency);
		assert_eq!(sub.metadata, None);
	}

	#[benchmark]
	fn pause_subscription() {
		let (subscriber, origin) = create_subscriber::<T>(None);
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id = None;

		T::Currency::set_balance(
			&subscriber,
			IdnManager::<T>::min_balance().saturating_mul(100_000u64.into()),
		);

		let result = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
				frequency,
				metadata,
				sub_id,
				origin_kind: OriginKind::Xcm,
			},
		);
		assert!(result.is_ok(), "Failed to create subscription: {:?}", result);

		// assert that the subscription state is correct
		let (sub_id, sub) = Subscriptions::<T>::iter().next().expect("Subscription should exist");
		assert_eq!(sub.state, SubscriptionState::Active);

		#[extrinsic_call]
		_(origin.clone(), sub_id);

		// assert that the subscription state is correct
		let sub = Subscriptions::<T>::get(sub_id).expect("Subscription should exist");
		assert_eq!(sub.state, SubscriptionState::Paused);
	}

	#[benchmark]
	fn kill_subscription() {
		let (subscriber, origin) = create_subscriber::<T>(None);
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id = None;

		T::Currency::set_balance(
			&subscriber,
			IdnManager::<T>::min_balance().saturating_mul(100_000u64.into()),
		);

		let result = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
				frequency,
				metadata,
				sub_id,
				origin_kind: OriginKind::Xcm,
			},
		);
		assert!(result.is_ok(), "Failed to create subscription: {:?}", result);

		// assert that the subscription was created
		let (sub_id, sub) = Subscriptions::<T>::iter().next().expect("Subscription should exist");
		assert_eq!(sub.state, SubscriptionState::Active);

		#[extrinsic_call]
		_(origin, sub_id);

		// assert that the subscription was removed
		assert!(Subscriptions::<T>::get(sub_id).is_none());
	}

	#[benchmark]
	fn update_subscription(m: Linear<0, { T::MaxMetadataLen::get() }>) {
		let (subscriber, origin) = create_subscriber::<T>(None);
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id = None;

		T::Currency::set_balance(
			&subscriber,
			IdnManager::<T>::min_balance().saturating_mul(100_000u64.into()),
		);

		let result = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
				frequency,
				metadata,
				sub_id,
				origin_kind: OriginKind::Xcm,
			},
		);
		assert!(result.is_ok(), "Failed to create subscription: {:?}", result);

		// assert that the subscription state is correct
		let (sub_id, sub) = Subscriptions::<T>::iter().next().expect("Subscription should exist");
		assert_eq!(sub.state, SubscriptionState::Active);

		let new_credits: T::Credits = 200u64.into();
		let new_frequency: BlockNumberFor<T> = 2u32.into();

		let new_metadata = if m == 0 {
			None
		} else {
			let metadata_vec = (0..m).map(|_| 1u8).collect::<Vec<_>>();
			Some(BoundedVec::try_from(metadata_vec).expect("Metadata vector should fit in bounds"))
		};

		let params = UpdateSubParamsOf::<T> {
			sub_id,
			credits: Some(new_credits),
			frequency: Some(new_frequency),
			metadata: Some(new_metadata),
		};

		#[extrinsic_call]
		_(origin, params);

		// assert that the subscription state is correct
		let sub = Subscriptions::<T>::get(sub_id).expect("Subscription should exist");
		assert_eq!(sub.credits, new_credits);
		assert_eq!(sub.frequency, new_frequency);
	}

	#[benchmark]
	fn reactivate_subscription() {
		let (subscriber, origin) = create_subscriber::<T>(None);
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id = None;

		T::Currency::set_balance(
			&subscriber,
			IdnManager::<T>::min_balance().saturating_mul(100_000u64.into()),
		);

		let result = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
				frequency,
				metadata,
				sub_id,
				origin_kind: OriginKind::Xcm,
			},
		);
		assert!(result.is_ok(), "Failed to create subscription: {:?}", result);

		// assert that the subscription state is correct
		let (sub_id, sub) = Subscriptions::<T>::iter().next().expect("Subscription should exist");
		assert_eq!(sub.state, SubscriptionState::Active);

		let pause_result = IdnManager::<T>::pause_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			sub_id,
		);
		assert!(pause_result.is_ok(), "Failed to pause subscription: {:?}", pause_result);

		let sub = Subscriptions::<T>::get(sub_id).expect("Subscription should exist");
		assert_eq!(sub.state, SubscriptionState::Paused);

		#[extrinsic_call]
		_(origin, sub_id);

		// assert that the subscription state is correct
		let sub = Subscriptions::<T>::get(sub_id).expect("Subscription should exist");
		assert_eq!(sub.state, SubscriptionState::Active);
	}

	#[benchmark]
	fn quote_subscription() {
		let sibling_account: T::AccountId = [88u8; 32].into();
		let sibling_para_id = 88;
		let (subscriber, origin) = create_subscriber::<T>(Some(sibling_account));
		let credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id = None;

		let params = CreateSubParamsOf::<T> {
			credits,
			target: target.clone(),
			call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
			frequency,
			metadata,
			sub_id,
			origin_kind: OriginKind::Xcm,
		};

		let req_ref = [1; 32];
		let lifetime_pulses: BlockNumberFor<T> = 10u32.into();

		let quote_request =
			QuoteRequest { req_ref, create_sub_params: params.clone(), lifetime_pulses };
		let quote_sub_params = QuoteSubParams {
			quote_request,
			call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
			origin_kind: OriginKind::Xcm,
		};

		#[extrinsic_call]
		_(origin, quote_sub_params);

		let deposit =
			IdnManager::<T>::calculate_storage_deposit_from_create_params(&subscriber, &params);

		System::<T>::assert_last_event(
			Event::<T>::SubQuoted {
				requester: Location::new(1, [Junction::Parachain(sibling_para_id)]),
				quote: Quote { req_ref, fees: 58000000u64.into(), deposit },
			}
			.into(),
		);
	}

	#[benchmark]
	fn get_subscription_info() {
		let sibling_id: T::AccountId = [88u8; 32].into();
		let (sibling_account, origin) = create_subscriber::<T>(Some(sibling_id));
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id: T::SubscriptionId = H256::default().into();

		T::Currency::set_balance(
			&sibling_account,
			IdnManager::<T>::min_balance().saturating_mul(100_000u64.into()),
		);

		// Create first subscription
		let result = IdnManager::<T>::create_subscription(
			origin.clone().into(),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
				frequency,
				metadata,
				sub_id: Some(sub_id),
				origin_kind: OriginKind::Xcm,
			},
		);
		assert!(result.is_ok(), "Failed to create subscription: {:?}", result);

		let req = SubInfoRequestOf::<T> {
			sub_id,
			req_ref: [1; 32],
			call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
			origin_kind: OriginKind::Xcm,
		};

		#[extrinsic_call]
		_(origin, req);

		System::<T>::assert_last_event(Event::<T>::SubscriptionDistributed { sub_id }.into());
	}

	/// Benchmark dispatching a single pulse to `p` subscriptions
	#[benchmark]
	fn dispatch_pulse(s: Linear<1, { T::MaxSubscriptions::get() }>) {
		let (subscriber, origin) = create_subscriber::<T>(None);
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id: T::SubscriptionId = H256::default().into();

		T::Currency::set_balance(
			&subscriber,
			IdnManager::<T>::min_balance().saturating_mul(100_000u64.into()),
		);

		// Create first subscription
		let result = IdnManager::<T>::create_subscription(
			origin.into(),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
				frequency,
				metadata,
				sub_id: Some(sub_id),
				origin_kind: OriginKind::Xcm,
			},
		);
		assert!(result.is_ok(), "Failed to create subscription: {:?}", result);

		// Fill up the subscriptions with the given number of subscriptions (minus the already
		// created one)
		if s > 1 {
			fill_up_subscriptions::<T>(s - 1);
		}

		assert_eq!(Subscriptions::<T>::iter().count(), s as usize);

		// Create a pulse to dispatch
		let pulse = T::Pulse::default();

		#[block]
		{
			IdnManager::<T>::dispatch(pulse);
		}

		// Verify the first subscription was updated
		let sub = Subscriptions::<T>::get(sub_id).expect("Subscription should exist");
		assert!(sub.last_delivered.is_some());
	}

	#[benchmark]
	fn on_finalize() {
		// We assume the worst case scenario, that is, we've got max subscriptions in the current
		// block. We can't set `s` as a parameter (e.g. to pass `SubCounter`) to use it instead of
		// `MaxSubscriptions`, because we don't know at the beginning of the block (when this is
		// measured) how many subscriptions will be created by the end of the block. Therefore we
		// don't know how big the for loop in the `on_finalize` hook will be.
		let s = T::MaxSubscriptions::get();
		let f = T::MaxTerminatableSubs::get();
		fill_up_subscriptions::<T>(s);

		assert_eq!(Subscriptions::<T>::iter().count(), s as usize);

		// Mark max terminatable subscriptions as finalized
		Subscriptions::<T>::iter().take(f as usize).for_each(
			|(sub_id, mut sub): (T::SubscriptionId, SubscriptionOf<T>)| {
				// update the subscription as finalized
				sub.state = SubscriptionState::Finalized;

				// update the subscription
				Subscriptions::<T>::insert(sub_id, sub.clone());
			},
		);

		let block_number = frame_system::Pallet::<T>::block_number();

		#[block]
		{
			Pallet::<T>::on_finalize(block_number);
		}

		assert_eq!(Subscriptions::<T>::iter().count(), s.saturating_sub(f) as usize);
	}

	/// Fill up the subscriptions with the given number of subscriptions
	fn fill_up_subscriptions<T: Config>(s: u32)
	where
		T::Credits: From<u64>,
		T::Currency: Mutate<T::AccountId>,
		<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance:
			From<u64>,
		T::AccountId: From<[u8; 32]>,
		<<T as Config>::Currency as Inspect<T::AccountId>>::Balance: From<u64>,
	{
		let credits: T::Credits = 1u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: BlockNumberFor<T> = 1u32.into();

		// Create s subscriptions
		for i in 0..s {
			let id: [u8; 32] = i.hash(&[i as u8]).into();
			let account_id: T::AccountId = id.into();
			let (subscriber, origin) = create_subscriber::<T>(Some(account_id));
			T::Currency::set_balance(
				&subscriber,
				IdnManager::<T>::min_balance().saturating_mul(100_000u64.into()),
			);
			let res = IdnManager::<T>::create_subscription(
				origin.into(),
				CreateSubParamsOf::<T> {
					credits,
					target: target.clone(),
					call: vec![0u8; T::MaxCallDataLen::get() as usize].try_into().unwrap(),
					frequency,
					metadata: None,
					sub_id: None,
					origin_kind: OriginKind::Xcm,
				},
			);

			if res.is_err() {
				panic!("Failed to create subscription {}: {:?}", i, res.unwrap_err());
			}

			frame_system::Pallet::<T>::set_block_number(
				frame_system::Pallet::<T>::block_number() + 1u32.into(),
			);
		}
	}

	impl_benchmark_test_suite!(
		IdnManager,
		crate::tests::mock::ExtBuilder::build(),
		crate::tests::mock::Test
	);
}
