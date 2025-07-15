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

use crate::{
	configs::RuntimeBlockWeights,
	constants,
	weights::{CumulusXcmpQueueWeightInfo, MessageQueueWeightInfo, XcmWeightInfo},
	AccountId, AllPalletsWithSystem, Balance, Balances, MessageQueue, ParachainInfo,
	ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin,
	TreasuryAccount, WeightToFee, XcmpQueue, CENTIUNIT,
};
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::{
	pallet_prelude::PhantomData,
	parameter_types,
	traits::{ConstU32, Contains, Disabled, Equals, Everything, Nothing, TransformOrigin},
	weights::Weight,
};
use frame_system::EnsureRoot;
use pallet_xcm::{AuthorizedAliasers, XcmPassthrough};
use parachains_common::{
	message_queue::{NarrowOriginToSibling, ParaIdToSibling},
	xcm_config::{ConcreteAssetFromSystem, ParentRelayOrSiblingParachains},
};
use polkadot_parachain_primitives::primitives::Sibling;
use polkadot_runtime_common::{impls::ToAuthor, xcm_sender::ExponentialPrice};
use sp_runtime::{Perbill, Vec};
use xcm::prelude::*;
use xcm_builder::{
	AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
	AllowTopLevelPaidExecutionFrom, DescribeAllTerminal, DescribeFamily, DescribeTerminus,
	EnsureXcmOrigin, FixedWeightBounds, FrameTransactionalProcessor, FungibleAdapter,
	HashedDescription, IsConcrete, ParentIsPreset, RelayChainAsNative, SendXcmFeeToAccount,
	SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
	SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit, TrailingSetTopicAsId,
	UsingComponents, WithComputedOrigin, WithUniqueTopic, XcmFeeManagerFromComponents,
};
use xcm_executor::{traits::ConvertLocation, XcmExecutor};

parameter_types! {
	pub const RelayLocation: Location = Location::parent();
	pub const RelayNetwork: Option<NetworkId> = Some(NetworkId::Polkadot);
	pub const TokenLocation: Location = Location::here();
	pub AssetHub: Location = Location::new(1, [Parachain(1000)]);
	pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();
	// For the real deployment, it is recommended to set `RelayNetwork` according to the relay chain
	// and prepend `UniversalLocation` with `GlobalConsensus(RelayNetwork::get())`.
	pub UniversalLocation: InteriorLocation = Parachain(ParachainInfo::parachain_id().into()).into();
	/// The asset ID for the asset that we use to pay for message delivery fees.
	pub FeeAssetId: AssetId = AssetId(RelayLocation::get());
	/// The base fee for the message delivery fees.
	pub const BaseDeliveryFee: u128 = CENTIUNIT.saturating_mul(3);
	pub const TransactionByteFee: Balance = constants::relay::fee::TRANSACTION_BYTE_FEE;
	pub RelayTreasuryLocation: Location =
		(Parent, PalletInstance(constants::relay::TREASURY_PALLET_ID)).into();
	pub RelayTreasuryPalletAccount: AccountId =
		LocationToAccountId::convert_location(&RelayTreasuryLocation::get())
			.unwrap_or(TreasuryAccount::get());
	pub MessageQueueIdleServiceWeight: Weight = Perbill::from_percent(20) * RuntimeBlockWeights::get().max_block;
	pub MessageQueueServiceWeight: Weight = Perbill::from_percent(35) * RuntimeBlockWeights::get().max_block;
}

impl pallet_message_queue::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = MessageQueueWeightInfo<Runtime>;
	#[cfg(feature = "runtime-benchmarks")]
	type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
		cumulus_primitives_core::AggregateMessageOrigin,
	>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MessageProcessor = xcm_builder::ProcessXcmMessage<
		AggregateMessageOrigin,
		xcm_executor::XcmExecutor<XcmConfig>,
		RuntimeCall,
	>;
	type Size = u32;
	// The XCMP queue pallet is only ever able to handle the `Sibling(ParaId)` origin:
	type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
	type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
	type HeapSize = sp_core::ConstU32<{ 103 * 1024 }>;
	type MaxStale = sp_core::ConstU32<8>;
	type ServiceWeight = MessageQueueServiceWeight;
	type IdleMaxServiceWeight = MessageQueueIdleServiceWeight;
}

/// Type for specifying how a `Location` can be converted into an `AccountId`.
///
/// This is used when determining ownership of accounts for asset transacting and when attempting to
/// use XCM `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
	// The parent (Relay-chain) origin converts to the parent `AccountId`.
	ParentIsPreset<AccountId>,
	// Sibling parachain origins convert to AccountId via the `ParaId::into`.
	SiblingParachainConvertsVia<Sibling, AccountId>,
	// Straight up local `AccountId32` origins just alias directly to `AccountId`.
	AccountId32Aliases<RelayNetwork, AccountId>,
	// Here/local root location to `AccountId`.
	HashedDescription<AccountId, DescribeTerminus>,
	// Foreign locations alias into accounts according to a hash of their standard description.
	HashedDescription<AccountId, DescribeFamily<DescribeAllTerminal>>,
);

/// Means for transacting the native currency on this chain.
pub type FungibleTransactor = FungibleAdapter<
	// Use this currency:
	Balances,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<RelayLocation>,
	// Convert an XCM `Location` into a local account ID:
	LocationToAccountId,
	// Our chain's account ID type (we can't get away without mentioning it explicitly):
	AccountId,
	// We don't track any teleports of `Balances`.
	(),
>;

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
	// Native converter for Relay-chain (Parent) location; will convert to a `Relay` origin when
	// recognized.
	RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
	// Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
	// recognized.
	SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
	// Native signed account converter; this just converts an `AccountId32` origin into a normal
	// `RuntimeOrigin::Signed` origin of the same 32-byte value.
	SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
	// Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
	XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
	pub const RootLocation: Location = Here.into_location();
	// One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 64 * 1024);
	pub const MaxInstructions: u32 = 100;
	pub const MaxAssetsIntoHolding: u32 = 64;
}

/// The barriers one of which must be passed for an XCM message to be executed.
pub type Barrier = TrailingSetTopicAsId<(
	// Weight that is paid for may be consumed.
	TakeWeightCredit,
	// Expected responses are OK.
	AllowKnownQueryResponses<PolkadotXcm>,
	// Allow XCMs with some computed origins to pass through.
	WithComputedOrigin<
		(
			// If the message is one that immediately attempts to pay for execution, then allow it.
			AllowTopLevelPaidExecutionFrom<Everything>,
			// Subscriptions for version tracking are OK.
			AllowSubscriptionsFrom<ParentRelayOrSiblingParachains>,
		),
		UniversalLocation,
		ConstU32<8>,
	>,
)>;

/// Locations that will not be charged fees in the executor, neither for execution nor delivery.
pub type WaivedLocations = (Equals<RootLocation>,);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
	type RuntimeCall = RuntimeCall;
	type XcmSender = XcmRouter;
	// How to withdraw and deposit an asset.
	type AssetTransactor = FungibleTransactor;
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	// Only allow reserve transfer of Relay Chain native token (e.g. DOT) from System or Relay
	// Chain.
	type IsReserve = ConcreteAssetFromSystem<RelayLocation>;
	// Telerporting is not enabled.
	type IsTeleporter = ();
	type UniversalLocation = UniversalLocation;
	type Barrier = Barrier;
	// TODO: use WeightInfoBounds https://github.com/ideal-lab5/idn-sdk/issues/262
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type Trader =
		UsingComponents<WeightToFee, RelayLocation, AccountId, Balances, ToAuthor<Runtime>>;
	type ResponseHandler = PolkadotXcm;
	type AssetTrap = PolkadotXcm;
	type AssetClaims = PolkadotXcm;
	type SubscriptionService = PolkadotXcm;
	type PalletInstancesInfo = AllPalletsWithSystem;
	type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
	type AssetLocker = ();
	type AssetExchanger = ();
	type FeeManager = XcmFeeManagerFromComponents<
		WaivedLocations,
		SendXcmFeeToAccount<Self::AssetTransactor, RelayTreasuryPalletAccount>,
	>;
	type MessageExporter = ();
	type UniversalAliases = Nothing;
	type CallDispatcher = RuntimeCall;
	type SafeCallFilter = Everything;
	type Aliasers = AuthorizedAliasers<Runtime>;
	type TransactionalProcessor = FrameTransactionalProcessor;
	type HrmpNewChannelOpenRequestHandler = ();
	type HrmpChannelAcceptedHandler = ();
	type HrmpChannelClosingHandler = ();
	type XcmRecorder = PolkadotXcm;
	type XcmEventEmitter = PolkadotXcm;
}

/// Converts a local signed origin into an XCM `Location`.
/// Forms the basis for local origins sending/executing XCMs.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
	// Two routers - use UMP to communicate with the relay chain:
	cumulus_primitives_utility::ParentAsUmp<ParachainSystem, PolkadotXcm, PriceForParentDelivery>,
	// ..and XCMP to communicate with the sibling chains.
	XcmpQueue,
)>;

/// Filter to determine if all specified assets are supported, used with
/// reserve-transfers.
pub struct FilterByAssets<Assets>(PhantomData<Assets>);
impl<Assets: Contains<Location>> Contains<(Location, Vec<Asset>)> for FilterByAssets<Assets> {
	fn contains(t: &(Location, Vec<Asset>)) -> bool {
		t.1.iter().all(|a| Assets::contains(&a.id.0))
	}
}

impl pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	// Any local signed origin can send XCM messages.
	// It is up to the destination chain to decide whether to accept the message.
	// This could be more restrictive, e.g. `EnsureXcmOrigin<RuntimeOrigin, ()>`, but we want to
	// allow any signed origin to be able to reserve transfer tokens back to other chains.
	type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmRouter = XcmRouter;
	type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type XcmExecuteFilter = Nothing;
	// ^ Disable dispatchable execute on the XCM pallet.
	// Needs to be `Everything` for local testing.
	type XcmExecutor = XcmExecutor<XcmConfig>;
	// We don't allow teleporting assets.
	type XcmTeleportFilter = Nothing;
	type XcmReserveTransferFilter = FilterByAssets<Equals<RelayLocation>>;
	// TODO: use WeightInfoBounds https://github.com/ideal-lab5/idn-sdk/issues/262
	type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
	type UniversalLocation = UniversalLocation;
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;

	const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
	// ^ Override for AdvertisedXcmVersion default
	type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
	type Currency = Balances;
	type CurrencyMatcher = ();
	type TrustedLockers = ();
	type SovereignAccountOf = LocationToAccountId;
	type MaxLockers = ConstU32<8>;
	type WeightInfo = XcmWeightInfo<Runtime>;
	type AdminOrigin = EnsureRoot<AccountId>;
	type MaxRemoteLockConsumers = ConstU32<0>;
	type RemoteLockConsumerIdentifier = ();
	// Aliasing is disabled: xcm_executor::Config::Aliasers is set to `Nothing`.
	type AuthorizedAliasConsideration = Disabled;
}

impl cumulus_pallet_xcm::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

/// Means to price the delivery of an XCM to the parent chain.
pub type PriceForParentDelivery =
	ExponentialPrice<RelayLocation, BaseDeliveryFee, TransactionByteFee, ParachainSystem>;

/// Means to price the delivery of an XCM to a sibling chain.
pub type PriceForSiblingDelivery =
	ExponentialPrice<RelayLocation, BaseDeliveryFee, TransactionByteFee, XcmpQueue>;

impl cumulus_pallet_xcmp_queue::Config for Runtime {
	type ChannelInfo = ParachainSystem;
	type ControllerOrigin = EnsureRoot<AccountId>;
	type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
	// Limit the number of messages and signals a HRMP channel can have at most.
	type MaxActiveOutboundChannels = ConstU32<128>;
	// Limits the number of inbound channels that we can suspend at the same time.
	// A value close to the number of possible channels seems to be a sensible configuration.
	type MaxInboundSuspended = ConstU32<128>;
	// Limit the number of HRMP channels.
	// note: https://github.com/polkadot-fellows/runtimes/blob/76d1fa680d00c3e447e40199e7b2250862ad4bfa/system-parachains/asset-hubs/asset-hub-polkadot/src/lib.rs#L692C2-L693C90
	type MaxPageSize = ConstU32<{ 103 * 1024 }>;
	type PriceForSiblingDelivery = PriceForSiblingDelivery;
	type RuntimeEvent = RuntimeEvent;
	type VersionWrapper = PolkadotXcm;
	type WeightInfo = CumulusXcmpQueueWeightInfo<Runtime>;
	// Enqueue XCMP messages from siblings for later processing.
	type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::{assert_ok, traits::fungible::Mutate};
	use scale_info::prelude::sync::Arc;
	use xcm::opaque::latest::{
		Junction,
		Junctions::{X1, X2},
		Location,
	};
	use xcm_builder::test_utils::TransactAsset;

	#[test]
	fn test_withdraw_asset_here() {
		let location = Location::here();
		let asset = (Location::here(), 1u128).into();
		let context = None;
		// Set up the balance for the account corresponding to Location::here()
		let account_id = LocationToAccountId::convert_location(&location).expect("should convert");
		// Give the account a balance so withdrawal can succeed
		println!("Location: {:?}, Account ID: {:?}", location, account_id);
		Balances::set_balance(&account_id, 100);
		assert_ok!(FungibleTransactor::withdraw_asset(&asset, &location, context));
	}

	#[test]
	fn test_convert_location() {
		let location = Location::new(1, X1(Arc::new([Junction::Parachain(2000)])));
		let account_id = LocationToAccountId::convert_location(&location).unwrap();
		println!("Sibling 2000. Location: {:?}, Account ID: {:?}", location, account_id);

		let location = Location::new(1, X1(Arc::new([Junction::Parachain(2001)])));
		let account_id = LocationToAccountId::convert_location(&location).unwrap();
		println!("Sibling 2001. Location: {:?}, Account ID: {:?}", location, account_id);

		let location = Location::new(0, X1(Arc::new([Junction::Parachain(2000)])));
		let account_id = LocationToAccountId::convert_location(&location).unwrap();
		println!("Child 2000. Location: {:?}, Account ID: {:?}", location, account_id);

		let location = Location::new(0, X1(Arc::new([Junction::Parachain(2001)])));
		let account_id = LocationToAccountId::convert_location(&location).unwrap();
		println!("Child 2001. Location: {:?}, Account ID: {:?}", location, account_id);

		let location = Location::new(1, Here);
		let account_id = LocationToAccountId::convert_location(&location).unwrap();
		println!("Here. Location: {:?}, Account ID: {:?}", location, account_id);

		let location = Location::new(
			1,
			X2(Arc::new([
				Junction::Parachain(2000),
				AccountId32 {
					network: Some(NetworkId::Polkadot),
					id: [
						28, 189, 45, 67, 83, 10, 68, 112, 90, 208, 136, 175, 49, 62, 24, 248, 11,
						83, 239, 22, 179, 97, 119, 205, 75, 119, 184, 70, 242, 165, 240, 124,
					],
				},
			])),
		);
		let account_id = LocationToAccountId::convert_location(&location).unwrap();
		println!("Child 2000. Location: {:?}, Account ID: {:?}", location, account_id);
	}
}
