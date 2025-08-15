// // This file is part of Substrate.

// // Copyright (C) Parity Technologies (UK) Ltd.
// // SPDX-License-Identifier: Apache-2.0

// // Licensed under the Apache License, Version 2.0 (the "License");
// // you may not use this file except in compliance with the License.
// // You may obtain a copy of the License at
// //
// // 	http://www.apache.org/licenses/LICENSE-2.0
// //
// // Unless required by applicable law or agreed to in writing, software
// // distributed under the License is distributed on an "AS IS" BASIS,
// // WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// // See the License for the specific language governing permissions and
// // limitations under the License.

// //! # Scheduler test environment.

use super::*;

use crate as scheduler;
use frame_support::{
	ord_parameter_types, parameter_types,
	traits::{
		ConstU32, ConstU64, ConstBool,
		Contains, EitherOfDiverse, EqualPrivilegeOnly,
		OnFinalize, OnInitialize, DisabledValidators
	},
	weights::constants::RocksDbWeight,
};
use frame_system::{EnsureRoot, EnsureSignedBy};
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, IdentityLookup},
	BuildStorage, Perbill,
	testing::UintAuthorityId
};
use sp_consensus_aura::{sr25519::AuthorityId, AuthorityIndex};

use ark_bls12_381::{Fr, G2Affine as G2};
// use etf_crypto_primitives::{
// 	proofs::dleq::DLEQProof,
// 	ibe::fullident::BfIbe,
// 	client::etf_client::{DecryptionResult, DefaultEtfClient, EtfClient},
// 	utils::hash_to_g1,
// };

// use pallet_etf::{TimelockError, TimelockEncryptionProvider};
use rand_chacha::{
	ChaCha20Rng,
	rand_core::SeedableRng,
};

use ark_ec::AffineRepr;
use ark_serialize::CanonicalSerialize;
use ark_std::One as Won;
type K = ark_bls12_381::G1Affine;

use pallet_aura;

// Logger module to track execution.
#[frame_support::pallet]
pub mod logger {
	use super::{OriginCaller, OriginTrait};
	use frame_support::{pallet_prelude::*, parameter_types};
	use frame_system::pallet_prelude::*;

	parameter_types! {
		static Log: Vec<(OriginCaller, u32)> = Vec::new();
	}
	pub fn log() -> Vec<(OriginCaller, u32)> {
		Log::get().clone()
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Logged(u32, Weight),
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		<T as frame_system::Config>::RuntimeOrigin: OriginTrait<PalletsOrigin = OriginCaller>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(*weight)]
		pub fn log(origin: OriginFor<T>, i: u32, weight: Weight) -> DispatchResult {
			Self::deposit_event(Event::Logged(i, weight));
			Log::mutate(|log| {
				log.push((origin.caller().clone(), i));
			});
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(*weight)]
		pub fn log_without_filter(origin: OriginFor<T>, i: u32, weight: Weight) -> DispatchResult {
			Self::deposit_event(Event::Logged(i, weight));
			Log::mutate(|log| {
				log.push((origin.caller().clone(), i));
			});
			Ok(())
		}
	}
}

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Logger: logger::{Pallet, Call, Event<T>},
		Scheduler: scheduler::{Pallet, Call, Storage, Event<T>},
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>, HoldReason},
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Aura: pallet_aura,
		// Etf: pallet_etf,
	}
);

// Scheduler must dispatch with root and no filter, this tests base filter is indeed not used.
pub struct BaseFilter;
impl Contains<RuntimeCall> for BaseFilter {
	fn contains(call: &RuntimeCall) -> bool {
		!matches!(call, RuntimeCall::Logger(LoggerCall::log { .. }))
	}
}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(
			Weight::from_parts(2_000_000_000_000, u64::MAX),
		);
}

pub struct MockWeights;
impl frame_system::ExtensionsWeightInfo for MockWeights {
	fn check_genesis() -> Weight {
		Weight::from_parts(10, 0)
	}

	fn check_mortality_mortal_transaction() -> Weight {
		Weight::from_parts(10, 0)
	}

	fn check_mortality_immortal_transaction() -> Weight {
		Weight::from_parts(10, 0)
	}

	fn check_non_zero_sender() -> Weight {
		Weight::from_parts(10, 0)
	}

	fn check_nonce() -> Weight {
		Weight::from_parts(10, 0)
	}

	fn check_spec_version() -> Weight {
		Weight::from_parts(10, 0)
	}

	fn check_tx_version() -> Weight {
		Weight::from_parts(10, 0)
	}

	fn check_weight() -> Weight {
		Weight::from_parts(10, 0)
	}

	fn weight_reclaim() -> Weight {
		Weight::from_parts(10, 0)
	}
}

impl system::Config for Test {
	type ExtensionsWeightInfo = MockWeights;
	type BaseCallFilter = BaseFilter;
	type BlockWeights = BlockWeights;
	type BlockLength = ();
	type DbWeight = RocksDbWeight;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
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
	type SS58Prefix = ();
	type OnSetCode = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
	type RuntimeTask = RuntimeTask;
	type MaxConsumers = ConstU32<16>;
}

impl logger::Config for Test {
	type RuntimeEvent = RuntimeEvent;
}
ord_parameter_types! {
	pub const One: u64 = 1;
}

impl pallet_preimage::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = ();
	type Currency = ();
	type ManagerOrigin = EnsureRoot<u64>;
	type Consideration = ();
}

impl pallet_timestamp::Config for Test {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<{ 6000 / 2 }>;
	type WeightInfo = ();
}

parameter_types! {
	static DisabledValidatorTestValue: Vec<AuthorityIndex> = Default::default();
	pub static AllowMultipleBlocksPerSlot: bool = false;
}

pub struct MockDisabledValidators;

impl MockDisabledValidators {
	pub fn disable_validator(index: AuthorityIndex) {
		DisabledValidatorTestValue::mutate(|v| {
			if let Err(i) = v.binary_search(&index) {
				v.insert(i, index);
			}
		})
	}
}

impl DisabledValidators for MockDisabledValidators {
	fn is_disabled(index: AuthorityIndex) -> bool {
		DisabledValidatorTestValue::get().binary_search(&index).is_ok()
	}

	fn disabled_validators() -> Vec<u32> {
		DisabledValidatorTestValue::get()
	}
}


const SLOT_DURATION: u64 = 2;

impl pallet_aura::Config for Test {
	type AuthorityId = AuthorityId;
	type DisabledValidators = MockDisabledValidators;
	type MaxAuthorities = ConstU32<10>;
	type AllowMultipleBlocksPerSlot = AllowMultipleBlocksPerSlot;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

// fn build_ext(authorities: Vec<u64>) -> sp_io::TestExternalities {
// 	let mut storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
// 	pallet_aura::GenesisConfig::<Test> {
// 		authorities: authorities.into_iter().map(|a| UintAuthorityId(a).to_public_key()).collect(),
// 	}
// 	.assimilate_storage(&mut storage)
// 	.unwrap();
// 	storage.into()
// }

// pub fn build_ext_and_execute_test(authorities: Vec<u64>, test: impl FnOnce() -> ()) {
// 	let mut ext = build_ext(authorities);
// 	ext.execute_with(|| {
// 		test();
// 		Aura::do_try_state().expect("Storage invariants should hold")
// 	});
// }

impl pallet_insecure_randomness_collective_flip::Config for Test {}

// impl pallet_etf::Config for Test {
// 	type RuntimeEvent = RuntimeEvent;
// 	type WeightInfo = pallet_etf::weights::SubstrateWeightInfo<Test>;
// 	type Randomness = RandomnessCollectiveFlip;
// 	type SlotSecretProvider = Aura;
// }

pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
	fn service_agendas_base() -> Weight {
		Weight::from_parts(0b0000_0001, 0)
	}
	fn service_agenda_base(i: u32) -> Weight {
		Weight::from_parts((i << 8) as u64 + 0b0000_0010, 0)
	}
	fn service_task_base() -> Weight {
		Weight::from_parts(0b0000_0100, 0)
	}
	fn service_task_periodic() -> Weight {
		Weight::from_parts(0b0000_1100, 0)
	}
	fn service_task_named() -> Weight {
		Weight::from_parts(0b0001_0100, 0)
	}
	fn service_task_fetched(s: u32) -> Weight {
		Weight::from_parts((s << 8) as u64 + 0b0010_0100, 0)
	}
	fn execute_dispatch_signed() -> Weight {
		Weight::from_parts(0b0100_0000, 0)
	}
	fn execute_dispatch_unsigned() -> Weight {
		Weight::from_parts(0b1000_0000, 0)
	}
	fn schedule(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn cancel(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn schedule_named(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn cancel_named(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn schedule_sealed(s: u32, ) -> Weight {
		Weight::from_parts(50, 0)
	}
}
parameter_types! {
	pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) *
		BlockWeights::get().max_block;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumSchedulerWeight;
	type ScheduleOrigin = EitherOfDiverse<EnsureRoot<u64>, EnsureSignedBy<One, u64>>;
	type MaxScheduledPerBlock = ConstU32<10>;
	type WeightInfo = TestWeightInfo;
	// type OriginPrivilegeCmp = EqualPrivilegeOnly;
	type Preimages = Preimage;
	// type TlockProvider = MockTlockProvider;
}

pub type LoggerCall = logger::Call<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = system::GenesisConfig::<Test>::default().build_storage().unwrap();
	t.into()
}

pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		Scheduler::on_finalize(System::block_number());
		System::set_block_number(System::block_number() + 1);
		Scheduler::on_initialize(System::block_number());
	}
}

pub fn convert_to_bytes<E: CanonicalSerialize, const N: usize>(k: E) -> [u8;N] {
	let mut out = Vec::with_capacity(k.compressed_size());
	k.serialize_compressed(&mut out).unwrap_or(());
	let o: [u8; N] = out.try_into().unwrap_or([0;N]);
	o
}

pub fn root() -> OriginCaller {
	system::RawOrigin::Root.into()
}

// pub struct MockSlotSecretProvider;


// impl pallet_aura::SlotSecretProvider for MockSlotSecretProvider {
// 	fn get() -> Option<OpaqueSecret> {
// 		let sk = Fr::one();
// 		let id = 4u64.to_string().as_bytes().to_vec();
// 		let pk = hash_to_g1(&id);
// 		let generator: K = K::generator();
// 		let mut rng = ChaCha20Rng::seed_from_u64(4u64);
// 		let proof = DLEQProof::new(sk, pk, generator, id, &mut rng);
// 		let sk = convert_to_bytes::<K, 48>(proof.secret_commitment_g)
// 			.try_into()
// 			.expect("The slot secret should be valid; qed;");
// 		Some(sk.to_vec())
// 	}
// }

// pub struct MockTlockProvider;

// impl TimelockEncryptionProvider for MockTlockProvider {
// 	// decrypts at block number 4
// 	fn decrypt_current(ciphertext: Ciphertext) -> Result<DecryptionResult, TimelockError> {
// 		let sk = Fr::one();
// 		let id = 4u64.to_string().as_bytes().to_vec();
// 		let pk = hash_to_g1(&id);
// 		let generator: K = K::generator();
// 		let mut rng = ChaCha20Rng::seed_from_u64(4u64);
// 		let proof = DLEQProof::new(sk, pk, generator, id, &mut rng);
// 		let sk: [u8;48] = convert_to_bytes::<K, 48>(proof.secret_commitment_g)
// 			.try_into()
// 			.expect("The slot secret should be valid; qed;");

// 		let ibe_pp_bytes: [u8;96] = convert_to_bytes::<G2, 96>(G2::generator())
// 			.try_into()
// 			.expect("The slot secret should be valid; qed;");

// 		let pt = DefaultEtfClient::<BfIbe>::decrypt(
// 			ibe_pp_bytes.to_vec(),
// 			ciphertext.ciphertext.to_vec(),
// 			ciphertext.nonce.to_vec(),
// 			vec![ciphertext.capsule.to_vec()],
// 			vec![sk.to_vec()],
// 		).map_err(|_| TimelockError::DecryptionFailed)?;
// 		Ok(pt)
// 	}
// }