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

use crate::{
	error::Error as GadgetError,
	gadget::{PulseSubmitter, SERIALIZED_SIG_SIZE},
};
use idn_runtime::UncheckedExtrinsic;
use sc_client_api::HeaderBackend;
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

const LOG_TARGET: &str = "pulse-worker";

/// The worker responsible for submitting pulses to the runtime
pub struct PulseWorker<Block, Client, Pool>
where
	Block: BlockT,
{
	client: Arc<Client>,
	transaction_pool: Arc<Pool>,
	_phantom: std::marker::PhantomData<Block>,
}

impl<Block, Client, Pool> PulseWorker<Block, Client, Pool>
where
	Block: BlockT,
	Block::Extrinsic: From<UncheckedExtrinsic>,
{
	pub fn new(client: Arc<Client>, transaction_pool: Arc<Pool>) -> Self {
		Self { client, transaction_pool, _phantom: Default::default() }
	}

	/// Build an unsigned extrinsic to encode new pulses in the runtime
	///
	/// * `asig`: The aggregated signature
	/// * `start`: The first round from which sigs are aggregated
	/// * `end`: The last round from which sigs are aggregated
	fn construct_extrinsic(
		asig: Vec<u8>,
		start: u64,
		end: u64,
	) -> Result<<Block as BlockT>::Extrinsic, GadgetError> {
		let formatted: [u8; SERIALIZED_SIG_SIZE] = asig.clone().try_into().map_err(|_| {
			GadgetError::InvalidSignatureSize(asig.len() as u8, SERIALIZED_SIG_SIZE as u8)
		})?;

		let call =
			idn_runtime::RuntimeCall::RandBeacon(pallet_randomness_beacon::Call::try_submit_asig {
				asig: formatted,
				start,
				end,
			});

		Ok(UncheckedExtrinsic::new_bare(call).into())
	}
}

impl<Block, Client, Pool> PulseSubmitter<Block> for PulseWorker<Block, Client, Pool>
where
	Block: BlockT,
	Block::Extrinsic: From<UncheckedExtrinsic>,
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	Pool: TransactionPool<Block = Block> + 'static,
{
	async fn submit_pulse(
		&self,
		asig: Vec<u8>,
		start: u64,
		end: u64,
	) -> Result<Block::Hash, GadgetError> {
		log::info!(
			target: LOG_TARGET,
			"Constructing pulse extrinsic for rounds {}-{}",
			start,
			end
		);

		let extrinsic = Self::construct_extrinsic(asig, start, end)?;

		// Submit to transaction pool
		let best_hash = self.client.info().best_hash;
		let _hash = self
			.transaction_pool
			.submit_one(best_hash, TransactionSource::Local, extrinsic)
			.await
			.map_err(|e| {
				log::error!(
					target: LOG_TARGET,
					"❌ Failed to submit to pool at hash {:?}: {:?}",
					best_hash,
					e
				);
				GadgetError::TransactionSubmissionFailed
			})?;

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
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());

		let _worker = PulseWorker::<TestBlock, MockClient, MockTransactionPool>::new(client, pool);
		// If we get here, construction succeeded
	}

	#[tokio::test]
	async fn test_submit_pulse_success() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());

		let worker = PulseWorker::new(client.clone(), pool.clone());

		// Create a test signature
		let asig = vec![0u8; SERIALIZED_SIG_SIZE];

		// Submit pulse
		let result = worker.submit_pulse(asig, 100, 101).await;

		assert!(result.is_ok(), "Pulse submission should succeed");

		// Verify transaction was submitted to pool
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 1, "The transaction should be included in the pool");
	}

	#[tokio::test]
	async fn test_submit_pulse_invalid_signature_size() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());

		let worker = PulseWorker::new(client, pool.clone());

		// Create invalid signature (wrong size)
		let asig = vec![0u8; 32]; // Too small

		// Should fail because signature size is wrong
		let result = worker.submit_pulse(asig, 100, 101).await;

		assert!(result.is_err(), "Should fail when signature size is invalid");
		assert!(matches!(result, Err(GadgetError::InvalidSignatureSize(32, 48))));

		// Verify nothing was submitted
		assert_eq!(pool.pool.lock().len(), 0, "Should not submit if signature size is invalid");
	}

	#[tokio::test]
	async fn test_submit_pulse_signature_too_long() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());

		let worker = PulseWorker::new(client, pool.clone());

		let asig = vec![0u8; 64]; // Too long

		let result = worker.submit_pulse(asig, 100, 101).await;

		assert!(result.is_err(), "Should fail when signature is too long");
		assert!(matches!(result, Err(GadgetError::InvalidSignatureSize(64, 48))));

		// Verify nothing was submitted
		assert_eq!(pool.pool.lock().len(), 0);
	}

	#[tokio::test]
	async fn test_submit_pulse_tx_pool_failure() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());

		// Make pool fail
		pool.set_should_fail(true);

		let worker = PulseWorker::new(client.clone(), pool.clone());
		let asig = vec![0u8; SERIALIZED_SIG_SIZE];

		// Submit pulse: should fail at pool submission
		let result = worker.submit_pulse(asig, 100, 101).await;

		assert!(result.is_err(), "Should fail when pool submission fails");
		assert!(matches!(result, Err(GadgetError::TransactionSubmissionFailed)));

		// Verify pool is empty
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 0, "The transaction pool should be empty");
	}

	#[tokio::test]
	async fn test_submit_multiple_pulses() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());

		let worker = PulseWorker::new(client, pool.clone());

		// Submit multiple pulses
		for i in 0..3 {
			let asig = vec![0u8; SERIALIZED_SIG_SIZE];
			let result = worker.submit_pulse(asig, 100 + i, 100 + i).await;
			assert!(result.is_ok(), "Pulse submission {} should succeed", i);
		}

		// Verify all transactions were submitted
		let submitted = pool.pool.lock().clone();
		assert_eq!(submitted.len(), 3, "Should have submitted three transactions");
	}

	#[tokio::test]
	async fn test_construct_extrinsic_valid() {
		let asig = vec![0u8; SERIALIZED_SIG_SIZE];

		let result = PulseWorker::<TestBlock, MockClient, MockTransactionPool>::construct_extrinsic(
			asig, 100, 101,
		);

		assert!(result.is_ok(), "Should successfully construct extrinsic with valid signature");
	}

	#[tokio::test]
	async fn test_construct_extrinsic_invalid_size() {
		let asig = vec![0u8; 32]; // Wrong size

		let result = PulseWorker::<TestBlock, MockClient, MockTransactionPool>::construct_extrinsic(
			asig, 100, 101,
		);

		assert!(result.is_err(), "Should fail with invalid signature size");
		assert!(matches!(result, Err(GadgetError::InvalidSignatureSize(32, 48))));
	}
}
