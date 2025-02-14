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

use crate::{tests::mock::*, Config, Error, SubscriptionState, Subscriptions};
use frame_support::{assert_noop, assert_ok, traits::fungible::Mutate, BoundedVec};
use idn_traits::rand::Consumer;
use xcm::v5::Location;

#[test]
fn create_subscription_works() {
	new_test_ext().execute_with(|| {
		let subscriber: u64 = 1;
		let para_id: u32 = 100;
		let amount: u64 = 50;
		let target = Location::new(1, [xcm::v5::Junction::PalletInstance(1)]);
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
	});
}

// #[test]
// fn create_subscription_fails_if_amount_exceeds_max() {
// 	new_test_ext().execute_with(|| {
// 		let subscriber: u64 = 1;
// 		let para_id: u32 = 100;
// 		let amount = MaxSubscriptionamount::get() + 1;
// 		let target = Location::new(1, [xcm::v5::Junction::PalletInstance(1)]);
// 		let frequency: u64 = 10;

// 		assert_noop!(
// 			IdnManager::create_subscription(
// 				RuntimeOrigin::signed(subscriber),
// 				para_id,
// 				amount,
// 				target.clone(),
// 				frequency,
// 				None
// 			),
// 			Error::<Test>::InvalidSubscriptionamount
// 		);
// 	});
// }

#[test]
fn create_subscription_fails_if_insufficient_balance() {
	new_test_ext().execute_with(|| {
		let subscriber: u64 = 1;
		let para_id: u32 = 100;
		let amount: u64 = 50;
		let target = Location::new(1, [xcm::v5::Junction::PalletInstance(1)]);
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
		let target = Location::new(1, [xcm::v5::Junction::PalletInstance(1)]);
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
