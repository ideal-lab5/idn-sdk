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

//! # Traits

/// Error type for fees management
///
/// Context is used to provide more information about uncategorized errors.
pub enum FeesError<Fees, Context> {
	NotEnoughBalance { needed: Fees, balance: Fees },
	Other(Context),
}

/// Enum to represent the direction of balance movement.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BalanceDirection {
	Collect,
	Release,
	// Balance isn't going anywhere. This is usually the case when diff is zero.
	None,
}

/// This struct represent movement of balance.
///
/// * `balance` - how much balance being moved.
/// * `direction` - if the balance are being collected or released.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DiffBalance<Balance> {
	pub balance: Balance,
	pub direction: BalanceDirection,
}

/// Trait for fees managing
pub trait FeesManager<Fees, Credits, Sub: Subscription<S>, Err, S> {
	/// Calculate the fees for a subscription based on the credits of random values required.
	fn calculate_subscription_fees(credits: &Credits) -> Fees;
	/// Calculate how much fees should be held or release when a subscription changes.
	///
	/// * `old_credits` - the credits of random values required before the change.
	/// * `new_credits` - the credits of random values required after the change, this will
	///   represent the updated credits in an update operation. Or the credits actually consumed in
	///   a kill operation.
	fn calculate_diff_fees(old_credits: &Credits, new_credits: &Credits) -> DiffBalance<Fees>;
	/// Distributes collected fees. Returns the fees that were effectively collected.
	fn collect_fees(fees: &Fees, sub: &Sub) -> Result<Fees, FeesError<Fees, Err>>;
}

pub trait Subscription<Subscriber> {
	fn subscriber(&self) -> &Subscriber;
}

/// Trait for storage deposit calculation
///
/// This trait is used to calculate the storage deposit required for a subscription based it.
pub trait DepositCalculator<Deposit, Sub> {
	/// Calculate the storage deposit required for a subscription.
	fn calculate_storage_deposit(sub: &Sub) -> Deposit;
	/// Calculate the difference in storage deposit between two subscriptions.
	///
	/// * `old_sub` - the old subscription.
	/// * `new_sub` - the new subscription.
	fn calculate_diff_deposit(old_sub: &Sub, new_sub: &Sub) -> DiffBalance<Deposit>;
}
