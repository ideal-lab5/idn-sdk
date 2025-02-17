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

//! # Mock runtime
//!
//! This file is a mock runtime for the pallet. It is used to test the pallet in a test environment.
//! It does not contain any tests.

use crate as pallet_idn_manager;
use codec::{Decode, Encode};
use frame_support::{
	construct_runtime, derive_impl, parameter_types, sp_runtime::BuildStorage, traits::Get,
};
use frame_system as system;
use scale_info::TypeInfo;

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
	type AccountData = pallet_balances::AccountData<u64>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub const MaxSubscriptionDuration: u64 = 100;
	pub const PalletId: frame_support::PalletId = frame_support::PalletId(*b"idn_mngr");
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Encode, Decode, TypeInfo, Default)]
pub struct SubMetadataLenWrapper;

impl Get<u32> for SubMetadataLenWrapper {
	fn get() -> u32 {
		8
	}
}

impl pallet_idn_manager::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FeesCalculator = FeesCalculatorImpl;
	type DepositCalculator = DepositCalculatorImpl;
	type PalletId = PalletId;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Rnd = [u8; 32];
	type WeightInfo = ();
	type Xcm = ();
	type SubMetadataLen = SubMetadataLenWrapper;
}

pub struct FeesCalculatorImpl;

impl pallet_idn_manager::FeesCalculator<u64, BlockNumber> for FeesCalculatorImpl {
	fn calculate_subscription_fees(amount: BlockNumber) -> u64 {
		let base_fee = 10u64;
		base_fee.saturating_mul(amount.into())
	}
	fn calculate_refund_fees(_init_amount: BlockNumber, current_amount: BlockNumber) -> u64 {
		// in this case of a liner function, the refund's is the same as the fees'
		Self::calculate_subscription_fees(current_amount)
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
