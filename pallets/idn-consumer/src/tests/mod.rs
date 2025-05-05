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

use frame_support::assert_ok;
use mock::*;

#[test]
fn test_create_subscription_success() {
	ExtBuilder::build().execute_with(|| {
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let pulse_filter = None;
		let sub_id = None;

		// Call the function and assert success
		assert_ok!(crate::Pallet::<Test>::create_subscription(
			credits,
			frequency,
			metadata,
			pulse_filter,
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
		let pulse_filter = None;
		let sub_id = Some([1; 32]);

		// Call the function and assert success
		assert_eq!(
			crate::Pallet::<Test>::create_subscription(
				credits,
				frequency,
				metadata,
				pulse_filter,
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
		let pulse_filter = None;
		let sub_id = None;

		// Call the function and assert success
		assert_eq!(
			crate::Pallet::<Test>::create_subscription(
				credits,
				frequency,
				metadata.clone(),
				pulse_filter.clone(),
				sub_id
			)
			.unwrap(),
			[
				68, 222, 26, 173, 209, 142, 232, 219, 56, 25, 194, 88, 209, 228, 188, 151, 233, 2,
				1, 31, 139, 135, 249, 157, 74, 243, 37, 231, 240, 34, 254, 52
			]
		);

		// advance one block
		System::set_block_number(2);

		// Call the function again in a different block should generate a different sub_id
		assert_eq!(
			crate::Pallet::<Test>::create_subscription(
				credits,
				frequency,
				metadata,
				pulse_filter,
				sub_id
			)
			.unwrap(),
			[
				187, 74, 239, 46, 85, 96, 244, 199, 111, 220, 81, 68, 223, 29, 42, 114, 48, 61,
				125, 38, 229, 8, 234, 29, 28, 196, 179, 132, 182, 73, 13, 246
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
		let pulse_filter = None;
		let sub_id = None;

		// mock xcm fails at block 1_234_567
		System::set_block_number(1_234_567);

		// Call the function and assert failure
		let result = crate::Pallet::<Test>::create_subscription(
			credits,
			frequency,
			metadata,
			pulse_filter,
			sub_id,
		);

		assert_eq!(result.unwrap_err(), crate::pallet::Error::<Test>::XcmSendError.into());
	});
}
