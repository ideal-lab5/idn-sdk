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

//! Types of IDN runtime
use frame_support::{parameter_types, PalletId};
use sp_runtime::AccountId32;

pub use pallet_idn_manager::impls::{DepositCalculatorImpl, DiffBalanceImpl, FeesManagerImpl};
use pallet_idn_manager::primitives::{
	CreateSubParams as MngCreateSubParams, PulseFilter as MngPulseFilter,
	SubscriptionMetadata as IdnMetadata,
};
pub use sp_consensus_randomness_beacon::types::RuntimePulse;

// TODO: correctly define these types https://github.com/ideal-lab5/idn-sdk/issues/186
// Primitive types
parameter_types! {
	pub const IdnManagerPalletId: PalletId = PalletId(*b"idn_mngr");
	pub const TreasuryAccount: AccountId32 = AccountId32::new([123u8; 32]);
	pub const BaseFee: u64 = 10;
	pub const SDMultiplier: u64 = 10;
	pub const MaxPulseFilterLen: u32 = 100;
	pub const MaxSubscriptions: u32 = 1_000_000;
	pub const MaxMetadataLen: u32 = 8;
}
pub type Credits = u64;
pub type Deposit = u128;
pub type SubscriptionId = [u8; 32];
pub type BlockNumber = u32;

// Derived types
pub type Metadata = IdnMetadata<MaxMetadataLen>;
pub type PulseFilter = MngPulseFilter<RuntimePulse, MaxPulseFilterLen>;
pub type CreateSubParams =
	MngCreateSubParams<Credits, BlockNumber, Metadata, PulseFilter, SubscriptionId>;
