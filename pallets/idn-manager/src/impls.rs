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

//! # Trait Implementations for the IDN Manager Pallet
//!
//! This file contains implementations of various traits required by the IDN Manager pallet.
//! These implementations provide the concrete logic for calculating fees, managing deposits,
//! and interacting with other pallets in the system.
//!
//! ## Key Implementations:
//!
//! - [`Subscription Trait`](SubscriptionTrait): Implemented for the [`Subscription`] struct,
//!   providing access to subscription details such as the subscriber account.
//! - [`FeesManager`]: Implemented by [`FeesManagerImpl`], providing the logic for calculating
//!   subscription fees, managing fee differences, and collecting fees from subscribers.
//! - [`DepositCalculator`]: Implemented by [`DepositCalculatorImpl`], providing the logic for
//!   calculating storage deposits and managing deposit differences.

use crate::{
	self as pallet_idn_manager,
	traits::{BalanceDirection, DiffBalance, FeesError},
	HoldReason, Subscription, SubscriptionTrait,
};
use codec::Encode;
use frame_support::{
	pallet_prelude::DispatchError,
	traits::{
		tokens::{
			fungible::{hold::Mutate as HoldMutate, Inspect, Mutate},
			Fortitude, Precision, Restriction,
		},
		Get,
	},
};
use pallet_idn_manager::{DepositCalculator, FeesManager};
use sp_arithmetic::traits::Unsigned;
use sp_runtime::{traits::Zero, AccountId32, Saturating};
use sp_std::{cmp::Ordering, marker::PhantomData};

impl<AccountId, BlockNumber, Credits: Unsigned, Metadata, SubscriptionId>
	SubscriptionTrait<AccountId>
	for Subscription<AccountId, BlockNumber, Credits, Metadata, SubscriptionId>
{
	fn subscriber(&self) -> &AccountId {
		&self.details.subscriber
	}
}

/// This struct represent movement of balance.
///
/// * `balance` - how much balance being moved.
/// * `direction` - if the balance are being collected or released.
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

/// A FeesManager implementation that holds a dynamic treasury account.
///
/// This implementation uses a dynamic treasury account specified in the `Config`
/// to manage fees. It provides functions for calculating subscription fees,
/// calculating the difference in fees, and collecting fees from subscribers.
///
/// # Type Parameters:
/// * `Treasury`: A `Get<AccountId32>` implementation that provides the treasury account ID.
/// * `Sub`: A `SubscriptionTrait<AccountId32>` implementation that provides access to subscription
///   details.
/// * `Balances`: A `Mutate<AccountId32>` implementation that provides the ability to hold and
///   release balances.
pub struct FeesManagerImpl<Treasury, Sub, Balances, Frequency, PulseIndex, Fee> {
	_phantom: PhantomData<(Treasury, Sub, Balances, Frequency, PulseIndex, Fee)>,
}

/// Implementation of the FeesManager trait for the FeesManagerImpl struct.
///
/// # Tests
///
/// ## Calculate Subscription Fees Test
#[doc = docify::embed!("./src/tests/pallet.rs", test_calculate_subscription_fees)]
impl<Treasury, Sub, Balances, Frequency, PulseIndex, Fee>
	FeesManager<
		Balances::Balance,
		u64,
		Frequency,
		PulseIndex,
		Sub,
		DispatchError,
		AccountId32,
		DiffBalanceImpl<Balances::Balance>,
	> for FeesManagerImpl<Treasury, Sub, Balances, Frequency, PulseIndex, Fee>
where
	Treasury: Get<AccountId32>,
	Sub: SubscriptionTrait<AccountId32>,
	Balances: HoldMutate<AccountId32> + Mutate<AccountId32> + Inspect<AccountId32>,
	Balances::Reason: From<HoldReason>,
	Balances::Balance: From<u64>,
	Fee: Get<u64>,
	Frequency: Saturating,
	PulseIndex: Saturating,
	u64: From<Frequency>,
	u64: From<PulseIndex>,
{
	/// Calculate the subscription fees based on the number of requested credits.
	///
	/// This function implements a tiered pricing model with volume discounts:
	/// - Tier 1 (1-10_000 credits): 100% of base fee per credit (no discount)
	/// - Tier 2 (10_001-100_000 credits): 95% of base fee per credit (5% discount)
	/// - Tier 3 (101-1000 credits): 90% of base fee per credit (10% discount)
	/// - Tier 4 (1001-10000 credits): 80% of base fee per credit (20% discount)
	/// - Tier 5 (10001+ credits): 70% of base fee per credit (30% discount)
	///
	/// The fee calculation processes each tier sequentially:
	/// 1. For each tier, calculate how many credits fall within that tier
	/// 2. Apply the corresponding discount rate to those credits
	/// 3. Sum up the fees across all tiers
	fn calculate_subscription_fees(credits: &u64) -> Balances::Balance {
		// Define tier boundaries and their respective discount rates (in basis points)
		const TIERS: [(u64, u64); 5] = [
			(1, 0),           // 1-10_000: 0% discount
			(10_001, 5),      // 10_001-100_000: 5% discount
			(100_001, 10),    // 100_001-1_000_000: 10% discount
			(1_000_001, 20),  // 1_000_001-10_000_000: 20% discount
			(10_000_001, 30), // 10_000_001+: 30% discount
		];

		let mut total_fee = 0u64;
		let mut remaining_credits = *credits;

		for (i, &(current_tier_start, current_tier_discount)) in TIERS.iter().enumerate() {
			// If no remaining credits exit loop.
			if remaining_credits == 0 {
				break;
			}

			let next_tier_start = TIERS.get(i + 1).map(|&(start, _)| start).unwrap_or(u64::MAX);

			let credits_in_tier =
				(credits.min(&next_tier_start.saturating_sub(1)) - current_tier_start + 1)
					.min(remaining_credits);

			let tier_fee = Fee::get()
				.saturating_mul(credits_in_tier)
				.saturating_mul(100 - current_tier_discount)
				.saturating_div(100);

			total_fee = total_fee.saturating_add(tier_fee);
			remaining_credits = remaining_credits.saturating_sub(credits_in_tier);
		}

		total_fee.into()
	}

	/// Calculate the difference in fees when a subscription changes, determining whether
	/// additional fees should be collected or excess fees should be released. This is useful for
	/// subscription updates and kills for holding or releasing fees. Or when collecting fees from a
	/// subscriber.
	///
	/// This function compares the fees required before and after a subscription change,
	/// then returns:
	/// - The fee difference amount
	/// - The direction of the balance transfer (collect from user, release to user, or no change)
	fn calculate_diff_fees(
		old_credits: &u64,
		new_credits: &u64,
	) -> DiffBalanceImpl<Balances::Balance> {
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
		DiffBalanceImpl { balance: fees, direction }
	}

	/// Attempts to collect subscription fees from a subscriber and transfer them to the treasury
	/// account.
	///
	/// This function:
	/// 1. Transfers the specified fees from the subscriber's held balance to the treasury account
	/// 2. Verifies that the full fee amount was successfully collected
	/// 3. Returns the actual amount collected or an appropriate error
	///
	/// # Notes
	/// - This function uses
	///   [`transfer_on_hold`](frame_support::traits::tokens::fungible::hold::Mutate::transfer_on_hold)
	///   which transfers from the subscriber's held balance
	/// - The fees are held under the [`HoldReason::Fees`] reason code
	/// - The transfer uses [`Precision::BestEffort`] which allows partial transfers if full amount
	///   isn't available
	/// - Despite using best effort, this function will return an error if less than the requested
	///   amount is collected
	fn collect_fees(
		fees: &Balances::Balance,
		sub: &Sub,
	) -> Result<Balances::Balance, FeesError<Balances::Balance, DispatchError>> {
		// Ensure the treasury account is set for benchmarking purposes
		#[cfg(feature = "runtime-benchmarks")]
		if Balances::balance(&Treasury::get()) < Balances::minimum_balance() {
			Balances::set_balance(
				&Treasury::get(),
				Balances::minimum_balance() - Balances::balance(&Treasury::get()),
			);
		}

		// Collect the held fees from the subscriber
		let collected = Balances::transfer_on_hold(
			&HoldReason::Fees.into(),
			sub.subscriber(),
			&Treasury::get(),
			*fees,
			Precision::BestEffort,
			Restriction::Free,
			Fortitude::Polite,
		)
		.map_err(FeesError::Other)?;

		// Ensure the correct credits were collected.
		if collected < *fees {
			return Err(FeesError::NotEnoughBalance { needed: *fees, balance: collected });
		}

		Ok(collected)
	}

	/// Get the number of credits consumed by a subscription when this one gets a pulse in a block.
	fn get_consume_credits(_sub: Option<&Sub>) -> u64 {
		1
	}

	/// Get the number of credits consumed by a subscription when this one is idle in a block.
	fn get_idle_credits(_sub: Option<&Sub>) -> u64 {
		1
	}
}

/// A DepositCalculator implementation that uses a configurable deposit multiplier.
///
/// This struct is used to calculate the storage deposit required for a subscription. The deposit
/// amount is calculated by multiplying the encoded size of the subscription data by the storage
/// deposit multiplier.
///
/// # Tests
///
/// ## Hold Deposit Test
#[doc = docify::embed!("./src/tests/pallet.rs", hold_deposit_works)]
///
/// ## Release Deposit Test
#[doc = docify::embed!("./src/tests/pallet.rs", release_deposit_works)]
pub struct DepositCalculatorImpl<SDMultiplier: Get<Deposit>, Deposit> {
	_phantom: PhantomData<(SDMultiplier, Deposit)>,
}

impl<
		Sub: SubscriptionTrait<AccountId32> + Encode,
		SDMultiplier: Get<Deposit>,
		Deposit: Saturating + From<u64> + Ord + Copy,
	> DepositCalculator<Deposit, Sub, DiffBalanceImpl<Deposit>>
	for DepositCalculatorImpl<SDMultiplier, Deposit>
{
	/// Calculate the storage deposit required for a subscription.
	///
	/// This function computes the storage deposit amount based on the encoded size of the
	/// subscription object multiplied by a configurable deposit multiplier.
	///
	/// # Security Considerations
	/// This function handles potential overflow scenarios by:
	/// 1. Converting the encoded size from `usize` to `u64` with fallback to `u64::MAX` if the
	///    conversion fails
	/// 2. Using saturating multiplication to prevent arithmetic overflow
	fn calculate_storage_deposit(sub: &Sub) -> Deposit {
		// [SRLabs] Note: There is a theoretical edge case where if the `Deposit` type (e.g., u64)
		// is larger than the machine architecture size (e.g., 32-bit platforms), an attacker
		// could create a subscription object larger than u32::MAX bits and only pay a deposit for
		// u32::MAX bits, not the full size. This risk is mitigated in practice by platform
		// constraints and cost barriers.
		let storage_deposit_multiplier = SDMultiplier::get();
		let encoded_size = u64::try_from(sub.encoded_size()).unwrap_or(u64::MAX);
		storage_deposit_multiplier.saturating_mul(encoded_size.into())
	}

	/// Calculate the difference in storage deposit between two subscriptions.
	///
	/// This function compares the storage deposit requirements of two subscription states
	/// and returns the difference along with the direction of the balance adjustment.
	///
	/// # How It Works
	/// 1. Calculates the storage deposit for both the old and new subscription states
	/// 2. Compares the two deposits to determine if more deposit is needed, some can be released,
	///    or no change is required
	/// 3. Returns both the amount and direction of the required deposit adjustment
	fn calculate_diff_deposit(old_sub: &Sub, new_sub: &Sub) -> DiffBalanceImpl<Deposit> {
		let old_deposit = Self::calculate_storage_deposit(old_sub);
		let new_deposit = Self::calculate_storage_deposit(new_sub);
		let direction = match new_deposit.cmp(&old_deposit) {
			Ordering::Greater => BalanceDirection::Collect,
			Ordering::Less => BalanceDirection::Release,
			Ordering::Equal => BalanceDirection::None,
		};
		DiffBalanceImpl { balance: new_deposit.saturating_sub(old_deposit), direction }
	}
}
