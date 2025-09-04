/*
 * Copyright 2025 by Ideal Labs, LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate as pallet_randomness_beacon;
use crate::*;
use bp_idn::types::*;
use frame_support::{
	derive_impl, ord_parameter_types, parameter_types,
	traits::{ConstU16, ConstU8},
};
use frame_system::EnsureRoot;
use sp_idn_crypto::verifier::{QuicknetVerifier, SignatureVerifier};
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::{traits::IdentityLookup, AccountId32, BuildStorage};

type Block = frame_system::mocking::MockBlock<Test>;

#[cfg(feature = "experimental")]
pub type SystemCall = frame_system::Call<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Timelock: pallet_timelock_transactions,
		Preimage: pallet_preimage,
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
		if let Ok(_) = sp_idn_crypto::verifier::QuicknetVerifier::verify(
			pubkey.as_ref().to_vec(),
			self.sig().as_ref().to_vec(),
			self.message().as_ref().to_vec(),
		) {
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
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type SignatureVerifier = QuicknetVerifier;
	type MaxDecryptionsPerBlock = ConstU16<100>;
	type MaxSigsPerBlock = ConstU8<3>;
	type Pulse = MockPulse;
	type Dispatcher = MockDispatcher;
	type FallbackRandomness = MockFallbackRandomness;
	#[cfg(feature = "experimental")]
	type Tlock = Test;
	#[cfg(feature = "experimental")]
	type TlockTxProvider = Timelock;
}

pub struct TestWeightInfo;
impl pallet_timelock_transactions::WeightInfo for TestWeightInfo {
	fn service_task_base() -> Weight {
		Weight::from_parts(0b0000_0100, 0)
	}
	fn service_task_fetched(s: u32) -> Weight {
		Weight::from_parts((s << 8) as u64 + 0b0010_0100, 0)
	}
	fn execute_dispatch_signed() -> Weight {
		Weight::from_parts(0b0100_0000, 0)
	}
	fn schedule_sealed(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn service_agenda(s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
}

parameter_types! {
	pub MaximumSchedulerWeight: Weight = Weight::from_parts(2_000_000_000_000, u64::MAX);
}

ord_parameter_types! {
	pub const One: u64 = 1;
}

impl pallet_timelock_transactions::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EnsureRoot<u64>;
	type MaxScheduledPerBlock = ConstU32<100>;
	type WeightInfo = TestWeightInfo;
	type Preimages = Preimage;
}

impl pallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = ();
	type ManagerOrigin = EnsureRoot<u64>;
	type Consideration = ();
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	let keystore = MemoryKeystore::new();
	ext.register_extension(KeystoreExt::new(keystore.clone()));

	ext
}
