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

use std::collections::BTreeMap;

use crate as pallet_randomness_beacon;
use crate::*;
use frame_support::{derive_impl, parameter_types, traits::ConstU8};
use pallet_session::{SessionHandler, ShouldEndSession};
use sp_idn_crypto::verifier::{QuicknetVerifier, SignatureVerifier};
use sp_keystore::{testing::MemoryKeystore, KeystoreExt};
use sp_runtime::{impl_opaque_keys, key_types::DUMMY, testing::UintAuthorityId, traits::{ConvertInto, IdentityLookup, OpaqueKeys}, AccountId32, BuildStorage, MultiSignature, MultiSigner, RuntimeAppPublic};

type Block = frame_system::mocking::MockBlock<Test>;
pub const ALICE: AccountId32 = AccountId32::new([1u8; 32]);

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Drand: pallet_randomness_beacon,
		Session: pallet_session
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

pub struct MockFindAuthor;
impl FindAuthor<AccountId32> for MockFindAuthor {
	fn find_author<'a, I>(_digests: I) -> Option<AccountId32>
	where
		I: 'a + IntoIterator<Item = (frame_support::ConsensusEngineId, &'a [u8])>,
	{
		Some([1;32].into())
	}
}

pub struct TestSessionManager;
impl pallet_session::SessionManager<AccountId32> for TestSessionManager {
	fn end_session(_: u32) {}
	fn start_session(_: u32) {}
	fn new_session(idx: u32) -> Option<Vec<AccountId32>> {
		if idx == 0 || idx == 1 {
			Some(vec![[1;32].into(), [2;32].into()])
		} else if idx == 2 {
			Some(vec![[3;32].into(), [4;32].into()])
		} else {
			None
		}
	}
}

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: UintAuthorityId,
	}
}

impl From<UintAuthorityId> for MockSessionKeys {
	fn from(dummy: UintAuthorityId) -> Self {
		Self { dummy }
	}
}

parameter_types! {
	pub static Validators: Vec<u64> = vec![1, 2, 3];
	pub static NextValidators: Vec<u64> = vec![1, 2, 3];
	pub static Authorities: Vec<UintAuthorityId> =
		vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)];
	pub static ForceSessionEnd: bool = false;
	pub static SessionLength: u64 = 2;
	pub static SessionChanged: bool = false;
	pub static TestSessionChanged: bool = false;
	pub static Disabled: bool = false;
	// Stores if `on_before_session_end` was called
	pub static BeforeSessionEndCalled: bool = false;
	pub static ValidatorAccounts: BTreeMap<u64, u64> = BTreeMap::new();
	pub static KeyDeposit: u64 = 10;
}

pub struct TestSessionHandler;
impl SessionHandler<AccountId32> for TestSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[UintAuthorityId::ID];
	fn on_genesis_session<T: OpaqueKeys>(_validators: &[(AccountId32, T)]) {}
	fn on_new_session<T: OpaqueKeys>(
		changed: bool,
		validators: &[(AccountId32, T)],
		_queued_validators: &[(AccountId32, T)],
	) {
		SessionChanged::mutate(|l| *l = changed);
		Authorities::mutate(|l| {
			*l = validators
				.iter()
				.map(|(_, id)| id.get::<UintAuthorityId>(DUMMY).unwrap_or_default())
				.collect()
		});
	}
	fn on_disabled(_validator_index: u32) {
		Disabled::mutate(|l| *l = true)
	}
	fn on_before_session_ending() {
		BeforeSessionEndCalled::mutate(|b| *b = true);
	}
}

pub struct TestShouldEndSession;
impl ShouldEndSession<u64> for TestShouldEndSession {
	fn should_end_session(now: u64) -> bool {
		let l = SessionLength::get();
		now % l == 0 ||
			ForceSessionEnd::mutate(|l| {
				let r = *l;
				*l = false;
				r
			})
	}
}

impl pallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId32;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = TestShouldEndSession;
	type NextSessionRotation = ();
	type SessionManager = TestSessionManager;
	type SessionHandler = TestSessionHandler;
	type Keys = MockSessionKeys;
	type DisablingStrategy = ();
	type WeightInfo = ();
}

type Signature = MultiSignature;

impl pallet_randomness_beacon::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type SignatureVerifier = QuicknetVerifier;
	type MaxSigsPerBlock = ConstU8<3>;
	type Pulse = MockPulse;
	type Dispatcher = MockDispatcher;
	type FallbackRandomness = MockFallbackRandomness;
	type Signature = Signature;
	type AccountIdentifier = MultiSigner;
	type FindAuthor = MockFindAuthor;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	let keystore = MemoryKeystore::new();
	ext.register_extension(KeystoreExt::new(keystore.clone()));

	ext
}
