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

impl<AccountId, BlockNumber: Unsigned, Metadata> SubscriptionTrait<AccountId>
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
		BlockNumber: Saturating + Ord + Clone,
		Balances: Mutate<AccountId32>,
	> pallet_idn_manager::FeesManager<Balances::Balance, BlockNumber, S, DispatchError, AccountId32>
	for FeesManagerImpl<T, B, S, BlockNumber, Balances>
where
	Balances::Balance: From<BlockNumber>,
	Balances::Reason: From<HoldReason>,
{
	fn calculate_subscription_fees(amount: &BlockNumber) -> Balances::Balance {
		let base_fee = B::get();
		base_fee.saturating_mul(amount.clone().into())
	}
	fn calculate_diff_fees(
		old_amount: &BlockNumber,
		new_amount: &BlockNumber,
	) -> DiffBalance<Balances::Balance> {
		let mut direction = BalanceDirection::None;
		let fees = match new_amount.cmp(&old_amount) {
			Ordering::Greater => {
				direction = BalanceDirection::Hold;
				Self::calculate_subscription_fees(
					&new_amount.clone().saturating_sub(old_amount.clone()),
				)
			},
			Ordering::Less => {
				direction = BalanceDirection::Release;
				Self::calculate_subscription_fees(
					&old_amount.clone().saturating_sub(new_amount.clone()),
				)
			},
			Ordering::Equal => Zero::zero(),
		};
		DiffBalance { balance: fees, direction }
	}
	fn collect_fees(
		fees: &Balances::Balance,
		sub: &S,
	) -> Result<Balances::Balance, FeesError<Balances::Balance, DispatchError>> {
		// Collect the held fees from the subscriber
		let collected = Balances::transfer_on_hold(
			&HoldReason::Fees.into(),
			sub.subscriber(),
			&T::get(),
			fees.clone(),
			Precision::BestEffort,
			Restriction::Free,
			Fortitude::Polite,
		)
		.map_err(FeesError::Other)?;

		// Ensure the correct amount was collected.
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
		Deposit: From<u32> + Saturating + Ord,
	> pallet_idn_manager::DepositCalculator<Deposit, S>
	for DepositCalculatorImpl<SDMultiplier, Deposit>
{
	fn calculate_storage_deposit(sub: &S) -> Deposit {
		let storage_deposit_multiplier = SDMultiplier::get();
		// calculate the size of scale encoded `sub`
		let encoded_size = sub.encode().len() as u32;
		storage_deposit_multiplier.saturating_mul(encoded_size.into())
	}

	fn calculate_diff_deposit(old_sub: &S, new_sub: &S) -> DiffBalance<Deposit> {
		let old_deposit = Self::calculate_storage_deposit(old_sub);
		let new_deposit = Self::calculate_storage_deposit(new_sub);
		let direction = match new_deposit.cmp(&old_deposit) {
			Ordering::Greater => BalanceDirection::Hold,
			Ordering::Less => BalanceDirection::Release,
			Ordering::Equal => BalanceDirection::None,
		};
		DiffBalance { balance: new_deposit.saturating_sub(old_deposit), direction }
	}
}
