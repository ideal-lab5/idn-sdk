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

use sp_inherents::{Error, InherentData, InherentIdentifier};
use alloc::vec::Vec;

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"rngpulse";

// TODO
pub type InherentType = Vec<Vec<u8>>;

pub trait BeaconInherentData {
	// get beacon data for the inherent
	fn beacon_inherent_data(&self) -> Result<Option<InherentType>, Error>;
	// Replace beacon inherent data
	fn beacon_replace_inherent_data(&mut self, new: InherentType);
}

impl BeaconInherentData for InherentData {
	fn beacon_inherent_data(&self) -> Result<Option<InherentType>, Error> {
		self.get_data(&INHERENT_IDENTIFIER)
	}

	fn beacon_replace_inherent_data(&mut self, new: InherentType) {
		self.replace_data(INHERENT_IDENTIFIER, &new);
	}
}

#[cfg(feature = "std")]
pub struct InherentDataProvider {
	data: InherentType,
}

#[cfg(feature = "std")]
impl InherentDataProvider {
	pub fn new(data: InherentType) -> Self {
		Self { data }
	}
}

#[cfg(feature = "std")]
impl core::ops::Deref for InherentDataProvider {
	type Target = InherentType;

	fn deref(&self) -> &Self::Target {
		&self.data
	}
}

#[cfg(feature = "std")]
#[async_trait::async_trait]
impl sp_inherents::InherentDataProvider for InherentDataProvider {
	async fn provide_inherent_data(&self, inherent_data: &mut InherentData) -> Result<(), Error> {
		inherent_data.put_data(INHERENT_IDENTIFIER, &self.data)
	}

	async fn try_handle_error(
		&self,
		_: &InherentIdentifier,
		_: &[u8],
	) -> Option<Result<(), Error>> {
		// There is no error anymore
		None
	}
}
