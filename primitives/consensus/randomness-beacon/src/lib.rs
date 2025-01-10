/*
 * Copyright 2024 by Ideal Labs, LLC
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
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod inherents;
pub mod types;

// /// provides timelock encryption using the current slot
// pub trait TimelockEncryptionProvider<BN> {
// 	/// attempt to decrypt the ciphertext with the current slot secret
// 	fn decrypt_at(ciphertext: &[u8], block_number: BN) -> Result<DecryptionResult, TimelockError>;

// 	/// get the latest block number for which randomness is known
// 	fn latest() -> BN;
// }
