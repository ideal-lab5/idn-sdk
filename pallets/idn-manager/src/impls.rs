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
	self as pallet_idn_manager, traits::FeesError, HoldReason, Subscription, SubscriptionTrait,
};
use codec::Encode;
use frame_support::{
	pallet_prelude::DispatchError,
	traits::{
		tokens::{fungible::hold::Mutate, Fortitude, Precision, Restriction},
		Get,
	},
};
use sp_runtime::{AccountId32, Saturating};
use sp_std::marker::PhantomData;

impl<AccountId, BlockNumber, Metadata> SubscriptionTrait<AccountId>
	for Subscription<AccountId, BlockNumber, Metadata>
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
pub struct FeesManagerImpl<Treasury, BaseFee, Sub, BlockNumber, Balances> {
	pub _phantom: FeesManagerPhantom<Treasury, BaseFee, Sub, BlockNumber, Balances>,
}
type FeesManagerPhantom<Treasury, BaseFee, Sub, BlockNumber, Balances> = (
	PhantomData<Treasury>,
	PhantomData<BaseFee>,
	PhantomData<Sub>,
	PhantomData<BlockNumber>,
	PhantomData<Balances>,
);

impl<
		T: Get<AccountId32>,
		B: Get<Balances::Balance>,
		S: SubscriptionTrait<AccountId32>,
		BlockNumber,
		Balances: Mutate<AccountId32>,
	> pallet_idn_manager::FeesManager<Balances::Balance, BlockNumber, S, DispatchError, AccountId32>
	for FeesManagerImpl<T, B, S, BlockNumber, Balances>
where
	Balances::Balance: From<BlockNumber>,
	Balances::Reason: From<HoldReason>,
{
	fn calculate_subscription_fees(amount: BlockNumber) -> Balances::Balance {
		let base_fee = B::get();
		base_fee.saturating_mul(amount.into())
	}
	fn calculate_refund_fees(
		_init_amount: BlockNumber,
		current_amount: BlockNumber,
	) -> Balances::Balance {
		// in this case of a linear function, the refund's is the same as the fees'
		Self::calculate_subscription_fees(current_amount)
	}
	fn collect_fees(
		fees: Balances::Balance,
		sub: S,
	) -> Result<Balances::Balance, FeesError<Balances::Balance, DispatchError>> {
		// Collect the held fees from the subscriber
		let collected = Balances::transfer_on_hold(
			&HoldReason::Fees.into(),
			sub.subscriber(),
			&T::get(),
			fees,
			Precision::BestEffort,
			Restriction::Free,
			Fortitude::Polite,
		)
		.map_err(FeesError::Other)?;

		// Ensure the correct amount was collected.
		// TODO: error to bubble up and be handled by caller https://github.com/ideal-lab5/idn-sdk/issues/107
		if collected < fees {
			return Err(FeesError::NotEnoughBalance { needed: fees, balance: collected });
		}

		Ok(collected)
	}
}

pub struct DepositCalculatorImpl<SDMultiplier: Get<Balance>, Balance> {
	pub _phantom: (PhantomData<SDMultiplier>, PhantomData<Balance>),
}

impl<
		S: SubscriptionTrait<AccountId32> + Encode,
		SDMultiplier: Get<Balance>,
		Balance: From<u32> + Saturating,
	> pallet_idn_manager::DepositCalculator<Balance, S>
	for DepositCalculatorImpl<SDMultiplier, Balance>
{
	fn calculate_storage_deposit(sub: &S) -> Balance {
		let storage_deposit_multiplier = SDMultiplier::get();
		// calculate the size of scale encoded `sub`
		let encoded_size = sub.encode().len() as u32;
		storage_deposit_multiplier.saturating_mul(encoded_size.into())
	}
}
