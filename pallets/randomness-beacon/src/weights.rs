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


//! Autogenerated weights for `pallet_randomness_beacon`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 46.0.0
//! DATE: 2025-04-15, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// pallet-randomness-beacon
// --extrinsic
// 
// --template
// ./weight-template.hbs
// --output
// ../../../pallets/randomness-beacon/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]
#![allow(dead_code)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pallet_randomness_beacon`.
pub trait WeightInfo {
	fn try_submit_asig(r: u32, ) -> Weight;
	fn on_finalize() -> Weight;
	fn set_beacon_config() -> Weight;
}

/// Weights for `pallet_randomness_beacon` using the IDN SDK Kitchensink Runtime and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	/// Storage: `RandBeacon::DidUpdate` (r:1 w:1)
	/// Proof: `RandBeacon::DidUpdate` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::BeaconConfig` (r:1 w:0)
	/// Proof: `RandBeacon::BeaconConfig` (`max_values`: Some(1), `max_size`: Some(104), added: 599, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::LatestRound` (r:1 w:1)
	/// Proof: `RandBeacon::LatestRound` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::SparseAccumulation` (r:1 w:1)
	/// Proof: `RandBeacon::SparseAccumulation` (`max_values`: Some(1), `max_size`: Some(96), added: 591, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::Subscriptions` (r:1 w:0)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5663), added: 8138, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[2, 30]`.
	fn try_submit_asig(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `205`
		//  Estimated: `9128`
		// Minimum execution time: 11_386_000_000 picoseconds.
		Weight::from_parts(6_767_076_781, 9128)
			// Standard Error: 978_617
			.saturating_add(Weight::from_parts(2_287_119_333, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(5_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `RandBeacon::DidUpdate` (r:1 w:1)
	/// Proof: `RandBeacon::DidUpdate` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::BeaconConfig` (r:1 w:0)
	/// Proof: `RandBeacon::BeaconConfig` (`max_values`: Some(1), `max_size`: Some(104), added: 599, mode: `MaxEncodedLen`)
	fn on_finalize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `63`
		//  Estimated: `1589`
		// Minimum execution time: 4_000_000 picoseconds.
		Weight::from_parts(5_000_000, 1589)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
	/// Storage: `RandBeacon::BeaconConfig` (r:1 w:1)
	/// Proof: `RandBeacon::BeaconConfig` (`max_values`: Some(1), `max_size`: Some(104), added: 599, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::LatestRound` (r:0 w:1)
	/// Proof: `RandBeacon::LatestRound` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn set_beacon_config() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6`
		//  Estimated: `1589`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(9_000_000, 1589)
			.saturating_add(T::DbWeight::get().reads(1_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	/// Storage: `RandBeacon::DidUpdate` (r:1 w:1)
	/// Proof: `RandBeacon::DidUpdate` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::BeaconConfig` (r:1 w:0)
	/// Proof: `RandBeacon::BeaconConfig` (`max_values`: Some(1), `max_size`: Some(104), added: 599, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::LatestRound` (r:1 w:1)
	/// Proof: `RandBeacon::LatestRound` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::SparseAccumulation` (r:1 w:1)
	/// Proof: `RandBeacon::SparseAccumulation` (`max_values`: Some(1), `max_size`: Some(96), added: 591, mode: `MaxEncodedLen`)
	/// Storage: `IdnManager::Subscriptions` (r:1 w:0)
	/// Proof: `IdnManager::Subscriptions` (`max_values`: None, `max_size`: Some(5663), added: 8138, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[2, 30]`.
	fn try_submit_asig(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `205`
		//  Estimated: `9128`
		// Minimum execution time: 11_386_000_000 picoseconds.
		Weight::from_parts(6_767_076_781, 9128)
			// Standard Error: 978_617
			.saturating_add(Weight::from_parts(2_287_119_333, 0).saturating_mul(r.into()))
			.saturating_add(RocksDbWeight::get().reads(5_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	/// Storage: `RandBeacon::DidUpdate` (r:1 w:1)
	/// Proof: `RandBeacon::DidUpdate` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::BeaconConfig` (r:1 w:0)
	/// Proof: `RandBeacon::BeaconConfig` (`max_values`: Some(1), `max_size`: Some(104), added: 599, mode: `MaxEncodedLen`)
	fn on_finalize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `63`
		//  Estimated: `1589`
		// Minimum execution time: 4_000_000 picoseconds.
		Weight::from_parts(5_000_000, 1589)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	/// Storage: `RandBeacon::BeaconConfig` (r:1 w:1)
	/// Proof: `RandBeacon::BeaconConfig` (`max_values`: Some(1), `max_size`: Some(104), added: 599, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::LatestRound` (r:0 w:1)
	/// Proof: `RandBeacon::LatestRound` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn set_beacon_config() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6`
		//  Estimated: `1589`
		// Minimum execution time: 9_000_000 picoseconds.
		Weight::from_parts(9_000_000, 1589)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
			.saturating_add(RocksDbWeight::get().writes(2_u64))
	}
}