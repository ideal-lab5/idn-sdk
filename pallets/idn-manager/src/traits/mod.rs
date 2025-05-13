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

//! # IDN Subscription traits
//!
//! This file defines the traits that enable the IDN Manager pallet to interact with other modules.
//! These traits abstract key functionalities.
//!
//! ## Core Concepts:
//!
//! - **Fees Management:** Governs the calculation, collection, and distribution of fees associated
//!   with subscriptions.
//! - **Subscription Handling:** Provides a standardized way to access subscriber information.
//! - **Storage Deposit Calculation:** Defines the logic for determining and managing storage
//!   deposits required for subscriptions.
//!
//! ## Key Traits:
//!
//! ### [`FeesManager`]
//! Manages the economic aspects of subscriptions, including fee calculation, collection, and
//! distribution.
//!
//! **Methods:**
//!   - [`calculate_subscription_fees`](FeesManager::calculate_subscription_fees): Determines the
//!     initial fee for a subscription based on the requested credits.
//!   - [`calculate_diff_fees`](FeesManager::calculate_diff_fees): Calculates the difference in fees
//!     when a subscription is modified (e.g., credits are added or removed).
//!   - [`collect_fees`](FeesManager::collect_fees): Transfers fees from the subscriber's account to
//!     the designated treasury.
//!   - [`get_consume_credits`](FeesManager::get_consume_credits): Returns the amount of credits
//!     that should be consumed when a subscription receives a pulse.
//!   - [`get_idle_credits`](FeesManager::get_idle_credits): Returns the amount of credits that
//!     should be consumed when a subscription skips receiving a pulse.
//!
//! ### [`Subscription`]
//! Provides an interface for accessing the subscriber associated with a subscription.
//!
//! **Methods:**
//!   - [`subscriber`](Subscription::subscriber): Returns the account ID of the subscriber.
//!
//! ### [`DepositCalculator`]
//! Manages the storage deposits required for subscriptions, ensuring that sufficient funds are
//! reserved to cover the cost of storing subscription data.
//!
//! **Methods:**
//!   - [`calculate_storage_deposit`](DepositCalculator::calculate_storage_deposit): Calculates the
//!     storage deposit required for a given subscription.
//!   - [`calculate_diff_deposit`](DepositCalculator::calculate_diff_deposit): Calculates the
//!     difference in storage deposit between two subscriptions (e.g., when a subscription is
//!     updated).
//!
//! ## Data Structures:
//!
//! ### [`FeesError`]
//! Represents potential errors that can occur during fees management.
//!
//! **Variants:**
//!   - [`NotEnoughBalance`](FeesError::NotEnoughBalance): Indicates that the subscriber's account
//!     has insufficient funds to cover the required fees.
//!   - [`Other`](FeesError::Other): Represents a generic error with an associated context.
//!
//! ### [`BalanceDirection`]
//! Specifies the direction of balance movement (either collecting or releasing funds).
//!
//! **Variants:**
//!   - [`Collect`](BalanceDirection::Collect): Indicates that funds should be collected from the
//!     subscriber.
//!   - [`Release`](BalanceDirection::Release): Indicates that funds should be released to the
//!     subscriber.
//!   - [`None`](BalanceDirection::None): Indicates that no balance movement is required.
//!
//! ### [`DiffBalance`]
//! Represents a change in balance, including the amount and direction of the change.
//!
//! **Fields:**
//!   - [`balance`](DiffBalance::balance): The amount of balance being moved.
//!   - [`direction`](DiffBalance::direction): The direction of the balance movement (using the
//!     `BalanceDirection` enum).

mod example;

/// Error type for fees management
///
/// Context is used to provide more information about uncategorized errors.
pub enum FeesError<Fees, Context> {
	/// Error indicating that the balance is insufficient to cover the required fees.
	///
	/// # Variants
	/// - `needed`: The amount of fees required.
	/// - `balance`: The current balance available.
	NotEnoughBalance {
		needed: Fees,
		balance: Fees,
	},
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

/// This trait represent movement of balance.
///
/// * `balance` - how much balance being moved.
/// * `direction` - if the balance are being collected or released.
pub trait DiffBalance<Balance> {
	fn balance(&self) -> Balance;
	fn direction(&self) -> BalanceDirection;
	fn new(balance: Balance, direction: BalanceDirection) -> Self;
}

/// Trait for fees managing
///
/// This is where the business model logic is specified. This logic is used to calculate, collect
/// and distribute fees for subscriptions.
///
/// These are some examples of how this trait can be implemented:
/// - Linear fee calculator: where the fees are calculated based on a linear function.
#[doc = docify::embed!("./src/traits/example.rs", linear_fee_calculator)]
/// - Tiered fee calculator: where the fees are calculated based on a tiered function.
#[doc = docify::embed!("./src/traits/example.rs", tiered_fee_calculator)]
pub trait FeesManager<Fees, Credits, Sub: Subscription<S>, Err, S, Diff: DiffBalance<Fees>> {
	/// Calculate the fees for a subscription based on the credits of pulses required.
	fn calculate_subscription_fees(credits: &Credits) -> Fees;
	/// Calculate how much fees should be held or release when a subscription changes.
	///
	/// * `old_credits` - the credits of pulses required before the change.
	/// * `new_credits` - the credits of pulses required after the change, this will represent the
	///   updated credits in an update operation. Or the credits actually consumed in a kill
	///   operation.
	fn calculate_diff_fees(old_credits: &Credits, new_credits: &Credits) -> Diff;
	/// Distributes collected fees. Returns the fees that were effectively collected.
	fn collect_fees(fees: &Fees, sub: &Sub) -> Result<Fees, FeesError<Fees, Err>>;
	/// Returns how many credits this subscription pays for receiving a pulse
	fn get_consume_credits(sub: &Sub) -> Credits;
	/// Returns how many credits this subscription pays for skipping to receive a pulse
	fn get_idle_credits(sub: &Sub) -> Credits;
}

/// Trait for accessing subscription information.
pub trait Subscription<Subscriber> {
	/// Returns a reference to the subscriber associated with the subscription.
	fn subscriber(&self) -> &Subscriber;
}

impl Subscription<()> for () {
	fn subscriber(&self) -> &() {
		&()
	}
}

/// Trait for storage deposit calculation
///
/// This trait is used to calculate the storage deposit required for a subscription based it.
pub trait DepositCalculator<Deposit, Sub, Diff: DiffBalance<Deposit>> {
	/// Calculate the storage deposit required for a subscription.
	fn calculate_storage_deposit(sub: &Sub) -> Deposit;
	/// Calculate the difference in storage deposit between two subscriptions.
	///
	/// * `old_sub` - the old subscription.
	/// * `new_sub` - the new subscription.
	fn calculate_diff_deposit(old_sub: &Sub, new_sub: &Sub) -> Diff;
}
