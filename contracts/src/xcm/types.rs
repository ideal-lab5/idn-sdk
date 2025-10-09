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

use super::constants::{
	CONSUMER_PARA_ID_PASEO, CONTRACTS_CALL_INDEX, CONTRACTS_PALLET_INDEX_PASEO,
	IDN_MANAGER_PALLET_INDEX_PASEO, IDN_PARA_ID_PASEO,
};
use ink::env::DefaultEnvironment;

pub use bp_idn::types::{
	xcm as IdnXcm, Balance as IdnBalance, BlockNumber as IdnBlockNumber, CallData, CreateSubParams,
	Credits, Metadata, OpaqueSignature as Pubkey, OriginKind, Quote, RuntimePulse as Pulse,
	SubInfoResponse, SubscriptionId, UpdateSubParams, RequestReference, SubscriptionState, MaxMetadataLen
};
pub type ParaId = u32;
pub type PalletIndex = u8;
pub use ink::primitives::AccountId;

// Get the Balance type from the environment
pub type Balance = <DefaultEnvironment as ink::env::Environment>::Balance;

/// Parachain ID for the Ideal Network on the relay chain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum IdnParaId {
	/// Parachain ID for the Ideal Network on the Paseo relay chain
	OnPaseo,
	/// Other parachain ID for different relay chains
	Other(ParaId),
}

impl From<IdnParaId> for ParaId {
	fn from(para_id: IdnParaId) -> Self {
		match para_id {
			IdnParaId::OnPaseo => IDN_PARA_ID_PASEO,
			IdnParaId::Other(id) => id,
		}
	}
}

/// Pallet index for the IDN Manager pallet in the Ideal Network runtime on different relay
/// chains
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum IdnManagerPalletIndex {
	/// IDN Manager pallet index in the Ideal Network runtime on Paseo
	OnPaseoIdn,
	/// Other pallet index for different runtimes
	Other(PalletIndex),
}

impl From<IdnManagerPalletIndex> for PalletIndex {
	fn from(index: IdnManagerPalletIndex) -> Self {
		match index {
			IdnManagerPalletIndex::OnPaseoIdn => IDN_MANAGER_PALLET_INDEX_PASEO,
			IdnManagerPalletIndex::Other(i) => i,
		}
	}
}

/// Parachain ID for the target chain where this contract is deployed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum ConsumerParaId {
	/// Parachain ID for the IDN Consumer chain on Paseo
	OnPaseo,
	/// Other parachain ID for different relay chains
	Other(ParaId),
}

impl From<ConsumerParaId> for ParaId {
	fn from(para_id: ConsumerParaId) -> Self {
		match para_id {
			ConsumerParaId::OnPaseo => CONSUMER_PARA_ID_PASEO,
			ConsumerParaId::Other(id) => id,
		}
	}
}
/// Pallet index for the Contracts pallet in the target chain runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum ContractsPalletIndex {
	/// Contracts pallet index in the IDN Consumer runtime on Paseo
	OnPaseoIdnConsumer,
	/// Other pallet index for different runtimes
	Other(PalletIndex),
}

impl From<ContractsPalletIndex> for PalletIndex {
	fn from(index: ContractsPalletIndex) -> Self {
		match index {
			ContractsPalletIndex::OnPaseoIdnConsumer => CONTRACTS_PALLET_INDEX_PASEO,
			ContractsPalletIndex::Other(i) => i,
		}
	}
}

/// Call index for dispatchables in the Contracts pallet
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[ink::scale_derive(Encode, Decode, TypeInfo)]
pub enum ContractsCallIndex {
	Call,
	Other(u8),
}

impl From<ContractsCallIndex> for u8 {
	fn from(index: ContractsCallIndex) -> Self {
		match index {
			ContractsCallIndex::Call => CONTRACTS_CALL_INDEX,
			ContractsCallIndex::Other(i) => i,
		}
	}
}

/// Enums used to store the SubscriptionState in the Ink! Smart contract
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
#[derive(Debug, Clone)]
#[ink::scale_derive(Encode, Decode)]
pub enum SubscriptionStateLocal {
    Active,
    Paused,
	Finalized
}

impl From<SubscriptionState> for SubscriptionStateLocal {
    fn from(s: SubscriptionState) -> Self {
        match s {
            SubscriptionState::Active => Self::Active,
            SubscriptionState::Paused => Self::Paused,
            SubscriptionState::Finalized => Self::Finalized,
        }
    }
}

/// Struct used to store some fields from SubInfoResponse returned from IDN when request_sub_info is invoked
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo, ink::storage::traits::StorageLayout))]
#[derive(Debug, Clone)]
#[ink::scale_derive(Encode, Decode)]
pub struct SubInfo {
    pub req_ref: [u8; 32],
    pub sub_id: [u8; 32],
	pub credits_left: u64,
	pub credits: u64,
	pub frequency: u32,
	pub last_delivered: Option<u32>,
	pub state: SubscriptionStateLocal,
}

impl From<SubInfoResponse> for SubInfo {
	fn from(sub_info: SubInfoResponse) -> Self {
		Self {
			req_ref: sub_info.req_ref,
			sub_id: sub_info.sub.id,
			credits_left: sub_info.sub.credits_left,
			credits: sub_info.sub.credits,
			frequency: sub_info.sub.frequency,
			last_delivered: sub_info.sub.last_delivered,
			state: sub_info.sub.state.into(),
		}
	}
}
