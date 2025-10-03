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

use std::fmt::Debug;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
	#[error("Extrinsic construction failed.")]
	ExtrinsicConstructionFailed,
	#[error("The signature is incorrectly sized: has {0} bytes, but must be {1}.")]
	InvalidSignatureSize(u8, u8),
	#[error("There are no authority keys available in the keystore.")]
	NoAuthorityKeys,
	#[error("The transaction failed to be included in the transaction pool.")]
	TransactionSubmissionFailed,
}

#[cfg(test)]
use sc_transaction_pool_api::error::{Error as PoolError, IntoPoolError};

#[cfg(test)]
impl From<PoolError> for Error {
	fn from(_err: PoolError) -> Self {
		Error::TransactionSubmissionFailed
	}
}

#[cfg(test)]
impl IntoPoolError for Error {
	fn into_pool_error(self) -> Result<PoolError, Self> {
		Err(Error::TransactionSubmissionFailed)
	}
}
