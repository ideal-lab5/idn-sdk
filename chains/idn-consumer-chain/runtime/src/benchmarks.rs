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

use crate::configs::{
	xcm_config::{
		FeeAssetId, RelayLocation, ToParentBaseDeliveryFee, ToSiblingBaseDeliveryFee,
		TransactionByteFee, XcmConfig,
	},
	ExistentialDeposit,
};
use frame_benchmarking::BenchmarkError;
use scale_info::prelude::vec::Vec;
use xcm::v5::{Asset, AssetId, Fungibility::Fungible, Location, Parent};

pub use super::*;
pub use cumulus_pallet_session_benchmarking::Pallet as SessionBench;
pub use frame_benchmarking::{BenchmarkBatch, BenchmarkList};
pub use frame_support::traits::{StorageInfoTrait, WhitelistedStorageKeys};
pub use frame_system_benchmarking::Pallet as SystemBench;
pub use pallet_xcm::benchmarking::Pallet as PalletXcmExtrinsicsBenchmark;

frame_support::parameter_types! {
	pub ExistentialDepositAsset: Option<xcm::v5::Asset> = Some((
		RelayLocation::get(),
		EXISTENTIAL_DEPOSIT
	).into());
	pub const RandomParaId: cumulus_primitives_core::ParaId =
		cumulus_primitives_core::ParaId::new(43211234);
}

type PriceForSiblingParachainDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	ToSiblingBaseDeliveryFee,
	TransactionByteFee,
	XcmpQueue,
>;

type PriceForParentDelivery = polkadot_runtime_common::xcm_sender::ExponentialPrice<
	FeeAssetId,
	ToParentBaseDeliveryFee,
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
			RandomParaId,
			ParachainSystem,
		>,
	);
	fn reachable_dest() -> Option<Location> {
		Some(Parent.into())
	}

	fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
		// Relay/native token can be teleported between IDN Consumer and Relay.
		Some((Self::get_asset(), Parent.into()))
	}

	fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
		None
	}

	fn get_asset() -> Asset {
		Asset { id: AssetId(RelayLocation::get()), fun: Fungible(ExistentialDeposit::get()) }
	}
}

// impl pallet_xcm_benchmarks::Config for Runtime {
// 	type XcmConfig = XcmConfig;
// 	type AccountIdConverter = LocationToAccountId;
// 	type DeliveryHelper = cumulus_primitives_utility::ToParentDeliveryHelper<
// 		XcmConfig,
// 		ExistentialDepositAsset,
// 		PriceForParentDelivery,
// 	>;
// 	fn valid_destination() -> Result<Location, BenchmarkError> {
// 		Ok(RelayLocation::get())
// 	}
// 	fn worst_case_holding(_depositable_count: u32) -> Assets {
// 		// just concrete assets according to relay chain.
// 		let assets: Vec<Asset> =
// 			vec![Asset { id: AssetId(RelayLocation::get()), fun: Fungible(1_000_000 * UNITS) }];
// 		assets.into()
// 	}
// }

frame_benchmarking::define_benchmarks!(
	// Only benchmark the following pallets
	[frame_system, SystemBench::<Runtime>]
	[cumulus_pallet_parachain_system, ParachainSystem]
	[pallet_timestamp, Timestamp]
	[pallet_balances, Balances]
	[pallet_sudo, Sudo]
	[pallet_collator_selection, CollatorSelection]
	[pallet_session, SessionBench::<Runtime>]
	[cumulus_pallet_xcmp_queue, XcmpQueue]
	[pallet_message_queue, MessageQueue]
	[pallet_transaction_payment, TransactionPayment]
	[pallet_idn_consumer, IdnConsumer]
	[pallet_contracts, Contracts]
	[pallet_revive, Revive]
	[pallet_xcm, PalletXcmExtrinsicsBenchmark::<Runtime>]
);
