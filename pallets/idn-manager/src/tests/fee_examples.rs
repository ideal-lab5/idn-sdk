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

//! # Fee Calculators Examples
//!
//! This module contains examples of fee calculators that can be used in the IDN Manager pallet.

use crate::traits::FeesCalculator;
use frame_support::sp_runtime::traits::Saturating;

/// Linear fee calculator with no discount
pub struct LinearFeeCalculator;

impl FeesCalculator<u32, u32> for LinearFeeCalculator {
	fn calculate_subscription_fees(amount: u32) -> u32 {
		let base_fee = 100u32;
		base_fee.saturating_mul(amount.into())
	}
}

/// Tiered fee calculator with predefined discount tiers
pub struct TieredFeeCalculator;

impl FeesCalculator<u32, u32> for TieredFeeCalculator {
	fn calculate_subscription_fees(amount: u32) -> u32 {
		let base_fee = 100;
		let discount = match amount {
			0..=10 => 0,
			11..=100 => 500,
			101..=1_000 => 1_000,
			1_001..=10_000 => 2_000,
			_ => 3_000,
		};

		let discounted_fee = base_fee
			.saturating_mul((10_000.saturating_sub(discount)) as u128)
			.saturating_div(10_000);

		discounted_fee.saturating_mul(amount.into()).try_into().unwrap()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_linear_fee_calculator() {
		assert_eq!(LinearFeeCalculator::calculate_subscription_fees(1), 100);
		assert_eq!(LinearFeeCalculator::calculate_subscription_fees(10), 1_000);
		assert_eq!(LinearFeeCalculator::calculate_subscription_fees(100), 10_000);
	}

	#[test]
	fn test_tiered_fee_calculator() {
		// No discount tier (0-10 blocks)
		assert_eq!(TieredFeeCalculator::calculate_subscription_fees(10), 1_000);

		// 5% discount tier (11-100 blocks)
		assert_eq!(TieredFeeCalculator::calculate_subscription_fees(50), 4_750); // 95% of 5_000

		// 10% discount tier (101-1_000 blocks)
		assert_eq!(TieredFeeCalculator::calculate_subscription_fees(500), 45_000); // 90% of 50_000

		// 20% discount tier (1_001-10_000 blocks)
		assert_eq!(TieredFeeCalculator::calculate_subscription_fees(5_000), 400_000); // 80% of 500_000

		// 30% discount tier (10_001+ blocks)
		assert_eq!(TieredFeeCalculator::calculate_subscription_fees(50_000), 3_500_000); // 70% of 5_000_000
	}
}
