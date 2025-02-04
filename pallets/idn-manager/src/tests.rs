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

use crate::{mock::*, traits::FeesCalculator, Config, Subscriptions};
use frame_support::{assert_noop, assert_ok, sp_runtime::traits::Hash, traits::fungible::Mutate};
use xcm::v5::Location;

#[test]
fn create_subscription_works() {
	new_test_ext().execute_with(|| {
		let subscriber: u64 = 1;
		let para_id: u32 = 100;
		let duration: u64 = 50;
		let target = Location::new(1, [xcm::v5::Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&subscriber, 10_000_000);

		// assert Subscriptions storage map is empty
		assert_eq!(Subscriptions::<Test>::iter().count(), 0);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(subscriber),
			para_id,
			duration,
			target.clone(),
			frequency
		));

		assert_eq!(Subscriptions::<Test>::iter().count(), 1);

		let fees = FeesCalculatorImpl::calculate_subscription_fees(duration);

		// let sub_id = IdnManager::calculate_subscription_id(
		// 	subscriber,
		// 	para_id,
		// 	duration,
		// 	target.clone(),
		// 	frequency,
		// );
		// let subscription = IdnManager::subscriptions(sub_id).unwrap();

		//  assert_eq!(subscription.details.subscriber, subscriber);
		//  assert_eq!(subscription.details.para_id, para_id);
		//  assert_eq!(subscription.details.end_block, frame_system::Pallet::<Test>::block_number()
		// + duration);  assert_eq!(subscription.details.target, target);
		//  assert_eq!(subscription.details.frequency, frequency);
		assert!(true);
	});
}

//  #[test]
//  fn create_subscription_fails_if_duration_exceeds_max() {
//      new_test_ext().execute_with(|| {
//          let subscriber: u64 = 1;
//          let para_id: u32 = 100;
//          let duration: u64 = 150; // Exceeds MaxSubscriptionDuration
//          let target = MultiLocation::X1(xcm::v0::Junction::Parent);
//          let fee: u64 = 10;
//          let frequency: u64 = 10;

//          assert_noop!(
//              IdnManager::create_subscription(
//                  Origin::signed(subscriber),
//                  para_id,
//                  duration,
//                  target,
//                  frequency
//              ),
//              Error::<Test>::InvalidSubscriptionDuration
//          );
//      });
//  }

//  #[test]
//  fn create_subscription_fails_if_insufficient_balance() {
//      new_test_ext().execute_with(|| {
//          let subscriber: u64 = 1;
//          let para_id: u32 = 100;
//          let duration: u64 = 50;
//          let target = MultiLocation::X1(xcm::v0::Junction::Parent);
//          let fee: u64 = 1000; // Assume this is more than the subscriber's balance
//          let frequency: u64 = 10;

//          assert_noop!(
//              IdnManager::create_subscription(
//                  Origin::signed(subscriber),
//                  para_id,
//                  duration,
//                  target,
//                  frequency
//              ),
//              Error::<Test>::InsufficientBalance
//          );
//      });
//  }

//  #[test]
//  fn distribute_randomness_works() {
//      new_test_ext().execute_with(|| {
//          let subscriber: u64 = 1;
//          let para_id: u32 = 100;
//          let duration: u64 = 50;
//          let target = MultiLocation::X1(xcm::v0::Junction::Parent);
//          let fee: u64 = 10;
//          let frequency: u64 = 10;

//          assert_ok!(IdnManager::create_subscription(
//              Origin::signed(subscriber),
//              para_id,
//              duration,
//              target.clone(),
//              frequency
//          ));

//          let sub_id = IdnManager::calculate_subscription_id(subscriber, para_id, duration,
// target.clone(), frequency);          let rnd: [u8; 32] = [0; 32];

//          assert_ok!(IdnManager::consume(rnd.into()));

//          let subscription = IdnManager::subscriptions(sub_id).unwrap();
//          assert_eq!(subscription.status, SubscriptionStatus::Active);
//      });
//  }
