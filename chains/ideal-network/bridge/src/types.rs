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
use pallet_idn_manager::{
	primitives::{
		CreateSubParams as MngCreateSubParams, PulseFilter as MngPulseFilter, Quote as MngQuote,
		QuoteRequest as MngQuoteRequest, QuoteSubParams as MngQuoteSubParams, SubscriptionMetadata,
	},
	UpdateSubParams as MngUpdateSubParams,
};
use sp_runtime::AccountId32;

pub use pallet_idn_manager::primitives::{CallIndex, QuoteReqRef};
pub use sp_consensus_randomness_beacon::types::RuntimePulse;

// TODO: correctly define these types https://github.com/ideal-lab5/idn-sdk/issues/186
// Primitive types
parameter_types! {
	/// The IDN Manager Pallet ID
	pub const IdnManagerPalletId: PalletId = PalletId(*b"idn_mngr");
	/// The IDN Treasury Account for fee collection
	pub const TreasuryAccount: AccountId32 = AccountId32::new([123u8; 32]);
	/// The base fee, used for calculating the subscription fee
	pub const BaseFee: u64 = 10;
	/// The Subscription Deposit Multiplier, used for calculating the subscription fee
	pub const SDMultiplier: u64 = 10;
	/// The maximum length of the pulse vector filter
	pub const MaxPulseFilterLen: u32 = 100;
	/// The maximum number of subscriptions allowed
	pub const MaxSubscriptions: u32 = 1_000_000;
	/// The maximum length of the metadata vector
	pub const MaxMetadataLen: u32 = 8;
}
/// A type that defines the amount of credits in a subscription
pub type Credits = u64;
/// The subscription ID
pub type SubscriptionId = [u8; 32];
/// An index to a block.
pub type BlockNumber = u32;
/// Balance of an account.
pub type Balance = u128;

// Derived types

/// The metadata type used in the pallet, represented as a bounded vector of bytes.
///
/// See [`pallet_idn_manager::primitives::SubscriptionMetadata`] for more details.
pub type Metadata = SubscriptionMetadata<MaxMetadataLen>;
/// A filter that controls which pulses are delivered to a subscription.
///
/// See [`pallet_idn_manager::primitives::PulseFilter`] for more details.
pub type PulseFilter = MngPulseFilter<RuntimePulse, MaxPulseFilterLen>;
/// The parameters for creating a new subscription, containing various details about the
/// subscription.
///
/// See [`pallet_idn_manager::primitives::CreateSubParams`] for more details.
pub type CreateSubParams =
	MngCreateSubParams<Credits, BlockNumber, Metadata, PulseFilter, SubscriptionId>;
/// The parameters for updating an existing subscription, containing various details about the
/// subscription.
///
/// See [`pallet_idn_manager::UpdateSubParams`] for more details.
pub type UpdateSubParams =
	MngUpdateSubParams<SubscriptionId, Credits, BlockNumber, PulseFilter, Metadata>;
/// The parameters for quoting a subscription.
///
/// See [`pallet_idn_manager::primitives::QuoteSubParams`] for more details.
pub type QuoteSubParams = MngQuoteSubParams<CreateSubParams>;
/// The request for a quote, containing the parameters for the describing the subscription.
///
/// See [`pallet_idn_manager::primitives::QuoteRequest`] for more details.
pub type QuoteRequest = MngQuoteRequest<CreateSubParams>;

pub type Quote = MngQuote<Balance>;
