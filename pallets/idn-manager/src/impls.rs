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

//! # Implementations of public traits

use crate::{
	self as pallet_idn_manager,
	traits::{BalanceDirection, DiffBalance, FeesError},
	HoldReason, Subscription, SubscriptionTrait,
};
use codec::Encode;
use frame_support::{
	pallet_prelude::DispatchError,
	traits::{
		tokens::{fungible::hold::Mutate, Fortitude, Precision, Restriction},
		Get,
	},
};
use sp_arithmetic::traits::Unsigned;
use sp_runtime::{traits::Zero, AccountId32, Saturating};
use sp_std::{cmp::Ordering, marker::PhantomData};

impl<AccountId, BlockNumber, Credits: Unsigned, Metadata> SubscriptionTrait<AccountId>
	for Subscription<AccountId, BlockNumber, Credits, Metadata>
{
	fn subscriber(&self) -> &AccountId {
		&self.details.subscriber
	}
}

impl SubscriptionTrait<()> for () {
	fn subscriber(&self) -> &() {
		&()
	}
}

/// A FeesManager implementation that holds a dynamic treasury account.
pub struct FeesManagerImpl<Treasury, BaseFee, Sub, Balances> {
	pub _phantom: FeesManagerPhantom<Treasury, BaseFee, Sub, Balances>,
}
type FeesManagerPhantom<Treasury, BaseFee, Sub, Balances> =
	(PhantomData<Treasury>, PhantomData<BaseFee>, PhantomData<Sub>, PhantomData<Balances>);

impl<
		T: Get<AccountId32>,
		B: Get<Balances::Balance>,
		S: SubscriptionTrait<AccountId32>,
		Balances: Mutate<AccountId32>,
	> pallet_idn_manager::FeesManager<Balances::Balance, u64, S, DispatchError, AccountId32>
	for FeesManagerImpl<T, B, S, Balances>
where
	Balances::Reason: From<HoldReason>,
	Balances::Balance: From<u64>,
{
	/// Calculate the subscription fees based on the number of requested credits.
	///
	/// This function implements a tiered pricing model with volume discounts:
	/// - Tier 1 (1-10 credits): 100% of base fee per credit (no discount)
	/// - Tier 2 (11-100 credits): 95% of base fee per credit (5% discount)
	/// - Tier 3 (101-1000 credits): 90% of base fee per credit (10% discount)
	/// - Tier 4 (1001-10000 credits): 80% of base fee per credit (20% discount)
	/// - Tier 5 (10001+ credits): 70% of base fee per credit (30% discount)
	///
	/// The fee calculation processes each tier sequentially:
	/// 1. For each tier, calculate how many credits fall within that tier
	/// 2. Apply the corresponding discount rate to those credits
	/// 3. Sum up the fees across all tiers
	///
	/// # Parameters
	/// * `credits` - The total number of credits requested for the subscription
	///
	/// # Returns
	/// The total fee required for the requested number of credits, converted to the
	/// appropriate balance type
	///
	/// # Example
	/// ```nocompile
	/// // 100 credits would incur a fee of:
	/// // - 10 credits at full price: 10 * 100 = 1000
	/// // - 90 credits at 5% discount: 90 * 95 = 8550
	/// // Total: 9550
	/// let fees = calculate_subscription_fees(&100u64);
	/// assert_eq!(fees, 9550u64.into());
	/// ```
	fn calculate_subscription_fees(credits: &u64) -> Balances::Balance {
		// Define tier boundaries and their respective discount rates (in basis points)
		const TIERS: [(u64, u64); 5] = [
			(1, 0),        // 0-10: 0% discount
			(11, 500),     // 11-100: 5% discount
			(101, 1000),   // 101-1000: 10% discount
			(1001, 2000),  // 1001-10000: 20% discount
			(10001, 3000), // 10001+: 30% discount
		];

		const BASE_FEE: u64 = 100;

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

			let tier_fee = BASE_FEE
				.saturating_mul(credits_in_tier)
				.saturating_mul(10_000 - current_tier_discount)
				.saturating_div(10_000);

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
	///
	/// # Parameters
	/// * `old_credits`
	///   - For an update operation, this represents the original requested credits
	///   - For a kill and collect operation, this is the number of credits left in the subscription
	/// * `new_credits`
	///   - For an update operation, this represents the new requested credits
	///   - For a kill operation, this is 0
	///   - For a collect operation, this is the actual number of credits consumed
	///
	/// # Returns
	/// A `DiffBalance` struct containing:
	/// - `balance`: The amount of fees to collect or release
	/// - `direction`: Whether to collect additional fees, release excess fees, or do nothing
	///
	/// # Examples
	/// ```nocompile
	/// // When increasing credits, additional fees are collected:
	/// // Old: 10 credits (1000 fee), New: 50 credits (5000 fee)
	/// let diff = calculate_diff_fees(&10, &50);
	/// assert_eq!(diff.balance, 4000);
	/// assert_eq!(diff.direction, BalanceDirection::Collect);
	///
	/// // When decreasing credits, excess fees are released:
	/// // Old: 100 credits (9550 fee), New: 10 credits (1000 fee)
	/// let diff = calculate_diff_fees(&100, &10);
	/// assert_eq!(diff.balance, 8550);
	/// assert_eq!(diff.direction, BalanceDirection::Release);
	///
	/// // When credits remain the same, no fee changes occur:
	/// // Old: 50 credits (5000 fee), New: 50 credits (5000 fee)
	/// let diff = calculate_diff_fees(&50, &50);
	/// assert_eq!(diff.balance, 0);
	/// assert_eq!(diff.direction, BalanceDirection::None);
	/// ```
	fn calculate_diff_fees(old_credits: &u64, new_credits: &u64) -> DiffBalance<Balances::Balance> {
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
		DiffBalance { balance: fees, direction }
	}

	/// Attempts to collect subscription fees from a subscriber and transfer them to the treasury
	/// account.
	///
	/// This function:
	/// 1. Transfers the specified fees from the subscriber's held balance to the treasury account
	/// 2. Verifies that the full fee amount was successfully collected
	/// 3. Returns the actual amount collected or an appropriate error
	///
	/// # Parameters
	/// * `fees` - The amount of fees to collect
	/// * `sub` - The subscription object containing the subscriber account information
	///
	/// # Returns
	/// - `Ok(collected)` - The amount of fees successfully collected and transferred
	/// - `Err(FeesError)` - If the transfer operation fails
	///
	/// # Notes
	/// - This function uses `transfer_on_hold` which transfers from the subscriber's held balance
	/// - The fees are held under the `HoldReason::Fees` reason code
	/// - The transfer uses `Precision::BestEffort` which allows partial transfers if full amount
	///   isn't available
	/// - Despite using best effort, this function will return an error if less than the requested
	///   amount is collected
	///
	/// # Example
	/// ```nocompile
	/// let fees = 1000u64.into();
	/// let result = FeesManagerImpl::<Treasury, BaseFee, Subscription, Balances>::collect_fees(
	///     &fees,
	///     &subscription
	/// );
	///
	/// match result {
	///     Ok(collected) => println!("Successfully collected {} in fees", collected),
	///     Err(FeesError::NotEnoughBalance { needed, balance }) => {
	///         println!("Insufficient balance: needed {}, had {}", needed, balance);
	///     },
	///     Err(FeesError::Other(err)) => println!("Transfer error: {:?}", err),
	/// }
	/// ```
	fn collect_fees(
		fees: &Balances::Balance,
		sub: &S,
	) -> Result<Balances::Balance, FeesError<Balances::Balance, DispatchError>> {
		// Collect the held fees from the subscriber
		let collected = Balances::transfer_on_hold(
			&HoldReason::Fees.into(),
			sub.subscriber(),
			&T::get(),
			*fees,
			Precision::BestEffort,
			Restriction::Free,
			Fortitude::Polite,
		)
		.map_err(FeesError::Other)?;

		// Ensure the correct credits was collected.
		// TODO: error to bubble up and be handled by caller https://github.com/ideal-lab5/idn-sdk/issues/107
		if collected < *fees {
			return Err(FeesError::NotEnoughBalance { needed: *fees, balance: collected });
		}

		Ok(collected)
	}
}

pub struct DepositCalculatorImpl<SDMultiplier: Get<Deposit>, Deposit> {
	pub _phantom: (PhantomData<SDMultiplier>, PhantomData<Deposit>),
}

impl<
		S: SubscriptionTrait<AccountId32> + Encode,
		SDMultiplier: Get<Deposit>,
		Deposit: Saturating + From<u64> + Ord,
	> pallet_idn_manager::DepositCalculator<Deposit, S>
	for DepositCalculatorImpl<SDMultiplier, Deposit>
{
	/// Calculate the storage deposit required for a subscription.
	///
	/// This function computes the storage deposit amount based on the encoded size of the
	/// subscription object multiplied by a configurable deposit multiplier.
	///
	/// # Parameters
	/// * `sub` - The subscription object for which to calculate the storage deposit
	///
	/// # Returns
	/// The amount of deposit required for the subscription's storage
	///
	/// # Security Considerations
	/// This function handles potential overflow scenarios by:
	/// 1. Converting the encoded size from `usize` to `u64` with fallback to `u64::MAX` if the
	///    conversion fails
	/// 2. Using saturating multiplication to prevent arithmetic overflow
	///
	/// # Example
	/// ```nocompile
	/// let subscription = Subscription {
	///     // subscription details...
	/// };
	///
	/// // If SDMultiplier is 2 and the subscription encodes to 100 bytes:
	/// let deposit = DepositCalculatorImpl::<SDMultiplier, u64>::calculate_storage_deposit(&subscription);
	/// assert_eq!(deposit, 200);
	/// ```
	fn calculate_storage_deposit(sub: &S) -> Deposit {
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
	/// # Parameters
	/// * `old_sub` - The original subscription before changes
	/// * `new_sub` - The updated subscription after changes
	///
	/// # Returns
	/// A `DiffBalance` struct containing:
	/// - `balance`: The absolute difference between the old and new deposit amounts
	/// - `direction`: Whether to collect additional deposit, release excess deposit, or make no
	///   change
	///
	/// # How It Works
	/// 1. Calculates the storage deposit for both the old and new subscription states
	/// 2. Compares the two deposits to determine if more deposit is needed, some can be released,
	///    or no change is required
	/// 3. Returns both the amount and direction of the required deposit adjustment
	///
	/// # Example
	/// ```nocompile
	/// // When subscription size increases (e.g., metadata added):
	/// let old_sub = /* subscription with 100 bytes encoded size */;
	/// let new_sub = /* same subscription with 150 bytes encoded size */;
	///
	/// // If SDMultiplier is 2:
	/// // Old deposit = 2 * 100 = 200
	/// // New deposit = 2 * 150 = 300
	/// let diff = DepositCalculatorImpl::<SDMultiplier, u64>::calculate_diff_deposit(&old_sub, &new_sub);
	/// assert_eq!(diff.balance, 100); // 300 - 200 = 100 more needed
	/// assert_eq!(diff.direction, BalanceDirection::Collect);
	///
	/// // When subscription size decreases (e.g., metadata removed):
	/// let old_sub = /* subscription with 150 bytes encoded size */;
	/// let new_sub = /* same subscription with 100 bytes encoded size */;
	///
	/// // Old deposit = 2 * 150 = 300
	/// // New deposit = 2 * 100 = 200
	/// let diff = DepositCalculatorImpl::<SDMultiplier, u64>::calculate_diff_deposit(&old_sub, &new_sub);
	/// assert_eq!(diff.balance, 100); // 300 - 200 = 100 to release
	/// assert_eq!(diff.direction, BalanceDirection::Release);
	/// ```
	fn calculate_diff_deposit(old_sub: &S, new_sub: &S) -> DiffBalance<Deposit> {
		let old_deposit = Self::calculate_storage_deposit(old_sub);
		let new_deposit = Self::calculate_storage_deposit(new_sub);
		let direction = match new_deposit.cmp(&old_deposit) {
			Ordering::Greater => BalanceDirection::Collect,
			Ordering::Less => BalanceDirection::Release,
			Ordering::Equal => BalanceDirection::None,
		};
		DiffBalance { balance: new_deposit.saturating_sub(old_deposit), direction }
	}
}
