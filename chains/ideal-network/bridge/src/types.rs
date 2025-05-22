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
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{parameter_types, PalletId};
use pallet_idn_manager::{
	primitives::{
		CreateSubParams as MngCreateSubParams, Quote as MngQuote, QuoteRequest as MngQuoteRequest,
		QuoteSubParams as MngQuoteSubParams, SubInfoRequest as MngSubInfoRequest,
		SubInfoResponse as MngSubInfoResponse, SubscriptionMetadata,
	},
	Subscription as MngSubscription, SubscriptionDetails as MngSubscriptionDetails,
	UpdateSubParams as MngUpdateSubParams,
};
use scale_info::TypeInfo;
use sha2::{Digest, Sha256};
use sp_core::crypto::Ss58Codec;
use sp_idn_crypto::verifier::{QuicknetVerifier, SignatureVerifier};
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	AccountId32, MultiSignature,
};

pub use pallet_idn_manager::{
	primitives::{CallIndex, RequestReference},
	SubscriptionState,
};
pub use sp_consensus_randomness_beacon::types::*;

/// The runtime pulse represents a verifiable pulse constructed from the aggregation
/// of one or more valid drand pulses. That is, this struct represents the format
/// in which subscribers recieve, consume, and verify on-chain randomness
/// More specifically, it represents an aggregation of pulses and associated messages
/// output from Drand's Quicknet
#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, DecodeWithMemTracking)]
pub struct RuntimePulse {
	message: OpaqueSignature,
	signature: OpaqueSignature,
}

impl Default for RuntimePulse {
	fn default() -> Self {
		Self { message: [0; 48], signature: [0; 48] }
	}
}

impl RuntimePulse {
	/// A contructor, usually reserved for testing
	pub fn new(message: OpaqueSignature, signature: OpaqueSignature) -> Self {
		Self { message, signature }
	}
}

impl sp_idn_traits::pulse::Pulse for RuntimePulse {
	type Rand = Randomness;
	type Sig = OpaqueSignature;
	type Pubkey = OpaquePublicKey;

	fn rand(&self) -> Self::Rand {
		let mut hasher = Sha256::default();
		hasher.update(self.signature);
		hasher.finalize().into()
	}

	fn message(&self) -> Self::Sig {
		self.message
	}

	fn sig(&self) -> Self::Sig {
		self.signature
	}

	fn authenticate(&self, pubkey: Self::Pubkey) -> bool {
		QuicknetVerifier::verify(
			pubkey.as_ref().to_vec(),
			self.sig().as_ref().to_vec(),
			self.message().as_ref().to_vec(),
			None,
		)
		.is_ok()
	}
}

// TODO: correctly define these types https://github.com/ideal-lab5/idn-sdk/issues/186
// Primitive types
parameter_types! {
	/// The IDN Manager Pallet ID
	pub const IdnManagerPalletId: PalletId = PalletId(*b"idn_mngr");
	/// The IDN Treasury Account for fee collection
	pub TreasuryAccount: AccountId32 =
		AccountId32::from_ss58check("5CQE1RtAnMdcdWgx4EuvnGYfdPa5qwQS2pQMzhjsPn7k3A1C")
			.expect("Invalid Treasury Account");
	/// The Subscription Deposit Multiplier, used for calculating the subscription fee
	pub const SDMultiplier: u64 = 10;
	/// The maximum number of subscriptions allowed
	pub const MaxSubscriptions: u32 = 1_000;
	/// Maximum number of subscriptions that can be terminated in a `on_finalize` execution.
	///
	/// This should be set to a number that keeps the estimated `on_finalize` Proof size under
	/// the relay chain's [`MAX_POV_SIZE`](https://github.com/paritytech/polkadot-sdk/blob/da8c374871cc97807935230e7c398876d5adce62/polkadot/primitives/src/v8/mod.rs#L441)
	pub const MaxTerminatableSubs: u32 = 100;
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
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

// Derived types

/// The metadata type used in the pallet, represented as a bounded vector of bytes.
///
/// See [`pallet_idn_manager::primitives::SubscriptionMetadata`] for more details.
pub type Metadata = SubscriptionMetadata<MaxMetadataLen>;
/// The parameters for creating a new subscription, containing various details about the
/// subscription.
///
/// See [`pallet_idn_manager::primitives::CreateSubParams`] for more details.
pub type CreateSubParams = MngCreateSubParams<Credits, BlockNumber, Metadata, SubscriptionId>;
/// The parameters for updating an existing subscription, containing various details about the
/// subscription.
///
/// See [`pallet_idn_manager::UpdateSubParams`] for more details.
pub type UpdateSubParams = MngUpdateSubParams<SubscriptionId, Credits, BlockNumber, Metadata>;
/// The parameters for quoting a subscription.
///
/// See [`pallet_idn_manager::primitives::QuoteSubParams`] for more details.
pub type QuoteSubParams = MngQuoteSubParams<CreateSubParams>;
/// The request for a quote, containing the parameters for the describing the subscription.
///
/// See [`pallet_idn_manager::primitives::QuoteRequest`] for more details.
pub type QuoteRequest = MngQuoteRequest<CreateSubParams>;
/// The quote for a subscription, containing the deposit and fees.
///
/// See [`pallet_idn_manager::primitives::Quote`] for more details.
pub type Quote = MngQuote<Balance>;
/// Represents a subscription in the system.
///
/// See [`pallet_idn_manager::Subscription`] for more details.
pub type Subscription = MngSubscription<AccountId, BlockNumber, Credits, Metadata, SubscriptionId>;
/// The subscription info returned by the IDN Manager to the target parachain.
///
/// See [`pallet_idn_manager::primitives::SubInfoResponse`] for more details.
pub type SubInfoResponse = MngSubInfoResponse<Subscription>;
/// Contains the parameters for requesting a subscription info by its Id.
///
/// See [`pallet_idn_manager::primitives::SubInfoRequest`] for more details.
pub type SubInfoRequest = MngSubInfoRequest<SubscriptionId>;
/// Details specific to a subscription for pulse delivery.
///
/// See [`pallet_idn_manager::SubscriptionDetails`] for more details.
pub type SubscriptionDetails = MngSubscriptionDetails<AccountId>;
