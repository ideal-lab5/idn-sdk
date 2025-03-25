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
use crate::{pallet::Pallet as IdnManager, PulsePropertyOf};
use frame_benchmarking::v2::*;
use frame_support::{
	traits::{fungible::Mutate, OriginTrait},
	BoundedVec,
};
use frame_system::RawOrigin;

#[benchmarks(
    where
        T::Credits: From<u64>,
        <T::Pulse as Pulse>::Round: From<u64>,
		T::Currency: Mutate<T::AccountId>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn create_subscription() {
		let subscriber: T::AccountId = whitelisted_caller();
		let origin = RawOrigin::Signed(subscriber.clone());
		let credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let pulse_filter_len = T::PulseFilterLen::get() as usize;
		let pulse_filter = Some(
			BoundedVec::try_from(
				// make the vec as big as possible
				vec![PulsePropertyOf::<T>::Round(1u64.into()); pulse_filter_len],
			)
			.unwrap(),
		);
		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		#[extrinsic_call]
		_(origin, credits, target.clone(), call_index, frequency, metadata, pulse_filter);

		// assert that the subscription details are correct
		let (_, sub) = Subscriptions::<T>::iter().next().unwrap();
		assert_eq!(sub.details.subscriber, subscriber);
		assert_eq!(sub.details.target, target);
		assert_eq!(sub.credits, credits);
		assert_eq!(sub.frequency, frequency);
		assert_eq!(
			sub.details.metadata,
			BoundedVec::<u8, T::SubMetadataLen>::try_from(vec![]).unwrap(),
		);
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

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			credits,
			target.clone(),
			call_index,
			frequency,
			metadata,
			pulse_filter,
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

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			credits,
			target.clone(),
			call_index,
			frequency,
			metadata,
			pulse_filter,
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
	fn update_subscription() {
		let subscriber: T::AccountId = whitelisted_caller();
		let origin = RawOrigin::Signed(subscriber.clone());
		let credits: T::Credits = 100u64.into();
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency: BlockNumberFor<T> = 1u32.into();
		let metadata = None;
		let pulse_filter = None;

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			credits,
			target.clone(),
			call_index,
			frequency,
			metadata,
			pulse_filter,
		);

		// assert that the subscription state is correct
		let (sub_id, sub) = Subscriptions::<T>::iter().next().unwrap();
		assert_eq!(sub.state, SubscriptionState::Active);

		let pulse_filter_len = T::PulseFilterLen::get() as usize;
		let new_credits: T::Credits = 200u64.into();
		let new_frequency: BlockNumberFor<T> = 2u32.into();
		let new_pulse_filter = Some(
			BoundedVec::try_from(
				// make the vec as big as possible
				vec![PulsePropertyOf::<T>::Round(1u64.into()); pulse_filter_len],
			)
			.unwrap(),
		);

		#[extrinsic_call]
		_(origin, sub_id, new_credits, new_frequency, new_pulse_filter.clone());

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

		T::Currency::set_balance(&subscriber, 1_000_000u32.into());

		let _ = IdnManager::<T>::create_subscription(
			<T as frame_system::Config>::RuntimeOrigin::signed(subscriber.clone()),
			credits,
			target.clone(),
			call_index,
			frequency,
			metadata,
			pulse_filter,
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

	impl_benchmark_test_suite!(
		IdnManager,
		crate::tests::mock::ExtBuilder::build(),
		crate::tests::mock::Test
	);
}
