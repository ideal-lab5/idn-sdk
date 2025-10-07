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
use sc_client_api::HeaderBackend;
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::AppCrypto;
use sp_core::sr25519;
use sp_keystore::KeystorePtr;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

const LOG_TARGET: &str = "pulse-worker";

/// Trait for constructing runtime-specific extrinsics
pub trait ExtrinsicConstructor<Block: BlockT>: Send + Sync {
	/// Construct a signed extrinsic for pulse submission
	fn construct_pulse_extrinsic(
		&self,
		signer: sr25519::Public,
		asig: Vec<u8>,
		start: u64,
		end: u64,
	) -> Result<Block::Extrinsic, GadgetError>;
}

/// The worker responsible for submitting pulses to the runtime
pub struct PulseWorker<Block, Client, Pool, Constructor>
where
	Block: BlockT,
{
	client: Arc<Client>,
	keystore: KeystorePtr,
	transaction_pool: Arc<Pool>,
	extrinsic_constructor: Arc<Constructor>,
	_phantom: std::marker::PhantomData<Block>,
}

impl<Block, Client, Pool, Constructor> PulseWorker<Block, Client, Pool, Constructor>
where
	Block: BlockT,
{
	pub fn new(
		client: Arc<Client>,
		keystore: KeystorePtr,
		transaction_pool: Arc<Pool>,
		extrinsic_constructor: Arc<Constructor>,
	) -> Self {
		Self {
			client,
			keystore,
			transaction_pool,
			extrinsic_constructor,
			_phantom: Default::default(),
		}
	}

	/// get aura keys
	fn get_authority_key(&self) -> Result<sr25519::Public, GadgetError> {
		self.keystore
			.sr25519_public_keys(sp_consensus_aura::sr25519::AuthorityPair::ID)
			.first()
			.cloned()
			.ok_or_else(|| GadgetError::NoAuthorityKeys)
	}
}

impl<Block, Client, Pool, Constructor> PulseSubmitter<Block>
	for PulseWorker<Block, Client, Pool, Constructor>
where
	Block: BlockT,
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	Pool: TransactionPool<Block = Block> + 'static,
	Constructor: ExtrinsicConstructor<Block> + 'static,
{
	async fn submit_pulse(
		&self,
		asig: Vec<u8>,
		start: u64,
		end: u64,
	) -> Result<Block::Hash, GadgetError> {
		let public = self.get_authority_key()?;

		log::info!(
			target: LOG_TARGET,
			"Constructing pulse extrinsic for rounds {}-{}",
			start,
			end
		);

		let extrinsic =
			self.extrinsic_constructor.construct_pulse_extrinsic(public, asig, start, end)?;

		// Submit to transaction pool
		let best_hash = self.client.info().best_hash;
		let _hash = self
			.transaction_pool
			.submit_one(best_hash, TransactionSource::Local, extrinsic)
			.await
			.map_err(|_| GadgetError::TransactionSubmissionFailed);

		log::info!(
			target: LOG_TARGET,
			"Submitted pulse extrinsic",
		);

		Ok(best_hash)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
	use std::sync::Arc;

	#[tokio::test]
	async fn test_can_construct_pulse_worker() {
		let keystore = create_test_keystore_with_key().await;
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let constructor = Arc::new(MockExtrinsicConstructor::new());

		let _worker = PulseWorker::<
			TestBlock,
			MockClient,
			MockTransactionPool,
			MockExtrinsicConstructor,
		>::new(client, keystore, pool, constructor);
		// If we get here, construction succeeded
	}

	#[tokio::test]
	async fn test_submit_pulse_success() {
		let keystore = create_test_keystore_with_key().await;
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let constructor = Arc::new(MockExtrinsicConstructor::new());

		let worker = PulseWorker::new(client.clone(), keystore, pool.clone(), constructor);

		// Create a test signature
		let asig = vec![0u8; 48];

		// Submit pulse
		let result = worker.submit_pulse(asig, 100, 101).await;

		assert!(result.is_ok(), "Pulse submission should succeed");

		// Verify transaction was submitted to pool: submit one should have been called
		let txs = pool.pool.lock().clone().leak();
		assert!(txs.len() == 1, "The transaction should be included in the pool");
	}

	#[tokio::test]
	async fn test_submit_pulse_no_authority_key() {
		let keystore = create_test_keystore_empty();
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let constructor = Arc::new(MockExtrinsicConstructor::new());

		let worker = PulseWorker::new(client, keystore, pool, constructor);

		let asig = vec![0u8; 48];

		// Should fail because no authority key exists
		let result = worker.submit_pulse(asig, 100, 101).await;

		assert!(result.is_err(), "Should fail when no authority key found");
		assert!(matches!(result, Err(GadgetError::NoAuthorityKeys)));
	}

	#[tokio::test]
	async fn test_submit_pulse_extrinsic_construction_fails() {
		let keystore = create_test_keystore_with_key().await;
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let constructor = Arc::new(MockExtrinsicConstructor::new());

		// Make constructor fail
		constructor.set_should_fail(true);

		let worker = PulseWorker::new(client, keystore, pool.clone(), constructor);

		let asig = vec![0u8; 48];

		// Should fail at extrinsic construction
		let result = worker.submit_pulse(asig, 100, 101).await;

		assert!(result.is_err(), "Should fail when extrinsic construction fails");

		// Verify nothing was submitted
		assert_eq!(
			pool.pool.lock().clone().leak().len(),
			0,
			"Should not submit if construction fails"
		);
	}

	#[tokio::test]
	async fn test_submit_pulse_tx_inclusion_failure() {
		let keystore = create_test_keystore_with_key().await;
		let client = Arc::new(MockClient::new());

		let tx_pool = MockTransactionPool::new();
		tx_pool.set_should_fail(true);
		let pool = Arc::new(tx_pool);

		let constructor = Arc::new(MockExtrinsicConstructor::new());

		let worker = PulseWorker::new(client.clone(), keystore, pool.clone(), constructor);
		let asig = vec![0u8; 48];

		// Submit pulse
		let _result = worker.submit_pulse(asig, 100, 101).await;
		let txs = pool.pool.lock().clone().leak();
		assert!(txs.len() == 0, "The transaction pool should be empty");
	}

	#[tokio::test]
	async fn test_get_authority_key_success() {
		let keystore = create_test_keystore_with_key().await;
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let constructor = Arc::new(MockExtrinsicConstructor::new());

		let worker: PulseWorker<
			TestBlock,
			MockClient,
			MockTransactionPool,
			MockExtrinsicConstructor,
		> = PulseWorker::new(client, keystore, pool, constructor);

		let key = worker.get_authority_key();
		assert!(key.is_ok(), "Should successfully retrieve authority key");
	}

	#[tokio::test]
	async fn test_get_authority_key_not_found() {
		let keystore = create_test_keystore_empty();
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let constructor = Arc::new(MockExtrinsicConstructor::new());

		let worker: PulseWorker<
			TestBlock,
			MockClient,
			MockTransactionPool,
			MockExtrinsicConstructor,
		> = PulseWorker::new(client, keystore, pool, constructor);

		let key = worker.get_authority_key();
		assert!(key.is_err(), "Should fail when no authority key exists");
	}

	#[tokio::test]
	async fn test_submit_multiple_pulses() {
		let keystore = create_test_keystore_with_key().await;
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let constructor = Arc::new(MockExtrinsicConstructor::new());

		let worker = PulseWorker::new(client, keystore, pool.clone(), constructor);

		// Submit multiple pulses
		for i in 0..3 {
			let asig = vec![0u8; 48];
			let result = worker.submit_pulse(asig, 100 + i, 100 + i).await;
			assert!(result.is_ok(), "Pulse submission {} should succeed", i);
		}

		// Verify all transactions were submitted
		let submitted = pool.pool.lock().clone().leak();
		assert_eq!(submitted.len(), 3, "Should have submitted three transactions");
	}
}
