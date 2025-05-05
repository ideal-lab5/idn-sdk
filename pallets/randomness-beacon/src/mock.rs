use crate as pallet_drand_bridge;
use crate::*;
use frame_support::{derive_impl, traits::ConstU8};
use pallet_idn_manager::{
	impls::{DepositCalculatorImpl, DiffBalanceImpl, FeesManagerImpl},
	BalanceOf, SubscriptionOf,
};
use sp_consensus_randomness_beacon::types::RuntimePulse;
use sp_idn_crypto::verifier::QuicknetVerifier;
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::{
	traits::{parameter_types, IdentityLookup},
	AccountId32, BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		IdnManager: pallet_idn_manager,
		Balances: pallet_balances,
		Drand: pallet_drand_bridge,
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

impl pallet_drand_bridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type SignatureVerifier = QuicknetVerifier;
	type MaxSigsPerBlock = ConstU8<10>;
	type MissedBlocksHistoryDepth = ConstU32<{ u8::MAX as u32 }>;
	type Pulse = RuntimePulse;
	type Dispatcher = IdnManager;
}

parameter_types! {
	pub const PalletId: frame_support::PalletId = frame_support::PalletId(*b"idn_mngr");
	pub const TreasuryAccount: AccountId32 = AccountId32::new([123u8; 32]);
	pub const BaseFee: u64 = 10;
	pub const SDMultiplier: u64 = 10;
	pub const MaxPulseFilterLen: u32 = 100;
	pub const MaxSubscriptions: u32 = 100;
	pub const MaxMetadataLen: u32 = 8;
}

impl pallet_idn_manager::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FeesManager = FeesManagerImpl<TreasuryAccount, BaseFee, SubscriptionOf<Test>, Balances>;
	type DepositCalculator = DepositCalculatorImpl<SDMultiplier, u64>;
	type PalletId = PalletId;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Pulse = RuntimePulse;
	type WeightInfo = ();
	type Xcm = ();
	type MaxMetadataLen = MaxMetadataLen;
	type Credits = u64;
	type MaxPulseFilterLen = MaxPulseFilterLen;
	type MaxSubscriptions = MaxSubscriptions;
	type SubscriptionId = [u8; 32];
	type DiffBalance = DiffBalanceImpl<BalanceOf<Test>>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	let keystore = MemoryKeystore::new();
	ext.register_extension(KeystoreExt::new(keystore.clone()));

	ext
}
