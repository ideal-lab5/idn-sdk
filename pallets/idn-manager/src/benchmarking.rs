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
	primitives::{PulsePropertyOf, QuoteRequest},
	CreateSubParamsOf, SubInfoRequestOf, UpdateSubParamsOf,
};
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{fungible::Mutate, OriginTrait},
	BoundedVec,
};
use frame_system::{Pallet as System, RawOrigin};
use sp_core::H256;
use xcm::v5::prelude::Junction;

#[benchmarks(
    where
        T::Credits: From<u64>,
        <T::Pulse as Pulse>::Round: From<u64>,
		T::Currency: Mutate<T::AccountId>,
		<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance: From<u64>,
		<T as frame_system::Config>::RuntimeEvent: From<Event<T>>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_subscription(l: Linear<0, { T::MaxPulseFilterLen::get() }>) {
		let subscriber: T::AccountId = whitelisted_caller();
		let origin = RawOrigin::Signed(subscriber.clone());
		let credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id = None;

		let pulse_filter = if l == 0 {
			None
		} else {
			let pulse_filter_vec = (0..l)
				.map(|_| PulsePropertyOf::<<T as pallet::Config>::Pulse>::Round(1u64.into()))
				.collect::<Vec<_>>();
			Some(BoundedVec::try_from(pulse_filter_vec).unwrap())
		};

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		let params = CreateSubParamsOf::<T> {
			credits,
			target: target.clone(),
			call_index,
			frequency,
			metadata,
			pulse_filter,
			sub_id,
		};

		#[extrinsic_call]
		_(origin, params);

		// assert that the subscription details are correct
		let (_, sub) = Subscriptions::<T>::iter().next().unwrap();
		assert_eq!(sub.details.subscriber, subscriber);
		assert_eq!(sub.details.target, target);
		assert_eq!(sub.credits, credits);
		assert_eq!(sub.frequency, frequency);
		assert_eq!(sub.metadata, None);
	}

	#[benchmark]
	fn pause_subscription() {
		let subscriber: T::AccountId = whitelisted_caller();
		let origin = RawOrigin::Signed(subscriber.clone());
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let pulse_filter = None;
		let sub_id = None;

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call_index,
				frequency,
				metadata,
				pulse_filter,
				sub_id,
			},
		);

		// assert that the subscription state is correct
		let (sub_id, sub) = Subscriptions::<T>::iter().next().unwrap();
		assert_eq!(sub.state, SubscriptionState::Active);

		#[extrinsic_call]
		_(origin, sub_id);

		// assert that the subscription state is correct
		let sub = Subscriptions::<T>::get(sub_id).unwrap();
		assert_eq!(sub.state, SubscriptionState::Paused);
	}

	#[benchmark]
	fn kill_subscription() {
		let subscriber: T::AccountId = whitelisted_caller();
		let origin = RawOrigin::Signed(subscriber.clone());
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let pulse_filter = None;
		let sub_id = None;

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call_index,
				frequency,
				metadata,
				pulse_filter,
				sub_id,
			},
		);

		// assert that the subscription was created
		let (sub_id, sub) = Subscriptions::<T>::iter().next().unwrap();
		assert_eq!(sub.state, SubscriptionState::Active);

		#[extrinsic_call]
		_(origin, sub_id);

		// assert that the subscription was removed
		assert!(Subscriptions::<T>::get(sub_id).is_none());
	}

	#[benchmark]
	fn update_subscription(
		l: Linear<0, { T::MaxPulseFilterLen::get() }>,
		m: Linear<0, { T::MaxMetadataLen::get() }>,
	) {
		let subscriber: T::AccountId = whitelisted_caller();
		let origin = RawOrigin::Signed(subscriber.clone());
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let pulse_filter = None;
		let sub_id = None;

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call_index,
				frequency,
				metadata,
				pulse_filter,
				sub_id,
			},
		);

		// assert that the subscription state is correct
		let (sub_id, sub) = Subscriptions::<T>::iter().next().unwrap();
		assert_eq!(sub.state, SubscriptionState::Active);

		let new_credits: T::Credits = 200u64.into();
		let new_frequency: BlockNumberFor<T> = 2u32.into();

		let new_metadata = if m == 0 {
			None
		} else {
			let metadata_vec = (0..m).map(|_| 1u8).collect::<Vec<_>>();
			Some(BoundedVec::try_from(metadata_vec).unwrap())
		};

		let new_pulse_filter = if l == 0 {
			None
		} else {
			let pulse_filter_vec = (0..l)
				.map(|_| PulsePropertyOf::<<T as pallet::Config>::Pulse>::Round(1u64.into()))
				.collect::<Vec<_>>();
			Some(BoundedVec::try_from(pulse_filter_vec).unwrap())
		};

		let params = UpdateSubParamsOf::<T> {
			sub_id,
			credits: Some(new_credits),
			frequency: Some(new_frequency),
			metadata: Some(new_metadata),
			pulse_filter: Some(new_pulse_filter.clone()),
		};

		#[extrinsic_call]
		_(origin, params);

		// assert that the subscription state is correct
		let sub = Subscriptions::<T>::get(sub_id).unwrap();
		assert_eq!(sub.credits, new_credits);
		assert_eq!(sub.frequency, new_frequency);
		assert_eq!(sub.pulse_filter, new_pulse_filter);
	}

	#[benchmark]
	fn reactivate_subscription() {
		let subscriber: T::AccountId = whitelisted_caller();
		let origin = RawOrigin::Signed(subscriber.clone());
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let pulse_filter = None;
		let sub_id = None;

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call_index,
				frequency,
				metadata,
				pulse_filter,
				sub_id,
			},
		);

		// assert that the subscription state is correct
		let (sub_id, sub) = Subscriptions::<T>::iter().next().unwrap();
		assert_eq!(sub.state, SubscriptionState::Active);

		let _ = IdnManager::<T>::pause_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			sub_id,
		);

		let sub = Subscriptions::<T>::get(sub_id).unwrap();
		assert_eq!(sub.state, SubscriptionState::Paused);

		#[extrinsic_call]
		_(origin, sub_id);

		// assert that the subscription state is correct
		let sub = Subscriptions::<T>::get(sub_id).unwrap();
		assert_eq!(sub.state, SubscriptionState::Active);
	}

	#[benchmark]
	fn quote_subscription(l: Linear<0, { T::MaxPulseFilterLen::get() }>) {
		let sibling_account: T::AccountId = [88u8; 32].into();
		let sibling_para_id = 88;
		let origin = RawOrigin::Signed(sibling_account.clone());
		let credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let sub_id = None;

		let pulse_filter = if l == 0 {
			None
		} else {
			let pulse_filter_vec = (0..l)
				.map(|_| PulsePropertyOf::<<T as pallet::Config>::Pulse>::Round(1u64.into()))
				.collect::<Vec<_>>();
			Some(BoundedVec::try_from(pulse_filter_vec).unwrap())
		};

		let params = CreateSubParamsOf::<T> {
			credits,
			target: target.clone(),
			call_index,
			frequency,
			metadata,
			pulse_filter,
			sub_id,
		};
		let req_ref = [1; 32];
		let quote_request = QuoteRequest { req_ref, create_sub_params: params.clone() };
		let quote_sub_params = QuoteSubParams { quote_request, call_index };

		#[extrinsic_call]
		_(origin, quote_sub_params);

		let deposit = IdnManager::<T>::calculate_storage_deposit_from_create_params(
			&sibling_account,
			&params,
		);

		System::<T>::assert_last_event(
			Event::<T>::SubQuoted {
				requester: Location::new(1, [Junction::Parachain(sibling_para_id)]),
				quote: Quote { req_ref, fees: 10_000u64.into(), deposit },
			}
			.into(),
		);
	}

	#[benchmark]
	fn get_subscription_info() {
		let sibling_account: T::AccountId = [88u8; 32].into();
		let origin = RawOrigin::Signed(sibling_account.clone());
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1, 0];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let pulse_filter = None;
		let sub_id: T::SubscriptionId = H256::default().into();

		T::Currency::set_balance(&sibling_account, 1_000_000u32.into());

		// Create first subscription
		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(sibling_account.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call_index,
				frequency,
				metadata,
				pulse_filter,
				sub_id: Some(sub_id),
			},
		);

		let req = SubInfoRequestOf::<T> { sub_id, req_ref: [1; 32], call_index: [1, 1] };

		#[extrinsic_call]
		_(origin, req);

		System::<T>::assert_last_event(Event::<T>::SubscriptionDistributed { sub_id }.into());
	}

	/// Benchmark dispatching a single pulse to `p` subscriptions
	#[benchmark]
	fn dispatch_pulse(
		p: Linear<0, { T::MaxPulseFilterLen::get() }>,
		s: Linear<1, { T::MaxSubscriptions::get() }>,
	) {
		let subscriber: T::AccountId = whitelisted_caller();
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let pulse_filter = None;
		let sub_id: T::SubscriptionId = H256::default().into();

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		// Create first subscription
		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			CreateSubParamsOf::<T> {
				credits,
				target: target.clone(),
				call_index,
				frequency,
				metadata,
				pulse_filter,
				sub_id: Some(sub_id),
			},
		);

		// Fill up the subscriptions with the given number of subscriptions (minus the already
		// created one)
		fill_up_subscriptions::<T>(s - 1, p);

		assert_eq!(Subscriptions::<T>::iter().count(), s as usize);

		// Create a pulse to dispatch
		let pulse = T::Pulse::default();

		#[block]
		{
			let _ = IdnManager::<T>::dispatch(vec![pulse]);
		}

		// Verify the first subscription was updated
		let sub = Subscriptions::<T>::get(sub_id).unwrap();
		assert!(sub.last_delivered.is_some());
	}

	/// Fill up the subscriptions with the given number of subscriptions and pulse filter length
	fn fill_up_subscriptions<T: Config>(s: u32, p: u32)
	where
		T::Credits: From<u64>,
		T::Currency: Mutate<T::AccountId>
	{
		let subscriber: T::AccountId = whitelisted_caller();
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();

		T::Currency::set_balance(&subscriber, u32::MAX.into());

		// Create s subscriptions
		for _ in 0..s {
			let pulse_filter = if p == 0 {
				None
			} else {
				let pulse_filter_vec = (0..p)
					.map(|_| {
						PulsePropertyOf::<<T as pallet::Config>::Pulse>::Round(
							T::Pulse::default().round(),
						)
					})
					.collect::<Vec<_>>();
				Some(BoundedVec::try_from(pulse_filter_vec).unwrap())
			};

			let res = IdnManager::<T>::create_subscription(
				<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
				CreateSubParamsOf::<T> {
					credits,
					target: target.clone(),
					call_index,
					frequency,
					metadata: None,
					pulse_filter,
					sub_id: None,
				},
			);

			assert!(res.is_ok(), "{:?}", res.unwrap_err());

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
