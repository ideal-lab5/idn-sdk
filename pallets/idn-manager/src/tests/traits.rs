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

//! # Tests for the IDN Manager traits
use crate::traits::{BalanceDirection, DiffBalance, FeesError, FeesManager};
use proptest::prelude::*;
use sp_runtime::traits::Zero;
use sp_std::cmp::Ordering;

struct DummyFeesManager;
/// A simple implementation of the `DiffBalance` trait.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DiffBalanceImpl<Balance> {
	balance: Balance,
	direction: BalanceDirection,
}

impl<Balance: Copy> DiffBalance<Balance> for DiffBalanceImpl<Balance> {
	fn balance(&self) -> Balance {
		self.balance
	}
	fn direction(&self) -> BalanceDirection {
		self.direction
	}
	fn new(balance: Balance, direction: BalanceDirection) -> Self {
		Self { balance, direction }
	}
}

impl FeesManager<u32, u32, u32, u32, (), (), (), DiffBalanceImpl<u32>> for DummyFeesManager {
	fn calculate_subscription_fees(_credits: &u32) -> u32 {
		0
	}

	fn calculate_diff_fees(old_credits: &u32, new_credits: &u32) -> DiffBalanceImpl<u32> {
		let mut direction = BalanceDirection::None;
		let fees = match new_credits.cmp(old_credits) {
			Ordering::Greater => {
				direction = BalanceDirection::Collect;
				Self::calculate_subscription_fees(&new_credits.saturating_sub(*old_credits))
			},
			Ordering::Less => {
				direction = BalanceDirection::Release;
				Self::calculate_subscription_fees(&old_credits.saturating_sub(*new_credits))
			},
			Ordering::Equal => Zero::zero(),
		};
		DiffBalanceImpl::new(fees, direction)
	}

	fn collect_fees(fees: &u32, _: &()) -> Result<u32, FeesError<u32, ()>> {
		// In this case we are not collecting any fees
		Ok(*fees)
	}

	fn get_consume_credits(_sub: Option<&()>) -> u32 {
		// Consuming a pulse costs 1000 credits
		1000
	}

	fn get_idle_credits(_sub: Option<&()>) -> u32 {
		// Skipping a pulse costs 10 credits
		10
	}
}

proptest! {
	#[test]
	fn test_calculate_credits_proptest(f in 1u32..u32::MAX, n in 1u32..u32::MAX) {
		let consume = DummyFeesManager::get_consume_credits(None);
		let idle = DummyFeesManager::get_idle_credits(None);
		// this line will overflow if it exceeds u32::MAX: n * (10f + 1000) > 4,294,967,295
		// so we need to make sure: n <= u32::MAX/(10f - 1000)
		// we must also have idle*f < u32::MAX, so we need f < u32::MAX/idle to avoid an overflowF
		if n < u32::MAX/(idle*f - consume) && f < u32::MAX/idle {
			let expected = n * (f * idle + consume);
			let result = DummyFeesManager::calculate_credits(f, n);

			prop_assert_eq!(result, expected);
		}
	}
}
