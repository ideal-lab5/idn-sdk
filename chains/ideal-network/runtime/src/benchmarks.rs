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

//! Benchmarking for the Ideal Network Runtime

use crate::configs::{
	xcm_config::{
		AssetHub, BaseDeliveryFee, FeeAssetId, LocationToAccountId, PriceForSiblingDelivery,
		RelayLocation, TransactionByteFee, XcmConfig,
	},
	ExistentialDeposit,
};
use cumulus_primitives_core::ParaId;
use frame_benchmarking::BenchmarkError;
use frame_support::parameter_types;
use scale_info::prelude::vec::Vec;
use xcm::prelude::{
	Asset, AssetId, Fungibility::Fungible, Here, InteriorLocation, Junction, Location, NetworkId,
	Response,
};
use xcm_executor::traits::ConvertLocation;

pub use super::*;
pub use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
pub use frame_benchmarking::{BenchmarkBatch, BenchmarkList};
pub use frame_support::traits::{StorageInfoTrait, WhitelistedStorageKeys};
pub use frame_system_benchmarking::Pallet as SystemBench;
pub use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;

frame_support::parameter_types! {
	pub ExistentialDepositAsset: Option<Asset> = Some((
		RelayLocation::get(),
		EXISTENTIAL_DEPOSIT
	).into());

	pub const AssetHubParaId: ParaId = ParaId::new(1000);
}

type PriceForSiblingParachainDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	TransactionByteFee,
	XcmpQueue,
>;

type PriceForParentDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	BaseDeliveryFee,
	TransactionByteFee,
	ParachainSystem,
>;

impl frame_system_benchmarking::Config for Runtime {
	fn setup_set_code_requirements(code: &Vec<u8>) -> Result<(), BenchmarkError> {
		ParachainSystem::initialize_for_set_code_benchmark(code.len() as u32);
		Ok(())
	}

	fn verify_set_code() {
		System::assert_last_event(
			cumulus_pallet_parachain_system::Event::<Runtime>::ValidationFunctionStored.into(),
		);
	}
}

impl cumulus_pallet_session_benchmarking::Config for Runtime {}

impl pallet_xcm::benchmarking::Config for Runtime {
	type DeliveryHelper = (
		cumulus_primitives_utility::ToParentDeliveryHelper<
			XcmConfig,
			ExistentialDepositAsset,
			PriceForParentDelivery,
		>,
		polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
			XcmConfig,
			ExistentialDepositAsset,
			PriceForSiblingParachainDelivery,
			AssetHubParaId,
			ParachainSystem,
		>,
	);
	fn reachable_dest() -> Option<Location> {
		Some(RelayLocation::get())
	}

	fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
		// Teleporting is not enabled.
		None
	}

	fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
		ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(AssetHubParaId::get());

		let who = frame_benchmarking::whitelisted_caller();
		// Give some multiple of the existential deposit.
		let balance = ExistentialDeposit::get() * 10_000;
		let _ =
			<Balances as frame_support::traits::Currency<_>>::make_free_balance_be(&who, balance);
		let ah_on_idn: AccountId = LocationToAccountId::convert_location(&AssetHub::get())?;
		let _ = <Balances as frame_support::traits::Currency<_>>::make_free_balance_be(
			&ah_on_idn, balance,
		);
		// IDN can reserve transfer relay chain native token to system chains.
		Some((Self::get_asset(), AssetHub::get()))
	}

	fn get_asset() -> Asset {
		Asset { id: AssetId(RelayLocation::get()), fun: Fungible(ExistentialDeposit::get()) }
	}
}
#[cfg(feature = "tlock")]
frame_benchmarking::define_benchmarks!(
	// Only benchmark the following pallets
	[frame_system, SystemBench::<Runtime>]
	[cumulus_pallet_parachain_system, ParachainSystem]
	[pallet_timestamp, Timestamp]
	[pallet_balances, Balances]
	[pallet_sudo, Sudo]
	[pallet_collator_selection, CollatorSelection]
	[pallet_contracts, Contracts]
	[pallet_session, SessionBench::<Runtime>]
	[cumulus_pallet_xcmp_queue, XcmpQueue]
	[pallet_message_queue, MessageQueue]
	[pallet_randomness_beacon, RandBeacon]
	[pallet_idn_manager, IdnManager]
	[pallet_transaction_payment, TransactionPayment]
	[pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
	[pallet_timelock_transactions, Timelock]
);

<<<<<<< HEAD
#[cfg(not(feature = "tlock"))]
=======
type DeliveryHelper = (
	cumulus_primitives_utility::ToParentDeliveryHelper<
		XcmConfig,
		ExistentialDepositAsset,
		PriceForParentDelivery,
	>,
	polkadot_runtime_common::xcm_sender::ToParachainDeliveryHelper<
		XcmConfig,
		ExistentialDepositAsset,
		PriceForSiblingDelivery,
		AssetHubParaId,
		ParachainSystem,
	>,
);

/// Pallet that benchmarks XCM's `AssetTransactor` trait via `Fungible`.
pub type XcmFungible = pallet_xcm_benchmarks::fungible::Pallet<Runtime>;
/// Pallet that serves no other purpose than benchmarking raw XCMs.
pub type XcmGeneric = pallet_xcm_benchmarks::generic::Pallet<Runtime>;

parameter_types! {
	pub TrustedReserve: Option<(Location, Asset)> = Some((AssetHub::get(), Asset::from((RelayLocation::get(), UNIT))));
	// We don't set any trusted teleporters in our XCM config, but we need this for the benchmarks.
	pub TrustedTeleporter: Option<(Location, Asset)> = Some((
		AssetHub::get(),
		Asset::from((RelayLocation::get(), UNIT)),
	));
}

impl pallet_xcm_benchmarks::Config for Runtime {
	type AccountIdConverter = LocationToAccountId;
	type DeliveryHelper = DeliveryHelper;
	type XcmConfig = XcmConfig;

	fn valid_destination() -> Result<Location, BenchmarkError> {
		Ok(RelayLocation::get())
	}

	fn worst_case_holding(_depositable_count: u32) -> xcm::prelude::Assets {
		// IDN only allows relay's native asset to be used cross chain for now.
		vec![Asset { id: AssetId(RelayLocation::get()), fun: Fungible(u128::MAX) }].into()
	}
}

impl pallet_xcm_benchmarks::fungible::Config for Runtime {
	type CheckedAccount = ();
	type TransactAsset = Balances;
	type TrustedReserve = TrustedReserve;
	type TrustedTeleporter = TrustedTeleporter;

	fn get_asset() -> Asset {
		Asset { id: AssetId(RelayLocation::get()), fun: Fungible(10 * UNIT) }
	}
}

impl pallet_xcm_benchmarks::generic::Config for Runtime {
	type RuntimeCall = RuntimeCall;
	type TransactAsset = Balances;

	fn worst_case_response() -> (u64, Response) {
		let notify = frame_system::Call::remark { remark: vec![] };
		PolkadotXcm::new_notify_query(Location::here(), notify, 10, Location::here());
		(0u64, Response::ExecutionResult(None))
	}

	fn worst_case_asset_exchange(
	) -> Result<(xcm::prelude::Assets, xcm::prelude::Assets), BenchmarkError> {
		// IDN doesn't support asset exchange for now.
		Err(BenchmarkError::Skip)
	}

	fn universal_alias() -> Result<(Location, Junction), BenchmarkError> {
		// IDN's `UniversalAliases` is configured to `Nothing`.
		Err(BenchmarkError::Skip)
	}

	fn transact_origin_and_runtime_call() -> Result<(Location, RuntimeCall), BenchmarkError> {
		Ok((RelayLocation::get(), frame_system::Call::remark_with_event { remark: vec![] }.into()))
	}

	fn subscribe_origin() -> Result<Location, BenchmarkError> {
		Ok(RelayLocation::get())
	}

	fn claimable_asset() -> Result<(Location, Location, xcm::prelude::Assets), BenchmarkError> {
		let origin = AssetHub::get();
		let assets: xcm::prelude::Assets = (AssetId(RelayLocation::get()), 1_000 * UNIT).into();
		let ticket = Location { parents: 0, interior: Here };
		Ok((origin, ticket, assets))
	}

	fn fee_asset() -> Result<Asset, BenchmarkError> {
		Ok(Asset { id: AssetId(RelayLocation::get()), fun: Fungible(1_000_000 * UNIT) })
	}

	fn unlockable_asset() -> Result<(Location, Location, Asset), BenchmarkError> {
		// IDN doesn't configure `AssetLocker` yet.
		Err(BenchmarkError::Skip)
	}

	fn export_message_origin_and_destination(
	) -> Result<(Location, NetworkId, InteriorLocation), BenchmarkError> {
		// IDN doesn't configure `MessageExporter` yet.
		Err(BenchmarkError::Skip)
	}

	fn alias_origin() -> Result<(Location, Location), BenchmarkError> {
		// IDN's `Aliasers` is configured to `Nothing`.
		Err(BenchmarkError::Skip)
	}
}

>>>>>>> main
frame_benchmarking::define_benchmarks!(
	// Only benchmark the following pallets
	[frame_system, SystemBench::<Runtime>]
	[cumulus_pallet_parachain_system, ParachainSystem]
	[pallet_timestamp, Timestamp]
	[pallet_balances, Balances]
	[pallet_sudo, Sudo]
	[pallet_collator_selection, CollatorSelection]
	[pallet_contracts, Contracts]
	[pallet_session, SessionBench::<Runtime>]
	[cumulus_pallet_xcmp_queue, XcmpQueue]
	[pallet_message_queue, MessageQueue]
	[pallet_randomness_beacon, RandBeacon]
	[pallet_idn_manager, IdnManager]
	[pallet_transaction_payment, TransactionPayment]
	[pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
	[pallet_xcm_benchmarks::fungible, XcmFungible]
	[pallet_xcm_benchmarks::generic, XcmGeneric]
);
