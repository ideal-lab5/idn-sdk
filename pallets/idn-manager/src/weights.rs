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


//! Autogenerated weights for `pallet_idn_manager`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 47.0.0
//! DATE: 2025-05-23, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `MacBook-Pro-3.local`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: `1024`

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --runtime
// ../../../target/release/wbuild/idn-sdk-kitchensink-runtime/idn_sdk_kitchensink_runtime.compact.compressed.wasm
// --pallet
// pallet-idn-manager
// --extrinsic
// 
// --template
// ./weight-template.hbs
// --output
// ../../../pallets/idn-manager/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]
#![allow(dead_code)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pallet_idn_manager`.
pub trait WeightInfo {
	fn create_subscription() -> Weight;
	fn pause_subscription() -> Weight;
	fn kill_subscription() -> Weight;
	fn update_subscription(m: u32, ) -> Weight;
	fn reactivate_subscription() -> Weight;
	fn quote_subscription() -> Weight;
	fn get_subscription_info() -> Weight;
	fn dispatch_pulse(s: u32, ) -> Weight;
	fn on_finalize() -> Weight;
}

/// Weights for `pallet_idn_manager` using the IDN SDK Kitchensink Runtime and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: `IdnManager::SubCounter` (r:1 w:1)
	/// Proof: `IdnManager::SubCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn create_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42`
		//  Estimated: `4225`
		// Minimum execution time: 85_000_000 picoseconds.
		Weight::from_parts(88_000_000, 4225)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	fn pause_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `243`
		//  Estimated: `4225`
		// Minimum execution time: 13_000_000 picoseconds.
		Weight::from_parts(14_000_000, 4225)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::SubCounter` (r:1 w:1)
	/// Proof: `IdnManager::SubCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn kill_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `301`
		//  Estimated: `4225`
		// Minimum execution time: 77_000_000 picoseconds.
		Weight::from_parts(78_000_000, 4225)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// The range of component `m` is `[0, 8]`.
	fn update_subscription(m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `301`
		//  Estimated: `4225`
		// Minimum execution time: 52_000_000 picoseconds.
		Weight::from_parts(55_497_244, 4225)
			// Standard Error: 12_328
			.saturating_add(Weight::from_parts(20_669, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	fn reactivate_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `243`
		//  Estimated: `4225`
		// Minimum execution time: 13_000_000 picoseconds.
		Weight::from_parts(14_000_000, 4225)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	fn quote_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 7_000_000 picoseconds.
		Weight::from_parts(8_000_000, 0)
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:0)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	fn get_subscription_info() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `243`
		//  Estimated: `4225`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 4225)
			.saturating_add(T::DbWeight::get().reads(1_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:2001 w:2000)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:2000 w:2000)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2000 w:2000)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(104), added: 2579, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[1, 2000]`.
	fn dispatch_pulse(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `396 + s * (343 ±0)`
		//  Estimated: `4225 + s * (3235 ±0)`
		// Minimum execution time: 57_000_000 picoseconds.
		Weight::from_parts(58_000_000, 4225)
			// Standard Error: 344_134
			.saturating_add(Weight::from_parts(54_852_962, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().reads((3_u64).saturating_mul(s.into())))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(s.into())))
			.saturating_add(Weight::from_parts(0, 3235).saturating_mul(s.into()))
	}
	/// Storage: `IdnManager::Subscriptions` (r:201 w:200)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:200 w:200)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(104), added: 2579, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:200 w:200)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::SubCounter` (r:1 w:1)
	/// Proof: `IdnManager::SubCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn on_finalize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `133118`
		//  Estimated: `651225`
		// Minimum execution time: 14_343_000_000 picoseconds.
		Weight::from_parts(14_742_000_000, 651225)
			.saturating_add(T::DbWeight::get().reads(602_u64))
			.saturating_add(T::DbWeight::get().writes(601_u64))
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	/// Storage: `IdnManager::SubCounter` (r:1 w:1)
	/// Proof: `IdnManager::SubCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn create_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42`
		//  Estimated: `4225`
		// Minimum execution time: 85_000_000 picoseconds.
		Weight::from_parts(88_000_000, 4225)
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	fn pause_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `243`
		//  Estimated: `4225`
		// Minimum execution time: 13_000_000 picoseconds.
		Weight::from_parts(14_000_000, 4225)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::SubCounter` (r:1 w:1)
	/// Proof: `IdnManager::SubCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn kill_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `301`
		//  Estimated: `4225`
		// Minimum execution time: 77_000_000 picoseconds.
		Weight::from_parts(78_000_000, 4225)
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// The range of component `m` is `[0, 8]`.
	fn update_subscription(m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `301`
		//  Estimated: `4225`
		// Minimum execution time: 52_000_000 picoseconds.
		Weight::from_parts(55_497_244, 4225)
			// Standard Error: 12_328
			.saturating_add(Weight::from_parts(20_669, 0).saturating_mul(m.into()))
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	fn reactivate_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `243`
		//  Estimated: `4225`
		// Minimum execution time: 13_000_000 picoseconds.
		Weight::from_parts(14_000_000, 4225)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn quote_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 7_000_000 picoseconds.
		Weight::from_parts(8_000_000, 0)
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:0)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	fn get_subscription_info() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `243`
		//  Estimated: `4225`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 4225)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:2001 w:2000)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:2000 w:2000)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2000 w:2000)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(104), added: 2579, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[1, 2000]`.
	fn dispatch_pulse(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `396 + s * (343 ±0)`
		//  Estimated: `4225 + s * (3235 ±0)`
		// Minimum execution time: 57_000_000 picoseconds.
		Weight::from_parts(58_000_000, 4225)
			// Standard Error: 344_134
			.saturating_add(Weight::from_parts(54_852_962, 0).saturating_mul(s.into()))
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().reads((3_u64).saturating_mul(s.into())))
			.saturating_add(RocksDbWeight::get().writes((3_u64).saturating_mul(s.into())))
			.saturating_add(Weight::from_parts(0, 3235).saturating_mul(s.into()))
	}
	/// Storage: `IdnManager::Subscriptions` (r:201 w:200)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(760), added: 3235, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:200 w:200)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(104), added: 2579, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:200 w:200)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::SubCounter` (r:1 w:1)
	/// Proof: `IdnManager::SubCounter` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn on_finalize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `133118`
		//  Estimated: `651225`
		// Minimum execution time: 14_343_000_000 picoseconds.
		Weight::from_parts(14_742_000_000, 651225)
			.saturating_add(RocksDbWeight::get().reads(602_u64))
			.saturating_add(RocksDbWeight::get().writes(601_u64))
	}
}