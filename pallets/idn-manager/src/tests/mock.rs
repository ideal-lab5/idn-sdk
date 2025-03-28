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

use crate::{
	self as pallet_idn_manager,
	impls::{DepositCalculatorImpl, FeesManagerImpl},
	BalanceOf, SubscriptionOf,
};
use codec::Encode;
use frame_support::{
	construct_runtime, derive_impl, parameter_types, sp_runtime::BuildStorage, traits::Get,
};
use frame_system as system;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{Block as BlockT, IdentityLookup},
	AccountId32,
};

type Block = frame_system::mocking::MockBlock<Test>;

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
	pub const TreasuryAccount: AccountId32 = AccountId32::new([123u8; 32]);
	pub const BaseFee: u64 = 10;
	pub const SDMultiplier: u64 = 10;
	pub const PulseFilterLen: u32 = 100;
	pub const MaxSubscriptions: u32 = 100;
}

#[derive(TypeInfo)]
pub struct SubMetadataLen;

impl Get<u32> for SubMetadataLen {
	fn get() -> u32 {
		8
	}
}

type Rand = [u8; 32];
type Sig = [u8; 64];
type Round = u64;

#[derive(Encode, Clone, Copy)]
pub struct Pulse {
	pub rand: Rand,
	pub round: Round,
	pub sig: Sig,
}

impl idn_traits::pulse::Pulse for Pulse {
	type Rand = Rand;
	type Round = Round;
	type Sig = Sig;

	fn sig(&self) -> Self::Sig {
		self.sig
	}

	fn rand(&self) -> Self::Rand {
		self.rand
	}

	fn round(&self) -> Self::Round {
		self.round
	}

	fn valid(&self) -> bool {
		true
	}
}

impl pallet_idn_manager::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FeesManager = FeesManagerImpl<TreasuryAccount, BaseFee, SubscriptionOf<Test>, Balances>;
	type DepositCalculator = DepositCalculatorImpl<SDMultiplier, u64>;
	type PalletId = PalletId;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Pulse = Pulse;
	type WeightInfo = ();
	type Xcm = ();
	type SubMetadataLen = SubMetadataLen;
	type Credits = u64;
	type PulseFilterLen = PulseFilterLen;
	type MaxSubscriptions = MaxSubscriptions;
}

sp_api::impl_runtime_apis! {
	impl pallet_idn_manager::IdnManagerApi<
			Block,
			BalanceOf<Test>,
			u64,
			AccountId32,
			SubscriptionOf<Test>
		> for Test {
		fn calculate_subscription_fees(credits: u64) -> BalanceOf<Test> {
			pallet_idn_manager::Pallet::<Test>::calculate_subscription_fees(&credits)
		}
		fn get_subscription(
			sub_id: pallet_idn_manager::SubscriptionId
		) -> Option<SubscriptionOf<Test>> {
			pallet_idn_manager::Pallet::<Test>::get_subscription(&sub_id)
		}
		fn get_subscriptions_for_subscriber(
			subscriber: AccountId32
		) -> Vec<SubscriptionOf<Test>> {
			pallet_idn_manager::Pallet::<Test>::get_subscriptions_for_subscriber(&subscriber)
		}
	}

	impl sp_api::Core<Block> for Test {
		fn version() -> sp_version::RuntimeVersion {
			unimplemented!()
		}
		fn execute_block(_: Block) {
			unimplemented!()
		}
		fn initialize_block(_: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			unimplemented!()
		}
	}
}

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build() -> sp_io::TestExternalities {
		let storage = system::GenesisConfig::<Test>::default().build_storage().unwrap();
		let mut ext = sp_io::TestExternalities::new(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
