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

//! # Tests for the IDN Manager pallet

use crate::{
	tests::mock::{Balances, ExtBuilder, Test, *},
	traits::{DepositCalculator, FeesManager},
	Config, Error, Event, HoldReason, SubscriptionState, Subscriptions,
};
use frame_support::{
	assert_noop, assert_ok,
	traits::fungible::{InspectHold, Mutate},
	BoundedVec,
};
use idn_traits::rand::Dispatcher;
use sp_core::H256;
use sp_runtime::AccountId32;
use xcm::v5::{Junction, Location};

const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
const BOB: AccountId32 = AccountId32::new([2u8; 32]);

fn event_emitted(event: Event<Test>) -> bool {
	System::events().iter().any(|record| {
		if let RuntimeEvent::IdnManager(ref e) = &record.event {
			e == &event
		} else {
			false
		}
	})
}

#[test]
fn create_subscription_works() {
	ExtBuilder::build().execute_with(|| {
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// assert Subscriptions storage map is empty before creating a subscription
		assert_eq!(Subscriptions::<Test>::iter().count(), 0);

		// assert that the subscription has been created
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			amount,
			target.clone(),
			frequency,
			None
		));

		assert_eq!(Subscriptions::<Test>::iter().count(), 1);

		let (sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		// assert that the correct fees have been held
		let fees = <Test as Config>::FeesManager::calculate_subscription_fees(amount);
		let deposit = <Test as Config>::DepositCalculator::calculate_storage_deposit(&subscription);
		assert_eq!(Balances::free_balance(&ALICE), initial_balance - fees - deposit);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE), fees);
		assert_eq!(Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE), deposit);

		// assert that the subscription details are correct
		assert_eq!(subscription.details.subscriber, ALICE);
		assert_eq!(subscription.details.amount, amount);
		assert_eq!(subscription.details.target, target);
		assert_eq!(subscription.details.frequency, frequency);
		assert_eq!(
			subscription.details.metadata,
			BoundedVec::<u8, SubMetadataLen>::try_from(vec![]).unwrap()
		);

		// assert that the correct event has been emitted
		assert!(event_emitted(Event::<Test>::SubscriptionCreated { sub_id }));
	});
}

#[test]
fn create_subscription_fails_if_insufficient_balance() {
	ExtBuilder::build().execute_with(|| {
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10);

		assert_noop!(
			IdnManager::create_subscription(
				RuntimeOrigin::signed(ALICE),
				amount,
				target,
				frequency,
				None
			),
			Error::<Test>::InsufficientBalance
		);

		// Assert the SubscriptionCreated event was not emitted
		assert!(!System::events().iter().any(|record| matches!(
			record.event,
			RuntimeEvent::IdnManager(Event::<Test>::SubscriptionCreated { sub_id: _ })
		)));
	});
}

#[test]
fn create_subscription_fails_if_sub_already_exists() {
	ExtBuilder::build().execute_with(|| {
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			amount,
			target.clone(),
			frequency,
			None
		));

		// erase all events
		System::reset_events();

		assert_noop!(
			IdnManager::create_subscription(
				RuntimeOrigin::signed(ALICE),
				amount,
				target,
				frequency,
				None
			),
			Error::<Test>::SubscriptionAlreadyExists
		);

		// Assert the SubscriptionCreated event was not emitted
		assert!(!System::events().iter().any(|record| matches!(
			record.event,
			RuntimeEvent::IdnManager(Event::<Test>::SubscriptionCreated { sub_id: _ })
		)));
	});
}

#[test]
// Todo: https://github.com/ideal-lab5/idn-sdk/issues/77
#[ignore]
fn distribute_randomness_works() {
	ExtBuilder::build().execute_with(|| {
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE),
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
	ExtBuilder::build().execute_with(|| {
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
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
		assert_ok!(IdnManager::kill_subscription(RuntimeOrigin::signed(ALICE), sub_id));
		assert!(Subscriptions::<Test>::get(sub_id).is_none());

		assert!(event_emitted(Event::<Test>::SubscriptionFinished { sub_id }));
	});
}

#[test]
fn kill_subscription_fails_if_sub_does_not_exist() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = H256::from_slice(&[1; 32]);

		assert_noop!(
			IdnManager::kill_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionDoesNotExist
		);

		// Assert the SubscriptionFinished event was not emitted
		assert!(!event_emitted(Event::<Test>::SubscriptionFinished { sub_id }));
	});
}

#[test]
fn test_update_subscription() {
	ExtBuilder::build().execute_with(|| {
		let original_amount = 10;
		let original_frequency = 2;
		// TODO as part of https://github.com/ideal-lab5/idn-sdk/issues/104
		// make these two variables dynamic and test lt and gt
		let new_amount = 20;
		let new_frequency = 4;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			original_amount,
			target.clone(),
			original_frequency,
			metadata.clone()
		));

		let (sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		let original_fees =
			<Test as Config>::FeesManager::calculate_subscription_fees(original_amount);
		let original_deposit =
			<Test as Config>::DepositCalculator::calculate_storage_deposit(&subscription);
		let balance_after_create = initial_balance - original_fees - original_deposit;

		// assert correct balance on subscriber after creating subscription
		assert_eq!(Balances::free_balance(&ALICE), balance_after_create);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE), original_fees);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			original_deposit
		);

		assert_ok!(IdnManager::update_subscription(
			RuntimeOrigin::signed(ALICE),
			sub_id,
			new_amount,
			new_frequency
		));

		// TODO implement a way to refund or take the difference in fees https://github.com/ideal-lab5/idn-sdk/issues/104

		let subscription = Subscriptions::<Test>::get(sub_id).unwrap();

		// assert subscription details has been updated
		assert_eq!(subscription.details.amount, new_amount);
		assert_eq!(subscription.details.frequency, new_frequency);

		assert!(event_emitted(Event::<Test>::SubscriptionUpdated { sub_id }));
	});
}

#[test]
fn update_subscription_fails_if_sub_does_not_exists() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = H256::from_slice(&[1; 32]);
		let new_amount = 20;
		let new_frequency = 4;

		assert_noop!(
			IdnManager::update_subscription(
				RuntimeOrigin::signed(ALICE),
				sub_id,
				new_amount,
				new_frequency
			),
			Error::<Test>::SubscriptionDoesNotExist
		);

		// Assert the SubscriptionUpdated event was not emitted
		assert!(!event_emitted(Event::<Test>::SubscriptionUpdated { sub_id }));
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
	ExtBuilder::build().execute_with(|| {
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		let free_balance = Balances::free_balance(&ALICE);

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// Test pause and reactivate subscription
		assert_ok!(IdnManager::pause_subscription(RuntimeOrigin::signed(ALICE.clone()), sub_id));
		assert_eq!(Subscriptions::<Test>::get(sub_id).unwrap().state, SubscriptionState::Paused);
		assert_ok!(IdnManager::reactivate_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			sub_id
		));
		assert_eq!(Subscriptions::<Test>::get(sub_id).unwrap().state, SubscriptionState::Active);

		// Assert current free balance is the same as the free balance before pausing and
		// reactivating
		assert_eq!(Balances::free_balance(&ALICE), free_balance);

		assert!(event_emitted(Event::<Test>::SubscriptionPaused { sub_id }));

		assert!(event_emitted(Event::<Test>::SubscriptionReactivated { sub_id }));
	});
}

#[test]
fn pause_subscription_fails_if_sub_does_not_exists() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = H256::from_slice(&[1; 32]);

		assert_noop!(
			IdnManager::pause_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionDoesNotExist
		);

		// Assert the SubscriptionPaused event was not emitted
		assert!(!event_emitted(Event::<Test>::SubscriptionPaused { sub_id }));
	});
}

#[test]
fn pause_subscription_fails_if_sub_already_paused() {
	ExtBuilder::build().execute_with(|| {
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		assert_ok!(IdnManager::pause_subscription(RuntimeOrigin::signed(ALICE.clone()), sub_id));

		// erase all events
		System::reset_events();

		assert_noop!(
			IdnManager::pause_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionAlreadyPaused
		);

		// Assert the SubscriptionPaused event was not emitted
		assert!(!event_emitted(Event::<Test>::SubscriptionPaused { sub_id }));
	});
}

#[test]
fn reactivate_subscription_fails_if_sub_does_not_exists() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = H256::from_slice(&[1; 32]);

		assert_noop!(
			IdnManager::reactivate_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionDoesNotExist
		);

		// Assert the SubscriptionReactivated event was not emitted
		assert!(!event_emitted(Event::<Test>::SubscriptionReactivated { sub_id }));
	});
}

#[test]
fn reactivate_subscriptio_fails_if_sub_already_active() {
	ExtBuilder::build().execute_with(|| {
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		assert_noop!(
			IdnManager::reactivate_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionAlreadyActive
		);

		// Assert the SubscriptionReactivated event was not emitted
		assert!(!event_emitted(Event::<Test>::SubscriptionReactivated { sub_id }));
	});
}

#[test]
fn operations_fail_if_origin_is_not_the_subscriber() {
	ExtBuilder::build().execute_with(|| {
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let metadata = None;
		let initial_balance = 10_000_000;

		// Set balance for Alice and Bob
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);
		<Test as Config>::Currency::set_balance(&BOB, initial_balance);

		// Create subscription for Alice
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		// Retrieve the subscription ID created
		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// Attempt to kill the subscription using Bob's origin (should fail)
		assert_noop!(
			IdnManager::kill_subscription(RuntimeOrigin::signed(BOB.clone()), sub_id),
			Error::<Test>::NotSubscriber
		);

		// Assert the SubscriptionFinished event was not emitted
		assert!(!event_emitted(Event::<Test>::SubscriptionFinished { sub_id }));

		// Attempt to pause the subscription using Bob's origin (should fail)
		assert_noop!(
			IdnManager::pause_subscription(RuntimeOrigin::signed(BOB.clone()), sub_id),
			Error::<Test>::NotSubscriber
		);

		// Assert the SubscriptionPaused event was not emitted
		assert!(!event_emitted(Event::<Test>::SubscriptionPaused { sub_id }));

		// Attempt to update the subscription using Bob's origin (should fail)
		let new_amount = amount + 10;
		let new_frequency = frequency + 1;
		assert_noop!(
			IdnManager::update_subscription(
				RuntimeOrigin::signed(BOB.clone()),
				sub_id,
				new_amount,
				new_frequency
			),
			Error::<Test>::NotSubscriber
		);

		// Attempt to reactivate the subscription using Bob's origin (should fail)
		assert_noop!(
			IdnManager::reactivate_subscription(RuntimeOrigin::signed(BOB.clone()), sub_id),
			Error::<Test>::NotSubscriber
		);

		// Assert the SubscriptionReactivated event was not emitted
		assert!(!event_emitted(Event::<Test>::SubscriptionReactivated { sub_id }));
	});
}
