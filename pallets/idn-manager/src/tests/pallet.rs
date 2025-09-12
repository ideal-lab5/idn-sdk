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
	primitives::{Quote, QuoteRequest, QuoteSubParams},
	runtime_decl_for_idn_manager_api::IdnManagerApiV1,
	tests::mock::{self, Balances, ExtBuilder, Test, *},
	traits::{BalanceDirection, DepositCalculator, DiffBalance, FeesManager},
	Config, CreateSubParamsOf, Error, Event, HoldReason, SubInfoRequestOf, SubscriptionState,
	Subscriptions, UpdateSubParamsOf,
};
use codec::Encode;
use frame_support::{
	assert_noop, assert_ok,
	pallet_prelude::Zero,
	traits::{
		fungible::{InspectHold, Mutate},
		OnFinalize,
	},
	BoundedVec,
};
use sp_idn_traits::pulse::Dispatcher;
use sp_runtime::{AccountId32, DispatchError::BadOrigin, TokenError};
use xcm::prelude::{Junction, Location};

pub const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
pub const BOB: AccountId32 = AccountId32::new([2u8; 32]);

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
	original_metadata: Option<Vec<u8>>,
	original_frequency: u64,
	new_credits: u64,
	new_metadata: Option<Vec<u8>>,
	new_frequency: u64,
) {
	let target =
		Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
	let metadata = original_metadata.map(|m| m.try_into().expect("Metadata too long")).clone();
	let initial_balance = 99_990_000_000_000_000;

	<Test as Config>::Currency::set_balance(&subscriber, initial_balance);

	assert_ok!(IdnManager::create_subscription(
		RuntimeOrigin::signed(subscriber.clone()),
		CreateSubParamsOf::<Test> {
			credits: original_credits,
			target: target.clone(),
			call: [1u8, 2u8].encode().try_into().unwrap(),
			frequency: original_frequency,
			metadata,
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
			credits: Some(new_credits),
			frequency: Some(new_frequency),
			metadata: Some(new_metadata.map(|m| m.try_into().expect("Metadata too long")).clone()),
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

#[test]
fn create_subscription_works() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// assert Subscriptions storage map is empty before creating a subscription
		assert_eq!(Subscriptions::<Test>::iter().count(), 0);

		// assert that the subscription has been created
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
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
		assert_eq!(subscription.metadata, None);

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
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;
		let custom_id = [7u8; 32];

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// assert Subscriptions storage map is empty before creating a subscription
		assert_eq!(Subscriptions::<Test>::iter().count(), 0);

		// assert that the subscription has been created
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
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
fn create_subscription_fails_if_insufficient_balance() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10);

		assert_noop!(
			IdnManager::create_subscription(
				RuntimeOrigin::signed(ALICE),
				CreateSubParamsOf::<Test> {
					credits,
					target,
					call: [1u8, 2u8].encode().try_into().unwrap(),
					frequency,
					metadata: None,
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
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
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
					call: [1u8, 2u8].encode().try_into().unwrap(),
					frequency,
					metadata: None,
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
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 2 * u32::MAX as u64;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		for i in 0..MaxSubscriptions::get() {
			assert_ok!(IdnManager::create_subscription(
				RuntimeOrigin::signed(ALICE.clone()),
				CreateSubParamsOf::<Test> {
					credits: credits + i as u64,
					target: target.clone(),
					call: [i as u8, i as u8].encode().try_into().unwrap(),
					frequency,
					metadata: None,
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
					call: [1u8, 2u8].encode().try_into().unwrap(),
					frequency,
					metadata: None,
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
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
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
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let metadata = None;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata,
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

		System::assert_last_event(RuntimeEvent::IdnManager(
			Event::<Test>::SubscriptionTerminated { sub_id },
		));
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

		// Assert the SubscriptionTerminated event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionTerminated { sub_id }));
	});
}

#[test]
fn on_finalize_removes_finalized_subscriptions() {
	ExtBuilder::build().execute_with(|| {
		// Setup - Create a subscription
		let credits: u64 = 50;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
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

		// Manually set state to finalized
		subscription.state = SubscriptionState::Finalized;
		// Set credits_left to zero to simulate a finalized subscription
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
		System::assert_last_event(RuntimeEvent::IdnManager(
			Event::<Test>::SubscriptionTerminated { sub_id },
		));
	});
}

#[test]
fn test_update_subscription() {
	ExtBuilder::build().execute_with(|| {
		struct SubParams {
			credits: u64,
			frequency: u64,
			metadata: Option<Vec<u8>>,
		}
		struct SubUpdate {
			old: SubParams,
			new: SubParams,
		}

		let updates: Vec<SubUpdate> = vec![
			SubUpdate {
				old: SubParams { credits: 10, frequency: 2, metadata: None },
				new: SubParams {
					credits: 20,
					frequency: 4,
					metadata: Some(vec![0x1, 0x2, 0x3, 0x4]),
				},
			},
			SubUpdate {
				old: SubParams { credits: 100, frequency: 20, metadata: Some(vec![0x1, 0xa]) },
				new: SubParams { credits: 20, frequency: 4, metadata: None },
			},
			SubUpdate {
				old: SubParams { credits: 100, frequency: 20, metadata: None },
				new: SubParams { credits: 100, frequency: 20, metadata: None },
			},
			SubUpdate {
				old: SubParams { credits: 100, frequency: 1, metadata: Some(vec![0x1, 0xa]) },
				new: SubParams {
					credits: 9_999_999_999_999,
					frequency: 1,
					metadata: Some(vec![0x1, 0xa]),
				},
			},
			SubUpdate {
				old: SubParams {
					credits: 9_999_999_999_999,
					frequency: 1,
					metadata: Some(vec![0x1, 0xa]),
				},
				new: SubParams { credits: 100, frequency: 1, metadata: Some(vec![0x1, 0xa, 0x10]) },
			},
		];
		for i in 0..updates.len() {
			let update = &updates[i];
			update_subscription(
				AccountId32::new([i.try_into().unwrap(); 32]),
				update.old.credits,
				update.old.metadata.clone(),
				update.old.frequency,
				update.new.credits,
				update.new.metadata.clone(),
				update.new.frequency,
			);
		}
	});
}

#[test]
fn update_does_not_update_when_params_are_none() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let metadata = Some(BoundedVec::try_from(vec![1, 2, 3]).unwrap());
		let initial_balance = 10_000_000;

		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: metadata.clone(),
				sub_id: None,
			}
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		assert_ok!(IdnManager::update_subscription(
			RuntimeOrigin::signed(ALICE),
			UpdateSubParamsOf::<Test> { sub_id, credits: None, frequency: None, metadata: None }
		));

		let sub = Subscriptions::<Test>::get(sub_id).unwrap();
		assert_eq!(sub.credits, credits);
		assert_eq!(sub.frequency, frequency);
		assert_eq!(sub.metadata, metadata);
	});
}

#[test]
fn update_subscription_fails_if_sub_does_not_exists() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [0xff; 32];
		let new_credits = 20;
		let new_frequency = 4;
		let new_metadata = None;

		assert_noop!(
			IdnManager::update_subscription(
				RuntimeOrigin::signed(ALICE),
				UpdateSubParamsOf::<Test> {
					sub_id,
					credits: Some(new_credits),
					frequency: Some(new_frequency),
					metadata: Some(new_metadata),
				}
			),
			Error::<Test>::SubscriptionDoesNotExist
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
		let frequency: u64 = 1;
		let idle = <Test as Config>::FeesManager::get_idle_credits(None);
		let dispatch = <Test as Config>::FeesManager::get_consume_credits(None);
		let fees = frequency.saturating_mul(idle).saturating_add(dispatch);
		// valid for  10 pulses delivered
		let credits: u64 = 10 * fees;

		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);

		let initial_balance = 10_000_000_000;
		let initial_treasury_balance = IdnManager::min_balance();
		let mut treasury_balance = initial_treasury_balance;
		let pulse = mock::Pulse { rand: [0u8; 32], start: 0, end: 1, sig: [1u8; 48] };

		// Set up account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);
		<Test as Config>::Currency::set_balance(&TreasuryAccount::get(), treasury_balance);

		// Create subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
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
		for i in 0..credits {
			// Advance block and run hooks
			System::set_block_number(System::block_number() + 1);

			// Dispatch randomness
			IdnManager::dispatch(pulse.into());

			System::assert_last_event(RuntimeEvent::IdnManager(
				Event::<Test>::RandomnessDistributed { sub_id },
			));

			// Verify credit consumption
			let sub = Subscriptions::<Test>::get(sub_id).unwrap();

			let consume_credits = <Test as Config>::FeesManager::get_consume_credits(Some(&sub));

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
			.balance();

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
				initial_fees - (treasury_balance - initial_treasury_balance)
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
		assert_eq!(
			Balances::free_balance(&TreasuryAccount::get()) - initial_treasury_balance,
			initial_fees
		);

		// Verify events
		System::assert_last_event(RuntimeEvent::IdnManager(
			Event::<Test>::SubscriptionTerminated { sub_id },
		));
	});
}

#[test]
fn test_credits_consumption_not_enough_balance() {
	ExtBuilder::build().execute_with(|| {
		// Setup initial conditions
		let credits: u64 = 1_010_000;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 1;
		let initial_balance = 10_000_000_000;
		let pulse = mock::Pulse { rand: [0u8; 32], start: 0, end: 1, sig: [1u8; 48] };

		// Set up account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Create subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
				sub_id: None,
			}
		));

		// Get subscription details
		let (sub_id, sub) = Subscriptions::<Test>::iter().next().unwrap();

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
				IdnManager::dispatch(pulse.into());

				let updated_sub = Subscriptions::<Test>::get(sub_id).unwrap();
				assert_eq!(updated_sub.state, SubscriptionState::Paused);
				break;
			} else {
				// Dispatch randomness
				IdnManager::dispatch(pulse.into());
			}

			// finalize block
			IdnManager::on_finalize(System::block_number());
		}
	});
}

#[test]
fn test_credits_consumption_xcm_send_fails() {
	ExtBuilder::build().execute_with(|| {
		// Setup initial conditions
		let credits: u64 = 1_010_000;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 1;
		let initial_balance = 10_000_000_000;
		let pulse = mock::Pulse { rand: [0u8; 32], start: 0, end: 1, sig: [1u8; 48] };

		// Set up account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Create subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
				sub_id: None,
			}
		));

		// Get subscription details
		let (sub_id, mut sub) = Subscriptions::<Test>::iter().next().unwrap();

		// Consume credits one by one
		for i in 0..credits {
			// Advance block and run hooks
			System::set_block_number(System::block_number() + 1);

			if i == 505 {
				// let's fake an incorrect config
				let bad_location: Location =
					Location { parents: 42, interior: xcm::opaque::latest::Junctions::Here };

				sub.details.target = bad_location;
				Subscriptions::<Test>::insert(sub_id, sub);

				IdnManager::dispatch(pulse.into());
				let updated_sub = Subscriptions::<Test>::get(sub_id).unwrap();
				assert_eq!(updated_sub.state, SubscriptionState::Paused);
				break;
			} else {
				// Dispatch randomness
				IdnManager::dispatch(pulse.into());
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
		let frequency: u64 = 3;
		// valid for 10 pulses delivered
		let lifetime_pulses = 10;
		let credits = <Test as Config>::FeesManager::calculate_credits(frequency, lifetime_pulses);

		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let initial_balance = 10_000_000;
		let pulse = mock::Pulse { rand: [0u8; 32], start: 0, end: 1, sig: [1u8; 48] };

		// Set up account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Create subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
				sub_id: None,
			}
		));

		// Get the subscription ID
		let (sub_id, sub) = Subscriptions::<Test>::iter().next().unwrap();

		assert_eq!(sub.credits_left, credits);
		assert!(sub.last_delivered.is_none());

		let deliveries = lifetime_pulses * (frequency + 1) - 1;
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
			IdnManager::dispatch(pulse.into());

			// Check the subscription state
			let sub = Subscriptions::<Test>::get(sub_id).unwrap();

			if last_delivered.is_none() || i % frequency == 0 {
				// Verify events
				System::assert_last_event(RuntimeEvent::IdnManager(
					Event::<Test>::RandomnessDistributed { sub_id },
				));
				assert_eq!(
					sub.credits_left,
					credits_left - <Test as Config>::FeesManager::get_consume_credits(Some(&sub))
				);
			} else {
				// Verify events
				assert!(event_not_emitted(Event::<Test>::RandomnessDistributed { sub_id }));
				assert_eq!(
					sub.credits_left,
					credits_left - <Test as Config>::FeesManager::get_idle_credits(Some(&sub))
				);
			}

			if i == deliveries {
				// by the end all credits should be consumed
				assert!(sub.credits_left == 0);
			}
			// Finalize the block
			IdnManager::on_finalize(System::block_number());
		}
		// Verify subscription is removed after being finalized
		assert!(Subscriptions::<Test>::get(sub_id).is_none());
	});
}

#[test]
fn test_sub_state_is_finalized_when_credits_left_goes_low() {
	ExtBuilder::build().execute_with(|| {
		// Setup initial conditions
		let credits: u64 = 10_000;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 3; // ignored
		let initial_balance = 10_000_000;
		let pulse = mock::Pulse { rand: [0u8; 32], start: 0, end: 1, sig: [1u8; 48] };

		// Set up account
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);

		// Create subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
				sub_id: None,
			}
		));

		// Get the subscription ID
		let (sub_id, mut sub) = Subscriptions::<Test>::iter().next().unwrap();

		assert_eq!(sub.credits_left, credits);

		// Set the subscription to a state where it should be finalized, by forcing the credits_left
		// go low
		sub.credits_left = IdnManager::get_min_credits(&sub);

		Subscriptions::<Test>::insert(sub_id, sub);

		// Dispatch randomness
		IdnManager::dispatch(pulse.into());

		let sub = Subscriptions::<Test>::get(sub_id).unwrap();

		// Assert the sub state transitioned to Finalized
		assert_eq!(sub.state, SubscriptionState::Finalized);

		// Finalize the block
		IdnManager::on_finalize(System::block_number());

		// Verify subscription is removed after being finalized
		assert!(Subscriptions::<Test>::get(sub_id).is_none());
	});
}

#[test]
fn test_pause_reactivate_subscription() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 2;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata,
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
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata,
				sub_id: None,
			}
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		assert_ok!(IdnManager::pause_subscription(RuntimeOrigin::signed(ALICE.clone()), sub_id));

		// erase all events
		System::reset_events();

		assert_noop!(
			IdnManager::pause_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionInvalidTransition
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
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let metadata = None;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata,
				sub_id: None,
			}
		));

		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		assert_noop!(
			IdnManager::reactivate_subscription(RuntimeOrigin::signed(ALICE), sub_id),
			Error::<Test>::SubscriptionInvalidTransition
		);

		// Assert the SubscriptionReactivated event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionReactivated { sub_id }));
	});
}

#[test]
fn operations_fail_if_origin_is_not_the_subscriber() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
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
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata,
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

		// Assert the SubscriptionTerminated event was not emitted
		assert!(event_not_emitted(Event::<Test>::SubscriptionTerminated { sub_id }));

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
					credits: Some(new_credits),
					frequency: Some(new_frequency),
					metadata: None
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
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let initial_balance = 10_000_000;

		// Create subscription
		<Test as Config>::Currency::set_balance(&ALICE, initial_balance);
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
				sub_id: None,
			}
		));

		let (sub_id, mut subscription) = Subscriptions::<Test>::iter().next().unwrap();

		// Manually simulate a finished subscription
		subscription.state = SubscriptionState::Finalized;
		subscription.credits_left = Zero::zero();
		Subscriptions::<Test>::insert(sub_id, subscription);

		// Before on_finalize, subscription should exist
		assert!(Subscriptions::<Test>::contains_key(sub_id));

		// Call on_finalize
		crate::Pallet::<Test>::on_finalize(System::block_number());

		// After on_finalize:
		// 1. Subscription should be removed
		assert!(!Subscriptions::<Test>::contains_key(sub_id));

		// 2. SubscriptionTerminated event should be emitted
		System::assert_last_event(RuntimeEvent::IdnManager(
			Event::<Test>::SubscriptionTerminated { sub_id },
		));
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
			<Test as Config>::DiffBalance::new(original_deposit, BalanceDirection::Collect);
		assert_ok!(crate::Pallet::<Test>::manage_diff_deposit(&ALICE, &hold_diff));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			original_deposit
		);
		// Test holding additional deposit
		let hold_diff =
			<Test as Config>::DiffBalance::new(additional_deposit, BalanceDirection::Collect);
		assert_ok!(crate::Pallet::<Test>::manage_diff_deposit(&ALICE, &hold_diff));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			original_deposit + additional_deposit
		);

		// Test releasing excess deposit
		let release_diff =
			<Test as Config>::DiffBalance::new(excess_deposit, BalanceDirection::Release);
		assert_ok!(crate::Pallet::<Test>::manage_diff_deposit(&ALICE, &release_diff));
		assert_eq!(
			Balances::balance_on_hold(&HoldReason::StorageDeposit.into(), &ALICE),
			original_deposit + additional_deposit - excess_deposit
		);

		// Test no change in deposit
		let no_change_diff = <Test as Config>::DiffBalance::new(0, BalanceDirection::None);
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
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		// Create a subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
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
		let target1 =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let target2 =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(2)]);
		let target3 =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(3)]);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits: 50,
				target: target1.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency: 10,
				metadata: None,
				sub_id: None,
			}
		));

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits: 100,
				target: target2.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency: 20,
				metadata: None,
				sub_id: None,
			}
		));

		// Create a subscription for BOB
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(BOB.clone()),
			CreateSubParamsOf::<Test> {
				credits: 75,
				target: target3.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency: 15,
				metadata: None,
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
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;

		<Test as Config>::Currency::set_balance(&ALICE, 10_000_000);

		// Create a subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
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
		let target1 =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let target2 =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(2)]);
		let target3 =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(3)]);

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits: 50,
				target: target1.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency: 10,
				metadata: None,
				sub_id: None,
			}
		));

		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(ALICE.clone()),
			CreateSubParamsOf::<Test> {
				credits: 100,
				target: target2.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency: 20,
				metadata: None,
				sub_id: None,
			}
		));

		// Create a subscription for BOB
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(BOB.clone()),
			CreateSubParamsOf::<Test> {
				credits: 75,
				target: target3.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency: 15,
				metadata: None,
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
fn test_quote_subscription_works() {
	ExtBuilder::build().execute_with(|| {
		// f*C_idle + C_dispatch
		let lifetime_pulses = 1000;
		let pulse_callback_index = [1, 2];
		let quote_callback_index = [1, 3];
		let frequency = 10u64;
		let idle = <Test as Config>::FeesManager::get_idle_credits(None);
		let dispatch = <Test as Config>::FeesManager::get_consume_credits(None);
		let credits = frequency
			.saturating_mul(idle)
			.saturating_add(dispatch)
			.saturating_mul(lifetime_pulses);
		let fees = <Test as Config>::FeesManager::calculate_subscription_fees(&credits);

		let origin = RuntimeOrigin::signed(mock::SIBLING_PARA_ACCOUNT);

		let create_sub_params = CreateSubParamsOf::<Test> {
			credits,
			target: Location::new(1, [Junction::PalletInstance(1)]),
			call: pulse_callback_index.encode().try_into().unwrap(),
			frequency,
			metadata: None,
			sub_id: None,
		};

		let req_ref = [1; 32];

		let quote_request = QuoteRequest { req_ref, create_sub_params, lifetime_pulses };
		let quote_sub_params = QuoteSubParams {
			quote_request,
			call: quote_callback_index.encode().try_into().unwrap(),
		};
		// Call the function
		assert_ok!(IdnManager::quote_subscription(origin, quote_sub_params));
		// Verify the XCM message was sent
		System::assert_last_event(RuntimeEvent::IdnManager(Event::<Test>::SubQuoted {
			requester: Location::new(1, [Junction::Parachain(mock::SIBLING_PARA_ID)]),
			quote: Quote { req_ref, fees, deposit: 1200 },
		}));
	});
}

#[test]
fn test_quote_subscription_fails_for_invalid_origin() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let pulse_callback_index = [1, 2];
		let quote_callback_index = [1, 3];

		// Use an invalid origin (not a sibling)
		let invalid_origin = RuntimeOrigin::signed(ALICE);

		let create_sub_params = CreateSubParamsOf::<Test> {
			credits,
			target: Location::new(1, [Junction::PalletInstance(1)]),
			call: pulse_callback_index.encode().try_into().unwrap(),
			frequency: 10,
			metadata: None,
			sub_id: None,
		};

		let req_ref = [1; 32];
		let lifetime_pulses = 10;

		let quote_request = QuoteRequest { req_ref, create_sub_params, lifetime_pulses };
		let quote_sub_params = QuoteSubParams {
			quote_request,
			call: quote_callback_index.encode().try_into().unwrap(),
		};

		// Call the function and expect it to fail
		assert_noop!(IdnManager::quote_subscription(invalid_origin, quote_sub_params), BadOrigin);
	});
}

#[test]
fn test_get_subscription_xcm_works() {
	ExtBuilder::build().execute_with(|| {
		let credits: u64 = 50;
		let target =
			Location::new(1, [Junction::Parachain(SIBLING_PARA_ID), Junction::PalletInstance(1)]);
		let frequency: u64 = 10;
		let req_ref = [1; 32];
		let call_index = [1, 1];

		// Set balance for the sibling parachain account
		<Test as Config>::Currency::set_balance(&SIBLING_PARA_ACCOUNT, 10_000_000);

		// Create a subscription
		assert_ok!(IdnManager::create_subscription(
			RuntimeOrigin::signed(SIBLING_PARA_ACCOUNT.clone()),
			CreateSubParamsOf::<Test> {
				credits,
				target: target.clone(),
				call: [1u8, 2u8].encode().try_into().unwrap(),
				frequency,
				metadata: None,
				sub_id: None,
			}
		));

		// Retrieve the subscription ID created
		let (sub_id, _) = Subscriptions::<Test>::iter().next().unwrap();

		// Prepare the request
		let req = SubInfoRequestOf::<Test> {
			sub_id,
			req_ref,
			call: call_index.encode().try_into().unwrap(),
		};

		// Call the function
		assert_ok!(IdnManager::get_subscription_info(
			RuntimeOrigin::signed(SIBLING_PARA_ACCOUNT.clone()),
			req
		));

		// Verify the XCM message was sent
		System::assert_last_event(RuntimeEvent::IdnManager(
			Event::<Test>::SubscriptionDistributed { sub_id },
		));
	});
}

#[test]
fn test_get_subscription_xcm_fails_invalid_origin() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];
		let req_ref = [1; 32];
		let call_index = [1, 1];

		// Prepare the request
		let req = SubInfoRequestOf::<Test> {
			sub_id,
			req_ref,
			call: call_index.encode().try_into().unwrap(),
		};

		// Call the function with an invalid origin
		assert_noop!(
			IdnManager::get_subscription_info(RuntimeOrigin::signed(ALICE), req),
			BadOrigin
		);
	});
}

#[test]
fn test_get_subscription_xcm_fails_subscription_not_found() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];
		let req_ref = [1; 32];
		let call_index = [1, 1];

		// Prepare the request
		let req = SubInfoRequestOf::<Test> {
			sub_id,
			req_ref,
			call: call_index.encode().try_into().unwrap(),
		};

		// Call the function with a non-existent subscription ID
		assert_noop!(
			IdnManager::get_subscription_info(RuntimeOrigin::signed(SIBLING_PARA_ACCOUNT), req),
			Error::<Test>::SubscriptionDoesNotExist
		);
	});
}
