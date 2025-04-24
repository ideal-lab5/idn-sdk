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

//! Primitives for the IDN runtime.

pub mod types;

use crate::Runtime;
use frame_support::pallet_prelude::{Decode, Encode, TypeInfo};

pub use pallet_idn_manager::{
	primitives::IdnManagerCall, BlockNumberFor, CreateSubParamsOf, MetadataOf, PulseFilterOf,
	SubscriptionIdOf, UpdateSubParamsOf,
};

#[derive(Encode, Decode, Debug, PartialEq, Clone, TypeInfo)]
pub enum Call {
	// This should match the index of the IDN Manager pallet in the runtime
	#[codec(index = 40)]
	IdnManager(
		IdnManagerCall<
			CreateSubParamsOf<Runtime>,
			UpdateSubParamsOf<Runtime>,
			SubscriptionIdOf<Runtime>,
		>,
	),
}

#[cfg(test)]
mod tests {
	use super::*;

	pub fn get_idn_manager_pallet_index() -> usize {
		<pallet_idn_manager::Pallet<Runtime> as frame_support::traits::PalletInfoAccess>::index()
	}

	/// Makes sure the call enum has the correct index.
	#[test]
	fn test_call_enum_has_correct_index() {
		let call = Call::IdnManager(IdnManagerCall::create_subscription {
			params: CreateSubParamsOf::<Runtime>::default(),
		});
		assert_eq!(get_idn_manager_pallet_index(), call.encode()[0] as usize);
	}
}
