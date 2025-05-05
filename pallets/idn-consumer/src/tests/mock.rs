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

use crate::{self as pallet_idn_consumer, ConsumerTrait, Pulse, SubscriptionId};
use cumulus_primitives_core::{relay_chain::AccountId, ParaId};
use frame_support::{
	construct_runtime, derive_impl, dispatch::DispatchResultWithPostInfo, pallet_prelude::Pays,
	parameter_types, PalletId,
};
use sp_runtime::{traits::IdentityLookup, AccountId32, BuildStorage};
use xcm::{
	v5::{prelude::*, Location},
	VersionedLocation, VersionedXcm,
};
use xcm_builder::{EnsureXcmOrigin, SignedToAccountId32};

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		IdnConsumer: pallet_idn_consumer,
	}
);

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig)]
impl frame_system::Config for Test {
	type Block = frame_system::mocking::MockBlock<Test>;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<u64>;
}

pub struct Consumer;
impl ConsumerTrait<Pulse, SubscriptionId, DispatchResultWithPostInfo> for Consumer {
	fn consume(pulse: Pulse, sub_id: SubscriptionId) -> DispatchResultWithPostInfo {
		log::info!("IDN Consumer: Consuming pulse: {:?}", pulse);
		log::info!("IDN Consumer: Subscription ID: {:?}", sub_id);
		Ok(Pays::No.into())
	}
}

pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

pub struct MockXcm;

impl xcm_builder::SendController<RuntimeOrigin> for MockXcm {
	type WeightInfo = ();

	fn send(
		_origin: RuntimeOrigin,
		_target: Box<VersionedLocation>,
		_msg: Box<VersionedXcm<()>>,
	) -> Result<[u8; 32], sp_runtime::DispatchError> {
		let block = System::block_number();
		// Simulate a failure if the block number is 1_234_567
		if block == 1_234_567 {
			Err(sp_runtime::DispatchError::Other("MockXcm send failed"))
		} else {
			Ok([0; 32])
		}
	}
}

parameter_types! {
	pub IdnLocation: Location = Location::new(1, Junction::Parachain(2000));
	pub IdnConsumerParaId: ParaId = 2001.into();
	pub const IdnConsumerPalletId: PalletId = PalletId(*b"idn_cons");
	pub const AssetHubFee: u128 = 1_000;
	pub RelayNetwork: Option<NetworkId> = Some(NetworkId::ByGenesis([0; 32]));
}

impl pallet_idn_consumer::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Consumer = Consumer;
	type SiblingIdnLocation = IdnLocation;
	type IdnOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
	type Xcm = MockXcm;
	type PalletId = IdnConsumerPalletId;
	type ParaId = IdnConsumerParaId;
	type AssetHubFee = AssetHubFee;
}

pub struct ExtBuilder;

impl ExtBuilder {
	pub fn build() -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		let mut ext = sp_io::TestExternalities::new(storage);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}
