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

//! Randomness-related traits and types for the IDN ecosystem
//!
//! This module contains all the traits and types needed for working with
//! randomness in the IDN system, including:
//!
//! * [`crate::pulse::Pulse`] - Core trait for randomness beacon pulses
//! * [`crate::pulse::Dispatcher`] - Trait for handling and distributing randomness

use codec::EncodeLike;
use frame_support::pallet_prelude::{Decode, MaxEncodedLen, TypeInfo, Weight};
use sp_std::fmt::Debug;

/// A trait for dispatching random data from pulses.
///
/// This trait provides a simple interface for components to handle incoming
/// randomness pulses and process them in a uniform way.
///
/// # Type Parameters
/// * `P: Pulse` - The type of pulse containing random data
/// * `O` - The output type returned after processing the pulse
///
/// # Usage
/// Typically implemented by modules that need to react to new random values,
/// such as the IDN Manager which distributes randomness to subscribers.
pub trait Dispatcher<P: Pulse, O> {
	/// Process and dispatch a pulse.
	///
	/// # Parameters
	/// * `pulse` - A pulse to be processed
	///
	/// # Returns
	/// The result of processing the pulse, type depends on implementation
	fn dispatch(pulse: P) -> O;

	/// Returns the weight of dispatching a pulse.
	fn dispatch_weight() -> Weight;
}

/// A trait defining the interface for randomness beacon pulses
///
/// This trait represents the fundamental behavior of a randomness beacon pulse,
/// which contains both a random value and a round number. Implementers of this
/// trait can be integrated with the IDN Manager for verifiable randomness distribution.
///
/// ## Type Parameters
/// - `Rand`: The type representing the random value
/// - `Round`: The type representing the round number
/// - `Sig`: The type representing the signature
///
/// ## Usage
/// This trait is used throughout the IDN ecosystem to:
/// 1. Provide a consistent interface for different randomness sources
/// 2. Enable subscriptions to filter pulses based on specific properties
/// 3. Allow the secure distribution of randomness through XCM
///
/// ## Example
/// ```rust
/// use sp_idn_traits::pulse::{Pulse};
/// struct MyPulse {
///     rand: [u8;3],
///     message: [u8;8],
///     signature: [u8;8],
/// }
/// impl Pulse for MyPulse {
///     type Rand = [u8;3];
///     type Sig = [u8;8];
///     type Pubkey = [u8;8];
///
///     fn rand(&self) -> Self::Rand { self.rand }
///     fn message(&self) -> Self::Sig { self.message }
///     fn sig(&self) -> Self::Sig { self.signature }
///     fn authenticate(&self, pubkey: Self::Pubkey) -> bool { true }
/// }
///
/// let my_pulse = MyPulse { rand: [1, 2, 3], message: [5, 4, 3, 2, 1, 2, 3, 4], signature: [1, 2, 3, 4, 5, 6, 7, 8] };
pub trait Pulse {
	/// The type of the random value contained in this pulse
	///
	/// This is typically a fixed-size byte array like `[u8; 32]` that represents
	/// the random value.
	type Rand: Decode
		+ TypeInfo
		+ MaxEncodedLen
		+ Debug
		+ PartialEq
		+ PartialOrd
		+ Clone
		+ EncodeLike;

	/// The type of the signature contained in this pulse
	///
	/// This is typically a fixed-size byte array, e.g. `[u8;48]` or a specific signature type
	/// that represents the signature for this pulse.
	type Sig: Decode
		+ TypeInfo
		+ MaxEncodedLen
		+ Debug
		+ PartialEq
		+ Clone
		+ EncodeLike
		+ AsRef<[u8]>;

	/// The type of the public key required to verify this signature
	///
	/// This is typically a fixed-size byte array, e.g. `[u8; 96]` or a specific public key type
	type Pubkey: Decode
		+ TypeInfo
		+ MaxEncodedLen
		+ Debug
		+ PartialEq
		+ Clone
		+ EncodeLike
		+ AsRef<[u8]>;

	/// Get the random value from this pulse
	///
	/// Returns the random value contained in this pulse.
	fn rand(&self) -> Self::Rand;

	/// Get the aggregated message
	///
	/// Returns the aggregated messages used to construct consecutive sigs from randomness beacon
	/// Messages are typically constructed by aggregating the hash of a monotonically increasing
	/// sequence starting at the network's known 'genesis' round up until the latest. That is, the
	/// message here looks like: H(r_0) + H(r_1) + ... + H(r_n) where r_1, ..., r_n are sequential
	/// round numbers and H is a crypto hash function (ex. sha256).
	fn message(&self) -> Self::Sig;

	/// Get the signature from this pulse
	///
	/// Returns the signature contained in this pulse.
	fn sig(&self) -> Self::Sig;

	/// Checks if the pulse ([message, signature]-combination) is valid against a given beacon
	/// public key
	///
	/// Returns `true` when valid, `false` otherwise.
	fn authenticate(&self, pubkey: Self::Pubkey) -> bool;
}
