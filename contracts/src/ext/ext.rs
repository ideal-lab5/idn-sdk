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

use ink::env::Environment;
// use pallet_randomness_beacon::TemporalDirection;

#[ink::chain_extension(extension = 42)]
pub trait RandExtension {
	type ErrorCode = RandomReadErr;
	// 1101 = chain extension func id on the target runtime (IDN)
	#[ink(function = 1101)]
	fn fetch_random(subject: [u8; 32]) -> [u8; 32];
	// // 1102 = chain extension func id on the tareget chain (IDN)
	// #[ink(function = 1102)]
	// fn check_time(round_number: u64) -> TemporalDirection;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum RandomReadErr {
	FailGetRandomSource,
}

impl ink::env::chain_extension::FromStatusCode for RandomReadErr {
	fn from_status_code(status_code: u32) -> Result<(), Self> {
		match status_code {
			0 => Ok(()),
			1 => Err(Self::FailGetRandomSource),
			_ => panic!("encountered unknown status code"),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[ink::scale_derive(TypeInfo)]
pub enum IDNEnvironment {}

impl Environment for IDNEnvironment {
	const MAX_EVENT_TOPICS: usize = <ink::env::DefaultEnvironment as Environment>::MAX_EVENT_TOPICS;

	type AccountId = <ink::env::DefaultEnvironment as Environment>::AccountId;
	type Balance = <ink::env::DefaultEnvironment as Environment>::Balance;
	type Hash = <ink::env::DefaultEnvironment as Environment>::Hash;
	type BlockNumber = <ink::env::DefaultEnvironment as Environment>::BlockNumber;
	type Timestamp = <ink::env::DefaultEnvironment as Environment>::Timestamp;

	type ChainExtension = RandExtension;
}
