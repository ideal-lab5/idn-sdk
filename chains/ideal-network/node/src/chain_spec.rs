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

use cumulus_primitives_core::ParaId;
use idn_runtime as runtime;
use runtime::{AccountId, AuraId, Signature, EXISTENTIAL_DEPOSIT};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Decode, Encode, Pair, Public};
use sp_runtime::traits::{IdentifyAccount, Verify};
use std::str::FromStr;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	#[serde(alias = "relayChain", alias = "RelayChain")]
	pub relay_chain: String,
	/// The id of the Parachain.
	#[serde(alias = "paraId", alias = "ParaId")]
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate collator keys from an account id.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_account(acc: &AccountId) -> AuraId {
	Decode::decode(&mut acc.encode().as_ref()).unwrap()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> runtime::SessionKeys {
	runtime::SessionKeys { aura: keys }
}

pub fn dev_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());
	properties.insert("ss58Format".into(), 0.into());

	ChainSpec::builder(
		runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions {
			relay_chain: "paseo-local".into(),
			// You MUST set this to the correct network!
			para_id: 1000,
		},
	)
	.with_name("IDN Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_patch(testnet_genesis(
		// initial collators.
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
		],
		vec![
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_account_id_from_seed::<sr25519::Public>("Charlie"),
			get_account_id_from_seed::<sr25519::Public>("Dave"),
			get_account_id_from_seed::<sr25519::Public>("Eve"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie"),
			get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
			get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
			get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
			get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
			get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		1000.into(),
	))
	.build()
}

pub fn local_testnet_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "PAS".into());
	properties.insert("tokenDecimals".into(), 10.into());
	properties.insert("ss58Format".into(), 0.into());

	#[allow(deprecated)]
	ChainSpec::builder(
		runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "paseo-local".into(), para_id: 2000 },
	)
	.with_name("IDN Local Testnet")
	.with_id("idn_local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(testnet_genesis(
		// initial collators.
		vec![
			sr25519::Public::from_str("5DG98CDpGab5LbqDCxJFdJrF6eDCVgKpdZqSEmWzcZA5Hq88") // seed: "//Idn-local-testnet-collator-01"
				.unwrap()
				.into(),
			sr25519::Public::from_str("5FHLJBPPa8R9SW4vBXLMsWQkQiX2J2ZczJNgtLpy46AkLx6T") // seed: "//Idn-local-testnet-collator-02"
				.unwrap()
				.into(),
		],
		vec![
			sr25519::Public::from_str("5DG98CDpGab5LbqDCxJFdJrF6eDCVgKpdZqSEmWzcZA5Hq88") // seed: "//Idn-local-testnet-collator-01"
				.unwrap()
				.into(),
			sr25519::Public::from_str("5FHLJBPPa8R9SW4vBXLMsWQkQiX2J2ZczJNgtLpy46AkLx6T") // seed: "//Idn-local-testnet-collator-02"
				.unwrap()
				.into(),
			sr25519::Public::from_str("5Cu7qY3UMoejWDnzR1ZVfEUgVTqzJnvM6AE5FTnqard4dRP2") // seed: "//Idn-local-testnet-root"
				.unwrap()
				.into(),
		],
		sr25519::Public::from_str("5Cu7qY3UMoejWDnzR1ZVfEUgVTqzJnvM6AE5FTnqard4dRP2") // seed: "//Idn-local-testnet-root"
			.unwrap()
			.into(),
		2000.into(),
	))
	.with_protocol_id("idn-local-testnet-protocol-id")
	.with_properties(properties)
	.build()
}

pub fn testnet_config() -> ChainSpec {
	// Give your base currency a unit name and decimal places
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "PAS".into());
	properties.insert("tokenDecimals".into(), 10.into());
	properties.insert("ss58Format".into(), 0.into());

	#[allow(deprecated)]
	ChainSpec::builder(
		runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "paseo".into(), para_id: 4502 },
	)
	.with_name("IDN Testnet")
	.with_id("idn_testnet")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_patch(testnet_genesis(
		// initial collators.
		vec![
			sr25519::Public::from_str("5CLVQw6AiHywt4w8RgqueWVvRCyRkjQiJsNfESDV2AsMdY2V") // idn-testnet-01
				.unwrap()
				.into(),
			sr25519::Public::from_str("5HDgmRx8pKeDGstHZrAMFzcRsXc3VFwf4yH6PQSJUvky7vHN") // idn-testnet-02
				.unwrap()
				.into(),
		],
		vec![
			sr25519::Public::from_str("5CLVQw6AiHywt4w8RgqueWVvRCyRkjQiJsNfESDV2AsMdY2V") // idn-testnet-01
				.unwrap()
				.into(),
			sr25519::Public::from_str("5HDgmRx8pKeDGstHZrAMFzcRsXc3VFwf4yH6PQSJUvky7vHN") // idn-testnet-02
				.unwrap()
				.into(),
			sr25519::Public::from_str("5Dcz93bWaQZuvjrgizvnPDSZDefrCFm4R58zsPmChrTe1ywQ") // idn-testnet-root
				.unwrap()
				.into(),
		],
		sr25519::Public::from_str("5Dcz93bWaQZuvjrgizvnPDSZDefrCFm4R58zsPmChrTe1ywQ") // idn-testnet-root
			.unwrap()
			.into(),
		4502.into(),
	))
	.with_protocol_id("idn-testnet-protocol-id")
	.with_properties(properties)
	.build()
}

fn testnet_genesis(
	invulnerables: Vec<AccountId>,
	endowed_accounts: Vec<AccountId>,
	root: AccountId,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().cloned().map(|k| (k, 1u64 << 60)).collect::<Vec<_>>(),
		},
		"parachainInfo": {
			"parachainId": id,
		},
		"collatorSelection": {
			"invulnerables": invulnerables.clone(),
			"candidacyBond": EXISTENTIAL_DEPOSIT * 16,
		},
		"session": {
			"keys": invulnerables
				.into_iter()
				.map(|acc| {
					let session_keys = template_session_keys(get_collator_keys_from_account(&acc));
					(
						acc.clone(),	// account id
						acc,			// validator id
						session_keys,
					)
				})
			.collect::<Vec<_>>(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"sudo": { "key": Some(root) }
	})
}
