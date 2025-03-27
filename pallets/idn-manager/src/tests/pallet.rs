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
	runtime_decl_for_idn_manager_api::IdnManagerApiV1,
	tests::mock::{self, Balances, ExtBuilder, Test, *},
	traits::{BalanceDirection, DepositCalculator, DiffBalance, FeesManager},
	Config, CreateSubParamsOf, Error, Event, HoldReason, PulseFilterOf, PulsePropertyOf,
	SubscriptionState, Subscriptions, UpdateSubParamsOf,
};
use frame_support::{
	assert_noop, assert_ok,
	pallet_prelude::Zero,
	traits::{
		fungible::{InspectHold, Mutate},
		OnFinalize,
	},
	BoundedVec,
};
use idn_traits::pulse::Dispatcher;
use sp_runtime::{AccountId32, DispatchError, TokenError};
use xcm::v5::{Junction, Location};

const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
const BOB: AccountId32 = AccountId32::new([2u8; 32]);

fn event_not_emitted(event: Event<Test>) -> bool {
	!System::events().iter().any(|record| {
		if let RuntimeEvent::IdnManager(ref e) = &record.event {
			e == &event
		} else {
			false
		}
	})
}

fn update_subscription(
	subscriber: AccountId32,
	original_credits: u64,
	original_frequency: u64,
	new_credits: u64,
	new_frequency: u64,
) {
	let target = Location::new(1, [Junction::PalletInstance(1)]);
	let metadata = None;
	let initial_balance = 99_990_000_000_000_000;

	<Test as Config>::Currency::set_balance(&subscriber, initial_balance);

	assert_ok!(IdnManager::create_subscription(
		RuntimeOrigin::signed(subscriber.clone()),
		CreateSubParamsOf::<Test> {
			credits: original_credits,
			target: target.clone(),
			call_index: [1; 2],
			frequency: original_frequency,
			metadata: metadata.clone(),
			pulse_filter: None,
			sub_id: None,
		}
	));

	// Get the sub_id from the last emitted event
	let sub_id = System::events()
		.iter()
		.rev()
		.find_map(|record| {
			if let RuntimeEvent::IdnManager(Event::<Test>::SubscriptionCreated { sub_id }) =
				&record.event
			{
				Some(*sub_id)
			} else {
				None
			}
		})
		.expect("SubscriptionCreated event should be emitted");

	let subscription = Subscriptions::<Test>::get(sub_id).unwrap();

	assert_eq!(subscription.created_at, System::block_number());
	assert_eq!(subscription.updated_at, System::block_number());

	assert_eq!(subscription.details.subscriber, subscriber);

	let original_fees =
		<Test as Config>::FeesManager::calculate_subscription_fees(&original_credits);
	let original_deposit =
		<Test as Config>::DepositCalculator::calculate_storage_deposit(&subscription);
	let balance_after_create = initial_balance - original_fees - original_deposit;

	// assert correct balance on subscriber after creating subscription
	assert_eq!(Balances::free_balance(&subscriber), balance_after_create);
	assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &subscriber), original_fees);
	assert_eq!(
		Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &subscriber),
		original_deposit
	);

	// Advance a block
	System::set_block_number(System::block_number() + 1);

	assert_ok!(IdnManager::update_subscription(
		RuntimeOrigin::signed(subscriber.clone()),
		UpdateSubParamsOf::<Test> {
			sub_id,
			credits: new_credits,
			frequency: new_frequency,
			pulse_filter: None
		}
	));

	let new_fees = <Test as Config>::FeesManager::calculate_subscription_fees(&new_credits);
	let new_deposit = <Test as Config>::DepositCalculator::calculate_storage_deposit(&subscription);

	let fees_diff: i64 = new_fees as i64 - original_fees as i64;

	let deposit_diff: i64 = new_deposit as i64 - original_deposit as i64;

	// We are using fixed-width integer types for credits and frequency, so Subscription objects
	// can't change in size with this mock. Unit tests are in place insted to ensure the correct
	// behaviour in case of other types used.
	assert!(deposit_diff.is_zero());

	let balance_after_update: u64 =
		(balance_after_create as i64 - fees_diff - deposit_diff).try_into().unwrap();

	// assert fees and deposit diff is correctly handled
	assert_eq!(Balances::free_balance(&subscriber), balance_after_update);
	assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &subscriber), new_fees);
	assert_eq!(
		Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &subscriber),
		new_deposit
	);
	assert_eq!(balance_after_update + new_fees + new_deposit, initial_balance);

	let subscription = Subscriptions::<Test>::get(sub_id).unwrap();

	assert_eq!(subscription.created_at, System::block_number() - 1);
	assert_eq!(subscription.updated_at, System::block_number());

	// assert subscription details has been updated
	assert_eq!(subscription.credits, new_credits);
	assert_eq!(subscription.frequency, new_frequency);

	System::assert_last_event(RuntimeEvent::IdnManager(Event::<Test>::SubscriptionUpdated {
		sub_id,
	}));
}

fn mock_rounds_filter(rounds: &Vec<u64>) -> PulseFilterOf<Test> {
	let v: Vec<PulsePropertyOf<Test>> =
		rounds.iter().map(|round| PulsePropertyOf::<Test>::Round(*round)).collect();
	BoundedVec::try_from(v).unwrap()
}

#[test]
fn create_subscription_works() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// assert Subscriptions storage map is empty before creating a subscription
		assert_eq!(Subscriptions::<Test>::iter().count(), 0);

		let rounds = vec![0u64, 1, 2];

		// assert that the subscription has been created
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: Some(mock_rounds_filter(&rounds)),
				sub_id: None,
			}
		));

		assert_eq!(Subscriptions::<Test>::iter().count(), 1);

		let (sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		// assert that the correct fees have been held
		let fees = <Test as Config>::FeesManager::calculate_subscription_fees(&credits);
		let deposit = <Test as Config>::DepositCalculator::calculate_storage_deposit(&subscription);
		assert_eq!(Balances::free_balance(&ALICE), initial_balance - fees - deposit);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE), fees);
		assert_eq!(Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE), deposit);

		// assert that the subscription details are correct
		assert_eq!(subscription.details.subscriber, ALICE);
		assert_eq!(subscription.credits, credits);
		assert_eq!(subscription.details.target, target);
		assert_eq!(subscription.frequency, frequency);
		assert_eq!(
			subscription.details.metadata,
			BoundedVec::<u8, SubMetadataLen>::try_from(vec![]).unwrap()
		);

		// assert that the correct event has been emitted
		System::assert_last_event(RuntimeEvent::IdnManager(Event::<Test>::SubscriptionCreated {
			sub_id,
		}));
	});
}

#[test]
fn create_subscription_with_custom_id_works() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;
		let custom_id = [7u8; 32];

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// assert Subscriptions storage map is empty before creating a subscription
		assert_eq!(Subscriptions::<Test>::iter().count(), 0);

		let rounds = vec![0u64, 1, 2];

		// assert that the subscription has been created
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: Some(mock_rounds_filter(&rounds)),
				sub_id: Some(custom_id),
			}
		));

		assert_eq!(Subscriptions::<Test>::iter().count(), 1);

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// assert that the correct event has been emitted
		System::assert_last_event(RuntimeEvent::IdnManager(Event::<Test>::SubscriptionCreated {
			sub_id: custom_id,
		}));

		assert_eq!(sub_id, custom_id);
	});
}

#[test]
fn create_subscription_fails_if_filtering_randomness() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		assert_noop!(
			IdnManager::create_subscription(
				RuntimeOrigin::signed(ALICE.clone()),
				CreateSubParamsOf::<Test> {
					credits,
					target: target.clone(),
					call_index: [1; 2],
					frequency,
					metadata: None,
					pulse_filter: Some(
						BoundedVec::try_from(vec![
							PulsePropertyOf::<Test>::Round(1),
							PulsePropertyOf::<Test>::Rand([1u8; 32]),
							PulsePropertyOf::<Test>::Sig([1u8; 64])
						])
						.unwrap()
					),
					sub_id: None,
				}
			),
			Error::<Test>::FilterRandNotPermitted
		);

		// Assert the SubscriptionCreated event was not emitted
		assert!(!System::events().iter().any(|record| matches!(
			record.event,
			RuntimeEvent::IdnManager(Event::<Test>::SubscriptionCreated { sub_id: _ })
		)));
	});
}

#[test]
fn create_subscription_fails_if_insufficient_balance() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10);

		assert_noop!(
			IdnManager::create_subscription(
				RuntimeOrigin::signed(ALICE),
				CreateSubParamsOf::<Test> {
					credits,
					target,
					call_index: [1; 2],
					frequency,
					metadata: None,
					pulse_filter: None,
					sub_id: None,
				}
			),
			TokenError::FundsUnavailable
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
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// erase all events
		System::reset_events();

		assert_noop!(
			IdnManager::create_subscription(
				RuntimeOrigin::signed(ALICE),
				CreateSubParamsOf::<Test> {
					credits,
					target,
					call_index: [1; 2],
					frequency,
					metadata: None,
					pulse_filter: None,
					sub_id: None,
				}
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
fn create_subscription_fails_if_too_many_subscriptions() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		for i in 0..MaxSubscriptions::get() {
			assert_ok!(IdnManager::create_subscription(
				RuntimeOrigin::signed(ALICE.clone()),
				CreateSubParamsOf::<Test> {
					credits,
					target: target.clone(),
					call_index: [i.try_into().unwrap(); 2],
					frequency,
					metadata: None,
					pulse_filter: None,
					sub_id: None,
				}
			));
		}

		let (latest_sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// erase all events
		System::reset_events();

		assert_noop!(
			IdnManager::create_subscription(
				RuntimeOrigin::signed(ALICE),
				CreateSubParamsOf::<Test> {
					credits,
					target: target.clone(),
					call_index: [1, 2],
					frequency,
					metadata: None,
					pulse_filter: None,
					sub_id: None,
				}
			),
			Error::<Test>::TooManySubscriptions
		);

		// Assert the SubscriptionCreated event was not emitted
		assert!(!System::events().iter().any(|record| matches!(
			record.event,
			RuntimeEvent::IdnManager(Event::<Test>::SubscriptionCreated { sub_id: _ })
		)));

		// Kill latest_sub_id to free up space for future tests
		assert_ok!(IdnManager::kill_subscription(RuntimeOrigin::signed(ALICE), latest_sub_id));

		// After removing the subscription, we should be able to create a new one
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE),
			CreateSubParamsOf::<Test> {
				credits,
				target,
				call_index: [1, 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));
	});
}

#[test]
fn test_kill_subscription() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: metadata.clone(),
				pulse_filter: None,
				sub_id: None,
			}
		));

		let (sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();

		// assert that the correct fees have been held
		let fees = <Test as Config>::FeesManager::calculate_subscription_fees(&credits);
		let deposit = <Test as Config>::DepositCalculator::calculate_storage_deposit(&subscription);
		assert_eq!(Balances::free_balance(&ALICE), initial_balance - fees - deposit);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE), fees);
		assert_eq!(Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE), deposit);
		assert_ok!(IdnManager::kill_subscription(RuntimeOrigin::signed(ALICE), sub_id));
		assert!(Subscriptions::<Test>::get(sub_id).is_none());

		// assert remaining fees and balance refunded
		assert_eq!(Balances::free_balance(&ALICE), initial_balance);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE), 0u64);
		assert_eq!(Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE), 0u64);

		System::assert_last_event(RuntimeEvent::IdnManager(Event::<Test>::SubscriptionRemoved {
			sub_id,
		}));
	});
}

#[test]
fn kill_subscription_fails_if_sub_does_not_exist() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [0xff; 32];

		assert_noop!(
			IdnManager::kill_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionDoesNotExist
		);

		// Assert the SubscriptionRemoved event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionRemoved { sub_id }));
	});
}

#[test]
fn on_finalize_removes_zero_credit_subscriptions() {
	ExtBuilder::build().execute_with(|| {
		// Setup - Create a subscription
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Get the subscription ID
		let (sub_id, mut subscription) = Subscriptions::<Test>::iter().next().unwrap();

		let fees = <Test as Config>::FeesManager::calculate_subscription_fees(&credits);
		let deposit = <Test as Config>::DepositCalculator::calculate_storage_deposit(&subscription);
		assert_eq!(Balances::free_balance(&ALICE), initial_balance - fees - deposit);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE), fees);
		assert_eq!(Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE), deposit);

		// Manually set credits to zero
		subscription.credits_left = Zero::zero();
		Subscriptions::<Test>::insert(sub_id, subscription);

		// Verify subscription exists before on_finalize
		assert!(Subscriptions::<Test>::contains_key(sub_id));

		// Call on_finalize directly
		let current_block = System::block_number();
		crate::Pallet::<Test>::on_finalize(current_block);

		// Verify subscription was removed
		assert!(!Subscriptions::<Test>::contains_key(sub_id));

		// assert there are no remaining fees and balance refunded
		assert_eq!(Balances::free_balance(&ALICE), initial_balance - fees);

		assert_eq!(Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE), 0u64);

		// Verify event was emitted
		System::assert_last_event(RuntimeEvent::IdnManager(Event::<Test>::SubscriptionRemoved {
			sub_id,
		}));
	});
}

#[test]
fn test_update_subscription() {
	ExtBuilder::build().execute_with(|| {
		struct SubParams {
			credits: u64,
			frequency: u64,
		}
		struct SubUpdate {
			old: SubParams,
			new: SubParams,
		}

		let updates: Vec<SubUpdate> = vec![
			SubUpdate {
				old: SubParams { credits: 10, frequency: 2 },
				new: SubParams { credits: 20, frequency: 4 },
			},
			SubUpdate {
				old: SubParams { credits: 100, frequency: 20 },
				new: SubParams { credits: 20, frequency: 4 },
			},
			SubUpdate {
				old: SubParams { credits: 100, frequency: 20 },
				new: SubParams { credits: 100, frequency: 20 },
			},
			SubUpdate {
				old: SubParams { credits: 100, frequency: 1 },
				new: SubParams { credits: 9_999_999_999_999, frequency: 1 },
			},
			SubUpdate {
				old: SubParams { credits: 9_999_999_999_999, frequency: 1 },
				new: SubParams { credits: 100, frequency: 1 },
			},
		];
		for i in 0..updates.len() {
			let update = &updates[i];
			update_subscription(
				AccountId32::new([i.try_into().unwrap(); 32]),
				update.old.credits,
				update.old.frequency,
				update.new.credits,
				update.new.frequency,
			);
		}
	});
}

#[test]
fn update_subscription_fails_if_sub_does_not_exists() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [0xff; 32];
		let new_credits = 20;
		let new_frequency = 4;

		assert_noop!(
			IdnManager::update_subscription(
				RuntimeOrigin::signed(ALICE),
				UpdateSubParamsOf::<Test> {
					sub_id,
					credits: new_credits,
					frequency: new_frequency,
					pulse_filter: None
				}
			),
			Error::<Test>::SubscriptionDoesNotExist
		);

		// Assert the SubscriptionUpdated event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionUpdated { sub_id }));
	});
}
#[test]
fn update_subscription_fails_if_filtering_randomness() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		assert_noop!(
			IdnManager::update_subscription(
				RuntimeOrigin::signed(ALICE),
				UpdateSubParamsOf::<Test> {
					sub_id,
					credits,
					frequency,
					pulse_filter: Some(
						BoundedVec::try_from(vec![
							PulsePropertyOf::<Test>::Round(1),
							PulsePropertyOf::<Test>::Rand([1u8; 32]),
							PulsePropertyOf::<Test>::Sig([1u8; 64])
						])
						.unwrap()
					)
				}
			),
			Error::<Test>::FilterRandNotPermitted
		);

		// Assert the SubscriptionUpdated event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionUpdated { sub_id }));
	});
}

#[test]
/// This test makes sure that the correct fees are collected, by consuming credits one by one.
fn test_credits_consumption_and_cleanup() {
	ExtBuilder::build().execute_with(|| {
		// Setup initial conditions
		let credits: u64 = 1_010_000;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 1;
		let initial_balance = 10_000_000_000;
		let mut treasury_balance = 0;
		let pulse = mock::Pulse { rand: [0u8; 32], round: 0, sig: [1u8; 64] };

		// Set up account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);
		<Test as Config>::Currency::set_balance(&TreasuryAccount::get(), treasury_balance);

		// Create subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Get subscription details
		let (sub_id, subscription) = Subscriptions::<Test>::iter().next().unwrap();
		let initial_fees = <Test as Config>::FeesManager::calculate_subscription_fees(&credits);
		let storage_deposit =
			<Test as Config>::DepositCalculator::calculate_storage_deposit(&subscription);

		// Verify initial state
		assert_eq!(
			Balances::free_balance(&ALICE),
			initial_balance - initial_fees - storage_deposit
		);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE), initial_fees);
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			storage_deposit
		);
		assert_eq!(subscription.credits_left, credits);

		// Consume credits one by one
		for i in 0..credits / 1000 {
			// Advance block and run hooks
			System::set_block_number(System::block_number() + 1);

			// Dispatch randomness
			assert_ok!(IdnManager::dispatch(pulse.into()));

			System::assert_last_event(RuntimeEvent::IdnManager(
				Event::<Test>::RandomnessDistributed { sub_id },
			));

			// Verify credit consumption
			let sub = Subscriptions::<Test>::get(sub_id).unwrap();

			let consume_credits = <Test as Config>::FeesManager::get_consume_credits(&sub);

			assert_eq!(
				sub.credits_left,
				credits - (i + 1) * consume_credits,
				"Credit not consumed correctly"
			);

			// Verify fees movement to treasury
			let fees = <Test as Config>::FeesManager::calculate_diff_fees(
				&(credits - i * consume_credits),
				&(credits - (i + 1) * consume_credits),
			)
			.balance;

			treasury_balance += fees;

			assert_eq!(
				Balances::free_balance(&TreasuryAccount::get()),
				treasury_balance,
				"Fees not moved to treasury correctly"
			);

			System::assert_has_event(RuntimeEvent::IdnManager(Event::<Test>::FeesCollected {
				sub_id,
				fees,
			}));

			// assert balance has been collected from the hold
			assert_eq!(
				Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE),
				initial_fees - treasury_balance
			);

			// assert free balance is still correct
			assert_eq!(
				Balances::free_balance(&ALICE),
				initial_balance - initial_fees - storage_deposit
			);

			// finalize block
			IdnManager::on_finalize(System::block_number());
		}

		// Verify subscription is removed after last credit
		assert!(!Subscriptions::<Test>::contains_key(sub_id));

		// Verify final balances
		assert_eq!(Balances::free_balance(&ALICE), initial_balance - initial_fees);
		assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE), 0);
		assert_eq!(Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE), 0);
		assert_eq!(Balances::free_balance(&TreasuryAccount::get()), initial_fees);

		// Verify events
		System::assert_last_event(RuntimeEvent::IdnManager(Event::<Test>::SubscriptionRemoved {
			sub_id,
		}));
	});
}

#[test]
fn test_credits_consumption_not_enogh_balance() {
	ExtBuilder::build().execute_with(|| {
		// Setup initial conditions
		let credits: u64 = 1_010_000;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 1;
		let initial_balance = 10_000_000_000;
		let pulse = mock::Pulse { rand: [0u8; 32], round: 0, sig: [1u8; 64] };

		// Set up account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Create subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Get subscription details
		let (_, sub) = Subscriptions::<Test>::iter().next().unwrap();

		// Consume credits one by one
		for i in 0..credits {
			// Advance block and run hooks
			System::set_block_number(System::block_number() + 1);

			if i == 505 {
				// let's fake an incorrect fees collection at some arbitrary point
				let _ = <Test as Config>::FeesManager::collect_fees(
					&Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE),
					&sub,
				);
				assert_eq!(Balances::balance_on_hold(&HoldReason::Fees.into(), &ALICE), 0);
				assert_noop!(
					IdnManager::dispatch(pulse.into()),
					DispatchError::Other("NotEnoughBalance")
				);
				break;
			} else {
				// Dispatch randomness
				assert_ok!(IdnManager::dispatch(pulse.into()));
			}

			// finalize block
			IdnManager::on_finalize(System::block_number());
		}
	});
}

#[test]
fn test_credits_consumption_frequency() {
	ExtBuilder::build().execute_with(|| {
		// Setup initial conditions
		let credits: u64 = 10_180; // 10 active blocks (at 1k credits each) + 18 idle blocks (at 10 credits each)
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 3; // Every 3 blocks
		let initial_balance = 10_000_000;
		let pulse = mock::Pulse { rand: [0u8; 32], round: 0, sig: [1u8; 64] };

		// Set up account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Create subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Get the subscription ID
		let (sub_id, sub) = Subscriptions::<Test>::iter().next().unwrap();

		assert_eq!(sub.credits_left, credits);
		assert!(sub.last_delivered.is_none());

		let deliveries = 27;
		// Run through the test blocks
		for i in 0..=deliveries {
			// Set the block number
			System::set_block_number(System::block_number() + 1);

			// Clear previous events
			System::reset_events();

			let sub = Subscriptions::<Test>::get(sub_id).unwrap();
			let last_delivered = sub.last_delivered;
			let credits_left = sub.credits_left;

			// Dispatch randomness
			assert_ok!(IdnManager::dispatch(pulse.into()));

			// Check the subscription state
			let sub = Subscriptions::<Test>::get(sub_id).unwrap();

			if last_delivered.is_none() || i % frequency == 0 {
				// Verify events
				System::assert_last_event(RuntimeEvent::IdnManager(
					Event::<Test>::RandomnessDistributed { sub_id },
				));
				assert_eq!(
					sub.credits_left,
					credits_left - <Test as Config>::FeesManager::get_consume_credits(&sub)
				);
			} else {
				// Verify events
				assert!(event_not_emitted(Event::<Test>::RandomnessDistributed { sub_id }));
				assert_eq!(
					sub.credits_left,
					credits_left - <Test as Config>::FeesManager::get_idle_credits(&sub)
				);
			}

			if i == deliveries {
				// by the end all credits should be consumed
				assert!(sub.credits_left == 0);
			}
			// Finalize the block
			IdnManager::on_finalize(System::block_number());
		}
		// Verify subscription is removed after last credit
		assert!(Subscriptions::<Test>::get(sub_id).is_none());
	});
}

#[test]
fn test_pause_reactivate_subscription() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: metadata.clone(),
				pulse_filter: None,
				sub_id: None,
			}
		));

		let free_balance = Balances::free_balance(&ALICE);

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// Test pause and reactivate subscription
		assert_ok!(IdnManager::pause_subscription(RuntimeOrigin::signed(ALICE.clone()), sub_id));

		System::assert_last_event(RuntimeEvent::IdnManager(Event::<Test>::SubscriptionPaused {
			sub_id,
		}));

		assert_eq!(Subscriptions::<Test>::get(sub_id).unwrap().state, SubscriptionState::Paused);
		assert_ok!(IdnManager::reactivate_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			sub_id
		));
		assert_eq!(Subscriptions::<Test>::get(sub_id).unwrap().state, SubscriptionState::Active);

		// Assert current free balance is the same as the free balance before pausing and
		// reactivating
		assert_eq!(Balances::free_balance(&ALICE), free_balance);

		System::assert_last_event(RuntimeEvent::IdnManager(
			Event::<Test>::SubscriptionReactivated { sub_id },
		));
	});
}

#[test]
fn pause_subscription_fails_if_sub_does_not_exists() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [0xff; 32];

		assert_noop!(
			IdnManager::pause_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionDoesNotExist
		);

		// Assert the SubscriptionPaused event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionPaused { sub_id }));
	});
}

#[test]
fn pause_subscription_fails_if_sub_already_paused() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: metadata.clone(),
				pulse_filter: None,
				sub_id: None,
			}
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
		assert!(event_not_emitted(Event::<Test>::SubscriptionPaused { sub_id }));
	});
}

#[test]
fn reactivate_subscription_fails_if_sub_does_not_exists() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		assert_noop!(
			IdnManager::reactivate_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionDoesNotExist
		);

		// Assert the SubscriptionReactivated event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionReactivated { sub_id }));
	});
}

#[test]
fn reactivate_subscriptio_fails_if_sub_already_active() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 2;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: metadata.clone(),
				pulse_filter: None,
				sub_id: None,
			}
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		assert_noop!(
			IdnManager::reactivate_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionAlreadyActive
		);

		// Assert the SubscriptionReactivated event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionReactivated { sub_id }));
	});
}

#[test]
fn operations_fail_if_origin_is_not_the_subscriber() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
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
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: metadata.clone(),
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Retrieve the subscription ID created
		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// Attempt to kill the subscription using Bob's origin (should fail)
		assert_noop!(
			IdnManager::kill_subscription(RuntimeOrigin::signed(BOB.clone()), sub_id),
			Error::<Test>::NotSubscriber
		);

		// assert subscription still exists
		assert!(Subscriptions::<Test>::get(sub_id).is_some());

		// Assert the SubscriptionRemoved event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionRemoved { sub_id }));

		// Attempt to pause the subscription using Bob's origin (should fail)
		assert_noop!(
			IdnManager::pause_subscription(RuntimeOrigin::signed(BOB.clone()), sub_id),
			Error::<Test>::NotSubscriber
		);

		// Assert the SubscriptionPaused event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionPaused { sub_id }));

		// Attempt to update the subscription using Bob's origin (should fail)
		let new_credits = credits + 10;
		let new_frequency = frequency + 1;
		assert_noop!(
			IdnManager::update_subscription(
				RuntimeOrigin::signed(BOB.clone()),
				UpdateSubParamsOf::<Test> {
					sub_id,
					credits: new_credits,
					frequency: new_frequency,
					pulse_filter: None
				}
			),
			Error::<Test>::NotSubscriber
		);

		// Attempt to reactivate the subscription using Bob's origin (should fail)
		assert_noop!(
			IdnManager::reactivate_subscription(RuntimeOrigin::signed(BOB.clone()), sub_id),
			Error::<Test>::NotSubscriber
		);

		// Assert the SubscriptionReactivated event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionReactivated { sub_id }));
	});
}

#[test]
fn test_on_finalize_removes_finished_subscriptions() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		// Create subscription
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		let (sub_id, mut subscription) = Subscriptions::<Test>::iter().next().unwrap();

		// Manually set credits to zero to simulate a finished subscription
		subscription.credits_left = Zero::zero();
		Subscriptions::<Test>::insert(sub_id, subscription);

		// Before on_finalize, subscription should exist
		assert!(Subscriptions::<Test>::contains_key(sub_id));

		// Call on_finalize
		crate::Pallet::<Test>::on_finalize(System::block_number());

		// After on_finalize:
		// 1. Subscription should be removed
		assert!(!Subscriptions::<Test>::contains_key(sub_id));

		// 2. SubscriptionRemoved event should be emitted
		System::assert_last_event(RuntimeEvent::IdnManager(Event::<Test>::SubscriptionRemoved {
			sub_id,
		}));
	});
}

#[test]
#[docify::export_content]
fn hold_deposit_works() {
	ExtBuilder::build().execute_with(|| {
		let initial_balance = 10_000_000;
		let deposit_credits = 1_000;

		// Setup account with initial balance
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Hold deposit
		assert_ok!(crate::Pallet::<Test>::hold_deposit(&ALICE, deposit_credits));

		// Verify deposit is held
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			deposit_credits
		);
		// Verify free balance is reduced
		assert_eq!(Balances::free_balance(&ALICE), initial_balance - deposit_credits);
	});
}

#[test]
#[docify::export_content]
fn release_deposit_works() {
	ExtBuilder::build().execute_with(|| {
		let initial_balance = 10_000_000;
		let deposit_credits = 1_000;

		// Setup account and hold deposit
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);
		assert_ok!(crate::Pallet::<Test>::hold_deposit(&ALICE, deposit_credits));

		// Release deposit
		assert_ok!(crate::Pallet::<Test>::release_deposit(&ALICE, deposit_credits));

		// Verify deposit is released
		assert_eq!(Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE), 0);
		// Verify free balance is restored
		assert_eq!(Balances::free_balance(&ALICE), initial_balance);
	});
}

#[test]
fn manage_diff_deposit_works() {
	ExtBuilder::build().execute_with(|| {
		let initial_balance = 10_000_000;
		let original_deposit = 1_000;
		let additional_deposit = 1_500;
		let excess_deposit = 500;

		// Setup account with initial balance
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Test holding deposit
		let hold_diff =
			DiffBalance { balance: original_deposit, direction: BalanceDirection::Collect };
		assert_ok!(crate::Pallet::<Test>::manage_diff_deposit(&ALICE, &hold_diff));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			original_deposit
		);
		// Test holding additional deposit
		let hold_diff =
			DiffBalance { balance: additional_deposit, direction: BalanceDirection::Collect };
		assert_ok!(crate::Pallet::<Test>::manage_diff_deposit(&ALICE, &hold_diff));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			original_deposit + additional_deposit
		);

		// Test releasing excess deposit
		let release_diff =
			DiffBalance { balance: excess_deposit, direction: BalanceDirection::Release };
		assert_ok!(crate::Pallet::<Test>::manage_diff_deposit(&ALICE, &release_diff));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			original_deposit + additional_deposit - excess_deposit
		);

		// Test no change in deposit
		let no_change_diff = DiffBalance { balance: 0, direction: BalanceDirection::None };
		let held_before = Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE);
		assert_ok!(crate::Pallet::<Test>::manage_diff_deposit(&ALICE, &no_change_diff));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			held_before
		);

		// assert free balance
		assert_eq!(
			Balances::free_balance(&ALICE),
			initial_balance - original_deposit - additional_deposit + excess_deposit
		);
	});
}

#[test]
fn hold_deposit_fails_with_insufficient_balance() {
	ExtBuilder::build().execute_with(|| {
		let initial_balance = 500;
		let deposit_credits = 1_000;

		// Setup account with insufficient balance
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Attempt to hold deposit should fail
		assert_noop!(
			crate::Pallet::<Test>::hold_deposit(&ALICE, deposit_credits),
			TokenError::FundsUnavailable
		);

		// Verify no deposit is held
		assert_eq!(Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE), 0);
		// Verify balance remains unchanged
		assert_eq!(Balances::free_balance(&ALICE), initial_balance);
	});
}

#[test]
#[docify::export_content]
fn test_calculate_subscription_fees() {
	ExtBuilder::build().execute_with(|| {
		// Test with different credit amounts
		// The tuples in these cases are (credits, expected_fee)
		let test_cases = vec![
			(0, 0),                  // Zero credits
			(1_000, 100_000),        // 1k credits
			(10_000, 1_000_000),     // 10k credits
			(50_000, 4_800_000),     // 50k credits, 5% off over 10k
			(1_000_000, 90_550_000), // 1M credits, 5% off over 50k, 10% over 10k
			(1_000_001, 90_550_080), // 1M + 1credits, 5% off over 50k, 10% over 10k, 20% over 1M
		];

		for (credits, expected_fee) in test_cases {
			let fee = IdnManager::calculate_subscription_fees(&credits);
			assert_eq!(
				fee, expected_fee,
				"Fee calculation incorrect for {} credits, expected {}, got {}",
				credits, expected_fee, fee
			);
		}
	});
}

#[test]
fn test_get_subscription() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		// Create a subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Retrieve the subscription ID created
		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// Test get_subscription with valid ID
		let subscription = IdnManager::get_subscription(&sub_id);
		assert!(subscription.is_some(), "Subscription should exist");

		let sub = subscription.unwrap();
		assert_eq!(sub.details.subscriber, ALICE);
		assert_eq!(sub.credits, credits);
		assert_eq!(sub.frequency, frequency);
		assert_eq!(sub.details.target, target);

		// Test get_subscription with invalid ID
		let invalid_sub_id = [0xff; 32];
		let invalid_subscription = IdnManager::get_subscription(&invalid_sub_id);
		assert!(invalid_subscription.is_none(), "Invalid subscription ID should return None");
	});
}

#[test]
fn test_get_subscriptions_for_subscriber() {
	ExtBuilder::build().execute_with(|| {
		// Set up accounts
		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);
		<Test as Config>::Currency::set_balance(&BOB, 10_000_000);

		// Create subscriptions for ALICE
		let target1 = Location::new(1, [Junction::PalletInstance(1)]);
		let target2 = Location::new(1, [Junction::PalletInstance(2)]);
		let target3 = Location::new(1, [Junction::PalletInstance(3)]);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits: 50,
				target: target1.clone(),
				call_index: [1; 2],
				frequency: 10,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits: 100,
				target: target2.clone(),
				call_index: [1; 2],
				frequency: 20,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Create a subscription for BOB
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(BOB.clone()),
			CreateSubParamsOf::<Test> {
				credits: 75,
				target: target3.clone(),
				call_index: [1; 2],
				frequency: 15,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Test get_subscriptions_for_subscriber with ALICE
		let alice_subs = IdnManager::get_subscriptions_for_subscriber(&ALICE);
		assert_eq!(alice_subs.len(), 2, "ALICE should have 2 subscriptions");

		// Verify subscription details
		let has_sub1 = alice_subs.iter().any(|sub| {
			sub.details.subscriber == ALICE &&
				sub.credits == 50 &&
				sub.frequency == 10 &&
				sub.details.target == target1
		});

		let has_sub2 = alice_subs.iter().any(|sub| {
			sub.details.subscriber == ALICE &&
				sub.credits == 100 &&
				sub.frequency == 20 &&
				sub.details.target == target2
		});

		assert!(has_sub1, "ALICE's first subscription not found");
		assert!(has_sub2, "ALICE's second subscription not found");

		// Test get_subscriptions_for_subscriber with BOB
		let bob_subs = IdnManager::get_subscriptions_for_subscriber(&BOB);
		assert_eq!(bob_subs.len(), 1, "BOB should have 1 subscription");

		// Verify subscription details
		let has_sub3 = bob_subs.iter().any(|sub| {
			sub.details.subscriber == BOB &&
				sub.credits == 75 &&
				sub.frequency == 15 &&
				sub.details.target == target3
		});

		assert!(has_sub3, "BOB's subscription not found");
	});
}

#[test]
fn test_runtime_api_calculate_subscription_fees() {
	ExtBuilder::build().execute_with(|| {
		// Test with different credit amounts
		let test_cases = vec![
			(0, 0),                  // Zero credits
			(1_000, 100_000),        // 1k credits
			(10_000, 1_000_000),     // 10k credits
			(50_000, 4_800_000),     // 50k credits, 5% off over 10k
			(1_000_000, 90_550_000), // 1M credits, 5% off over 50k, 10% over 10k
			(1_000_001, 90_550_080), // 1M + 1credits, 5% off over 50k, 10% over 10k, 20% over 1M
		];

		for (credits, expected_fee) in test_cases {
			let fee = Test::calculate_subscription_fees(credits.clone());
			assert_eq!(
				fee, expected_fee,
				"Fee calculation incorrect for {} credits, expected {}, got {}",
				credits, expected_fee, fee
			);
		}
	});
}

#[test]
fn test_runtime_api_get_subscription() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		// Create a subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index: [1; 2],
				frequency,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Retrieve the subscription ID created
		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// Test get_subscription with valid ID
		let subscription = Test::get_subscription(sub_id);
		assert!(subscription.is_some(), "Subscription should exist");

		let sub = subscription.unwrap();
		assert_eq!(sub.details.subscriber, ALICE);
		assert_eq!(sub.credits, credits);
		assert_eq!(sub.frequency, frequency);
		assert_eq!(sub.details.target, target);

		// Test get_subscription with invalid ID
		let invalid_sub_id = [0xff; 32];
		let invalid_subscription = Test::get_subscription(invalid_sub_id);
		assert!(invalid_subscription.is_none(), "Invalid subscription ID should return None");
	});
}

#[test]
fn test_runtime_api_get_subscriptions_for_subscriber() {
	ExtBuilder::build().execute_with(|| {
		// Set up accounts
		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);
		<Test as Config>::Currency::set_balance(&BOB, 10_000_000);

		// Create subscriptions for ALICE
		let target1 = Location::new(1, [Junction::PalletInstance(1)]);
		let target2 = Location::new(1, [Junction::PalletInstance(2)]);
		let target3 = Location::new(1, [Junction::PalletInstance(3)]);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits: 50,
				target: target1.clone(),
				call_index: [1; 2],
				frequency: 10,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits: 100,
				target: target2.clone(),
				call_index: [1; 2],
				frequency: 20,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Create a subscription for BOB
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(BOB.clone()),
			CreateSubParamsOf::<Test> {
				credits: 75,
				target: target3.clone(),
				call_index: [1; 2],
				frequency: 15,
				metadata: None,
				pulse_filter: None,
				sub_id: None,
			}
		));

		// Test get_subscriptions_for_subscriber with ALICE
		let alice_subs = Test::get_subscriptions_for_subscriber(ALICE);
		assert_eq!(alice_subs.len(), 2, "ALICE should have 2 subscriptions");

		// Verify subscription details
		let has_sub1 = alice_subs.iter().any(|sub| {
			sub.details.subscriber == ALICE &&
				sub.credits == 50 &&
				sub.frequency == 10 &&
				sub.details.target == target1
		});

		let has_sub2 = alice_subs.iter().any(|sub| {
			sub.details.subscriber == ALICE &&
				sub.credits == 100 &&
				sub.frequency == 20 &&
				sub.details.target == target2
		});

		assert!(has_sub1, "ALICE's first subscription not found");
		assert!(has_sub2, "ALICE's second subscription not found");

		// Test get_subscriptions_for_subscriber with BOB
		let bob_subs = Test::get_subscriptions_for_subscriber(BOB);
		assert_eq!(bob_subs.len(), 1, "BOB should have 1 subscription");

		// Verify subscription details
		let has_sub3 = bob_subs.iter().any(|sub| {
			sub.details.subscriber == BOB &&
				sub.credits == 75 &&
				sub.frequency == 15 &&
				sub.details.target == target3
		});

		assert!(has_sub3, "BOB's subscription not found");
	});
}

#[test]
fn test_pulse_filter_functionality() {
	ExtBuilder::build().execute_with(|| {
		// Setup common parameters
		let initial_balance = 10_000_000;
		let credits: u64 = 100_000;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		let frequency = 1; // Every block

		// Create test account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Rounds subscription should receive pulses from
		let rounds = vec![2u64, 4, 5, 7, 10];
		let rounds_filter = mock_rounds_filter(&rounds);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index,
				frequency,
				metadata: None,
				pulse_filter: Some(rounds_filter),
				sub_id: None,
			}
		));

		// Get subscription ID
		let (sub_id, sub) = Subscriptions::<Test>::iter().next().unwrap();

		// Helper to get current credits left for a subscription
		let credits_left =
			|| Subscriptions::<Test>::get(sub_id).map(|sub| sub.credits_left).unwrap_or(0);

		// Initial verification
		assert_eq!(credits_left(), credits);

		let mut credits_consumed = 0u64;

		let mut count_active_rounds = 0;
		let mut count_idle_rounds = 0;

		// Process 10 blocks with pulses of increasing rounds
		for block in 0..10 {
			// Set block number
			System::set_block_number(block);

			// Zero-based round numbers (0-9)
			let round = block + 1;

			// Create pulse with current round
			let pulse = mock::Pulse { rand: [block as u8; 32], round, sig: [1u8; 64] };

			// Clear previous events
			System::reset_events();

			// Process pulse
			assert_ok!(IdnManager::dispatch(pulse.into()));

			// rounds go 1 ahead of block number
			let round = block + 1;
			let should_distribute = rounds.contains(&round);

			// Verify credits consumption
			if should_distribute {
				credits_consumed += <Test as Config>::FeesManager::get_consume_credits(&sub);
				System::assert_has_event(RuntimeEvent::IdnManager(
					Event::<Test>::RandomnessDistributed { sub_id },
				));
				count_active_rounds += 1;
			} else {
				credits_consumed += <Test as Config>::FeesManager::get_idle_credits(&sub);
				assert!(
					event_not_emitted(Event::<Test>::RandomnessDistributed { sub_id }),
					"Randomness should not be distributed for round {}",
					round
				);
				count_idle_rounds += 1;
			}

			assert_eq!(
				credits_left(),
				credits - credits_consumed,
				"Credits not consumed correctly after round {}",
				round
			);

			// Finalize the block
			IdnManager::on_finalize(block);
		}

		// Verify the subscription credits were consumed correctly
		assert_eq!(
			credits_left(),
			credits -
				(count_active_rounds * <Test as Config>::FeesManager::get_consume_credits(&sub) +
					count_idle_rounds * <Test as Config>::FeesManager::get_idle_credits(&sub))
					as u64
		);
	});
}

#[test]
fn test_pulse_filter_functionality_with_low_frequency() {
	ExtBuilder::build().execute_with(|| {
		// Setup common parameters
		let initial_balance = 10_000_000;
		let credits: u64 = 100_000;
		let target = Location::new(1, [Junction::PalletInstance(1)]);
		let call_index = [1; 2];
		// With this frequency we should miss some pulses even if they are in the desired rounds
		let frequency = 2; // Every 3 block

		// Create test account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Rounds subscription should receive pulses from. We should miss round 5 as it is less than
		// 2 rounds away from the previous one, as specified by the frequency
		let rounds = vec![2u64, 4, 5, 7, 10];
		let rounds_filter = mock_rounds_filter(&rounds);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call_index,
				frequency,
				metadata: None,
				pulse_filter: Some(rounds_filter),
				sub_id: None,
			}
		));

		// Get subscription ID
		let (sub_id, sub) = Subscriptions::<Test>::iter().next().unwrap();

		// Helper to get current credits left for a subscription
		let credits_left =
			|| Subscriptions::<Test>::get(sub_id).map(|sub| sub.credits_left).unwrap_or(0);

		// Initial verification
		assert_eq!(credits_left(), credits);

		let mut credits_consumed = 0u64;
		let mut prev_dist = 0;
		let mut count_active_rounds = 0;
		let mut count_idle_rounds = 0;

		// Process 10 blocks with pulses of increasing rounds
		for block in 0..10 {
			// Set block number
			System::set_block_number(block);

			// Zero-based round numbers (0-9)
			let round = block + 1;

			// Create pulse with current round
			let pulse = mock::Pulse { rand: [block as u8; 32], round, sig: [1u8; 64] };

			// Clear previous events
			System::reset_events();

			// Process pulse
			assert_ok!(IdnManager::dispatch(pulse.into()));

			// rounds go 1 ahead of block number
			let round = block + 1;
			let should_distribute = rounds.contains(&round) && prev_dist + frequency <= round;

			// Verify credits consumption
			if should_distribute {
				credits_consumed += <Test as Config>::FeesManager::get_consume_credits(&sub);
				prev_dist = round;
				System::assert_has_event(RuntimeEvent::IdnManager(
					Event::<Test>::RandomnessDistributed { sub_id },
				));
				count_active_rounds += 1;
			} else {
				credits_consumed += <Test as Config>::FeesManager::get_idle_credits(&sub);
				assert!(
					event_not_emitted(Event::<Test>::RandomnessDistributed { sub_id }),
					"Randomness should not be distributed for round {}",
					round
				);
				count_idle_rounds += 1;
			}

			assert_eq!(
				credits_left(),
				credits - credits_consumed,
				"Credits not consumed correctly after round {}",
				round
			);

			// Finalize the block
			IdnManager::on_finalize(block);
		}

		// Verify the subscription credits were consumed correctly
		assert_eq!(
			credits_left(),
			credits -
				(count_active_rounds * <Test as Config>::FeesManager::get_consume_credits(&sub) +
					count_idle_rounds * <Test as Config>::FeesManager::get_idle_credits(&sub))
					as u64
		);
	});
}
