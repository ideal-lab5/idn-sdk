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

//! # Tests for the IDN Manager pallet

use crate::{
	tests::mock::{new_test_ext, Balances, DepositCalculatorImpl, FeesCalculatorImpl, Test, *},
	traits::{DepositCalculator, FeesCalculator},
	Config, Error, HoldReason, SubscriptionState, Subscriptions,
};
use frame_support::{
	assert_noop, assert_ok,
	traits::fungible::{InspectHold, Mutate},
	BoundedVec,
};
use idn_traits::rand::Dispatcher;
use xcm::v5::{Junction, Location};

#[test]
fn create_subscription_works() {
	new_test_ext().execute_with(|| {
		let subscriber: u64 = 1;
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&subscriber, initial_balance);

		// assert Subscriptions storage map is empty before creating a subscription
		assert_eq!(Subscriptions::<Test>::iter().count(), 0);

		// assert that the subscription has been created
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			amount,
			target.clone(),
			frequency,
			None
		));
		assert_eq!(Subscriptions::<Test>::iter().count(), 1);

		let (_sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		// assert that the correct fees have been held
		let fees = FeesCalculatorImpl::calculate_subscription_fees(amount);
		let deposit = DepositCalculatorImpl::calculate_storage_deposit(&subscription);
		assert_eq!(Balances::free_balance(&subscriber), initial_balance - fees - deposit);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &subscriber), fees);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &subscriber),
			deposit
		);

		// assert that the subscription details are correct
		assert_eq!(subscription.details.subscriber, subscriber);
		assert_eq!(subscription.details.amount, amount);
		assert_eq!(subscription.details.target, target);
		assert_eq!(subscription.details.frequency, frequency);
		assert_eq!(
			subscription.details.metadata,
			BoundedVec::<u8, SubMetadataLenWrapper>::try_from(vec![]).unwrap()
		);
	});
}

#[test]
fn create_subscription_fails_if_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let subscriber: u64 = 1;
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&subscriber, 10);

		assert_noop!(
			IdnManager::create_subscription(
				RuntimeOrigin::signed(subscriber),
				amount,
				target,
				frequency,
				None
			),
			Error::<Test>::InsufficientBalance
		);
	});
}

#[test]
// Todo: https://github.com/ideal-lab5/idn-sdk/issues/77
#[ignore]
fn distribute_randomness_works() {
	new_test_ext().execute_with(|| {
		let subscriber: u64 = 1;
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			amount,
			target.clone(),
			frequency,
			None
		));

		let rnd = [0; 32];

		assert_ok!(IdnManager::dispatch(rnd.into()));

		assert_eq!(Subscriptions::<Test>::iter().count(), 1, "Subscriptions count is not 1");

		let (_sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		assert_eq!(subscription.state, SubscriptionState::Active, "Subscription is not Active");
	});
}

#[test]
fn test_kill_subscription() {
	new_test_ext().execute_with(|| {
		let subscriber = 1;
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();
		// TOOD assert:
		// - correct fees are refunded
		// - correct storage deposit is refunded
		// - correct fees were collected
		// https://github.com/ideal-lab5/idn-sdk/issues/107
		assert_ok!(IdnManager::kill_subscription(RuntimeOrigin::signed(subscriber), sub_id));
		assert!(Subscriptions::<Test>::get(sub_id).is_none());
	});
}

#[test]
fn test_update_subscription() {
	new_test_ext().execute_with(|| {
		let subscriber = 1;
		let original_amount = 10;
		let original_frequency = 2;
		// TODO as part of https://github.com/ideal-lab5/idn-sdk/issues/104
		// make these two variables dynamic and test lt and gt
		let new_amount = 20;
		let new_frequency = 4;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&subscriber, initial_balance);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			original_amount,
			target.clone(),
			original_frequency,
			metadata.clone()
		));

		let (sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		let original_fees = FeesCalculatorImpl::calculate_subscription_fees(original_amount);
		let original_deposit = DepositCalculatorImpl::calculate_storage_deposit(&subscription);
		let balance_after_create = initial_balance - original_fees - original_deposit;

		// assert correct balance on subscriber after creating subscription
		assert_eq!(Balances::free_balance(&subscriber), balance_after_create);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &subscriber), original_fees);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &subscriber),
			original_deposit
		);

		assert_ok!(IdnManager::update_subscription(
			RuntimeOrigin::signed(subscriber),
			sub_id,
			new_amount,
			new_frequency
		));

		// TODO implement a way to refund or take the difference in fees https://github.com/ideal-lab5/idn-sdk/issues/104
		// let new_fees = FeesCalculatorImpl::calculate_subscription_fees(new_amount);
		// let new_deposit = DepositCalculatorImpl::calculate_storage_deposit(&subscription);

		// let fees_diff = new_fees - original_fees;
		// let deposit_diff = new_deposit - original_deposit;

		// let balance_after_update = balance_after_create - fees_diff - deposit_diff;

		// // assert correct balances after update
		// assert_eq!(Balances::free_balance(&subscriber), balance_after_update);
		// assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &subscriber), new_fees);
		// assert_eq!(
		// 	Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &subscriber),
		// 	new_deposit
		// );
		// assert_eq!(balance_after_update + new_fees + new_deposit, initial_balance);

		let subscription = Subscriptions::<Test>::get(sub_id).unwrap();

		// assert subscription details has been updated
		assert_eq!(subscription.details.amount, new_amount);
		assert_eq!(subscription.details.frequency, new_frequency);
	});
}

// todo: test credits consumption, it consumes credit by credit and verify that
// - fees are moved to treasury
// - credits are consumed
// - storage dep is refunded
// - subscription is removed
// https://github.com/ideal-lab5/idn-sdk/issues/108

#[test]
fn test_pause_reactivate_subscription() {
	new_test_ext().execute_with(|| {
		let subscriber = 1;
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		let free_balance = Balances::free_balance(&subscriber);

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// Test pause and reactivate subscription
		assert_ok!(IdnManager::pause_subscription(RuntimeOrigin::signed(subscriber), sub_id));
		assert_eq!(Subscriptions::<Test>::get(sub_id).unwrap().state, SubscriptionState::Paused);
		assert_ok!(IdnManager::reactivate_subscription(RuntimeOrigin::signed(subscriber), sub_id));
		assert_eq!(Subscriptions::<Test>::get(sub_id).unwrap().state, SubscriptionState::Active);

		// Assert current free balance is the same as the free balance before pausing and
		// reactivating
		assert_eq!(Balances::free_balance(&subscriber), free_balance);
	});
}
