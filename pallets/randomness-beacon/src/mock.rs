use crate as pallet_randomness_beacon;
use crate::*;
use bp_idn::types::*;
use frame_support::{derive_impl, traits::ConstU8};
use sp_idn_crypto::verifier::{QuicknetVerifier, SignatureVerifier};
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

#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, DecodeWithMemTracking)]
pub struct MockPulse {
	message: OpaqueSignature,
	signature: OpaqueSignature,
}

impl From<Accumulation> for MockPulse {
	fn from(acc: Accumulation) -> Self {
		MockPulse { signature: acc.signature, message: acc.message_hash }
	}
}

impl sp_idn_traits::pulse::Pulse for MockPulse {
	type Rand = [u8; 32];
	type Sig = OpaqueSignature;
	type Pubkey = OpaquePublicKey;

	fn rand(&self) -> Self::Rand {
		[0u8; 32]
	}

	fn message(&self) -> Self::Sig {
		self.message
	}

	fn sig(&self) -> Self::Sig {
		self.signature
	}

	fn authenticate(&self, pubkey: Self::Pubkey) -> bool {
		if let Ok(_) = sp_idn_crypto::verifier::QuicknetVerifier::verify(
			pubkey.as_ref().to_vec(),
			self.sig().as_ref().to_vec(),
			self.message().as_ref().to_vec(),
			None,
		) {
			return true;
		}

		false
	}
}

pub struct MockDispatcher;
impl sp_idn_traits::pulse::Dispatcher<MockPulse> for MockDispatcher {
	fn dispatch(_pulse: MockPulse) { }

	fn dispatch_weight() -> Weight {
		0.into()
	}
}

impl pallet_randomness_beacon::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type SignatureVerifier = QuicknetVerifier;
	type MaxSigsPerBlock = ConstU8<10>;
	type Pulse = MockPulse;
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
