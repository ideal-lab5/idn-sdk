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

use crate::*;
use sp_runtime::traits::Saturating;

/// Linear fee calculator with no discount
pub struct LinearFeeCalculator;

impl<Balance: From<u128> + Saturating> FeeCalculator<Balance> for LinearFeeCalculator {
	fn calculate_fee(duration: u32) -> Balance {
		base_fee.saturating_mul(duration.into())
	}
}

/// Tiered fee calculator with predefined discount tiers
pub struct TieredFeeCalculator;

impl<Balance: From<u128> + Saturating> FeeCalculator<Balance> for TieredFeeCalculator {
	fn calculate_fee(duration: u32) -> Balance {
		let discount = match duration {
			0..=10 => 0,
			11..=100 => 500,
			101..=1000 => 1000,
			1001..=10000 => 2000,
			_ => 3000,
		};

		let discounted_fee = base_fee
			.saturating_mul((10000u32.saturating_sub(discount)) as u128)
			.saturating_div(10000u128);

		discounted_fee.saturating_mul(duration.into())
	}
}

/// Logarithmic fee calculator
pub struct LogarithmicFeeCalculator;

impl<Balance: From<u128> + Saturating> FeeCalculator<Balance> for LogarithmicFeeCalculator {
	fn calculate_fee(duration: u32) -> Balance {
		let log_duration = 32u32.saturating_sub((duration + 1).leading_zeros());
		let discount = log_duration.saturating_mul(100); // 1% per log2 step
		let capped_discount = discount.min(5000); // Max 50% discount

		let discounted_fee = base_fee
			.saturating_mul((10000u32.saturating_sub(capped_discount)) as u128)
			.saturating_div(10000u128);

		discounted_fee.saturating_mul(duration.into())
	}
}

/// Square root fee calculator
pub struct SqrtFeeCalculator;

impl<Balance: From<u128> + Saturating> FeeCalculator<Balance> for SqrtFeeCalculator {
	fn calculate_fee(duration: u32) -> Balance {
		let sqrt_duration = (duration as f64).sqrt() as u32;
		let discount = sqrt_duration.saturating_mul(100); // 1% per sqrt unit
		let capped_discount = discount.min(5000); // Max 50% discount

		let discounted_fee = base_fee
			.saturating_mul((10000u32.saturating_sub(capped_discount)) as u128)
			.saturating_div(10000u128);

		discounted_fee.saturating_mul(duration.into())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::assert_ok;
	use sp_runtime::DispatchError;

	// Mock runtime configuration
	frame_support::parameter_types! {
		pub const BaseSubscriptionFeePerBlock: u128 = 100;
	}

	#[test]
	fn test_linear_fee_calculator() {
		let base_fee = 100u128;

		assert_eq!(LinearFeeCalculator::calculate_fee(base_fee, 1), 100);
		assert_eq!(LinearFeeCalculator::calculate_fee(base_fee, 10), 1000);
		assert_eq!(LinearFeeCalculator::calculate_fee(base_fee, 100), 10000);
	}

	#[test]
	fn test_tiered_fee_calculator() {
		let base_fee = 100u128;

		// No discount tier (0-10 blocks)
		assert_eq!(TieredFeeCalculator::calculate_fee(base_fee, 10), 1000);

		// 5% discount tier (11-100 blocks)
		assert_eq!(TieredFeeCalculator::calculate_fee(base_fee, 50), 4750); // 95% of 5000

		// 10% discount tier (101-1000 blocks)
		assert_eq!(TieredFeeCalculator::calculate_fee(base_fee, 500), 45000); // 90% of 50000
	}

	#[test]
	fn test_logarithmic_fee_calculator() {
		let base_fee = 100u128;

		// Test various durations
		let fee_1 = LogarithmicFeeCalculator::calculate_fee(base_fee, 1);
		let fee_10 = LogarithmicFeeCalculator::calculate_fee(base_fee, 10);
		let fee_100 = LogarithmicFeeCalculator::calculate_fee(base_fee, 100);

		// Assert discount increases logarithmically
		assert!(fee_10 < fee_1.saturating_mul(10));
		assert!(fee_100 < fee_10.saturating_mul(10));
	}

	#[test]
	fn test_sqrt_fee_calculator() {
		let base_fee = 100u128;

		// Test various durations
		let fee_1 = SqrtFeeCalculator::calculate_fee(base_fee, 1);
		let fee_4 = SqrtFeeCalculator::calculate_fee(base_fee, 4);
		let fee_9 = SqrtFeeCalculator::calculate_fee(base_fee, 9);

		// Assert discount increases with square root
		assert!(fee_4 < fee_1.saturating_mul(4));
		assert!(fee_9 < fee_4.saturating_mul(2));
	}
}
