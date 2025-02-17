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
	Config, Error, SubscriptionOf, SubscriptionState, Subscriptions,
};
use frame_support::{assert_noop, assert_ok, traits::fungible::Mutate, BoundedVec};
use idn_traits::rand::Consumer;
use xcm::v5::{Junction, Location};

#[test]
fn create_subscription_works() {
	new_test_ext().execute_with(|| {
		let subscriber: u64 = 1;
		let para_id: u32 = 100;
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		// assert Subscriptions storage map is empty before creating a subscription
		assert_eq!(Subscriptions::<Test>::iter().count(), 0);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			para_id,
			amount,
			target.clone(),
			frequency,
			None
		));

		assert_eq!(Subscriptions::<Test>::iter().count(), 1);

		let (_sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		assert_eq!(subscription.details.subscriber, subscriber);
		assert_eq!(subscription.details.para_id, para_id);
		assert_eq!(subscription.details.amount, amount);
		assert_eq!(subscription.details.target, target);
		assert_eq!(subscription.details.frequency, frequency);
		assert_eq!(
			subscription.details.metadata,
			BoundedVec::<u8, SubMetadataLenWrapper>::try_from(vec![]).unwrap()
		);
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
		let para_id: u32 = 100;
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		assert_noop!(
			IdnManager::create_subscription(
				RuntimeOrigin::signed(subscriber),
				para_id,
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
fn distribute_randomness_works() {
	new_test_ext().execute_with(|| {
		let subscriber: u64 = 1;
		let para_id: u32 = 100;
		let amount: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			para_id,
			amount,
			target.clone(),
			frequency,
			None
		));

		let rnd = [0; 32];

		assert_ok!(IdnManager::consume(rnd.into()));

		assert_eq!(Subscriptions::<Test>::iter().count(), 1, "Subscriptions count is not 1");

		let (_sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		assert_eq!(subscription.state, SubscriptionState::Active, "Subscription is not Active");
	});
}

#[test]
fn test_pause_subscription() {
	new_test_ext().execute_with(|| {
		let subscriber = 1;
		let para_id = 100;
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			para_id,
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		assert_ok!(IdnManager::pause_subscription(RuntimeOrigin::signed(subscriber), sub_id));
		let subscription = Subscriptions::<Test>::get(sub_id).unwrap();
		assert_eq!(subscription.state, SubscriptionState::Paused);
	});
}

#[test]
fn test_kill_subscription() {
	new_test_ext().execute_with(|| {
		let subscriber = 1;
		let para_id = 100;
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			para_id,
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();
		assert_ok!(IdnManager::kill_subscription(RuntimeOrigin::signed(subscriber), sub_id));
		assert!(Subscriptions::<Test>::get(sub_id).is_none());
	});
}

#[test]
fn test_update_subscription() {
	new_test_ext().execute_with(|| {
		let subscriber = 1;
		let para_id = 100;
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			para_id,
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();
		let new_amount = 20;
		let new_frequency = 4;
		assert_ok!(IdnManager::update_subscription(
			RuntimeOrigin::signed(subscriber),
			sub_id,
			new_amount,
			new_frequency
		));
		let subscription = Subscriptions::<Test>::get(sub_id).unwrap();
		assert_eq!(subscription.details.amount, new_amount);
		assert_eq!(subscription.details.frequency, new_frequency);
	});
}

#[test]
fn test_reactivate_subscription() {
	new_test_ext().execute_with(|| {
		let subscriber = 1;
		let para_id = 100;
		let amount = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			para_id,
			amount,
			target.clone(),
			frequency,
			metadata.clone()
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();
		assert_ok!(IdnManager::pause_subscription(RuntimeOrigin::signed(subscriber), sub_id));
		assert_eq!(Subscriptions::<Test>::get(sub_id).unwrap().state, SubscriptionState::Paused);
		assert_ok!(IdnManager::reactivate_subscription(RuntimeOrigin::signed(subscriber), sub_id));
		assert_eq!(Subscriptions::<Test>::get(sub_id).unwrap().state, SubscriptionState::Active);
	});
}

/// A helper function to compute expected fees.
fn expected_fees(amount: u64) -> u64 {
	FeesCalculatorImpl::calculate_subscription_fees(amount)
}

/// A helper function to compute expected storage deposit for a subscription.
fn expected_storage_deposit(sub: SubscriptionOf<Test>) -> u64 {
	// For example’s sake, assume a constant deposit per subscription.
	DepositCalculatorImpl::calculate_storage_deposit(&sub)
}

#[test]
fn create_subscription_takes_correct_fees_and_deposit() {
	new_test_ext().execute_with(|| {
		let subscriber: u64 = 1;
		let para_id: u32 = 100;
		let amount: u64 = 10; // number of random values
		let frequency: u64 = 5;
		let target = Location::new(1, [Junction::PalletInstance(1)]);

		let initial_balance = 10_000_000;
		<Test as Config>::Currency::set_balance(&subscriber, initial_balance);

		// Create subscription.
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			para_id,
			amount,
			target.clone(),
			frequency,
			None // metadata
		));

		// Check that exactly one subscription is stored.
		assert_eq!(Subscriptions::<Test>::iter().count(), 1);
		let (_, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		// Expected fees and deposit.
		let fees = expected_fees(amount);
		let deposit = expected_storage_deposit(subscription);

		// Check that subscriber's balance decreased by fees + deposit.
		let final_balance = Balances::free_balance(&subscriber);
		assert_eq!(initial_balance - final_balance, fees + deposit);
	});
}

// #[test]
// fn update_subscription_adjusts_storage_deposit_and_reimburses_difference() {
//     new_test_ext().execute_with(|| {
//         let subscriber: u64 = 1;
//         let para_id: u32 = 100;
//         let original_amount: u64 = 10;
//         let original_frequency: u64 = 5;
//         let updated_amount: u64 = 15; // increase subscription amount
//         let updated_frequency: u64 = 5;
//         let target = Location::new(1, [Junction::PalletInstance(1)]);

//         // Create initial subscription.
//         assert_ok!(IdnManager::create_subscription(
//             RuntimeOrigin::signed(subscriber),
//             para_id,
//             original_amount,
//             target.clone(),
//             original_frequency,
//             None
//         ));

//         // Get expected deposits.
//         let initial_deposit = expected_storage_deposit(original_amount);
//         let updated_deposit = expected_storage_deposit(updated_amount);

//         // Assume update_subscription extrinsic adjusts deposit.
//         // For this test the update subtracts the difference (if increased)
//         // or reimburses the difference (if decreased) accordingly.
//         // Capture balance before update.
//         let balance_before = Balances::free_balance(&subscriber);

//         // Call update_subscription.
//         assert_ok!(IdnManager::update_subscription(
//             RuntimeOrigin::signed(subscriber),
//             Subscriptions::<Test>::iter().next().unwrap().0,
//             updated_amount,
//             updated_frequency
//         ));

//         // After update, compute expected balance change.
//         // In case of increase, additional deposit is held.
//         // In case of decrease, difference is refunded.
//         let deposit_diff = if updated_deposit > initial_deposit {
//             updated_deposit - initial_deposit
//         } else {
//             0 // or negative value if refunded; for testing, compare held amounts.
//         };

//         // Check balance differences
//         let balance_after = Balances::free_balance(&subscriber);
//         // For example, if increased, balance decreased further by deposit_diff.
//         // (If decreased, your update_subscription implementation should reimburse the
// difference.)         assert_eq!(balance_before - balance_after, deposit_diff);
//     });
// }

// #[test]
// fn kill_subscription_reimburses_full_deposit_and_remaining_credits() {
// 	new_test_ext().execute_with(|| {
// 		let subscriber: u64 = 1;
// 		let para_id: u32 = 100;
// 		let amount: u64 = 10;
// 		let frequency: u64 = 5;
// 		let target = Location::new(1, [Junction::PalletInstance(1)]);

// 		// Capture balance before creation.
// 		let initial_balance = Balances::free_balance(&subscriber);

// 		// Create subscription.
// 		assert_ok!(IdnManager::create_subscription(
// 			RuntimeOrigin::signed(subscriber),
// 			para_id,
// 			amount,
// 			target.clone(),
// 			frequency,
// 			None
// 		));

// 		// Expected fees and deposit.
// 		let fees = expected_fees(amount);
// 		let deposit = expected_storage_deposit(amount);

// 		// Subscriber's balance should be reduced by fees+deposit.
// 		let balance_after_create = Balances::free_balance(&subscriber);
// 		assert_eq!(initial_balance - balance_after_create, fees + deposit);

// 		// Now, kill the subscription.
// 		let (sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();
// 		assert_ok!(IdnManager::kill_subscription(RuntimeOrigin::signed(subscriber), sub_id));

// 		// After killing, full deposit should be refunded.
// 		let balance_after_kill = Balances::free_balance(&subscriber);
// 		// Also, if any randomness credits remain on the subscription they should be
// 		// refunded.
// 		// For this test, assume all remaining credits (if any) are simply
// 		// refunded.
// 		// The total refund equals deposit plus any remaining credits
// 		// value; assume credits are valued 1:1.
// 		let expected_refund = deposit + subscription.credits_left;
// 		assert_eq!(balance_after_kill - balance_after_create, expected_refund);

// 		// Finally, subscription storage should be removed.
// 		assert!(!Subscriptions::<Test>::contains_key(sub_id));
// 	});
// }
