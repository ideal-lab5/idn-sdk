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
//! DATE: 2025-03-18, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `DESKTOP-RN9BJOQ`, CPU: `Intel(R) Core(TM) i7-9700KF CPU @ 3.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: `1024`

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --runtime
// ./target/release/wbuild/solochain-template-runtime/solochain_template_runtime.compact.compressed.wasm
// --pallet
// pallet_randomness_beacon
// --extrinsic
// 
// --template
// frame-weight-template.hbs
// --output
// ../../pallets/randomness-beacon/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]
#![allow(dead_code)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

/// Weight functions needed for `pallet_randomness_beacon`.
pub trait WeightInfo {
	fn try_submit_asig() -> Weight;
	fn on_finalize() -> Weight;
}

/// Weights for `pallet_randomness_beacon` using the Substrate node and recommended hardware.
pub struct SubstrateWeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeightInfo<T> {
	/// Storage: `RandBeacon::DidUpdate` (r:1 w:1)
	/// Proof: `RandBeacon::DidUpdate` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::GenesisRound` (r:1 w:1)
	/// Proof: `RandBeacon::GenesisRound` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::LatestRound` (r:1 w:1)
	/// Proof: `RandBeacon::LatestRound` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::AggregatedSignature` (r:1 w:1)
	/// Proof: `RandBeacon::AggregatedSignature` (`max_values`: Some(1), `max_size`: Some(98), added: 593, mode: `MaxEncodedLen`)
	fn try_submit_asig() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6`
		//  Estimated: `1583`
		// Minimum execution time: 20_709_419_000 picoseconds.
		Weight::from_parts(20_951_654_000, 1583)
			.saturating_add(T::DbWeight::get().reads(4_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	fn on_finalize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `117`
		//  Estimated: `0`
		// Minimum execution time: 8_781_000 picoseconds.
		Weight::from_parts(9_068_000, 0)
	}
}

// For backwards compatibility and tests.
impl WeightInfo for () {
	/// Storage: `RandBeacon::DidUpdate` (r:1 w:1)
	/// Proof: `RandBeacon::DidUpdate` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::GenesisRound` (r:1 w:1)
	/// Proof: `RandBeacon::GenesisRound` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::LatestRound` (r:1 w:1)
	/// Proof: `RandBeacon::LatestRound` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `RandBeacon::AggregatedSignature` (r:1 w:1)
	/// Proof: `RandBeacon::AggregatedSignature` (`max_values`: Some(1), `max_size`: Some(98), added: 593, mode: `MaxEncodedLen`)
	fn try_submit_asig() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6`
		//  Estimated: `1583`
		// Minimum execution time: 20_709_419_000 picoseconds.
		Weight::from_parts(20_951_654_000, 1583)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
	fn on_finalize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `117`
		//  Estimated: `0`
		// Minimum execution time: 8_781_000 picoseconds.
		Weight::from_parts(9_068_000, 0)
	}
}