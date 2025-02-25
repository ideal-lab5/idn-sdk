/*
 * Copyright 2024 by Ideal Labs, LLC
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

use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use serde::{Deserialize, Serialize};

/// Represents an opaque public key used in drand's quicknet
pub type OpaquePublicKey = BoundedVec<u8, ConstU32<96>>;
/// Represents an element of the signature group
pub type OpaqueSignature = BoundedVec<u8, ConstU32<48>>;
/// an opaque bounded storage type for 64 bit hashes
pub type OpaqueHash = BoundedVec<u8, ConstU32<64>>;
/// the round number to track rounds of the beacon
pub type RoundNumber = u64;

/// Represents an aggregated signature and aggregated public key pair
#[derive(
	Clone,
	Debug,
	Decode,
	Default,
	PartialEq,
	Encode,
	// Serialize,
	// Deserialize,
	MaxEncodedLen,
	TypeInfo,
)]
pub struct Aggregate {
	/// A signature (e.g. output from the randomness beacon) in G1
	pub signature: OpaqueSignature,
	/// The message signed by the signature, hashed to G1
	pub message_hash: OpaqueSignature,
}

/// A drand chain configuration
#[derive(
	Clone,
	Debug,
	Decode,
	Default,
	PartialEq,
	Encode,
	Serialize,
	Deserialize,
	MaxEncodedLen,
	TypeInfo,
)]
pub struct BeaconConfiguration {
	pub public_key: OpaquePublicKey,
	pub period: u32,
	pub genesis_time: u32,
	pub hash: OpaqueHash,
	pub group_hash: OpaqueHash,
	pub scheme_id: OpaqueHash,
	pub metadata: Metadata,
}

/// metadata for the drand beacon configuration
#[derive(
	Clone,
	Debug,
	Decode,
	Default,
	PartialEq,
	Encode,
	Serialize,
	Deserialize,
	MaxEncodedLen,
	TypeInfo,
)]
pub struct Metadata {
	pub beacon_id: OpaqueHash,
}
