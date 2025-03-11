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
	use frame_support::traits::Get;

	/// A trait for dispatching random data.
	pub trait Dispatcher<P, O> {
		/// Dispatch the given random data.
		fn dispatch(pulse: P) -> O;
	}

	/// A trait to define the Pulse type behaviour
	pub trait Pulse {
		type Rand;
		type Round;
		type MaxSize: Get<u32>;
		/// Get the random value for this pulse.
		fn random(&self) -> Self::Rand;
		/// Get the round number for this pulse.
		fn round(&self) -> Self::Round;
		// / Filter by round
		// fn filter_by_round(&self, rounds: BoundedVec<Self::Round, Self::MaxSize::get()>) -> Self;
	}
}
