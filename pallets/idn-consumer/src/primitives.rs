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

use frame_support::{pallet_prelude::Get, traits::Contains};
use sp_std::marker::PhantomData;
use xcm::prelude::{Junction, Location};

/// XCM filter for allowing only a particular sibling parachains or accounts of that parachain
/// (usually IDN) to call certain functions in the Consumer.
pub struct AllowSiblingOnly<SiblingParaId>(PhantomData<SiblingParaId>);
impl<SiblingParaId: Get<u32>> Contains<Location> for AllowSiblingOnly<SiblingParaId> {
	fn contains(location: &Location) -> bool {
		match location.unpack() {
			(1, [Junction::Parachain(para_id)]) => *para_id == SiblingParaId::get(),
			(1, [Junction::Parachain(para_id), Junction::AccountId32 { .. }]) =>
				*para_id == SiblingParaId::get(),
			_ => false,
		}
	}
}
