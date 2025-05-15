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

use crate::traits::{BalanceDirection, DiffBalance, FeesError, FeesManager};
use sp_runtime::traits::Zero;
use sp_std::cmp::Ordering;

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

/// Linear fee calculator with no discount
pub struct LinearFeeCalculator;

const BASE_FEE: u32 = 100;

#[docify::export_content]
mod linear_fee_calculator {
	use super::*;
	impl FeesManager<u32, u32, (), (), (), DiffBalanceImpl<u32>, u32, u32> for LinearFeeCalculator {
		fn calculate_subscription_fees(credits: &u32) -> u32 {
			BASE_FEE.saturating_mul(*credits)
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
		fn get_consume_credits(_sub: &()) -> u32 {
			// Consuming a pulse costs 1000 credits
			1000
		}
		fn get_idle_credits(_sub: &()) -> u32 {
			// Skipping a pulse costs 10 credits
			10
		}
		/// Estimate the cost in credits for a certain number of pulses with a certain frequency of use.
		/// Here, frequency is specified in number of blocks per pulse where 1 means "every block", 2 is "every other block",...
		/// frequency: blocks per pulse, pulses: how many pulses they wish to consume
		/// This function also includes the cost to create a subscription so that users aren't quoted
		/// only what fees they incur.
		fn calculate_credits(pulses: &u32, frequency: &u32) -> u32 {

			let subscription_cost: u32 = 30;
			let subscription_management_fee: u32 = 10;
			let pulse_cost: u32 = 1;

			let total_fees:u32 = pulses * (subscription_management_fee*(frequency - 1) + pulse_cost);

			if subscription_cost < total_fees {
				let multiplier:u32 = total_fees.div_ceil(subscription_cost);
				total_fees + multiplier * subscription_cost
			} else {
				total_fees + subscription_cost
			}
		}
	}
}

/// Tiered fee calculator with predefined discount tiers
pub struct SteppedTieredFeeCalculator;

#[docify::export_content]
mod tiered_fee_calculator {
	use super::*;
	impl FeesManager<u32, u32, (), (), (), DiffBalanceImpl<u32>, u32, u32> for SteppedTieredFeeCalculator {
		fn calculate_subscription_fees(credits: &u32) -> u32 {
			// Define tier boundaries and their respective discount rates (in basis points)
			const TIERS: [(u32, u32); 5] = [
				(1, 0),        // 0-10: 0% discount
				(11, 500),     // 11-100: 5% discount
				(101, 1000),   // 101-1000: 10% discount
				(1001, 2000),  // 1001-10000: 20% discount
				(10001, 3000), // 10001+: 30% discount
			];

			let mut total_fee = 0u32;
			let mut remaining_credits = *credits;

			for (i, &(current_tier_start, current_tier_discount)) in TIERS.iter().enumerate() {
				// If no remaining credits exit loop.
				if remaining_credits == 0 {
					break;
				}

				let next_tier_start = TIERS.get(i + 1).map(|&(start, _)| start).unwrap_or(u32::MAX);
				let credits_in_tier =
					(credits.min(&next_tier_start.saturating_sub(1)) - current_tier_start + 1)
						.min(remaining_credits);

				let tier_fee = BASE_FEE
					.saturating_mul(credits_in_tier)
					.saturating_mul(10_000 - current_tier_discount)
					.saturating_div(10_000);

				total_fee = total_fee.saturating_add(tier_fee);
				remaining_credits = remaining_credits.saturating_sub(credits_in_tier);
			}

			total_fee
		}

		fn calculate_diff_fees(old_credits: &u32, new_credits: &u32) -> DiffBalanceImpl<u32> {
			let old_fees = Self::calculate_subscription_fees(old_credits);
			let new_fees = Self::calculate_subscription_fees(new_credits);
			let mut direction = BalanceDirection::None;
			let fees = match new_fees.cmp(&old_fees) {
				Ordering::Greater => {
					direction = BalanceDirection::Collect;
					new_fees - old_fees
				},
				Ordering::Less => {
					direction = BalanceDirection::Release;
					old_fees - new_fees
				},
				Ordering::Equal => Zero::zero(),
			};
			DiffBalanceImpl::new(fees, direction)
		}
		fn collect_fees(fees: &u32, _: &()) -> Result<u32, FeesError<u32, ()>> {
			// In this case we are not collecting any fees
			Ok(*fees)
		}
		fn get_consume_credits(_sub: &()) -> u32 {
			// Consuming a pulse costs 1000 credits
			1000
		}
		fn get_idle_credits(_sub: &()) -> u32 {
			// Skipping a pulse costs 10 credits
			10
		}
		/// Estimate the cost in credits for a certain number of pulses with a certain frequency of use.
		/// Here, frequency is specified in number of blocks per pulse where 1 means "every block", 2 is "every other block",...
		/// frequency: blocks per pulse, pulses: how many pulses they wish to consume
		/// This function also includes the cost to create a subscription so that users aren't quoted
		/// only what fees they incur.
		fn calculate_credits(pulses: &u32, frequency: &u32) -> u32 {

			let subscription_cost: u32 = 30;
			let subscription_management_fee: u32 = 10;
			let pulse_cost: u32 = 1;

			let total_fees:u32 = pulses * (subscription_management_fee*(frequency - 1) + pulse_cost);
			

			if subscription_cost < total_fees {
				let multiplier:u32 = total_fees.div_ceil(subscription_cost);
				total_fees + multiplier * subscription_cost
			} else {
				total_fees + subscription_cost
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_linear_fee_calculator() {
		assert_eq!(LinearFeeCalculator::calculate_subscription_fees(&1), 100);
		assert_eq!(LinearFeeCalculator::calculate_subscription_fees(&10), 1_000);
		assert_eq!(LinearFeeCalculator::calculate_subscription_fees(&100), 10_000);
	}

	/// Thest when a subscription is fully used before being killed or the update does not change
	/// the credits.
	#[test]
	fn test_calculate_release_linear_fees_no_diff() {
		let old_credits: u32 = 50;
		let new_credits: u32 = 50; // no credits diff
		let refund = LinearFeeCalculator::calculate_diff_fees(&old_credits, &new_credits);
		// there should be no refund as all credit has been used, or no difference in the update
		let expected = 0;
		assert_eq!(
			refund,
			DiffBalanceImpl::new(expected, BalanceDirection::None),
			"There should be no release when no diff in credits"
		);
	}

	/// Test when a subscription is reduced or killed.
	#[test]
	fn test_calculate_release_linear_fees_lower_diff() {
		let old_credits: u32 = 50;
		let new_credits: u32 = 30; // 20 credits used
		let refund = LinearFeeCalculator::calculate_diff_fees(&old_credits, &new_credits);
		let expected = 20 * BASE_FEE;
		assert_eq!(
			refund,
			DiffBalanceImpl::new(expected, BalanceDirection::Release),
			"There should be a release when new credits is lower than old credits"
		);
	}

	/// Test when the subscription is extended.
	#[test]
	fn test_calculate_hold_linear_fees_greater_diff() {
		let old_credits: u32 = 50;
		let new_credits: u32 = 60; // all credits used
		let hold = LinearFeeCalculator::calculate_diff_fees(&old_credits, &new_credits);
		let expected = 10 * BASE_FEE;
		assert_eq!(
			hold,
			DiffBalanceImpl::new(expected, BalanceDirection::Collect),
			"There should be more held balance"
		);
	}

	#[test]
	fn test_stepped_tiered_calculator() {
		// Test first tier (no discount)
		assert_eq!(SteppedTieredFeeCalculator::calculate_subscription_fees(&10), 1_000); // 10 * 100 = 1,000

		// Test crossing into second tier
		let fee_11 = SteppedTieredFeeCalculator::calculate_subscription_fees(&11);
		assert_eq!(fee_11, 1_095); // (10 * 100) + (1 * 95) = 1,000 + 95 = 1,095

		// Test middle of second tier
		let fee_50 = SteppedTieredFeeCalculator::calculate_subscription_fees(&50);
		assert_eq!(fee_50, 4_800); // (10 * 100) + (40 * 95) = 1,000 + 3,800 = 4,800

		// Test edge of second tier
		let fee_100 = SteppedTieredFeeCalculator::calculate_subscription_fees(&100);
		assert_eq!(fee_100, 9550); // (10 * 100) + (90 * 95) = 1,000 + 8,550 = 9,550

		// Test edge of second tier
		let fee_101 = SteppedTieredFeeCalculator::calculate_subscription_fees(&101);
		assert_eq!(fee_101, 9_640); // (10 * 100) + (90 * 95) + (1 * 90)= 1,000 + 8,550 + 90 = 9,640

		// Test crossing multiple tiers
		let fee_150 = SteppedTieredFeeCalculator::calculate_subscription_fees(&150);
		// First 10 at 100% = 1,000
		// Next 90 at 95% = 8,550
		// Next 50 at 90% = 4,500
		// Total should be 14,050
		assert_eq!(fee_150, 14_050);
	}

	#[test]
	fn test_no_price_inversion() {
		// Test that buying more never costs less
		let fee_10 = SteppedTieredFeeCalculator::calculate_subscription_fees(&10);
		let fee_11 = SteppedTieredFeeCalculator::calculate_subscription_fees(&11);
		assert!(fee_11 > fee_10, "11 credits should cost more than 10 credits");

		// Test around the 100 credit boundary
		let fee_100 = SteppedTieredFeeCalculator::calculate_subscription_fees(&100);
		let fee_101 = SteppedTieredFeeCalculator::calculate_subscription_fees(&101);
		assert!(fee_101 > fee_100, "101 credits should cost more than 100 credits");
	}

	/// Thest when a subscription is fully used before being killed or the update does not change
	/// the credits.
	#[test]
	fn test_calculate_release_stepped_fees_no_diff() {
		let old_credits: u32 = 50;
		let new_credits: u32 = 50; // no credits diff
		let refund = SteppedTieredFeeCalculator::calculate_diff_fees(&old_credits, &new_credits);
		let expected = 0;
		assert_eq!(
			refund,
			DiffBalanceImpl::new(expected, BalanceDirection::None),
			"There should be no release when no diff in credits"
		);
	}

	/// Test for a partial usage scenario.
	#[test]
	fn test_calculate_release_stepped_fees_lower_diff() {
		let old_credits: u32 = 110;
		let new_credits: u32 = 100; // 1 value decrease
		let refund = SteppedTieredFeeCalculator::calculate_diff_fees(&old_credits, &new_credits);
		let expected = 10 * BASE_FEE.saturating_mul(9_000).saturating_div(10_000); // 10% discount on the extra value
		assert_eq!(
			refund,
			DiffBalanceImpl::new(expected, BalanceDirection::Release),
			"There should be a release when new credits is lower than old credits"
		);
	}

	/// Test when the subscription is fully used.
	#[test]
	fn test_calculate_hold_stepped_fees_greater_diff() {
		let old_credits: u32 = 100;
		let new_credits: u32 = 101; // 1 value increase
		let hold = SteppedTieredFeeCalculator::calculate_diff_fees(&old_credits, &new_credits);
		let expected = 1 * BASE_FEE.saturating_mul(9_000).saturating_div(10_000); // 10% discount on the extra value
		assert_eq!(
			hold,
			DiffBalanceImpl::new(expected, BalanceDirection::Collect),
			"There should be more held balance"
		);
	}
}
