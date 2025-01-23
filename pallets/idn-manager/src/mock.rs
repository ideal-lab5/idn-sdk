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

/// Calculate the subscription fee for a given duration, applying duration-based discounts
fn calculate_subscription_fee(duration: BlockNumberFor<T>) -> BalanceOf<T> {
	let base_fee = T::BaseSubscriptionFeePerBlock::get();
	let duration_num: u32 = duration.saturated_into();

	// Calculate discount percentage (in basis points)
	// The longer the duration, the higher the discount
	let discount = duration_num.saturating_mul(T::DiscountRatePerBlock::get());

	// Convert basis points to a multiplier (10000 basis points = 100%)
	let discount_multiplier = (10000u32.saturating_sub(discount)) as u128;

	// Apply discount to base fee
	let discounted_fee_per_block = base_fee
		.saturating_mul(discount_multiplier.into())
		.saturating_div(10000u32.into());

	// Calculate total fee for the duration
	discounted_fee_per_block.saturating_mul(duration.saturated_into())
}
