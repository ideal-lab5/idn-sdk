use crate as pallet_randomness_beacon;
use crate::*;
use frame_support::{derive_impl, traits::ConstU8};
use sp_consensus_randomness_beacon::types::RuntimePulse;
use sp_idn_crypto::verifier::QuicknetVerifier;
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::{traits::IdentityLookup, AccountId32, BuildStorage};

type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Drand: pallet_randomness_beacon,
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

pub struct MockDispatcher;
impl sp_idn_traits::pulse::Dispatcher<RuntimePulse, Result<(), sp_runtime::DispatchError>>
	for MockDispatcher
{
	fn dispatch(_pulses: Vec<RuntimePulse>) -> Result<(), sp_runtime::DispatchError> {
		Ok(())
	}

	fn dispatch_weight(_pulses: usize) -> Weight {
		0.into()
	}
}

impl pallet_drand_bridge::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type SignatureVerifier = QuicknetVerifier;
	type MaxSigsPerBlock = ConstU8<10>;
	type Pulse = RuntimePulse;
	type Dispatcher = MockDispatcher;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	let keystore = MemoryKeystore::new();
	ext.register_extension(KeystoreExt::new(keystore.clone()));

	ext
}
