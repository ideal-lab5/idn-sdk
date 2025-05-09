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

//! # IDN Traits
//!
//! Core traits for the Ideal Network (IDN) ecosystem.
//!
//! This crate provides fundamental interfaces for handling randomness pulses.
//!
//! ## Modules
//!
//! * [`pulse`] - Traits and types for randomness pulses handling and distribution
//!
//! ## Overview
//!
//! The IDN traits define the foundational interfaces that allow different
//! components of the system to interact in a standardized way. These traits
//! enable a modular architecture where randomness sources, dispatchers, and
//! consumers can all operate together seamlessly.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod pulse;

use frame_support::pallet_prelude::Encode;
use sp_core::H256;
use sp_io::hashing::blake2_256;
/// Trait for hashing
pub trait Hashable {
	fn hash(&self, salt: Vec<u8>) -> H256;
}
impl<T> Hashable for T
where
	T: Encode,
{
	fn hash(&self, salt: Vec<u8>) -> H256 {
		let id_tuple = (self, salt);
		// Encode the tuple using SCALE codec.
		let encoded = id_tuple.encode();
		// Hash the encoded bytes using blake2_256.
		H256::from_slice(&blake2_256(&encoded))
	}
}
