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

//! # Fee Calculators Examples
//!
//! This module contains examples of fee calculators that can be used in the IDN Manager pallet.

use crate::traits::FeesManager;

/// Linear fee calculator with no discount
pub struct LinearFeeCalculator;

impl FeesManager<u32, u32, (), (), ()> for LinearFeeCalculator {
	fn calculate_subscription_fees(amount: u32) -> u32 {
		let base_fee = 100u32;
		base_fee.saturating_mul(amount.into())
	}
	fn calculate_refund_fees(_init_amount: u32, current_amount: u32) -> u32 {
		// in this case of a liner function, the refund's is the same as the fees'
		Self::calculate_subscription_fees(current_amount)
	}
	fn collect_fees(fees: u32, _: ()) -> Result<u32, crate::traits::FeesError<u32, ()>> {
		// In this case, we don't need to do anything with the fees, so we just return them
		Ok(fees)
	}
}

/// Tiered fee calculator with predefined discount tiers
pub struct SteppedTieredFeeCalculator;

impl FeesManager<u32, u32, (), (), ()> for SteppedTieredFeeCalculator {
	fn calculate_subscription_fees(amount: u32) -> u32 {
		let base_fee = 100u32;

		// Define tier boundaries and their respective discount rates (in basis points)
		const TIERS: [(u32, u32); 5] = [
			(1, 0),        // 0-10: 0% discount
			(11, 500),     // 11-100: 5% discount
			(101, 1000),   // 101-1000: 10% discount
			(1001, 2000),  // 1001-10000: 20% discount
			(10001, 3000), // 10001+: 30% discount
		];

		let mut total_fee = 0u32;
		let mut remaining_amount = amount;

		for (i, &(current_tier_start, current_tier_discount)) in TIERS.iter().enumerate() {
			// If no remaining credits or the tier starts above the requested amount, exit loop.
			if remaining_amount == 0 || amount < current_tier_start {
				break;
			}

			let next_tier_start = TIERS.get(i + 1).map(|&(start, _)| start).unwrap_or(u32::MAX);
			let credits_in_tier =
				(amount.min(next_tier_start.saturating_sub(1)) - current_tier_start + 1)
					.min(remaining_amount);

			let tier_fee = base_fee
				.saturating_mul(credits_in_tier)
				.saturating_mul((10_000 - current_tier_discount) as u32)
				.saturating_div(10_000);

			total_fee = total_fee.saturating_add(tier_fee);
			remaining_amount = remaining_amount.saturating_sub(credits_in_tier);
		}

		total_fee
	}
	// The discount for the `used_amount` (`init_amount` - `current_amount`) is taken from the lower
	// tieres (those with less discounts)
	fn calculate_refund_fees(init_amount: u32, current_amount: u32) -> u32 {
		let used_amount = init_amount - current_amount;
		Self::calculate_subscription_fees(init_amount) -
			Self::calculate_subscription_fees(used_amount)
	}
	fn collect_fees(fees: u32, _: ()) -> Result<u32, crate::traits::FeesError<u32, ()>> {
		// In this case, we don't need to do anything with the fees, so we just return them
		Ok(fees)
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
	/// Test when no subscription credits were used.
	#[test]
	fn test_calculate_refund_linear_fees_no_usage() {
		let init_amount: u32 = 50;
		let current_amount: u32 = 50; // no credits used
								// In this case the refund should equal the fee for the entire subscription.
		let refund = LinearFeeCalculator::calculate_refund_fees(init_amount, current_amount);
		let expected = LinearFeeCalculator::calculate_subscription_fees(init_amount) -
			LinearFeeCalculator::calculate_subscription_fees(0); // used_amount is 0
		assert_eq!(
			refund, expected,
			"Refund must equal entire subscription fee when no credits were used"
		);
	}

	/// Test for a partial usage scenario.
	#[test]
	fn test_calculate_refund_linear_fees_partial_usage() {
		let init_amount: u32 = 50;
		let current_amount: u32 = 30; // 20 credits used
		let refund = LinearFeeCalculator::calculate_refund_fees(init_amount, current_amount);
		let used_amount = init_amount - current_amount; // 20
		let expected = LinearFeeCalculator::calculate_subscription_fees(init_amount) -
			LinearFeeCalculator::calculate_subscription_fees(used_amount);
		assert_eq!(
			refund, expected,
			"Refund must equal the fee difference between full subscription and the used portion"
		);
	}

	/// Test when the subscription is fully used.
	#[test]
	fn test_calculate_refund_linear_fees_full_usage() {
		let init_amount: u32 = 50;
		let current_amount: u32 = 0; // all credits used
		let refund = LinearFeeCalculator::calculate_refund_fees(init_amount, current_amount);
		let expected = LinearFeeCalculator::calculate_subscription_fees(init_amount) -
			LinearFeeCalculator::calculate_subscription_fees(init_amount); // should be 0
		assert_eq!(refund, expected, "Refund must be zero when all subscription credits are used");
	}

	#[test]
	fn test_stepped_tiered_calculator() {
		// Test first tier (no discount)
		assert_eq!(SteppedTieredFeeCalculator::calculate_subscription_fees(10), 1_000); // 10 * 100 = 1,000

		// Test crossing into second tier
		let fee_11 = SteppedTieredFeeCalculator::calculate_subscription_fees(11);
		assert_eq!(fee_11, 1_095); // (10 * 100) + (1 * 95) = 1,000 + 95 = 1,095

		// Test middle of second tier
		let fee_50 = SteppedTieredFeeCalculator::calculate_subscription_fees(50);
		assert_eq!(fee_50, 4_800); // (10 * 100) + (40 * 95) = 1,000 + 3,800 = 4,800

		// Test crossing multiple tiers
		let fee_150 = SteppedTieredFeeCalculator::calculate_subscription_fees(150);
		// First 10 at 100% = 1,000
		// Next 90 at 95% = 8,550
		// Next 50 at 90% = 4,500
		// Total should be 14,050
		assert_eq!(fee_150, 14_050);
	}

	#[test]
	fn test_no_price_inversion() {
		// Test that buying more never costs less
		let fee_10 = SteppedTieredFeeCalculator::calculate_subscription_fees(10);
		let fee_11 = SteppedTieredFeeCalculator::calculate_subscription_fees(11);
		assert!(fee_11 > fee_10, "11 credits should cost more than 10 credits");

		// Test around the 100 credit boundary
		let fee_100 = SteppedTieredFeeCalculator::calculate_subscription_fees(100);
		let fee_101 = SteppedTieredFeeCalculator::calculate_subscription_fees(101);
		assert!(fee_101 > fee_100, "101 credits should cost more than 100 credits");
	}

	/// Test when no subscription credits were used.
	#[test]
	fn test_calculate_refund_fees_no_usage() {
		let init_amount: u32 = 50;
		let current_amount: u32 = 50; // no credits used
								// In this case the refund should equal the fee for the entire subscription.
		let refund = SteppedTieredFeeCalculator::calculate_refund_fees(init_amount, current_amount);
		let expected = SteppedTieredFeeCalculator::calculate_subscription_fees(init_amount) -
			SteppedTieredFeeCalculator::calculate_subscription_fees(0); // used_amount is 0
		assert_eq!(
			refund, expected,
			"Refund must equal entire subscription fee when no credits were used"
		);
	}

	/// Test for a partial usage scenario.
	#[test]
	fn test_calculate_refund_fees_partial_usage() {
		let init_amount: u32 = 50;
		let current_amount: u32 = 30; // 20 credits used
		let refund = SteppedTieredFeeCalculator::calculate_refund_fees(init_amount, current_amount);
		let used_amount = init_amount - current_amount; // 20
		let expected = SteppedTieredFeeCalculator::calculate_subscription_fees(init_amount) -
			SteppedTieredFeeCalculator::calculate_subscription_fees(used_amount);
		assert_eq!(
			refund, expected,
			"Refund must equal the fee difference between full subscription and the used portion"
		);
	}

	/// Test when the subscription is fully used.
	#[test]
	fn test_calculate_refund_fees_full_usage() {
		let init_amount: u32 = 50;
		let current_amount: u32 = 0; // all credits used
		let refund = SteppedTieredFeeCalculator::calculate_refund_fees(init_amount, current_amount);
		let expected = SteppedTieredFeeCalculator::calculate_subscription_fees(init_amount) -
			SteppedTieredFeeCalculator::calculate_subscription_fees(init_amount); // should be 0
		assert_eq!(refund, expected, "Refund must be zero when all subscription credits are used");
	}
}
