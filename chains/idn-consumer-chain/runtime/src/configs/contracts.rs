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

use super::xcm_config;
use crate::{
	configs::EnsureXcm, weights::ContractsWeightInfo, AccountId, Balance, Balances, BalancesCall,
	Perbill, RandomnessCollectiveFlip, Runtime, RuntimeCall, RuntimeEvent, RuntimeHoldReason,
	RuntimeOrigin, Timestamp, MILLIUNIT, UNIT,
};
use frame_support::{
	parameter_types,
	traits::{ConstBool, ConstU32, EnsureOrigin, Equals},
};
use frame_system::EnsureSigned;
use xcm_executor::traits::ConvertLocation;

pub enum AllowBalancesCall {}

impl frame_support::traits::Contains<RuntimeCall> for AllowBalancesCall {
	fn contains(call: &RuntimeCall) -> bool {
		matches!(call, RuntimeCall::Balances(BalancesCall::transfer_allow_death { .. }))
	}
}

/// Custom origin type that allows both signed users and XCM from IDN, converting both to AccountId.
pub struct ContractsInstantiateOrigin;

impl EnsureOrigin<RuntimeOrigin> for ContractsInstantiateOrigin {
	type Success = AccountId;

	fn try_origin(o: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		// Try signed origin first
		if let Ok(account_id) = EnsureSigned::<AccountId>::try_origin(o.clone()) {
			return Ok(account_id);
		}

		// Try XCM origin from IDN location
		if let Ok(location) = EnsureXcm::<Equals<xcm_config::IdnLocation>>::try_origin(o.clone()) {
			// Convert IDN location to its sovereign account on this chain
			if let Some(account_id) = xcm_config::LocationToAccountId::convert_location(&location) {
				return Ok(account_id);
			}
		}

		Err(o)
	}
}

const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * UNIT + (bytes as Balance) * (5 * MILLIUNIT / 100)) / 10
}

fn schedule<T: pallet_contracts::Config>() -> pallet_contracts::Schedule<T> {
	pallet_contracts::Schedule {
		limits: pallet_contracts::Limits {
			runtime_memory: 1024 * 1024 * 1024,
			validator_runtime_memory: 1024 * 1024 * 1024 * 2,
			..Default::default()
		},
		..Default::default()
	}
}

parameter_types! {
	pub const DepositPerItem: Balance = deposit(1, 0);
	pub const DepositPerByte: Balance = deposit(0, 1);
	pub Schedule: pallet_contracts::Schedule<Runtime> = schedule::<Runtime>();
	pub const DefaultDepositLimit: Balance = deposit(1024, 1024 * 1024);
	pub const CodeHashLockupDepositPercent: Perbill = Perbill::from_percent(0);
	pub const MaxDelegateDependencies: u32 = 32;
}

impl pallet_contracts::Config for Runtime {
	type Time = Timestamp;
	type Randomness = RandomnessCollectiveFlip;
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	/// The safest default is to allow no calls at all.
	///
	/// Runtimes should whitelist dispatchables that are allowed to be called from contracts
	/// and make sure they are stable. Dispatchables exposed to contracts are not allowed to
	/// change because that would break already deployed contracts. The `RuntimeCall` structure
	/// itself is not allowed to change the indices of existing pallets, too.
	type CallFilter = AllowBalancesCall;
	type DepositPerItem = DepositPerItem;
	type DepositPerByte = DepositPerByte;
	type CallStack = [pallet_contracts::Frame<Self>; 23];
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type WeightInfo = ContractsWeightInfo<Runtime>;
	type ChainExtension = ();
	type Schedule = Schedule;
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	/// The maximum size of the code that can be uploaded to the chain. It should be consistent with
	/// runtime heap memory limits.
	type MaxCodeLen = ConstU32<{ 123 * 1024 }>;
	type DefaultDepositLimit = DefaultDepositLimit;
	type MaxStorageKeyLen = ConstU32<128>;
	type MaxTransientStorageSize = ConstU32<{ 1024 * 1024 }>;
	type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
	type UnsafeUnstableInterface = ConstBool<true>;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type MaxDelegateDependencies = MaxDelegateDependencies;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Environment = ();
	type Debug = ();
	type ApiVersion = ();
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Migrations = ();
	#[cfg(feature = "runtime-benchmarks")]
	type Migrations = pallet_contracts::migration::codegen::BenchMigrations;
	type Xcm = pallet_xcm::Pallet<Self>;
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type InstantiateOrigin = ContractsInstantiateOrigin;
}
