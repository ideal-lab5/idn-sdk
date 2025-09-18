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

#[path = "xcm.rs"]
pub(crate) mod xcm_config;

mod contracts;

// Substrate and Polkadot dependencies
use bp_idn::{
	impls,
	types::{self},
};
use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::{
	derive_impl,
	dispatch::DispatchClass,
	parameter_types,
	traits::{ConstBool, ConstU32, ConstU64, ConstU8, EitherOfDiverse, VariantCountOf},
	weights::{ConstantMultiplier, Weight},
	PalletId,
};
use frame_system::{
	limits::{BlockLength, BlockWeights},
	EnsureRoot,
};
#[cfg(not(feature = "runtime-benchmarks"))]
use pallet_idn_manager::primitives::AllowSiblingsOnly;
use pallet_idn_manager::{BalanceOf, SubscriptionOf};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use polkadot_runtime_common::{BlockHashCount, SlowAdjustingFeeUpdate};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_version::RuntimeVersion;
use xcm::prelude::BodyId;

// Local module imports
#[cfg(not(feature = "runtime-benchmarks"))]
use super::PolkadotXcm;
use super::{
	weights::{
		BalancesWeightInfo, BlockExecutionWeight, CollatorSelectionWeightInfo,
		CumulusParachainSystemWeightInfo, ExtrinsicBaseWeight, IdnManagerWeightInfo,
		RandomnessBeaconWeightInfo, RocksDbWeight, SessionWeightInfo, SudoWeightInfo,
		SystemWeightInfo, TimestampWeightInfo, TransactionPaymentWeightInfo,
	},
	AccountId, Aura, Balance, Balances, Block, BlockNumber, CollatorSelection, ConsensusHook, Hash,
	MessageQueue, Nonce, PalletInfo, RandomnessCollectiveFlip, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin, RuntimeTask, Session, SessionKeys,
	System, WeightToFee, XcmpQueue, AVERAGE_ON_INITIALIZE_RATIO, EXISTENTIAL_DEPOSIT, HOURS,
	MAXIMUM_BLOCK_WEIGHT, MICROUNIT, NORMAL_DISPATCH_RATIO, SLOT_DURATION, VERSION,
};
use xcm_config::RelayLocation;

parameter_types! {
	pub const Version: RuntimeVersion = VERSION;

	// This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
	//  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
	// `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
	// the lazy contract deletion.
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u16 = 42;
}

/// The default types are being injected by [`derive_impl`](`frame_support::derive_impl`) from
/// [`ParaChainDefaultConfig`](`struct@frame_system::config_preludes::ParaChainDefaultConfig`),
/// but overridden as needed.
#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig)]
impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The index type for storing how many extrinsics an account has signed.
	type Nonce = Nonce;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The block type.
	type Block = Block;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// The data to be stored in an account.
	type AccountData = pallet_balances::AccountData<Balance>;
	/// The weight of database operations that the runtime can invoke.
	type DbWeight = RocksDbWeight;
	/// Block & extrinsics weights: base values and limits.
	type BlockWeights = RuntimeBlockWeights;
	/// The maximum length of a block (in bytes).
	type BlockLength = RuntimeBlockLength;
	/// This is used as an identifier of the chain. 42 is the generic substrate prefix.
	type SS58Prefix = SS58Prefix;
	/// The action to take on a Runtime Upgrade
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type SystemWeightInfo = SystemWeightInfo<Runtime>;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<0>;
	type WeightInfo = TimestampWeightInfo<Runtime>;
}

impl pallet_authorship::Config for Runtime {
	type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
	type EventHandler = (CollatorSelection,);
}

parameter_types! {
	pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

impl pallet_balances::Config for Runtime {
	type MaxLocks = ConstU32<50>;
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	// TODO: use `ResolveTo<TreasuryAccount, Balances>` https://github.com/ideal-lab5/idn-sdk/issues/275
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = BalancesWeightInfo<Runtime>;
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type DoneSlashHandler = ();
}

parameter_types! {
	/// Relay Chain `TransactionByteFee` / 10
	pub const TransactionByteFee: Balance = 10 * MICROUNIT;
}

impl pallet_transaction_payment::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, ()>;
	type WeightToFee = WeightToFee;
	type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
	type FeeMultiplierUpdate = SlowAdjustingFeeUpdate<Self>;
	type OperationalFeeMultiplier = ConstU8<5>;
	type WeightInfo = TransactionPaymentWeightInfo<Runtime>;
}

impl pallet_sudo::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type WeightInfo = SudoWeightInfo<Runtime>;
}

parameter_types! {
	pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
	pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type WeightInfo = CumulusParachainSystemWeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type OnSystemEvent = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = XcmpQueue;
	type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = XcmpQueue;
	type ReservedXcmpWeight = ReservedXcmpWeight;
	type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
	type ConsensusHook = ConsensusHook;
	type SelectCore = cumulus_pallet_parachain_system::DefaultCoreSelector<Runtime>;
}

impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

parameter_types! {
	pub const Period: u32 = 6 * HOURS;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	// we don't have stash and controller, thus we don't need the convert as well.
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = CollatorSelection;
	// Essentially just Aura, but let's be pedantic.
	type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
	type Keys = SessionKeys;
	type WeightInfo = SessionWeightInfo<Runtime>;
	type DisablingStrategy = ();
}

impl pallet_aura::Config for Runtime {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<100_000>;
	type AllowMultipleBlocksPerSlot = ConstBool<true>;
	type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
	pub const PotId: PalletId = PalletId(*b"PotStake");
	pub const SessionLength: BlockNumber = 6 * HOURS;
	// StakingAdmin pluralistic body.
	pub const StakingAdminBodyId: BodyId = BodyId::Defense;
}

/// We allow root and the StakingAdmin to execute privileged collator selection operations.
pub type CollatorSelectionUpdateOrigin = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsVoiceOfBody<RelayLocation, StakingAdminBodyId>>,
>;

impl pallet_collator_selection::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type UpdateOrigin = CollatorSelectionUpdateOrigin;
	type PotId = PotId;
	type MaxCandidates = ConstU32<100>;
	type MinEligibleCollators = ConstU32<4>;
	type MaxInvulnerables = ConstU32<20>;
	// should be a multiple of session or things will get inconsistent
	type KickThreshold = Period;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = pallet_collator_selection::IdentityCollator;
	type ValidatorRegistration = Session;
	type WeightInfo = CollatorSelectionWeightInfo<Runtime>;
}

/// This function helps to keep benchmarks in line with the test mock
/// environemnt. When running benchmarks without this mock, when a request is made
/// that matches the sibling account id, the id is translated to that of the
/// sibling's account on the IDN.
#[cfg(feature = "runtime-benchmarks")]
mod bench_sibling_conversion {
	use sp_runtime::AccountId32;
	use xcm::prelude::{Junction::Parachain, Location};
	use xcm_executor::traits::ConvertLocation;
	pub struct MockSiblingConversion;
	pub const SIBLING_PARA_ID: u32 = 88;
	pub const SIBLING_PARA_ACCOUNT: AccountId32 = AccountId32::new([88u8; 32]);
	impl ConvertLocation<AccountId32> for MockSiblingConversion {
		fn convert_location(location: &Location) -> Option<AccountId32> {
			match location.unpack() {
				(1, [Parachain(SIBLING_PARA_ID)]) => Some(SIBLING_PARA_ACCOUNT),
				_ => None,
			}
		}
	}
}

#[cfg(feature = "runtime-benchmarks")]
mod bench_ensure_origin {
	use crate::RuntimeOrigin;
	use frame_support::pallet_prelude::EnsureOrigin;
	use frame_system::ensure_signed;
	use log;
	use sp_runtime::AccountId32;
	use xcm::prelude::{Junction, Location};

	pub const SIBLING_PARA_ACCOUNT: AccountId32 = AccountId32::new([88u8; 32]);
	pub const SIBLING_PARA_ID: u32 = 88;

	pub struct BenchEnsureOrigin;
	impl EnsureOrigin<RuntimeOrigin> for BenchEnsureOrigin {
		type Success = Location;

		fn try_origin(origin: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
			let caller: AccountId32 = ensure_signed(origin.clone()).unwrap();

			if caller == SIBLING_PARA_ACCOUNT {
				log::info!("yeah that was a good id you used: {:?}", caller);
				return Ok(Location::new(1, Junction::Parachain(SIBLING_PARA_ID)));
			}
			log::info!("not that great tbh");
			Err(origin)
		}
		fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
			Ok(RuntimeOrigin::root())
		}
	}
}

parameter_types! {
	pub const BaseFee: u64 = 2_900_000u64;
}

impl pallet_idn_manager::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type FeesManager = impls::FeesManagerImpl<
		types::TreasuryAccount,
		SubscriptionOf<Runtime>,
		Balances,
		BlockNumber,
		BlockNumber,
		BaseFee,
	>;
	type DepositCalculator = impls::DepositCalculatorImpl<types::SDMultiplier, BalanceOf<Runtime>>;
	type PalletId = types::IdnManagerPalletId;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Pulse = types::RuntimePulse;
	type WeightInfo = IdnManagerWeightInfo<Runtime>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Xcm = PolkadotXcm;
	#[cfg(feature = "runtime-benchmarks")]
	type Xcm = ();
	type MaxMetadataLen = types::MaxMetadataLen;
	type MaxCallDataLen = types::MaxCallDataLen;
	type Credits = types::Credits;
	type MaxSubscriptions = types::MaxSubscriptions;
	type MaxTerminatableSubs = types::MaxTerminatableSubs;
	type SubscriptionId = types::SubscriptionId;
	type DiffBalance = impls::DiffBalanceImpl<BalanceOf<Runtime>>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmOriginFilter = EnsureXcm<AllowSiblingsOnly>;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmOriginFilter = bench_ensure_origin::BenchEnsureOrigin;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type XcmLocationToAccountId = xcm_config::LocationToAccountId;
	#[cfg(feature = "runtime-benchmarks")]
	type XcmLocationToAccountId = bench_sibling_conversion::MockSiblingConversion;
}

parameter_types! {
	pub const MaxSigsPerBlock: u8 = crate::constants::idn::MAX_QUEUE_SIZE as u8;
}

impl pallet_randomness_beacon::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = RandomnessBeaconWeightInfo<Runtime>;
	type SignatureVerifier = sp_idn_crypto::verifier::QuicknetVerifier;
	type MaxSigsPerBlock = MaxSigsPerBlock;
	type Pulse = types::RuntimePulse;
	type Dispatcher = crate::IdnManager;
	type FallbackRandomness = RandomnessCollectiveFlip;
}

impl pallet_insecure_randomness_collective_flip::Config for Runtime {}
