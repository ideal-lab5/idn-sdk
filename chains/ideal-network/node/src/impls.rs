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

use idn_runtime::{opaque::Block, UncheckedExtrinsic};
use pallet_randomness_beacon::ExtrinsicBuilderApi;
use sc_client_api::HeaderBackend;
use sc_consensus_randomness_beacon::{
	error::Error as GadgetError, gadget::SERIALIZED_SIG_SIZE, worker::ExtrinsicConstructor,
};
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::AppCrypto;
use sp_consensus_aura::sr25519::AuthorityPair;
use sp_consensus_randomness_beacon::types::OpaqueSignature;
use sp_core::sr25519;
use sp_keystore::KeystorePtr;
use sp_runtime::{
	traits::{Block as BlockT, IdentifyAccount},
	MultiSignature, MultiSigner,
};
use std::sync::{Arc, Mutex};
use substrate_frame_rpc_system::AccountNonceApi;

// we only need to proceed if it is our turn (round robin aura style)
// Get our authority ID
// TODO: eventually we should designate named keys for this (like aura does)
// let our_authority = match get_authority_id(&keystore).await {
// 	Some(id) => id,
// 	None => {
// 		log::warn!("Not an authority, skipping submission");
// 		return Ok(Default::default());
// 	}
// };

// let local_id = keystore
// .sr25519_public_keys(RANDOMNESS_BEACON_KEY_TYPE)
// .into_iter()
// .next()
// .ok_or("No authority key found in keystore")?;
// Calculate designated submitter
// let submitter_index = (round as usize) % authorities.len();
// let designated = &authorities[submitter_index];

// // Only designated authority submits
// if designated != &account {
// 	log::debug!("Not designated submitter for round {}", round);
// 	return Ok(Default::default());
// }

// log::info!("🎯 Designated submitter for round {}, submitting...", round);

// ! impls for constructing extrinsics

pub(crate) struct RuntimeExtrinsicConstructor {
	pub(crate) client: Arc<Client>,
	pub(crate) keystore: KeystorePtr,
}

impl ExtrinsicConstructor<Block> for RuntimeExtrinsicConstructor
where
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
{
	fn construct_pulse_extrinsic(
		&self,
		signer: sr25519::Public,
		asig: Vec<u8>,
		start: u64,
		end: u64,
	) -> Result<<Block as BlockT>::Extrinsic, GadgetError> {
		let account: idn_runtime::AccountId = MultiSigner::Sr25519(signer).into_account();
		let at_hash = self.client.info().best_hash;
		let nonce = self.client.runtime_api().account_nonce(at_hash, account.clone()).unwrap();

		// TODO: reject unless by expected authority + check in the pallet

		let formatted: [u8; SERIALIZED_SIG_SIZE] = asig.clone().try_into().map_err(|_| {
			GadgetError::InvalidSignatureSize(asig.len() as u8, SERIALIZED_SIG_SIZE as u8)
		})?;

		let (payload, call, tx_ext) = self
			.client
			.runtime_api()
			.construct_pulse_payload(at_hash, formatted, start, end, nonce)
			.unwrap();

		let signature = self
			.keystore
			.sr25519_sign(AuthorityPair::ID, &signer.into(), &payload)
			.unwrap()
			.unwrap();

		Ok(UncheckedExtrinsic::new_signed(
			call,
			account.into(),
			MultiSignature::Sr25519(signature),
			tx_ext,
		)
		.into())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	// use async_trait::async_trait;
	// use parking_lot::Mutex;
	use sc_client_api::blockchain::{BlockStatus, Info};
	use sc_transaction_pool_api::{
		ImportNotificationStream, PoolStatus, ReadyTransactions, TransactionFor,
		TransactionStatusStreamFor, TxHash, TxInvalidityReportMap,
	};
	use sp_blockchain::Result as BlockchainResult;
	use sp_consensus_aura::sr25519::AuthorityPair;
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
	use sp_runtime::{
		generic::Header,
		traits::{BlakeTwo256, Block as BlockT},
		OpaqueExtrinsic,
	};
	use std::{collections::HashMap, pin::Pin, sync::Arc};

	// Test block type
	type TestBlock =
		sp_runtime::generic::Block<Header<u64, BlakeTwo256>, sp_runtime::OpaqueExtrinsic>;

	async fn create_test_keystore_with_key() -> KeystorePtr {
		let keystore = MemoryKeystore::new();
		keystore
			.sr25519_generate_new(AuthorityPair::ID, Some("//Alice"))
			.expect("Failed to generate key");
		Arc::new(keystore)
	}

	fn create_test_keystore_empty() -> KeystorePtr {
		Arc::new(MemoryKeystore::new())
	}

	impl MockClient {
		fn new() -> Self {
			Self { best_hash: Mutex::new(Some(Default::default())), best_number: Mutex::new(0) }
		}
	}

	impl sc_client_api::HeaderBackend<TestBlock> for MockClient {
		fn header(
			&self,
			_hash: <TestBlock as BlockT>::Hash,
		) -> BlockchainResult<Option<<TestBlock as BlockT>::Header>> {
			Ok(None)
		}

		fn info(&self) -> Info<TestBlock> {
			Info {
				best_hash: self.best_hash.lock().unwrap_or_default(),
				best_number: *self.best_number.lock(),
				finalized_hash: Default::default(),
				finalized_number: 0,
				genesis_hash: Default::default(),
				number_leaves: 1,
				finalized_state: None,
				block_gap: None,
			}
		}

		fn status(&self, _hash: <TestBlock as BlockT>::Hash) -> BlockchainResult<BlockStatus> {
			Ok(BlockStatus::Unknown)
		}

		fn number(&self, _hash: <TestBlock as BlockT>::Hash) -> BlockchainResult<Option<u64>> {
			Ok(Some(*self.best_number.lock()))
		}

		fn hash(&self, _number: u64) -> BlockchainResult<Option<<TestBlock as BlockT>::Hash>> {
			Ok(*self.best_hash.lock())
		}
	}

	impl sp_api::ProvideRuntimeApi<TestBlock> for MockClient {
		type Api = MockRuntimeApi;

		fn runtime_api(&self) -> sp_api::ApiRef<'_, Self::Api> {
			MockRuntimeApi.into()
		}
	}

	// this struct never gets constructed
	#[allow(dead_code)]
	struct MockExtension;
	use std::any::{Any, TypeId};
	impl sp_externalities::Extension for MockExtension {
		fn as_mut_any(&mut self) -> &mut dyn Any {
			self
		}

		fn type_id(&self) -> TypeId {
			TypeId::of::<u32>()
		}
	}

	struct MockRuntimeApi;
	impl sp_api::ApiExt<TestBlock> for MockRuntimeApi {
		fn execute_in_transaction<F: FnOnce(&Self) -> sp_api::TransactionOutcome<R>, R>(
			&self,
			_call: F,
		) -> R
		where
			Self: Sized,
		{
			unimplemented!("execute_in_transaction not needed for tests")
		}

		fn has_api<A: sp_api::RuntimeApiInfo + ?Sized>(
			&self,
			_at: <TestBlock as BlockT>::Hash,
		) -> Result<bool, sp_api::ApiError>
		where
			Self: Sized,
		{
			Ok(false)
		}

		fn has_api_with<A: sp_api::RuntimeApiInfo + ?Sized, P: Fn(u32) -> bool>(
			&self,
			_at: <TestBlock as BlockT>::Hash,
			_pred: P,
		) -> Result<bool, sp_api::ApiError>
		where
			Self: Sized,
		{
			Ok(false)
		}

		fn api_version<A: sp_api::RuntimeApiInfo + ?Sized>(
			&self,
			_at: <TestBlock as BlockT>::Hash,
		) -> Result<Option<u32>, sp_api::ApiError>
		where
			Self: Sized,
		{
			Ok(None)
		}

		fn record_proof(&mut self) {
			// No-op for tests
		}

		fn proof_recorder(&self) -> Option<sp_api::ProofRecorder<TestBlock>> {
			None
		}

		fn extract_proof(&mut self) -> Option<sp_api::StorageProof> {
			None
		}

		fn into_storage_changes<
			B: sp_state_machine::backend::Backend<sp_runtime::traits::HashingFor<TestBlock>>,
		>(
			&self,
			_backend: &B,
			_parent_hash: <TestBlock as BlockT>::Hash,
		) -> Result<sp_api::StorageChanges<TestBlock>, String>
		where
			Self: Sized,
		{
			Err("into_storage_changes not implemented for mock".to_string())
		}

		fn set_call_context(&mut self, _call_context: sp_api::CallContext) {
			unimplemented!("set_call_context not needed for tests")
		}
		fn register_extension<MockExtension>(&mut self, _extension: MockExtension) {
			unimplemented!("register_extension not needed for tests")
		}
	}

	#[test]
	fn test_error_on_invalid_sig_size() {
		let client = Arc::new(MockParachainClient::new());
		let keystore = create_test_keystore_empty();

		let constructor = RuntimeExtrinsicConstructor { client, keystore };

		// Test with invalid signature size (too short)
		let signer = sr25519::Public::from_raw([0u8; 32]);
		let invalid_sig = vec![0u8; 32];

		let result = constructor.construct_pulse_extrinsic(signer, invalid_sig.clone(), 100, 101);

		assert!(result.is_err());
		match result {
			Err(GadgetError::InvalidSignatureSize(got, expected)) => {
				assert_eq!(got, invalid_sig.len() as u8);
				assert_eq!(expected, SERIALIZED_SIG_SIZE as u8);
			},
			_ => panic!("Expected InvalidSignatureSize error"),
		}
	}

	// #[test]
	// fn test_error_on_invalid_sig_size_too_long() {
	//     let client = Arc::new(MockParachainClient);
	//     let keystore = create_test_keystore_empty();

	//     let constructor = RuntimeExtrinsicConstructor {
	//         client,
	//         keystore,
	//     };

	//     let signer = sr25519::Public::from_raw([0u8; 32]);
	//     let invalid_sig = vec![0u8; 64]; // Too long

	//     let result = constructor.construct_pulse_extrinsic(
	//         signer,
	//         invalid_sig.clone(),
	//         100,
	//         101,
	//     );

	//     assert!(result.is_err());
	//     match result {
	//         Err(GadgetError::InvalidSignatureSize(got, expected)) => {
	//             assert_eq!(got, 64);
	//             assert_eq!(expected, SERIALIZED_SIG_SIZE as u8);
	//         }
	//         _ => panic!("Expected InvalidSignatureSize error"),
	//     }
	// }

	// #[test]
	// fn test_correct_sig_size_accepted() {
	//     let client = Arc::new(MockParachainClient);
	//     let keystore = create_test_keystore_empty();

	//     let constructor = RuntimeExtrinsicConstructor {
	//         client,
	//         keystore,
	//     };

	//     let signer = sr25519::Public::from_raw([0u8; 32]);
	//     let valid_sig = vec![0u8; SERIALIZED_SIG_SIZE]; // Correct size

	//     // This will fail for other reasons (no runtime API mock),
	//     // but should pass the signature size check
	//     let result = constructor.construct_pulse_extrinsic(
	//         signer,
	//         valid_sig,
	//         100,
	//         101,
	//     );

	//     // Should not be InvalidSignatureSize error
	//     match result {
	//         Err(GadgetError::InvalidSignatureSize(_, _)) => {
	//             panic!("Should not fail on signature size with correct size")
	//         }
	//         _ => {} // Expected - will fail on runtime_api call
	//     }
	// }
}
