// // Copyright (C) Parity Technologies (UK) Ltd.
// // This file is part of Polkadot.

// // Polkadot is free software: you can redistribute it and/or modify
// // it under the terms of the GNU General Public License as published by
// // the Free Software Foundation, either version 3 of the License, or
// // (at your option) any later version.

// // Polkadot is distributed in the hope that it will be useful,
// // but WITHOUT ANY WARRANTY; without even the implied warranty of
// // MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// // GNU General Public License for more details.

// // You should have received a copy of the GNU General Public License
// // along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use frame_support::{construct_runtime, derive_impl, parameter_types, traits::Everything};
use sp_core::H256;
use sp_runtime::{traits::IdentityLookup, AccountId32, BuildStorage};

use crate::bls12::Bls12_381;

pub type AccountId = AccountId32;
pub type Balance = u128;
type Block = frame_system::mocking::MockBlock<Test>;

parameter_types! {
	pub const MinimumPeriod: u64 = 1;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = ();
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Timestamp: pallet_timestamp,
		Revive: pallet_revive,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = ::sp_runtime::traits::BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockWeights = ();
	type BlockLength = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = Everything;
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub ExistentialDeposit: Balance = 1;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
}

#[derive_impl(pallet_revive::config_preludes::TestDefaultConfig)]
impl pallet_revive::Config for Test {
	type AddressMapper = pallet_revive::AccountId32Mapper<Self>;
	type Currency = Balances;
	type Precompiles = (Bls12_381<Self>,);
	type Time = Timestamp;
	type UploadOrigin = frame_system::EnsureSigned<AccountId>;
	type InstantiateOrigin = frame_system::EnsureSigned<AccountId>;
}

pub const ALICE: AccountId32 = AccountId::new([0u8; 32]);

pub(crate) fn new_test_ext_with_balances(
	balances: Vec<(AccountId, Balance)>,
) -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_balances::GenesisConfig::<Test> { balances, ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	pallet_revive::GenesisConfig::<Test> { mapped_accounts: vec![ALICE], ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
