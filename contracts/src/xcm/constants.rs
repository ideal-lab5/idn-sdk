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

//! Constants for the IDN Client Contract Library

// use crate::types::{PalletIndex, ParaId};

pub use bp_idn::constants::*;

pub type ParaId = u32;
pub type PalletIndex = u8;

/// Parachain ID for the Ideal Network on the Paseo relay chain
pub const IDN_PARA_ID_PASEO: ParaId = 4502;

/// Parachain ID for the IDN Consumer chain on the Paseo relay chain
pub const CONSUMER_PARA_ID_PASEO: ParaId = 4594;

/// IDN Manager pallet index in the Ideal Network runtime on Paseo
pub const IDN_MANAGER_PALLET_INDEX_PASEO: PalletIndex = 40;

/// Contracts pallet index in the IDN Consumer runtime on Paseo
pub const CONTRACTS_PALLET_INDEX_PASEO: PalletIndex = 16;

/// Call index for the `call` dispatchable in the Contracts pallet
pub const CONTRACTS_CALL_INDEX: u8 = 6;

/// Sovereign account ID for the IDN parachain on Paseo relay chain
/// This is the account that represents the IDN parachain in cross-chain operations
pub const SOVEREIGN_ACCOUNT_IDN_PASEO: [u8; 32] =
	hex_literal::hex!("7369626c96110000000000000000000000000000000000000000000000000000");
