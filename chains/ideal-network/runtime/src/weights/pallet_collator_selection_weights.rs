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

//! Autogenerated weights for `pallet_collator_selection`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 47.0.0
//! DATE: 2025-05-15, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `MacBook-Pro-3.local`, CPU: `<UNKNOWN>`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --extrinsic=*
// --runtime=target/release/wbuild/idn-runtime/idn_runtime.wasm
// --pallet=pallet_collator_selection
// --header=/Users/juan/Projects/IdealLabs/idn-sdk/scripts/file_header.txt
// --output=./chains/ideal-network/runtime/src/weights
// --wasm-execution=compiled
// --steps=50
// --repeat=20
// --heap-pages=4096

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_collator_selection`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_collator_selection::WeightInfo for WeightInfo<T> {
	/// Storage: `Session::NextKeys` (r:20 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::Invulnerables` (r:0 w:1)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 20]`.
	fn set_invulnerables(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `197 + b * (79 ±0)`
		//  Estimated: `1188 + b * (2555 ±0)`
		// Minimum execution time: 13_000_000 picoseconds.
		Weight::from_parts(11_017_602, 0)
			.saturating_add(Weight::from_parts(0, 1188))
			// Standard Error: 9_062
			.saturating_add(Weight::from_parts(3_727_784, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(Weight::from_parts(0, 2555).saturating_mul(b.into()))
	}
	/// Storage: `Session::NextKeys` (r:1 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:1)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 19]`.
	/// The range of component `c` is `[1, 99]`.
	fn add_invulnerable(b: u32, c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `758 + b * (32 ±0) + c * (53 ±0)`
		//  Estimated: `6287 + b * (37 ±0) + c * (53 ±0)`
		// Minimum execution time: 42_000_000 picoseconds.
		Weight::from_parts(40_908_677, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 17_038
			.saturating_add(Weight::from_parts(180_880, 0).saturating_mul(b.into()))
			// Standard Error: 3_229
			.saturating_add(Weight::from_parts(60_912, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 37).saturating_mul(b.into()))
			.saturating_add(Weight::from_parts(0, 53).saturating_mul(c.into()))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:1)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[5, 20]`.
	fn remove_invulnerable(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `82 + b * (32 ±0)`
		//  Estimated: `6287`
		// Minimum execution time: 11_000_000 picoseconds.
		Weight::from_parts(11_831_819, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 4_216
			.saturating_add(Weight::from_parts(31_462, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `CollatorSelection::DesiredCandidates` (r:0 w:1)
	/// Proof: `CollatorSelection::DesiredCandidates` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_desired_candidates() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_000_000 picoseconds.
		Weight::from_parts(6_000_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:100 w:100)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:100)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[0, 100]`.
	/// The range of component `k` is `[0, 100]`.
	fn set_candidacy_bond(c: u32, k: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + c * (182 ±0) + k * (115 ±0)`
		//  Estimated: `6287 + c * (901 ±29) + k * (901 ±29)`
		// Minimum execution time: 10_000_000 picoseconds.
		Weight::from_parts(10_000_000, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 180_411
			.saturating_add(Weight::from_parts(5_858_931, 0).saturating_mul(c.into()))
			// Standard Error: 180_411
			.saturating_add(Weight::from_parts(5_773_286, 0).saturating_mul(k.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(c.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(k.into())))
			.saturating_add(Weight::from_parts(0, 901).saturating_mul(c.into()))
			.saturating_add(Weight::from_parts(0, 901).saturating_mul(k.into()))
	}
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[3, 100]`.
	fn update_bond(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `282 + c * (49 ±0)`
		//  Estimated: `6287`
		// Minimum execution time: 28_000_000 picoseconds.
		Weight::from_parts(30_563_668, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 3_526
			.saturating_add(Weight::from_parts(24_075, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `Session::NextKeys` (r:1 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:1)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[1, 99]`.
	fn register_as_candidate(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `728 + c * (52 ±0)`
		//  Estimated: `6287 + c * (54 ±0)`
		// Minimum execution time: 40_000_000 picoseconds.
		Weight::from_parts(43_490_280, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 2_861
			.saturating_add(Weight::from_parts(25_954, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 54).saturating_mul(c.into()))
	}
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `Session::NextKeys` (r:1 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:2)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[3, 100]`.
	fn take_candidate_slot(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `868 + c * (53 ±0)`
		//  Estimated: `6287 + c * (54 ±0)`
		// Minimum execution time: 57_000_000 picoseconds.
		Weight::from_parts(58_638_419, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 3_819
			.saturating_add(Weight::from_parts(63_574, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 54).saturating_mul(c.into()))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:1)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[3, 100]`.
	fn leave_intent(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `310 + c * (48 ±0)`
		//  Estimated: `6287`
		// Minimum execution time: 29_000_000 picoseconds.
		Weight::from_parts(31_277_027, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 3_445
			.saturating_add(Weight::from_parts(57_290, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:1)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	fn note_author() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `155`
		//  Estimated: `6196`
		// Minimum execution time: 45_000_000 picoseconds.
		Weight::from_parts(47_000_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:100 w:0)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::DesiredCandidates` (r:1 w:0)
	/// Proof: `CollatorSelection::DesiredCandidates` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:97 w:97)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 100]`.
	/// The range of component `c` is `[1, 100]`.
	fn new_session(r: u32, c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2265 + c * (97 ±0) + r * (114 ±0)`
		//  Estimated: `6287 + c * (2519 ±0) + r * (2603 ±0)`
		// Minimum execution time: 24_000_000 picoseconds.
		Weight::from_parts(24_000_000, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 326_858
			.saturating_add(Weight::from_parts(14_032_911, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(c.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(c.into())))
			.saturating_add(Weight::from_parts(0, 2519).saturating_mul(c.into()))
			.saturating_add(Weight::from_parts(0, 2603).saturating_mul(r.into()))
	}
}
