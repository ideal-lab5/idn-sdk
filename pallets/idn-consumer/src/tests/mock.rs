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
	self as pallet_idn_consumer,
	traits::{PulseConsumer, QuoteConsumer, SubInfoConsumer},
	Pulse, Quote, SubInfoResponse, SubscriptionId,
};
use bp_idn::types::{Subscription, SubscriptionDetails, SubscriptionState};
use cumulus_primitives_core::ParaId;
use frame_support::{
	construct_runtime, derive_impl, parameter_types, traits::OriginTrait, PalletId,
};
use sp_runtime::{traits::IdentityLookup, AccountId32, BuildStorage};
use xcm::{
	v5::{prelude::*, Location},
	VersionedLocation, VersionedXcm,
};

construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		IdnConsumer: pallet_idn_consumer,
	}
);
pub const IDN_PARA_ACCOUNT: AccountId32 = AccountId32::new([88u8; 32]);
pub const IDN_PARA_ID: u32 = 88;
pub const ALICE: AccountId32 = AccountId32::new([1u8; 32]);
pub const MOCK_SUB: Subscription = Subscription {
	id: [1u8; 32],
	state: SubscriptionState::Active,
	credits_left: 0,
	details: SubscriptionDetails {
		subscriber: AccountId32::new([0u8; 32]),
		target: Location::here(),
		call_index: [0, 0],
	},
	created_at: 0,
	updated_at: 0,
	credits: 0,
	frequency: 0,
	metadata: None,
	last_delivered: None,
};

#[derive_impl(frame_system::config_preludes::ParaChainDefaultConfig)]
impl frame_system::Config for Test {
	type Block = frame_system::mocking::MockBlock<Test>;
	type AccountId = AccountId32;
	type Lookup = IdentityLookup<Self::AccountId>;
	type AccountData = pallet_balances::AccountData<u64>;
}

#[docify::export_content]
pub mod pulse_consumer_impl {
	use super::*;
	pub struct PulseConsumerImpl;
	impl PulseConsumer<Pulse, SubscriptionId, (), ()> for PulseConsumerImpl {
		fn consume_pulse(pulse: Pulse, sub_id: SubscriptionId) -> Result<(), ()> {
			// Simulate a failure if sub_id is [123; 32]
			if sub_id == [123; 32] {
				return Err(());
			}
			log::info!("IDN Consumer: Consuming pulse: {:?}, from sub id: {:?}", pulse, sub_id);
			Ok(())
		}
	}
}

#[docify::export_content]
pub mod quote_consumer_impl {
	use super::*;
	pub struct QuoteConsumerImpl;
	impl QuoteConsumer<Quote, (), ()> for QuoteConsumerImpl {
		fn consume_quote(quote: Quote) -> Result<(), ()> {
			// Simulate a failure if req_ref is [123; 32]
			if quote.req_ref == [123; 32] {
				return Err(());
			}
			log::info!("IDN Consumer: Consuming quote: {:?}", quote);
			Ok(())
		}
	}
}

#[docify::export_content]
pub mod sub_info_consumer_impl {
	use super::*;
	pub struct SubInfoConsumerImpl;
	impl SubInfoConsumer<SubInfoResponse, (), ()> for SubInfoConsumerImpl {
		fn consume_sub_info(sub_info: SubInfoResponse) -> Result<(), ()> {
			// Simulate a failure if subscription is [123; 32]
			if sub_info.sub.id == [123; 32] {
				return Err(());
			}
			log::info!("IDN Consumer: Consuming subscription info: {:?}", sub_info);
			Ok(())
		}
	}
}

// Mock implementation of EnsureXcm
pub struct MockEnsureXcmIdn;

impl frame_support::traits::EnsureOrigin<RuntimeOrigin> for MockEnsureXcmIdn {
	type Success = Location;

	fn try_origin(origin: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		if origin.clone().into_signer().unwrap() == IDN_PARA_ACCOUNT {
			return Ok(Location::new(1, [Parachain(IDN_PARA_ID)]));
		}
		Err(origin)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::root())
	}
}
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
}

impl pallet_idn_consumer::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PulseConsumer = pulse_consumer_impl::PulseConsumerImpl;
	type QuoteConsumer = quote_consumer_impl::QuoteConsumerImpl;
	type SubInfoConsumer = sub_info_consumer_impl::SubInfoConsumerImpl;
	type SiblingIdnLocation = IdnLocation;
	type IdnOrigin = MockEnsureXcmIdn;
	type Xcm = MockXcm;
	type PalletId = IdnConsumerPalletId;
	type ParaId = IdnConsumerParaId;
	type AssetHubFee = AssetHubFee;
	type WeightInfo = ();
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
