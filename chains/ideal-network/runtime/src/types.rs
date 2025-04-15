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

//! Types for the IDN runtime.

use codec::{Decode, Encode};
use scale_info::TypeInfo;

// TODO: correctly define these types https://github.com/ideal-lab5/idn-sdk/issues/186

type Rand = [u8; 32];
type Round = u64;
type Sig = [u8; 48];

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug, TypeInfo)]
pub struct Pulse {
	pub rand: Rand,
	pub round: Round,
	pub sig: Sig,
}

impl sp_idn_traits::pulse::Pulse for Pulse {
	type Rand = Rand;
	type Round = Round;
	type Sig = Sig;

	fn rand(&self) -> Self::Rand {
		self.rand
	}

	fn round(&self) -> Self::Round {
		self.round
	}

	fn sig(&self) -> Self::Sig {
		self.sig
	}

	fn valid(&self) -> bool {
		true
	}
}
