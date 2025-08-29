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

//! Benchmarking setup for pallet-idn-consumer

use super::*;
use crate::pallet::Pallet as IdnConsumer;
use bp_idn::types::{Subscription, SubscriptionDetails, SubscriptionState};
use frame_benchmarking::v2::*;
use frame_support::sp_runtime::AccountId32;
use frame_system::{Pallet as System, RawOrigin};

pub const MOCK_QUOTE: Quote = Quote { req_ref: [0u8; 32], deposit: 1_000_000_000, fees: 1_000_000 };
pub const MOCK_SUB: Subscription = Subscription {
	id: [1u8; 32],
	state: SubscriptionState::Active,
	credits_left: 0,
	details: SubscriptionDetails {
		subscriber: AccountId32::new([0u8; 32]),
		target: Location::here(),
		call_index: [0, 0],
	},
	created_at: 0,
	updated_at: 0,
	credits: 0,
	frequency: 0,
	metadata: None,
	last_delivered: None,
};
pub const MOCK_SUB_INFO: SubInfoResponse = SubInfoResponse { req_ref: [0u8; 32], sub: MOCK_SUB };

#[benchmarks(
	where
		<T as frame_system::Config>::RuntimeEvent: From<Event<T>>,
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn consume_pulse() {
		let sibling_account: T::AccountId = [88u8; 32].into();
		let origin = RawOrigin::Signed(sibling_account.clone());
		let sub_id = [1u8; 32];

		let pulse: Pulse = Pulse::new([0u8; 48], 0, 1);
		#[extrinsic_call]
		_(origin, pulse, sub_id);

		System::<T>::assert_last_event(Event::<T>::RandomnessConsumed { sub_id }.into());
	}

	#[benchmark]
	fn consume_quote() {
		let sibling_account: T::AccountId = [88u8; 32].into();
		let origin = RawOrigin::Signed(sibling_account.clone());

		#[extrinsic_call]
		_(origin, MOCK_QUOTE);

		System::<T>::assert_last_event(Event::<T>::QuoteConsumed { quote: MOCK_QUOTE }.into());
	}

	#[benchmark]
	fn consume_sub_info() {
		let sibling_account: T::AccountId = [88u8; 32].into();
		let origin = RawOrigin::Signed(sibling_account.clone());

		#[extrinsic_call]
		_(origin, MOCK_SUB_INFO);

		System::<T>::assert_last_event(
			Event::<T>::SubInfoConsumed { sub_id: MOCK_SUB_INFO.sub.id }.into(),
		);
	}

	#[benchmark]
	fn create_subscription() {
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = None;
		let origin = RawOrigin::Signed([88u8; 32].into());

		#[block]
		{
			IdnConsumer::<T>::create_subscription(
				origin.into(),
				credits,
				frequency,
				metadata,
				sub_id,
			)
			.expect("create_subscription should not fail");
		}
	}

	#[benchmark]
	fn pause_subscription() {
		let sub_id = [1u8; 32];
		let origin = RawOrigin::Signed([88u8; 32].into());

		#[block]
		{
			IdnConsumer::<T>::pause_subscription(origin.into(), sub_id)
				.expect("pause_subscription should not fail");
		}
	}

	#[benchmark]
	fn kill_subscription() {
		let sub_id = [1u8; 32];
		let origin = RawOrigin::Signed([88u8; 32].into());

		#[block]
		{
			IdnConsumer::<T>::kill_subscription(origin.into(), sub_id)
				.expect("kill_subscription should not fail");
		}
	}

	#[benchmark]
	fn update_subscription() {
		let sub_id = [1u8; 32];
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let origin = RawOrigin::Signed([88u8; 32].into());

		#[block]
		{
			IdnConsumer::<T>::update_subscription(
				origin.into(),
				sub_id,
				Some(credits),
				Some(frequency),
				metadata,
			)
			.expect("update_subscription should not fail");
		}
	}

	#[benchmark]
	fn reactivate_subscription() {
		let sub_id = [1u8; 32];
		let origin = RawOrigin::Signed([88u8; 32].into());

		#[block]
		{
			IdnConsumer::<T>::reactivate_subscription(origin.into(), sub_id)
				.expect("reactivate_subscription should not fail");
		}
	}

	#[benchmark]
	fn request_quote() {
		let sub_id = None;
		let req_ref = None;
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let origin = RawOrigin::Signed([88u8; 32].into());

		#[block]
		{
			IdnConsumer::<T>::request_quote(
				origin.into(),
				credits,
				frequency,
				metadata,
				sub_id,
				req_ref,
			)
			.expect("request_quote should not fail");
		}
	}

	#[benchmark]
	fn request_sub_info() {
		let sub_id = [1u8; 32];
		let origin = RawOrigin::Signed([88u8; 32].into());

		#[block]
		{
			IdnConsumer::<T>::request_sub_info(origin.into(), sub_id, None)
				.expect("request_sub_info should not fail");
		}
	}

	#[benchmark]
	fn sudo_create_subscription() {
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let sub_id = None;
		let origin = RawOrigin::Root;

		#[block]
		{
			IdnConsumer::<T>::sudo_create_subscription(
				origin.into(),
				credits,
				frequency,
				metadata,
				sub_id,
			)
			.expect("sudo_create_subscription should not fail");
		}
	}

	#[benchmark]
	fn sudo_pause_subscription() {
		let sub_id = [1u8; 32];
		let origin = RawOrigin::Root;

		#[block]
		{
			IdnConsumer::<T>::sudo_pause_subscription(origin.into(), sub_id)
				.expect("sudo_pause_subscription should not fail");
		}
	}

	#[benchmark]
	fn sudo_kill_subscription() {
		let sub_id = [1u8; 32];
		let origin = RawOrigin::Root;

		#[block]
		{
			IdnConsumer::<T>::sudo_kill_subscription(origin.into(), sub_id)
				.expect("sudo_kill_subscription should not fail");
		}
	}

	#[benchmark]
	fn sudo_update_subscription() {
		let sub_id = [1u8; 32];
		let credits = 10;
		let frequency = 5;
		let metadata = None;
		let origin = RawOrigin::Root;

		#[block]
		{
			IdnConsumer::<T>::sudo_update_subscription(
				origin.into(),
				sub_id,
				Some(credits),
				Some(frequency),
				metadata,
			)
			.expect("sudo_update_subscription should not fail");
		}
	}

	#[benchmark]
	fn sudo_reactivate_subscription() {
		let sub_id = [1u8; 32];
		let origin = RawOrigin::Root;

		#[block]
		{
			IdnConsumer::<T>::sudo_reactivate_subscription(origin.into(), sub_id)
				.expect("sudo_reactivate_subscription should not fail");
		}
	}

	#[benchmark]
	fn sudo_request_quote() {
		let sub_id = None;
		let req_ref = None;
		let number_of_pulses = 10;
		let frequency = 5;
		let metadata = None;
		let origin = RawOrigin::Root;

		#[block]
		{
			IdnConsumer::<T>::sudo_request_quote(
				origin.into(),
				number_of_pulses,
				frequency,
				metadata,
				sub_id,
				req_ref,
			)
			.expect("sudo_request_quote should not fail");
		}
	}

	#[benchmark]
	fn sudo_request_sub_info() {
		let sub_id = [1u8; 32];
		let origin = RawOrigin::Root;

		#[block]
		{
			IdnConsumer::<T>::sudo_request_sub_info(origin.into(), sub_id, None)
				.expect("sudo_request_sub_info should not fail");
		}
	}
}
