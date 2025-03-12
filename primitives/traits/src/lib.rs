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
//! * [`rand`] - Traits and types for randomness handling and distribution
//!
//! ## Overview
//!
//! The IDN traits define the foundational interfaces that allow different
//! components of the system to interact in a standardized way. These traits
//! enable a modular architecture where randomness sources, dispatchers, and
//! consumers can all operate together seamlessly.

#![cfg_attr(not(feature = "std"), no_std)]

/// Randomness-related traits and types for the IDN ecosystem
///
/// This module contains all the traits and types needed for working with
/// randomness in the IDN system, including:
///
/// * [`crate::rand::Pulse`] - Core trait for randomness beacon pulses
/// * [`crate::rand::PulseMatch`] - Extension trait for filtering pulses by properties
/// * [`crate::rand::Dispatcher`] - Trait for handling and distributing randomness
/// * [`crate::rand::PulseProperty`] - Enum for referencing pulse properties in a type-safe way
pub mod rand {
	use frame_support::pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo};
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
		/// Process and dispatch the given random data from a pulse.
		///
		/// # Parameters
		/// * `pulse` - The pulse containing random data to be processed
		///
		/// # Returns
		/// The result of processing the random data, type depends on implementation
		fn dispatch(pulse: P) -> O;
	}

	/// An enum representing properties of a randomness pulse
	///
	/// This enum allows systems to refer to the properties of a pulse in a type-safe way. It's
	/// commonly used in filtering logic to specify which property and value subscriptions should
	/// match against.
	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Clone)]
	pub enum PulseProperty<RandType, RoundType> {
		/// The random value for a pulse.
		Rand(RandType),
		/// The round number for a pulse.
		Round(RoundType),
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
	///
	/// ## Usage
	/// This trait is used throughout the IDN ecosystem to:
	/// 1. Provide a consistent interface for different randomness sources
	/// 2. Enable subscriptions to filter pulses based on specific properties
	/// 3. Allow the secure distribution of randomness through XCM
	pub trait Pulse {
		/// The type of the random value contained in this pulse
		///
		/// This is typically a fixed-size byte array like `[u8; 32]` that represents
		/// the random value.
		type Rand: Decode + TypeInfo + MaxEncodedLen + Debug + PartialEq + Clone;

		/// The type of the round number contained in this pulse
		///
		/// This is typically an unsigned integer that represents the sequential
		/// identifier for this pulse from the randomness beacon.
		type Round: Decode + TypeInfo + MaxEncodedLen + Debug + PartialEq + Clone;

		/// Get the random value from this pulse
		///
		/// Returns the random value contained in this pulse.
		fn rand(&self) -> Self::Rand;

		/// Get the round number from this pulse
		///
		/// Returns the sequential identifier for this pulse in the randomness beacon sequence.
		/// Round numbers typically increase monotonically.
		fn round(&self) -> Self::Round;
	}

	/// A trait for matching pulse properties against specific values
	///
	/// This trait extends the basic [`Pulse`] trait with the ability to match
	/// specific properties (random values or round numbers) against the pulse's
	/// actual values. It provides a default implementation that performs equality
	/// checks for each property type.
	///
	/// ## Usage
	/// This trait is primarily used in the filtering system to determine whether
	/// a pulse matches specified criteria. For example, a subscription might want
	/// to only receive randomness from specific rounds.
	///
	/// ## Default Implementation
	/// The default implementation provides simple equality checking:
	/// - For `PulseProperty::Rand`, it checks if the pulse's random value equals the provided value
	/// - For `PulseProperty::Round`, it checks if the pulse's round number equals the provided
	///   value
	///
	/// ## Example
	/// ```rust
	/// use idn_traits::rand::{Pulse, PulseMatch, PulseProperty};
	/// struct MyPulse {
	///     rand: [u8; 3],
	///     round: u8,
	/// }
	/// impl Pulse for MyPulse {
	///     type Rand = [u8; 3];
	///     type Round = u8;
	///     fn rand(&self) -> Self::Rand { self.rand }
	///     fn round(&self) -> Self::Round { self.round }
	/// }
	///
	/// let my_pulse = MyPulse { rand: [1, 2, 3], round: 42 };
	///
	/// // Check if pulse matches a specific round
	/// assert!(my_pulse.match_prop(PulseProperty::Round(42)));
	///
	/// // Check if pulse matches an invalid round
	/// assert!(!my_pulse.match_prop(PulseProperty::Round(43)));
	///
	/// // Check if pulse matches a specific random value
	/// assert!(my_pulse.match_prop(PulseProperty::Rand([1, 2, 3])));
	/// ```
	///
	/// ## Customization
	/// Types implementing this trait can override the default implementation to provide
	/// more sophisticated matching logic, such as range-based matching or pattern matching.
	pub trait PulseMatch: Pulse {
		/// Checks whether this pulse matches the provided property value
		///
		/// # Parameters
		/// * `prop` - The property to check against this pulse
		///
		/// # Returns
		/// * `true` - If the pulse matches the property value
		/// * `false` - If the pulse does not match the property value
		fn match_prop(&self, prop: PulseProperty<Self::Rand, Self::Round>) -> bool {
			match prop {
				PulseProperty::Rand(rand) => self.rand() == rand,
				PulseProperty::Round(round) => self.round() == round,
			}
		}
	}

	/// Blanket implementation of PulseMatch for all types that implement Pulse.
	///
	/// This provides PulseMatch functionality for any type implementing the Pulse trait. It
	/// ensures all pulse types can be filtered with the default equality-based matching logic
	/// without requiring additional implementation work.
	impl<T: Pulse> PulseMatch for T {}
}
