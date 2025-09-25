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
	signature: OpaqueSignature,
	start: u64,
	end: u64,
}

impl From<Accumulation> for MockPulse {
	fn from(acc: Accumulation) -> Self {
		MockPulse { signature: acc.signature, start: 0, end: 100 }
	}
}

impl sp_idn_traits::pulse::Pulse for MockPulse {
	type Rand = [u8; 32];
	type Sig = OpaqueSignature;
	type Pubkey = OpaquePublicKey;
	type RoundNumber = u64;

	fn rand(&self) -> Self::Rand {
		[0u8; 32]
	}

	fn start(&self) -> Self::RoundNumber {
		0
	}

	fn end(&self) -> Self::RoundNumber {
		0
	}

	fn message(&self) -> Self::Sig {
		[0u8; 48]
	}

	fn sig(&self) -> Self::Sig {
		self.signature
	}

	fn authenticate(&self, pubkey: Self::Pubkey) -> bool {
		if sp_idn_crypto::verifier::QuicknetVerifier::verify(
			pubkey.as_ref().to_vec(),
			self.sig().as_ref().to_vec(),
			self.message().as_ref().to_vec(),
		)
		.is_ok()
		{
			return true;
		}

		false
	}
}

pub struct MockDispatcher;
impl sp_idn_traits::pulse::Dispatcher<MockPulse> for MockDispatcher {
	fn dispatch(_pulse: MockPulse) {}

	fn dispatch_weight() -> Weight {
		0.into()
	}
}

pub struct MockFallbackRandomness;
impl frame_support::traits::Randomness<H256, BlockNumberFor<Test>> for MockFallbackRandomness {
	fn random(_subject: &[u8]) -> (H256, BlockNumberFor<Test>) {
		(H256::default(), BlockNumberFor::<Test>::default())
	}
}

impl pallet_randomness_beacon::Config for Test {
	type WeightInfo = ();
	type SignatureVerifier = QuicknetVerifier;
	type MaxSigsPerBlock = ConstU8<3>;
	type Pulse = MockPulse;
	type Dispatcher = MockDispatcher;
	type FallbackRandomness = MockFallbackRandomness;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	let keystore = MemoryKeystore::new();
	ext.register_extension(KeystoreExt::new(keystore.clone()));

	ext
}
