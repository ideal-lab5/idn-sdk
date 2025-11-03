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

use crate::{error::Error as GadgetError, gadget::PulseSubmitter};
use alloc::collections::btree_map::BTreeMap;
use codec::Encode;
use pallet_randomness_beacon::RandomnessBeaconApi;
use parking_lot::Mutex;
use sc_client_api::HeaderBackend;
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::ByteArray;
use sp_consensus_aura::{sr25519::AuthorityId as AuraId, AuraApi};
use sp_keystore::KeystorePtr;
use sp_runtime::{
	traits::{Block as BlockT, NumberFor},
	RuntimeAppPublic,
};
use std::sync::Arc;

const LOG_TARGET: &str = "pulse-worker";

/// The worker responsible for submitting pulses to the runtime
pub struct PulseWorker<Block, Client, Pool>
where
	Block: BlockT,
{
	client: Arc<Client>,
	transaction_pool: Arc<Pool>,
	keystore: KeystorePtr,
	last_submitted_block: Arc<Mutex<Option<NumberFor<Block>>>>,
	_phantom: std::marker::PhantomData<Block>,
}

impl<Block, Client, Pool> PulseWorker<Block, Client, Pool>
where
	Block: BlockT,
{
	pub fn new(client: Arc<Client>, transaction_pool: Arc<Pool>, keystore: KeystorePtr) -> Self {
		let last_submitted_block = Arc::new(Mutex::new(None));
		Self {
			client,
			transaction_pool,
			keystore,
			last_submitted_block,
			_phantom: Default::default(),
		}
	}
}

impl<Block, Client, Pool> PulseSubmitter<Block> for PulseWorker<Block, Client, Pool>
where
	Block: BlockT,
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	Client::Api: RandomnessBeaconApi<Block> + AuraApi<Block, AuraId>,
	Pool: TransactionPool<Block = Block> + 'static,
{
	async fn handle_pulse(
		&self,
		asig: Vec<u8>,
		start: u64,
		end: u64,
		recovered_calls: BTreeMap<u64, Vec<(Vec<u8>, Vec<u8>)>>,
	) -> Result<Block::Hash, GadgetError> {
		let authority_id = self
			.keystore
			.sr25519_public_keys(AuraId::ID)
			.into_iter()
			.next()
			.map(|key| AuraId::from(key))
			.ok_or(GadgetError::NoAuthorityKeys)?;

		let best_hash = {
			let mut last_block = self.last_submitted_block.lock();

			let info = self.client.info();
			let current_block = info.best_number;
			let best_hash = info.best_hash;

			if let Some(last) = *last_block {
				if last == current_block {
					log::debug!(
						target: LOG_TARGET,
						"⏭️  Already submitted in block #{}, skipping rounds {}-{}",
						current_block, start, end
					);
					return Ok(best_hash);
				}
			}

			*last_block = Some(current_block);
			best_hash
		};

		log::info!(
			target: LOG_TARGET,
			"Constructing pulse extrinsic for rounds {}-{} (our turn)",
			start,
			end
		);

		// sign pulse payload
		let payload = (asig.clone(), start, end).encode();
		let signature = self
			.keystore
			.sign_with(
				AuraId::ID,
				sp_application_crypto::sr25519::CRYPTO_ID,
				&authority_id.as_slice(),
				&payload,
			)
			.map_err(|e| {
				log::error!(target: LOG_TARGET, "Keystore signing error: {:?}", e);
				GadgetError::KeystoreError(e.to_string())
			})?
			.ok_or_else(|| {
				log::error!(target: LOG_TARGET, "Key not found in keystore");
				GadgetError::NoAuthorityKeys
			})?;

		// Build unsigned extrinsic with signed payload
		let extrinsic = self
			.client
			.runtime_api()
			.build_extrinsic(best_hash, asig.clone(), start, end, signature, recovered_calls)
			.map_err(|e| {
				log::error!(target: LOG_TARGET, "Failed to build extrinsic: {:?}", e);
				GadgetError::RuntimeApiError(e.to_string())
			})?;

		let tx_hash = match self
			.transaction_pool
			.submit_one(best_hash, TransactionSource::Local, extrinsic)
			.await
		{
			Ok(hash) => hash,
			Err(e) => {
				log::error!(target: LOG_TARGET, "❌ Failed to submit to pool: {:?}", e);
				// reset tracker if pool submission failed
				*self.last_submitted_block.lock() = None;
				return Err(GadgetError::TransactionSubmissionFailed);
			},
		};

		log::info!(
			target: LOG_TARGET,
			"✅ Submitted pulse extrinsic for rounds {}-{}, tx_hash: {:?}",
			start,
			end,
			tx_hash
		);

		Ok(best_hash)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use sp_consensus_randomness_beacon::types::SERIALIZED_SIG_SIZE;
	use sp_keystore::{testing::MemoryKeystore, Keystore};
	use std::sync::Arc;

	fn create_test_keystore() -> KeystorePtr {
		Arc::new(MemoryKeystore::new())
	}

	#[tokio::test]
	async fn test_can_construct_pulse_worker() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		let _worker =
			PulseWorker::<TestBlock, MockClient, MockTransactionPool>::new(client, pool, keystore);
	}

	#[tokio::test]
	async fn test_handle_pulse_with_signed_payload() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		// insert key to keystore
		keystore.sr25519_generate_new(AuraId::ID, None).expect("Failed to generate key");

		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];
		let result = worker.handle_pulse(asig, 100, 101).await;

		assert!(result.is_ok(), "Pulse submission should succeed");

		// Verify transaction was submitted to pool
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 1, "The transaction should be included in the pool");
	}

	#[tokio::test]
	async fn test_handle_pulse_with_signed_payload_fails_with_empty_keystore() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();
		// DO NOT insert key to keystore
		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);
		let asig = vec![0u8; SERIALIZED_SIG_SIZE];
		let result = worker.handle_pulse(asig, 100, 101).await;

		assert!(result.is_err(), "Pulse submission should succeed");
		assert!(matches!(result, Err(GadgetError::NoAuthorityKeys)));

		// Verify transaction was NOT submitted to pool
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 0, "The transaction should be included in the pool");
	}

	#[tokio::test]
	async fn test_handle_pulse_with_signed_payload_fails_with_signing_error() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();
		// DO NOT insert key to keystore
		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);
		let asig = vec![0u8; SERIALIZED_SIG_SIZE];
		let result = worker.handle_pulse(asig, 100, 101).await;

		assert!(result.is_err(), "Pulse submission should succeed");
		assert!(matches!(result, Err(GadgetError::NoAuthorityKeys)));

		// Verify transaction was NOT submitted to pool
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 0, "The transaction should be included in the pool");
	}

	#[tokio::test]
	async fn test_handle_pulse_fails_on_signing_error() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let failing_keystore = Arc::new(FailingKeystore::new(true));

		// Key exists, but signing will fail
		failing_keystore
			.sr25519_generate_new(AuraId::ID, None)
			.expect("Failed to generate key");

		let worker = PulseWorker::new(client.clone(), pool.clone(), failing_keystore);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];
		let result = worker.handle_pulse(asig, 100, 101).await;

		assert!(result.is_err(), "Pulse submission should fail");
		assert!(matches!(result, Err(GadgetError::KeystoreError(_))));

		// Verify transaction was NOT submitted to pool
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 0, "No transaction should be in the pool");
	}

	#[tokio::test]
	async fn test_handle_pulse_prevents_duplicate_submission_in_same_block() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		keystore.sr25519_generate_new(AuraId::ID, None).expect("Failed to generate key");

		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];

		// First submission should succeed
		let result1 = worker.handle_pulse(asig.clone(), 100, 101).await;
		assert!(result1.is_ok(), "First pulse submission should succeed");

		// Second submission in same block should be skipped (returns Ok but doesn't submit)
		let result2 = worker.handle_pulse(asig.clone(), 102, 103).await;
		assert!(result2.is_ok(), "Second submission should return Ok (but skip)");

		// Verify only ONE transaction was submitted to pool
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 1, "Only one transaction should be in the pool");
	}

	#[tokio::test]
	async fn test_handle_pulse_allows_submission_in_different_blocks() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		keystore.sr25519_generate_new(AuraId::ID, None).expect("Failed to generate key");

		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];

		// First submission in block 1
		let result1 = worker.handle_pulse(asig.clone(), 100, 101).await;
		assert!(result1.is_ok(), "First pulse submission should succeed");

		// Advance to next block
		client.set_block_number(1);

		// Second submission in block 2 should also succeed
		let result2 = worker.handle_pulse(asig.clone(), 102, 103).await;
		assert!(result2.is_ok(), "Second submission should succeed in new block");

		// Verify TWO transactions were submitted
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 2, "Two transactions should be in the pool");
	}

	#[tokio::test]
	async fn test_handle_pulse_with_different_round_ranges() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		keystore.sr25519_generate_new(AuraId::ID, None).expect("Failed to generate key");

		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];

		// Test single round
		let result1 = worker.handle_pulse(asig.clone(), 100, 100).await;
		assert!(result1.is_ok(), "Single round submission should succeed");
		// Advance to next block
		client.set_block_number(1);

		// Test multiple rounds
		let result2 = worker.handle_pulse(asig.clone(), 200, 250).await;
		assert!(result2.is_ok(), "Multi-round submission should succeed");

		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 2, "Both submissions should be in the pool");
	}

	#[tokio::test]
	async fn test_handle_pulse_fails_when_transaction_pool_rejects() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		keystore.sr25519_generate_new(AuraId::ID, None).expect("Failed to generate key");

		// Configure pool to reject submissions
		pool.set_should_fail(true);

		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];
		let result = worker.handle_pulse(asig, 100, 101).await;

		assert!(result.is_err(), "Pulse submission should fail");
		assert!(matches!(result, Err(GadgetError::TransactionSubmissionFailed)));

		// Verify transaction was NOT added to pool
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 0, "No transaction should be in the pool");
	}

	#[tokio::test]
	async fn test_handle_pulse_fails_when_runtime_api_fails() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		keystore.sr25519_generate_new(AuraId::ID, None).expect("Failed to generate key");

		// // Configure client to fail on build_extrinsic
		// client.set_build_extrinsic_should_fail(true);

		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];
		// the api MOCK is hardcoded to fail when start == 0 when building the extrinsic\
		let result = worker.handle_pulse(asig, 0, 101).await;

		assert!(result.is_err(), "Pulse submission should fail");
		assert!(matches!(result, Err(GadgetError::RuntimeApiError(_))));

		// Verify transaction was NOT submitted to pool
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 0, "No transaction should be in the pool");
	}

	#[tokio::test]
	async fn test_handle_pulse_with_empty_signature() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		keystore.sr25519_generate_new(AuraId::ID, None).expect("Failed to generate key");

		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);

		// Empty signature
		let asig = vec![];
		let result = worker.handle_pulse(asig, 100, 101).await;

		// Should still succeed - validation happens in runtime
		assert!(result.is_ok(), "Submission with empty sig should succeed");
	}

	#[tokio::test]
	async fn test_handle_pulse_with_maximum_round_values() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		keystore.sr25519_generate_new(AuraId::ID, None).expect("Failed to generate key");

		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];

		// Test with maximum u64 values
		let result = worker.handle_pulse(asig, u64::MAX - 1, u64::MAX).await;
		assert!(result.is_ok(), "Submission with max round values should succeed");
	}

	#[tokio::test]
	async fn test_worker_thread_safety_concurrent_submissions() {
		use std::sync::Arc;
		use tokio::task;

		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		keystore.sr25519_generate_new(AuraId::ID, None).expect("Failed to generate key");

		let worker = Arc::new(PulseWorker::new(client.clone(), pool.clone(), keystore));

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];

		// Spawn multiple concurrent submission attempts
		let mut handles = vec![];
		for i in 0..10 {
			let worker_clone = Arc::clone(&worker);
			let asig_clone = asig.clone();

			let handle = task::spawn(async move {
				worker_clone.handle_pulse(asig_clone, 100 + i, 101 + i).await
			});
			handles.push(handle);
		}

		// Wait for all submissions
		let results: Vec<_> = futures::future::join_all(handles).await;

		// At least one should succeed, rest should be skipped (all in same block)
		let succeeded = results.iter().filter(|r| r.as_ref().unwrap().is_ok()).count();
		assert!(succeeded >= 1, "At least one submission should succeed");

		// Should only have ONE transaction in pool due to duplicate prevention
		let txs = pool.pool.lock().clone();
		assert_eq!(
			txs.len(),
			1,
			"Only one transaction should be in the pool despite concurrent attempts"
		);
	}
}
