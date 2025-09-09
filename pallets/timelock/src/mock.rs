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
// Timelock test environment.

use super::*;

use crate as timelock;
use frame_support::{derive_impl, ord_parameter_types, parameter_types, traits::ConstU32};
use frame_system::{EnsureRoot, EnsureSigned};
use sp_runtime::BuildStorage;

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
	}
}

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Logger: logger::{Pallet, Call, Event<T>},
		Timelock: timelock::{Pallet, Call, Storage, Event<T>},
		Preimage: pallet_preimage::{Pallet, Call, Storage, Event<T>, HoldReason},
		RandomnessCollectiveFlip: pallet_insecure_randomness_collective_flip,
		Balances: pallet_balances
	}
);

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

parameter_types! {
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(
			Weight::from_parts(2_000_000_000_000, u64::MAX),
		);
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

impl pallet_insecure_randomness_collective_flip::Config for Test {}

pub struct TestWeightInfo;
impl WeightInfo for TestWeightInfo {
	fn service_task_base() -> Weight {
		Weight::from_parts(50, 0)
	}
	fn service_task_fetched(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn execute_dispatch_signed() -> Weight {
		Weight::from_parts(50, 0)
	}
	fn schedule_sealed(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
	fn service_agenda(_s: u32) -> Weight {
		Weight::from_parts(50, 0)
	}
}
parameter_types! {
	pub MaximumTimelockWeight: Weight = Weight::from_parts(1000, 1000);
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u64>;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeOrigin = RuntimeOrigin;
	type PalletsOrigin = OriginCaller;
	type RuntimeCall = RuntimeCall;
	type MaximumWeight = MaximumTimelockWeight;
	type ScheduleOrigin = EnsureSigned<u64>;
	type MaxScheduledPerBlock = ConstU32<10>;
	type WeightInfo = TestWeightInfo;
	type Preimages = Preimage;
	type Currency = Balances;
	type HoldReason = RuntimeHoldReason;
	
}

pub type LoggerCall = logger::Call<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = system::GenesisConfig::<Test>::default().build_storage().unwrap();
	t.into()
}
