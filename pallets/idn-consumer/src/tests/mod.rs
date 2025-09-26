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

pub(crate) mod mock;

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
		assert_ok!(crate::Pallet::<Test>::do_create_subscription(
			RuntimeOrigin::signed(ALICE),
			credits,
			frequency,
			metadata,
			sub_id
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
			crate::Pallet::<Test>::do_create_subscription(
				RuntimeOrigin::signed(ALICE),
				credits,
				frequency,
				metadata,
				sub_id
			)
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
			crate::Pallet::<Test>::do_create_subscription(
				RuntimeOrigin::signed(ALICE),
				credits,
				frequency,
				metadata.clone(),
				sub_id
			)
			.unwrap(),
			[
				178, 117, 254, 41, 171, 62, 62, 226, 86, 105, 136, 96, 187, 23, 29, 116, 166, 208,
				159, 190, 137, 157, 228, 249, 225, 131, 199, 160, 151, 144, 43, 133
			]
		);

		// advance one block
		System::set_block_number(2);

		// Call the function again in a different block should generate a different sub_id
		assert_eq!(
			crate::Pallet::<Test>::do_create_subscription(
				RuntimeOrigin::signed(ALICE),
				credits,
				frequency,
				metadata,
				sub_id
			)
			.unwrap(),
			[
				85, 122, 213, 233, 200, 12, 191, 4, 119, 177, 182, 206, 37, 65, 56, 19, 77, 106,
				197, 167, 90, 162, 63, 114, 101, 210, 89, 185, 181, 35, 219, 206
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
		let result = crate::Pallet::<Test>::do_create_subscription(
			RuntimeOrigin::signed(ALICE),
			credits,
			frequency,
			metadata,
			sub_id,
		);

		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError);
	});
}

#[test]
fn test_pause_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::do_pause_subscription(
			RuntimeOrigin::signed(ALICE),
			sub_id
		));
	});
}

#[test]
fn test_kill_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::do_kill_subscription(
			RuntimeOrigin::signed(ALICE),
			sub_id
		));
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
		assert_ok!(crate::Pallet::<Test>::do_update_subscription(
			RuntimeOrigin::signed(ALICE),
			sub_id,
			credits,
			frequency,
			metadata,
		));
	});
}

#[test]
fn test_reactivate_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::do_reactivate_subscription(
			RuntimeOrigin::signed(ALICE),
			sub_id
		));
	});
}

#[test]
fn test_pause_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Simulate failure at block 1_234_567
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result =
			crate::Pallet::<Test>::do_pause_subscription(RuntimeOrigin::signed(ALICE), sub_id);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError);
	});
}

#[test]
fn test_kill_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Simulate failure at block 1_234_567
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result =
			crate::Pallet::<Test>::do_kill_subscription(RuntimeOrigin::signed(ALICE), sub_id);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError);
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
		let result = crate::Pallet::<Test>::do_update_subscription(
			RuntimeOrigin::signed(ALICE),
			sub_id,
			credits,
			frequency,
			metadata,
		);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError);
	});
}

#[test]
fn test_reactivate_subscription_fails() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];

		// Simulate failure at block 1_234_567
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result =
			crate::Pallet::<Test>::do_reactivate_subscription(RuntimeOrigin::signed(ALICE), sub_id);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError);
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
		let result = crate::Pallet::<Test>::do_request_quote(
			RuntimeOrigin::signed(ALICE),
			credits,
			frequency,
			metadata,
			sub_id,
			req_ref,
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
		let result = crate::Pallet::<Test>::do_request_quote(
			RuntimeOrigin::signed(ALICE),
			credits,
			frequency,
			metadata,
			sub_id,
			req_ref,
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
		let pulse = Pulse::new([0u8; 48], 1, 2);
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
		let pulse = Pulse::new([0u8; 48], 1, 2);
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
		let pulse = Pulse::new([0u8; 48], 1, 2);
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
		let sub_info = SubInfoResponse { sub: mock_sub(), req_ref: [1; 32] };

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
		let sub_info = SubInfoResponse { sub: mock_sub(), req_ref: [1; 32] };

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
		let mut sub = mock_sub().clone();
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
		assert_ok!(crate::Pallet::<Test>::do_request_sub_info(
			RuntimeOrigin::signed(ALICE),
			sub_id,
			req_ref
		));
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
		let result = crate::Pallet::<Test>::do_request_sub_info(
			RuntimeOrigin::signed(ALICE),
			sub_id,
			req_ref,
		);
		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError);
	});
}

#[test]
fn test_sudo_create_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = None;
		// Call the sudo dispatchable and assert success
		assert_ok!(crate::Pallet::<Test>::create_subscription(
			RuntimeOrigin::signed(ALICE),
			credits,
			frequency,
			metadata,
			sub_id
		));
	});
}

#[test]
fn test_sudo_pause_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];
		assert_ok!(crate::Pallet::<Test>::pause_subscription(RuntimeOrigin::signed(ALICE), sub_id));
	});
}

#[test]
fn test_sudo_kill_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];
		assert_ok!(crate::Pallet::<Test>::kill_subscription(RuntimeOrigin::signed(ALICE), sub_id));
	});
}

#[test]
fn test_sudo_update_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];
		let credits = Some(20);
		let frequency = Some(10);
		let metadata = Some(None);
		assert_ok!(crate::Pallet::<Test>::update_subscription(
			RuntimeOrigin::signed(ALICE),
			sub_id,
			credits,
			frequency,
			metadata
		));
	});
}

#[test]
fn test_sudo_reactivate_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];
		assert_ok!(crate::Pallet::<Test>::reactivate_subscription(
			RuntimeOrigin::signed(ALICE),
			sub_id
		));
	});
}

#[test]
fn test_sudo_request_quote_success() {
	ExtBuilder::build().execute_with(|| {
		let number_of_pulses = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = None;
		let req_ref = None;
		assert_ok!(crate::Pallet::<Test>::request_quote(
			RuntimeOrigin::signed(ALICE),
			number_of_pulses,
			frequency,
			metadata,
			sub_id,
			req_ref
		));
	});
}

#[test]
fn test_sudo_request_sub_info_success() {
	ExtBuilder::build().execute_with(|| {
		let sub_id = [1; 32];
		let req_ref = None;
		assert_ok!(crate::Pallet::<Test>::request_sub_info(
			RuntimeOrigin::signed(ALICE),
			sub_id,
			req_ref
		));
	});
}
