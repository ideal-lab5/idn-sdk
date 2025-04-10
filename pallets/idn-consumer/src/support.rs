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

use idn_runtime::Runtime as IdnRuntime;
use pallet_idn_manager::Pallet as IdnManagerPallet;

/// Helper function to get the index of the IDN manager pallet.
///
/// This is useful for the consumers to get the first byte of the
/// [`pallet_idn_manager::primitives::CreateSubParams::call_index`] for the XCM message to
/// interact with the IDN manager pallet.
pub fn get_idn_manager_pallet_index() -> usize {
	<IdnManagerPallet<IdnRuntime> as frame_support::traits::PalletInfoAccess>::index()
}
