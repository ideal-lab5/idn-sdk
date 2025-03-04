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
/// Trait for fees managing
pub trait FeesManager<Fees, Amount, Sub: Subscription<S>, Err, S> {
	/// Calculate the fees for a subscription based on the amount of random values required.
	fn calculate_subscription_fees(amount: Amount) -> Fees;
	/// Calculate how much fees should be refunded for a subscription that is being cancelled.
	fn calculate_refund_fees(init_amount: Amount, current_amount: Amount) -> Fees;
	/// Distributes collected fees. Returns the fees that were effectively collected.
	fn collect_fees(fees: Fees, sub: Sub) -> Result<Fees, FeesError<Fees, Err>>;
}

pub trait Subscription<Subscriber> {
	fn subscriber(&self) -> &Subscriber;
}

/// Trait for storage deposit calculation
///
/// This trait is used to calculate the storage deposit required for a subscription based it.
pub trait DepositCalculator<Deposit, Sub> {
	fn calculate_storage_deposit(sub: &Sub) -> Deposit;
}
