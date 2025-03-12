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
pub mod rand {
	use std::fmt::Debug;

	use frame_support::pallet_prelude::{Decode, Encode, MaxEncodedLen, TypeInfo};

	/// A trait for dispatching random data.
	pub trait Dispatcher<P: Pulse, O> {
		/// Dispatch the given random data.
		fn dispatch(pulse: P) -> O;
	}

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Clone)]
	pub enum PulseProperty<RandType, RoundType> {
		/// The random value for a pulse.
		Rand(RandType),
		/// The round number for a pulse.
		Round(RoundType),
	}

	/// A trait to define the Pulse type behaviour
	pub trait Pulse {
		type Rand: Encode + Decode + TypeInfo + MaxEncodedLen + Debug + PartialEq + Clone;
		type Round: Encode + Decode + TypeInfo + MaxEncodedLen + Debug + PartialEq + Clone;
		/// Get the random value for this pulse.
		fn rand(&self) -> Self::Rand;
		/// Get the round number for this pulse.
		fn round(&self) -> Self::Round;
		/// Get a property for this pulse.
		fn get(&self, property: PulseProperty<(), ()>) -> PulseProperty<Self::Rand, Self::Round> {
			match property {
				PulseProperty::Rand(_) => PulseProperty::Rand(self.rand()),
				PulseProperty::Round(_) => PulseProperty::Round(self.round()),
			}
		}
	}
}
