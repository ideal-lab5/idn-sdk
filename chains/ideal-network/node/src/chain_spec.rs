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
use runtime::{AccountId, AuraId, Signature};
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
	properties.insert("tokenSymbol".into(), "PAS".into());
	properties.insert("tokenDecimals".into(), 10.into());
	properties.insert("ss58Format".into(), 0.into());

	ChainSpec::builder(
		runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "paseo-local".into(), para_id: 4502 },
	)
	.with_name("IDN Dev")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_genesis_config_patch(genesis_config(
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
			sr25519::Public::from_str("1LX9m9Ee8u653hU1sxvvRNpV1ZjYExa7K8r9zjDws9GDLvp") // Treasury Account
				.unwrap()
				.into(),
		],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		4502.into(),
	))
	.with_protocol_id("idn-dev-protocol-id")
	.with_properties(properties)
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
		Extensions { relay_chain: "paseo-local".into(), para_id: 4502 },
	)
	.with_name("IDN Local Testnet")
	.with_id("idn_local_testnet")
	.with_chain_type(ChainType::Local)
	.with_genesis_config_patch(genesis_config(
		// initial collators.
		vec![
			sr25519::Public::from_str("12CSGXUt8MrYn8qjAbMFmTgPxGCrBysxi4ZvQ4WMAeBbURTK") // seed: "//Idn-local-testnet-collator-01"
				.unwrap()
				.into(),
			sr25519::Public::from_str("14DdSWeTRugct35S9APN1fEuGLWfzL7m4o7B3dpKcBCGXYRu") // seed: "//Idn-local-testnet-collator-02"
				.unwrap()
				.into(),
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
			sr25519::Public::from_str("1LX9m9Ee8u653hU1sxvvRNpV1ZjYExa7K8r9zjDws9GDLvp") // Treasury Account
				.unwrap()
				.into(),
			sr25519::Public::from_str("1qQysJYDavCwkoWNecVoPJqM5qe16UVAexZQknC8weaofXe") // local-testnet-root
				.unwrap()
				.into(),
		],
		sr25519::Public::from_str("1qQysJYDavCwkoWNecVoPJqM5qe16UVAexZQknC8weaofXe")
			.unwrap()
			.into(),
		4502.into(),
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
	.with_genesis_config_patch(genesis_config(
		// initial collators.
		vec![
			sr25519::Public::from_str("1GnZGMEa5FRKbwePKtunfL5Gpy5T2xrPN79PjCqaFtsopS5") // idn-testnet-01
				.unwrap()
				.into(),
			sr25519::Public::from_str("169yumDCg6ugiQtoXVDMQ9Saj9bhBZVo9U1aYhRf31nVJ7sS") // idn-testnet-02
				.unwrap()
				.into(),
		],
		vec![],
		sr25519::Public::from_str("12ZHHNraSBqPNGsCgdynXNGi5GfVtZKCVZsV2gkZFwVACRDh") // idn-testnet-root
			.unwrap()
			.into(),
		4502.into(),
	))
	.with_protocol_id("idn-testnet-protocol-id")
	.with_properties(properties)
	.build()
}

pub fn mainnet_config() -> ChainSpec {
	// DOT token properties (native token via XCM from Polkadot relay chain)
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());
	properties.insert("ss58Format".into(), 0.into());

	#[allow(deprecated)]
	ChainSpec::builder(
		runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
		Extensions { relay_chain: "polkadot".into(), para_id: 3414 },
	)
	.with_name("Ideal Network")
	.with_id("idn_mainnet")
	.with_chain_type(ChainType::Live)
	.with_genesis_config_patch(genesis_config(
		// initial collators (34 total)
		vec![
			sr25519::Public::from_str("163Eu3nccweuBK6tbRWNR1XgQtVwzkftiYDKzS3RSbNcYMGM") // collator-01
				.unwrap()
				.into(),
			sr25519::Public::from_str("14r8r48N19N4wS7KNJadnf2M4LoiBc5CvtNCrvQdfiJaiP8u") // collator-02
				.unwrap()
				.into(),
			sr25519::Public::from_str("13dAF8APzbfQpVGmywCgKn9zYRHxyXErDpwtcUYWnSk3aXFV") // collator-03
				.unwrap()
				.into(),
			sr25519::Public::from_str("15Gfa8hDChMLC62L6YgHuxHBhVgqD5SDVHjSboT7jVhoHh9V") // collator-04
				.unwrap()
				.into(),
			sr25519::Public::from_str("13JrhmUn1gvcdELAgho6r1pem6oAUeNGcygf97sYe5eW3eaP") // collator-05
				.unwrap()
				.into(),
			sr25519::Public::from_str("12wznqWPTFWbxkTVnpxJxPjXzBzJpQz6qpNE2F5YP1DtngXE") // collator-06
				.unwrap()
				.into(),
			sr25519::Public::from_str("15zLNxXBz9WjWJMB9Kc6D3Hw7vPWkjMjURgyCpqkdAwuUNSW") // collator-07
				.unwrap()
				.into(),
			sr25519::Public::from_str("14owjXKV2VabeKwpQTFBt7embg96FNS2Q8No5V8fD8iH3nCb") // collator-08
				.unwrap()
				.into(),
			sr25519::Public::from_str("16Pvp6EqZD3aRa77cgsdVrPsVoLX8rXdVG3S9XB9JfsoVkAF") // collator-09
				.unwrap()
				.into(),
			sr25519::Public::from_str("16cKDxToLy63NLM9Yv1mTDYUWbAzQG84FDTozPWBhkKUMbBR") // collator-10
				.unwrap()
				.into(),
			sr25519::Public::from_str("12mcbCVmhqtKVbJZStbYgqdc9TUMpPM6k3CryVmecgb4XCgX") // collator-11
				.unwrap()
				.into(),
			sr25519::Public::from_str("13at9e8C16NyNdaEiatDhQf4yXoHdug3AZxy9s2H6wv3J97Q") // collator-12
				.unwrap()
				.into(),
			sr25519::Public::from_str("16LTswmZDvbvEufityzVpXTxfxkg2yFiWHsGNSZQYnZWGMW1") // collator-13
				.unwrap()
				.into(),
			sr25519::Public::from_str("1ZJNMcWxPrzbN7D3TWkN5qucQ7ryrYzwfejySyD1pShTgz1") // collator-14
				.unwrap()
				.into(),
			sr25519::Public::from_str("16ZnNqa88HMpECwwaRvGUftUieK4cynnCeYBRMKUXQhScRYj") // collator-15
				.unwrap()
				.into(),
			sr25519::Public::from_str("129pJuvFGjaWtjfAs1y3iqNdhew1mYwwUwNEmk9r4K4yUs9q") // collator-16
				.unwrap()
				.into(),
			sr25519::Public::from_str("16UkJHQAVaEAGmHM2ERNiRv1dTzPKcs5MQTrpa1ifz8uvtpb") // collator-17
				.unwrap()
				.into(),
			sr25519::Public::from_str("1NCCvj2jyGfLLSoiYaMV8tsmSjr9XwnvvwjfumgMEaPUqaG") // collator-18
				.unwrap()
				.into(),
			sr25519::Public::from_str("16CtoYEZDz5SgT2RkxwUpyzRCc5oje9je2vE3i2jce2GTArA") // collator-19
				.unwrap()
				.into(),
			sr25519::Public::from_str("13g55igdFoMSJFfrqBeMCHxnPi3T4MNChcsvTYv7Z2nFhtHu") // collator-20
				.unwrap()
				.into(),
		],
		vec![], // empty balances - DOT comes via XCM reserve transfers
		sr25519::Public::from_str("1LX9m9Ee8u653hU1sxvvRNpV1ZjYExa7K8r9zjDws9GDLvp") // mainnet-root
			.unwrap()
			.into(),
		3414.into(),
	))
	.with_protocol_id("idn-mainnet-protocol-id")
	.with_properties(properties)
	.build()
}

fn genesis_config(
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
			"candidacyBond": 0,
		},
		"randBeacon": {
			"beaconPubkeyHex": b"83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a".to_vec(),
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
