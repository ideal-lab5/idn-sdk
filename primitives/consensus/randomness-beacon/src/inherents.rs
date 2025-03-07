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

use alloc::vec::Vec;
use sp_inherents::{Error, InherentData, InherentIdentifier};

pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"rngpulse";

pub type InherentType = Vec<Vec<u8>>;

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
		None
	}
}

#[cfg(test)]
mod tests {

	use super::{InherentDataProvider as RandInherentDataProvider, *};
	use core::ops::Deref;
	use sp_inherents::InherentDataProvider;

	#[test]
	pub fn can_construct_inherent_data_provider() {
		let data: InherentType = vec![vec![1]];
		let provider = RandInherentDataProvider::new(data.clone());
		assert_eq!(&data, provider.deref())
	}

	#[tokio::test]
	pub async fn can_provide_inherent_data() {
		let extra_data = vec![vec![1]];
		let mut inherent_data = InherentData::new();
		let provider = RandInherentDataProvider::new(extra_data.clone());

		let res = provider.provide_inherent_data(&mut inherent_data).await;
		assert!(res.is_ok());

		let data = inherent_data.get_data::<Vec<Vec<u8>>>(&INHERENT_IDENTIFIER).unwrap().unwrap();
		assert_eq!(extra_data, data)
	}

	#[tokio::test]
	pub async fn try_handle_error_returns_none() {
		let extra_data = vec![vec![1]];
		let mut inherent_data = InherentData::new();
		let provider = RandInherentDataProvider::new(extra_data.clone());
		let res = provider.try_handle_error(&[1u8; 8], &vec![1]).await;
		assert!(res.is_none());
	}
}
