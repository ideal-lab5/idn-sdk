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
use codec::{Decode, Encode};
use pallet_randomness_beacon::RandomnessBeaconApi;
use sc_client_api::HeaderBackend;
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::ByteArray;
use sp_consensus_aura::{AuraApi, sr25519, sr25519::AuthorityId as AuraId};
use sp_keystore::KeystorePtr;
use sp_runtime::{traits::Block as BlockT, RuntimeAppPublic};
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
	_phantom: std::marker::PhantomData<Block>,
}

impl<Block, Client, Pool> PulseWorker<Block, Client, Pool>
where
	Block: BlockT,
{
	pub fn new(client: Arc<Client>, transaction_pool: Arc<Pool>, keystore: KeystorePtr) -> Self {
		Self { client, transaction_pool, keystore, _phantom: Default::default() }
	}

	/// Check if this authority should submit for the current round (Aura-style round-robin)
	pub fn is_our_turn(&self, round: u64, authority: AuraId, authorities: Vec<AuraId>) -> bool {

		if authorities.is_empty() {
			return false;
		}

		let author_index = (round as usize) % authorities.len();
		authorities.get(author_index) == Some(&authority)
	}
}

impl<Block, Client, Pool> PulseSubmitter<Block> for PulseWorker<Block, Client, Pool>
where
	Block: BlockT,
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	Client::Api: RandomnessBeaconApi<Block> + AuraApi<Block, AuraId>,
	Pool: TransactionPool<Block = Block> + 'static,
{
	async fn submit_pulse(
		&self,
		asig: Vec<u8>,
		start: u64,
		end: u64,
	) -> Result<Block::Hash, GadgetError> {
		let best_hash = self.client.info().best_hash;

		// Get current authorities from runtime
		let authorities = self.client.runtime_api().authorities(best_hash).unwrap_or(vec![]);
		let authority_id = self
			.keystore
			.sr25519_public_keys(AuraId::ID)
			.into_iter()
			.next()
			.map(|key| AuraId::from(key))
			.unwrap();

		// Check if it's our turn to submit (round-robin based on start round)
		if self.is_our_turn(start, authority_id.clone(), authorities) {
			log::debug!(
				target: LOG_TARGET,
				"Skipping pulse submission for round {} - not our turn",
				start
			);
			log::info!(
				target: LOG_TARGET,
				"Constructing pulse extrinsic for rounds {}-{} (our turn)",
				start,
				end
			);

			// sign pulse payload
			let payload = (asig.clone(), start, end).encode();
			let signature_bytes = self
				.keystore
				.sign_with(
					AuraId::ID,
					sp_application_crypto::sr25519::CRYPTO_ID,
					&authority_id.as_slice(), 
					&payload.encode(),
				)
				.unwrap()
				.unwrap();

			// .map_err(|e| format!("Keystore signing error: {:?}", e))?
			// .ok_or_else(|| "Key not found in keystore".to_string())?;

			// Decode signature
			// TODO: pass the sig to the extrinsic as a param
			let signature =
				sp_consensus_aura::sr25519::AuthoritySignature::decode(&mut &signature_bytes[..])
					.unwrap();
			// .map_err(|_| "Failed to decode signature".to_string())?;

			// Build unsigned extrinsic with signed payload
			let extrinsic = self
				.client
				.runtime_api()
				.build_extrinsic(
					best_hash,
					asig.clone(),
					start,
					end,
				)
				.map_err(|e| GadgetError::RuntimeApiError(e.to_string()))?;

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
				"✅ Submitted pulse extrinsic for rounds {}-{}",
				start,
				end
			);
		}

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
		let authority_id = MockAuthorityId::generate_pair(None);

		let _worker =
			PulseWorker::<TestBlock, MockClient, MockTransactionPool, MockAuthorityId>::new(
				client,
				pool,
				keystore,
				authority_id,
			);
	}

	#[tokio::test]
	async fn test_round_robin_our_turn() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		// Create authority set
		let auth1 = MockAuthorityId::generate_pair(None);
		let auth2 = MockAuthorityId::generate_pair(None);
		let auth3 = MockAuthorityId::generate_pair(None);

		client.set_authorities(vec![auth1.clone(), auth2.clone(), auth3.clone()]);

		// Worker with first authority
		let worker =
			PulseWorker::new(client.clone(), pool.clone(), keystore.clone(), auth1.clone());

		// Round 0, 3, 6, etc. should be auth1's turn
		let asig = vec![0u8; SERIALIZED_SIG_SIZE];
		let result = worker.submit_pulse(asig.clone(), 0, 0).await;
		assert!(result.is_ok(), "Should succeed on our turn (round 0)");

		let result = worker.submit_pulse(asig.clone(), 3, 3).await;
		assert!(result.is_ok(), "Should succeed on our turn (round 3)");

		// Round 1 should be auth2's turn
		let result = worker.submit_pulse(asig.clone(), 1, 1).await;
		assert!(result.is_err(), "Should fail when not our turn (round 1)");
		assert!(matches!(result, Err(GadgetError::NotOurTurn)));
	}

	#[tokio::test]
	async fn test_submit_pulse_with_signed_payload() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();
		let authority_id = MockAuthorityId::generate_pair(None);

		client.set_authorities(vec![authority_id.clone()]);

		let worker = PulseWorker::new(client.clone(), pool.clone(), keystore, authority_id);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];
		let result = worker.submit_pulse(asig, 100, 101).await;

		assert!(result.is_ok(), "Pulse submission should succeed");

		// Verify transaction was submitted to pool
		let txs = pool.pool.lock().clone();
		assert_eq!(txs.len(), 1, "The transaction should be included in the pool");
	}

	#[tokio::test]
	async fn test_multiple_authorities_rotation() {
		let client = Arc::new(MockClient::new());
		let pool = Arc::new(MockTransactionPool::new());
		let keystore = create_test_keystore();

		let auth1 = MockAuthorityId::generate_pair(None);
		let auth2 = MockAuthorityId::generate_pair(None);

		client.set_authorities(vec![auth1.clone(), auth2.clone()]);

		let worker1 = PulseWorker::new(client.clone(), pool.clone(), keystore.clone(), auth1);
		let worker2 = PulseWorker::new(client.clone(), pool.clone(), keystore, auth2);

		let asig = vec![0u8; SERIALIZED_SIG_SIZE];

		// Round 0: worker1 should succeed
		assert!(worker1.submit_pulse(asig.clone(), 0, 0).await.is_ok());
		assert!(worker2.submit_pulse(asig.clone(), 0, 0).await.is_err());

		// Round 1: worker2 should succeed
		assert!(worker1.submit_pulse(asig.clone(), 1, 1).await.is_err());
		assert!(worker2.submit_pulse(asig.clone(), 1, 1).await.is_ok());
	}
}
