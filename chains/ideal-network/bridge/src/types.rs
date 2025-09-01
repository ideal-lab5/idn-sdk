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
use ark_serialize::CanonicalSerialize;
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
use sp_idn_crypto::prelude::*;
use sp_idn_traits::pulse::Pulse as TPulse;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	MultiSignature, Vec,
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
///
/// The aggregated signature is the sum of the aggregated signatures output from
/// a randomness beacon for monotonically increasing rounds `[start, ..., end]`.
/// Explicitly, for valid runtime pulses:
/// ```latex
///     $sig = \sum_{i \in [n]} \sum_{j \in [m]} sk_i * H(r_j)$
/// ```
/// where `$sk_i$` is the secret key of the `$i^ th$` worker and `$r_j$` is the `$j^{th}$`
#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, DecodeWithMemTracking)]
pub struct RuntimePulse {
	/// The aggregated signature
	signature: OpaqueSignature,
	/// The first round from which round numbers begin
	start: RoundNumber,
	/// The round when round numbers cease
	end: RoundNumber,
}

impl Default for RuntimePulse {
	fn default() -> Self {
		Self { signature: [0; 48], start: 0, end: 0 }
	}
}

impl RuntimePulse {
	/// Construct a new RuntimePulse
	pub fn new(signature: OpaqueSignature, start: RoundNumber, end: RoundNumber) -> Self {
		Self { signature, start, end }
	}
}

impl TPulse for RuntimePulse {
	type Rand = Randomness;
	type RoundNumber = RoundNumber;
	type Sig = OpaqueSignature;
	type Pubkey = OpaquePublicKey;

	fn rand(&self) -> Self::Rand {
		let mut hasher = Sha256::default();
		hasher.update(self.signature);
		hasher.finalize().into()
	}

	fn message(&self) -> Self::Sig {
		let msg = (self.start..self.end)
			.map(|r| compute_round_on_g1(r).expect("it should be a valid integer"))
			.fold(zero_on_g1(), |amsg, val| (amsg + val).into());
		let mut bytes = Vec::new();
		msg.serialize_compressed(&mut bytes)
			.expect("The message should be well formed.");
		bytes.try_into().unwrap_or([0u8; 48])
	}

	fn start(&self) -> Self::RoundNumber {
		self.start
	}

	fn end(&self) -> Self::RoundNumber {
		self.end
	}

	fn sig(&self) -> Self::Sig {
		self.signature
	}

	fn authenticate(&self, pubkey: Self::Pubkey) -> bool {
		QuicknetVerifier::verify(
			pubkey.as_ref().to_vec(),
			self.sig().as_ref().to_vec(),
			self.message().as_ref().to_vec(),
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
	pub TreasuryAccount: AccountId =
		AccountId::from_ss58check("5CQE1RtAnMdcdWgx4EuvnGYfdPa5qwQS2pQMzhjsPn7k3A1C")
			.expect("Invalid Treasury Account");
	/// The Subscription Deposit Multiplier, used for calculating the subscription fee
	pub const SDMultiplier: u64 = 10;
	/// Maximum number of subscriptions allowed
	///
	/// This and [`MaxTerminatableSubs`] should be set to a number that keeps the estimated
	/// `dispatch_pulse` Proof size in combinations with the `on_finalize` Proof size under
	/// the relay chain's [`MAX_POV_SIZE`](https://github.com/paritytech/polkadot-sdk/blob/da8c374871cc97807935230e7c398876d5adce62/polkadot/primitives/src/v8/mod.rs#L441)
	pub const MaxSubscriptions: u32 = 2_000;
	/// Maximum number of subscriptions that can be terminated in a `on_finalize` execution
	///
	/// This and [`MaxSubscriptions`] should be set to a number that keeps the estimated Proof
	/// Size in the weights under the relay chain's [`MAX_POV_SIZE`](https://github.com/paritytech/polkadot-sdk/blob/da8c374871cc97807935230e7c398876d5adce62/polkadot/primitives/src/v8/mod.rs#L441)
	pub const MaxTerminatableSubs: u32 = 200;
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
pub type QuoteSubParams = MngQuoteSubParams<CreateSubParams, BlockNumber>;
/// The request for a quote, containing the parameters for the describing the subscription.
///
/// See [`pallet_idn_manager::primitives::QuoteRequest`] for more details.
pub type QuoteRequest = MngQuoteRequest<CreateSubParams, BlockNumber>;
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
