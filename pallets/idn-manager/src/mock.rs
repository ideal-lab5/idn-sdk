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

use crate as pallet_idn_manager;
use frame_support::{construct_runtime, derive_impl, parameter_types, sp_runtime::BuildStorage};
use frame_system as system;

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
	// pub const BaseSubscriptionFeePerBlock: u64 = 10;
	pub const PalletId: frame_support::PalletId = frame_support::PalletId(*b"idn_mngr");
}

impl pallet_idn_manager::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type MaxSubscriptionDuration = MaxSubscriptionDuration;
	type FeesCalculator = FeesCalculatorImpl;
	type PalletId = PalletId;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Rnd = ();
	type WeightInfo = ();
	type Xcm = ();
}

pub struct FeesCalculatorImpl;

impl pallet_idn_manager::FeesCalculator<u64, BlockNumber> for FeesCalculatorImpl {
	fn calculate_subscription_fees(duration: BlockNumber) -> u64 {
		let base_fee = 10u64;
		base_fee.saturating_mul(duration.into())
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = system::GenesisConfig::<Test>::default().build_storage().unwrap();
	t.into()
}
