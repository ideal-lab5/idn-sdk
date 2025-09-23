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
	weights::ContractsWeightInfo, Balance, Balances, Perbill, RandBeacon, Runtime, RuntimeCall,
	RuntimeEvent, RuntimeHoldReason, Timestamp, MILLIUNIT, UNIT,
};
use codec::Encode;
use frame_support::{
	parameter_types,
	traits::{ConstBool, ConstU32, Nothing, Randomness},
};
use frame_system::EnsureSigned;
use pallet_contracts::chain_extension::{
	ChainExtension, Environment, Ext, InitState, RetVal, SysConfig,
};
use sp_core::{crypto::UncheckedFrom, H256};
use sp_runtime::DispatchError;
use sp_std::convert::AsRef;

const fn deposit(items: u32, bytes: u32) -> Balance {
	(items as Balance * UNIT + (bytes as Balance) * (5 * MILLIUNIT / 100)) / 10
}

#[derive(Default)]
pub struct RandExtension;

impl ChainExtension<Runtime> for RandExtension {
	fn call<E: Ext>(&mut self, env: Environment<E, InitState>) -> Result<RetVal, DispatchError>
	where
		<E::T as SysConfig>::AccountId: UncheckedFrom<<E::T as SysConfig>::Hash> + AsRef<[u8]>,
	{
		let func_id = env.func_id();
		log::trace!(
			target: "runtime",
			"[ChainExtension]|call|func_id:{:}",
			func_id
		);
		match func_id {
			1101 => {
				let mut env = env.buf_in_buf_out();
				let arg: [u8; 32] = env.read_as()?;
				let caller = env.ext().caller().clone();
				let seed = [caller.encode(), env.ext().address().encode(), arg.encode()].concat();
				let (rand_hash, _block_number): (H256, _) = RandBeacon::random(&seed);
				env.write(&rand_hash.encode(), false, None)
					.map_err(|_| DispatchError::Other("Failed to write output randomness"))?;

				Ok(RetVal::Converging(0))
			},
			_ => {
				log::error!("Called an unregistered `func_id`: {:}", func_id);
				Err(DispatchError::Other("Unimplemented func_id"))
			},
		}
	}

	fn enabled() -> bool {
		true
	}
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
	type AddressGenerator = pallet_contracts::DefaultAddressGenerator;
	type ApiVersion = ();
	// IMPORTANT: only runtime calls through the api are allowed.
	type CallFilter = Nothing;
	type CallStack = [pallet_contracts::Frame<Self>; 23];
	type ChainExtension = RandExtension;
	type CodeHashLockupDepositPercent = CodeHashLockupDepositPercent;
	type Currency = Balances;
	type Debug = ();
	type DefaultDepositLimit = DefaultDepositLimit;
	type DepositPerByte = DepositPerByte;
	type DepositPerItem = DepositPerItem;
	type Environment = ();
	type InstantiateOrigin = EnsureSigned<Self::AccountId>;
	type MaxCodeLen = ConstU32<{ 128 * 1024 }>;
	type MaxDebugBufferLen = ConstU32<{ 2 * 1024 * 1024 }>;
	type MaxDelegateDependencies = ConstU32<32>;
	type MaxStorageKeyLen = ConstU32<128>;
	type MaxTransientStorageSize = ConstU32<{ 1024 * 1024 }>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Migrations = ();
	#[cfg(feature = "runtime-benchmarks")]
	type Migrations = pallet_contracts::migration::codegen::BenchMigrations;
	/// Contracts randomness provider is the randomness beacon pallet.
	type Randomness = RandBeacon;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Schedule = Schedule;
	type Time = Timestamp;
	type UnsafeUnstableInterface = ConstBool<true>;
	type UploadOrigin = EnsureSigned<Self::AccountId>;
	type WeightInfo = ContractsWeightInfo<Runtime>;
	type WeightPrice = pallet_transaction_payment::Pallet<Self>;
	type Xcm = pallet_xcm::Pallet<Self>;
}
