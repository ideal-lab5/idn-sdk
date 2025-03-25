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
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 46.0.0
//! DATE: 2025-03-24, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `MacBook-Pro-3.local`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: `1024`

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --runtime
// ../../target/release/wbuild/idn-sdk-kitchensink-runtime/idn_sdk_kitchensink_runtime.compact.compressed.wasm
// --pallet
// pallet-idn-manager
// --extrinsic
// 
// --template
// ../../kitchensink/benchmarking/weight-template.hbs
// --output
// src/weights.rs

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
	fn update_subscription() -> Weight;
	fn reactivate_subscription() -> Weight;
}

/// Weights for `pallet_idn_manager` using the IDN SDK Kitchensink Runtime and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	fn create_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42`
		//  Estimated: `9095`
		// Minimum execution time: 92_000_000 picoseconds.
		Weight::from_parts(94_000_000, 9095)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	fn pause_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `189`
		//  Estimated: `9095`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 9095)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn kill_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `247`
		//  Estimated: `9095`
		// Minimum execution time: 77_000_000 picoseconds.
		Weight::from_parts(79_000_000, 9095)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn update_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `247`
		//  Estimated: `9095`
		// Minimum execution time: 77_000_000 picoseconds.
		Weight::from_parts(78_000_000, 9095)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	fn reactivate_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `189`
		//  Estimated: `9095`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 9095)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	fn create_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42`
		//  Estimated: `9095`
		// Minimum execution time: 92_000_000 picoseconds.
		Weight::from_parts(94_000_000, 9095)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	fn pause_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `189`
		//  Estimated: `9095`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 9095)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn kill_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `247`
		//  Estimated: `9095`
		// Minimum execution time: 77_000_000 picoseconds.
		Weight::from_parts(79_000_000, 9095)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn update_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `247`
		//  Estimated: `9095`
		// Minimum execution time: 77_000_000 picoseconds.
		Weight::from_parts(78_000_000, 9095)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
	/// Storage: `IdnManager::Subscriptions` (r:1 w:1)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5630), added: 8105, mode: `MaxEncodedLen`)
	fn reactivate_subscription() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `189`
		//  Estimated: `9095`
		// Minimum execution time: 14_000_000 picoseconds.
		Weight::from_parts(15_000_000, 9095)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
}