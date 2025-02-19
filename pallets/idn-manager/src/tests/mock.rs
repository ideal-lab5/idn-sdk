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

//! # Mock runtime
//!
//! This file is a mock runtime for the pallet. It is used to test the pallet in a test environment.
//! It does not contain any tests.

use crate::{self as pallet_idn_manager, traits::FeesError, HoldReason, SubscriptionOf};
use codec::{Decode, Encode};
use frame_support::{
	construct_runtime, derive_impl,
	pallet_prelude::DispatchError,
	parameter_types,
	sp_runtime::BuildStorage,
	traits::{
		fungible::MutateHold,
		tokens::{Fortitude, Precision, Restriction},
		Get,
	},
};
use frame_system as system;
use scale_info::TypeInfo;
use sp_runtime::{traits::IdentityLookup, AccountId32};
use sp_std::marker::PhantomData;

type Block = frame_system::mocking::MockBlock<Test>;
type BlockNumber = frame_system::pallet_prelude::BlockNumberFor<Test>;

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		IdnManager: pallet_idn_manager,
		Balances: pallet_balances,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub const MaxSubscriptionDuration: u64 = 100;
	pub const PalletId: frame_support::PalletId = frame_support::PalletId(*b"idn_mngr");
	pub const TreasuryAccount: AccountId32 = AccountId32::new([1u8; 32]);
	pub const BaseFee: u64 = 10;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, Default)]
pub struct SubMetadataLen;

impl Get<u32> for SubMetadataLen {
	fn get() -> u32 {
		8
	}
}

impl pallet_idn_manager::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FeesManager = FeesManagerImpl<TreasuryAccount, BaseFee>;
	type DepositCalculator = DepositCalculatorImpl;
	type PalletId = PalletId;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Rnd = [u8; 32];
	type WeightInfo = ();
	type Xcm = ();
	type SubMetadataLen = SubMetadataLen;
}

/// A FeesManager implementation that holds a dynamic treasury account.
pub struct FeesManagerImpl<Treasury: Get<AccountId32>, BaseFee: Get<u64>> {
	pub _phantom: (PhantomData<Treasury>, PhantomData<BaseFee>),
}

impl<T: Get<AccountId32>, B: Get<u64>>
	pallet_idn_manager::FeesManager<u64, BlockNumber, SubscriptionOf<Test>, DispatchError>
	for FeesManagerImpl<T, B>
{
	fn calculate_subscription_fees(amount: BlockNumber) -> u64 {
		let base_fee = B::get();
		base_fee.saturating_mul(amount.into())
	}
	fn calculate_refund_fees(_init_amount: BlockNumber, current_amount: BlockNumber) -> u64 {
		// in this case of a liner function, the refund's is the same as the fees'
		Self::calculate_subscription_fees(current_amount)
	}
	fn collect_fees(
		fees: u64,
		sub: SubscriptionOf<Test>,
	) -> Result<u64, FeesError<u64, DispatchError>> {
		// Collect the held fees from the subscriber
		let collected = Balances::transfer_on_hold(
			&HoldReason::Fees.into(),
			&sub.details.subscriber,
			&T::get(),
			fees,
			Precision::BestEffort,
			Restriction::Free,
			Fortitude::Polite,
		)
		.map_err(|e| FeesError::Other(e))?;

		// Ensure the correct amount was collected.
		if collected < fees {
			return Err(FeesError::NotEnoughBalance { needed: fees, balance: collected });
		}

		Ok(collected)
	}
}

pub struct DepositCalculatorImpl;

impl pallet_idn_manager::DepositCalculator<u64, pallet_idn_manager::SubscriptionOf<Test>>
	for DepositCalculatorImpl
{
	fn calculate_storage_deposit(sub: &pallet_idn_manager::SubscriptionOf<Test>) -> u64 {
		let storage_deposit_multiplier = 10;
		// calculate the size of scale encoded `sub`
		let encoded_size = sub.encode().len() as u64;
		encoded_size.saturating_mul(storage_deposit_multiplier)
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = system::GenesisConfig::<Test>::default().build_storage().unwrap();
	t.into()
}
