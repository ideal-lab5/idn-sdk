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

mod mock;

use crate::{Pulse, Quote, SubInfoResponse};
use frame_support::assert_ok;
use mock::*;
use sp_runtime::traits::BadOrigin;

#[test]
fn test_create_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = None;

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::create_subscription(
			credits, frequency, metadata, sub_id
		));
	});
}

#[test]
fn test_create_subscription_with_id() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = Some([1; 32]);

		// Call the function and assert success
		assert_eq!(
			crate::Pallet::<Test>::create_subscription(credits, frequency, metadata, sub_id)
				.unwrap(),
			sub_id.unwrap()
		);
	});
}

#[test]
fn test_create_subscription_correct_sub_id_multiple_blocks() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = None;

		// Call the function and assert success
		assert_eq!(
			crate::Pallet::<Test>::create_subscription(
				credits,
				frequency,
				metadata.clone(),
				sub_id
			)
			.unwrap(),
			[
				19, 186, 14, 132, 18, 39, 114, 2, 45, 12, 118, 139, 97, 117, 54, 114, 96, 215, 225,
				93, 220, 62, 163, 234, 37, 219, 27, 23, 214, 70, 136, 236
			]
		);

		// advance one block
		System::set_block_number(2);

		// Call the function again in a different block should generate a different sub_id
		assert_eq!(
			crate::Pallet::<Test>::create_subscription(credits, frequency, metadata, sub_id)
				.unwrap(),
			[
				148, 22, 236, 76, 207, 28, 67, 223, 90, 164, 72, 237, 33, 157, 205, 43, 26, 253,
				113, 150, 155, 205, 86, 103, 74, 213, 248, 187, 176, 114, 246, 39
			]
		);
	});
}

#[test]
fn test_create_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = None;

		// mock xcm fails at block 1_234_567
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result =
			crate::Pallet::<Test>::create_subscription(credits, frequency, metadata, sub_id);

		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError.into());
	});
}

#[test]
fn test_pause_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::pause_subscription(sub_id));
	});
}

#[test]
fn test_kill_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::kill_subscription(sub_id));
	});
}

#[test]
fn test_update_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];
		let credits = Some(20);
		let frequency = Some(10);
		let metadata = Some(None);

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::update_subscription(
			sub_id, credits, frequency, metadata,
		));
	});
}

#[test]
fn test_reactivate_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::reactivate_subscription(sub_id));
	});
}

#[test]
fn test_pause_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Simulate failure at block 1_234_567
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::pause_subscription(sub_id);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError.into());
	});
}

#[test]
fn test_kill_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Simulate failure at block 1_234_567
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::kill_subscription(sub_id);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError.into());
	});
}

#[test]
fn test_update_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];
		let credits = Some(20);
		let frequency = Some(10);
		let metadata = Some(None);

		// Simulate failure at block 1_234_567
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result =
			crate::Pallet::<Test>::update_subscription(sub_id, credits, frequency, metadata);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError.into());
	});
}

#[test]
fn test_reactivate_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Simulate failure at block 1_234_567
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::reactivate_subscription(sub_id);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError.into());
	});
}

#[test]
fn test_quote_subscription() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = None;
		let req_ref = None;

		// Call the function
		let result = crate::Pallet::<Test>::quote_subscription(
			credits, frequency, metadata, sub_id, req_ref,
		);

		// Assert the result is Ok and contains the expected request reference
		assert_ok!(result);
	});
}

#[test]
fn test_quote_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = None;
		let req_ref = None;

		// Simulate failure by setting a block number that triggers an error
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::quote_subscription(
			credits, frequency, metadata, sub_id, req_ref,
		);

		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError);
	});
}

#[test]
fn test_consume_quote_success() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let quote = Quote { req_ref: [1; 32], deposit: 100, fees: 100 };

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::consume_quote(
			RuntimeOrigin::signed(mock::IDN_PARA_ACCOUNT),
			quote.clone()
		));

		// Verify the event was emitted
		System::assert_last_event(crate::Event::QuoteConsumed { quote }.into());
	});
}

#[test]
fn test_consume_quote_fails_wrong_origin() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let quote = Quote { req_ref: [1; 32], deposit: 100, fees: 100 };

		// Call the function and assert failure
		let result =
			crate::Pallet::<Test>::consume_quote(RuntimeOrigin::signed(mock::ALICE), quote.clone());
		assert_eq!(result.unwrap_err(), BadOrigin.into());
	});
}

#[test]
fn test_consume_quote_bubbles_up_consumer_trait_failure() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let quote = Quote {
			// This is a mock value that will trigger the failure in the consumer
			req_ref: [123; 32],
			deposit: 100,
			fees: 100,
		};

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::consume_quote(
			RuntimeOrigin::signed(mock::IDN_PARA_ACCOUNT),
			quote.clone(),
		);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::ConsumeQuoteError.into());
	});
}

#[test]
fn test_consume_pulse_success() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let pulse = Pulse::new([0u8; 48], [1u8; 48]);
		let sub_id = [1; 32];

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::consume_pulse(
			RuntimeOrigin::signed(mock::IDN_PARA_ACCOUNT),
			pulse.clone(),
			sub_id
		));

		// Verify the event was emitted
		System::assert_last_event(crate::Event::RandomnessConsumed { sub_id }.into());
	});
}

#[test]
fn test_consume_pulse_fails_wrong_origin() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let pulse = Pulse::new([0u8; 48], [1u8; 48]);
		let sub_id = [1; 32];

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::consume_pulse(
			RuntimeOrigin::signed(mock::ALICE),
			pulse.clone(),
			sub_id,
		);
		assert_eq!(result.unwrap_err(), BadOrigin.into());
	});
}

#[test]
fn test_consume_pulse_bubbles_up_consumer_trait_failure() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let pulse = Pulse::new([0u8; 48], [1u8; 48]);
		let sub_id = [123; 32]; // This sub_id triggers a failure in the consumer

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::consume_pulse(
			RuntimeOrigin::signed(mock::IDN_PARA_ACCOUNT),
			pulse.clone(),
			sub_id,
		);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::ConsumePulseError.into());
	});
}

#[test]
fn test_consume_sub_info_success() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let sub_info = SubInfoResponse { sub: MOCK_SUB, req_ref: [1; 32] };

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::consume_sub_info(
			RuntimeOrigin::signed(mock::IDN_PARA_ACCOUNT),
			sub_info.clone()
		));

		// Verify the event was emitted
		System::assert_last_event(crate::Event::SubInfoConsumed { sub_id: sub_info.sub.id }.into());
	});
}

#[test]
fn test_consume_sub_info_fails_wrong_origin() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let sub_info = SubInfoResponse { sub: MOCK_SUB, req_ref: [1; 32] };

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::consume_sub_info(
			RuntimeOrigin::signed(mock::ALICE),
			sub_info.clone(),
		);
		assert_eq!(result.unwrap_err(), BadOrigin.into());
	});
}

#[test]
fn test_consume_sub_info_bubbles_up_consumer_trait_failure() {
	ExtBuilder::build().execute_with(|| {
		let mut sub = MOCK_SUB.clone();
		sub.id = [123; 32]; // This sub_id triggers a failure in the consumer

		// Mock inputs
		let sub_info = SubInfoResponse { sub, req_ref: [1; 32] };

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::consume_sub_info(
			RuntimeOrigin::signed(mock::IDN_PARA_ACCOUNT),
			sub_info.clone(),
		);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::ConsumeSubInfoError.into());
	});
}

#[test]
fn test_get_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let sub_id = [1; 32];
		let req_ref = None;

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::get_subscription(sub_id, req_ref));
	});
}

#[test]
fn test_get_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		// Mock inputs
		let sub_id = [1; 32];
		let req_ref = None;

		// Simulate failure by setting a block number that triggers an error
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::get_subscription(sub_id, req_ref);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError.into());
	});
}
