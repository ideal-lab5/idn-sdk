use crate as pallet_drand_bridge;
use crate::*;
use frame_support::{
	derive_impl,
	traits::{ConstU16, ConstU64, ConstU8},
};
use sp_consensus_randomness_beacon::types::OpaquePulse;
use sp_idn_crypto::verifier::QuicknetVerifier;
use sp_core::{sr25519::Signature, H256, parameter_types};
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup, Verify},
	BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Drand: pallet_drand_bridge,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::DefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = sp_core::sr25519::Public;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl pallet_drand_bridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type SignatureVerifier = QuicknetVerifier;
	type MaxSigsPerBlock = ConstU8<10>;
	type MissedBlocksHistoryDepth = ConstU32<{ u8::MAX as u32 }>;
	type Pulse = OpaquePulse;
	type Dispatcher = IdnManager;
}


parameter_types! {
	pub const MaxSubscriptionDuration: u64 = 100;
	pub const PalletId: frame_support::PalletId = frame_support::PalletId(*b"idn_mngr");
	pub const TreasuryAccount: AccountId32 = AccountId32::new([123u8; 32]);
	pub const BaseFee: u64 = 10;
	pub const SDMultiplier: u64 = 10;
	pub const MaxPulseFilterLen: u32 = 100;
	pub const MaxSubscriptions: u32 = 1_000_000;
}

#[derive(TypeInfo)]
pub struct MaxMetadataLen;

impl Get<u32> for MaxMetadataLen {
	fn get() -> u32 {
		8
	}
}

impl pallet_idn_manager::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FeesManager = FeesManagerImpl<TreasuryAccount, BaseFee, SubscriptionOf<Runtime>, Balances>;
	type DepositCalculator = DepositCalculatorImpl<SDMultiplier, u64>;
	type PalletId = PalletId;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Pulse = sp_consensus_randomness_beacon::types::OpaquePulse;
	type WeightInfo = ();
	type Xcm = ();
	type MaxMetadataLen = MaxMetadataLen;
	type Credits = u64;
	type MaxPulseFilterLen = MaxPulseFilterLen;
	type MaxSubscriptions = MaxSubscriptions;
	type SubscriptionId = [u8; 32];
	type DiffBalance = DiffBalanceImpl<BalanceOf<Runtime>>;
}

parameter_types! {
	pub const MaxSubscriptionDuration: u64 = 100;
	pub const PalletId: frame_support::PalletId = frame_support::PalletId(*b"idn_mngr");
	pub const TreasuryAccount: AccountId32 = AccountId32::new([123u8; 32]);
	pub const BaseFee: u64 = 10;
	pub const SDMultiplier: u64 = 10;
	pub const MaxPulseFilterLen: u32 = 100;
	pub const MaxSubscriptions: u32 = 1_000_000;
}

#[derive(TypeInfo)]
pub struct MaxMetadataLen;

impl Get<u32> for MaxMetadataLen {
	fn get() -> u32 {
		8
	}
}

impl pallet_idn_manager::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FeesManager = FeesManagerImpl<TreasuryAccount, BaseFee, SubscriptionOf<Runtime>, Balances>;
	type DepositCalculator = DepositCalculatorImpl<SDMultiplier, u64>;
	type PalletId = PalletId;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Pulse = OpaquePulse;
	type WeightInfo = ();
	type Xcm = ();
	type MaxMetadataLen = MaxMetadataLen;
	type Credits = u64;
	type MaxPulseFilterLen = MaxPulseFilterLen;
	type MaxSubscriptions = MaxSubscriptions;
	type SubscriptionId = [u8; 32];
	type DiffBalance = DiffBalanceImpl<BalanceOf<Runtime>>;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	let keystore = MemoryKeystore::new();
	ext.register_extension(KeystoreExt::new(keystore.clone()));

	ext
}
