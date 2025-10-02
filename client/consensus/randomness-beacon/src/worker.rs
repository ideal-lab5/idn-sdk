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

use crate::gadget::PulseSubmitter;
use async_trait::async_trait;
use sc_client_api::HeaderBackend;
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_api::ProvideRuntimeApi;
use sp_application_crypto::AppCrypto;
use sp_consensus_randomness_beacon::types::OpaqueSignature;
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
		asig: OpaqueSignature,
		start: u64,
		end: u64,
	) -> Result<Block::Extrinsic, Box<dyn std::error::Error + Send + Sync>>;
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

	/// get aura keys (for now: TODO)
	fn get_authority_key(
		&self,
	) -> Result<sr25519::Public, Box<dyn std::error::Error + Send + Sync>> {
		self.keystore
			.sr25519_public_keys(sp_consensus_aura::sr25519::AuthorityPair::ID)
			.first()
			.cloned()
			.ok_or_else(|| "No authority key found in keystore".into())
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
		asig: OpaqueSignature,
		start: u64,
		end: u64,
	) -> Result<Block::Hash, Box<dyn std::error::Error + Send + Sync>> {
		let public = self.get_authority_key()?;

		log::info!(
			target: LOG_TARGET,
			"Constructing pulse extrinsic for rounds {}-{}",
			start,
			end
		);

		let extrinsic = self
			.extrinsic_constructor
			.construct_pulse_extrinsic(public, asig, start, end)
			.unwrap();

		// Submit to transaction pool
		let best_hash = self.client.info().best_hash;
		let _hash = self
			.transaction_pool
			.submit_one(best_hash, TransactionSource::Local, extrinsic)
			.await?;

		log::info!(
			target: LOG_TARGET,
			"Submitted pulse extrinsic",
		);

		Ok(best_hash)
	}
}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
//     use sp_core::crypto::KeyTypeId;

//      fn create_test_keystore() -> KeystorePtr {
//         Arc::new(MemoryKeystore::new())
//     }

//     mockall::mock! {
// 		pub Client<B: BlockT> {}

// 		impl<B: BlockT> HeaderBackend<B> for Client<B> {
// 			fn header(&self, hash: B::Hash) -> Result<Option<B::Header>, BlockchainError>;
// 			fn info(&self) -> Info<B>;
// 			fn status(&self, hash: B::Hash) -> Result<BlockStatus, BlockchainError>;
// 			fn number(
// 				&self,
// 				hash: B::Hash,
// 			) -> Result<Option<<<B as BlockT>::Header as HeaderT>::Number>, BlockchainError>;
// 			fn hash(&self, number: NumberFor<B>) -> Result<Option<B::Hash>, BlockchainError>;
// 		}
// 	}

//     // fn mock_client() {

//     // }

//     #[tokio::test]
//     async fn test_can_construct_pulse_worker() {
//         let keystore = create_test_keystore();
//         let client = Arc::new(mock_client());
//         let pool = Arc::new(mock_pool());
//         let constructor = Arc::new(mock_constructor());

//         let worker = PulseWorker::new(
//             client,
//             keystore,
//             pool,
//             constructor,
//         );
//     }
// }

#[cfg(test)]
mod tests {
	use super::*;
	use parking_lot::Mutex;
	use sc_client_api::blockchain::{BlockStatus, Info};
	use sp_blockchain::Result as BlockchainResult;
	use sp_consensus_aura::sr25519::AuthorityPair;
	use sp_core::crypto::KeyTypeId;
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
	use sp_runtime::{
		generic::Header,
		traits::{BlakeTwo256, Block as BlockT},
	};
	use std::sync::Arc;

	// Test block type
	type TestBlock =
		sp_runtime::generic::Block<Header<u64, BlakeTwo256>, sp_runtime::OpaqueExtrinsic>;

	// mock client => must impl HeaderBackend and ProvideRuntimeApi
	struct MockClient {
		best_hash: Mutex<Option<<TestBlock as BlockT>::Hash>>,
		best_number: Mutex<u64>,
	}

	impl MockClient {
		fn new() -> Self {
			Self { best_hash: Mutex::new(Some(Default::default())), best_number: Mutex::new(0) }
		}

		fn set_best_block(&self, hash: <TestBlock as BlockT>::Hash, number: u64) {
			*self.best_hash.lock() = Some(hash);
			*self.best_number.lock() = number;
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

	use std::any::{Any, TypeId};
	struct MockExtension;
	impl sp_externalities::Extension for MockExtension {
		fn as_mut_any(&mut self) -> &mut dyn Any {
			self
		}

		fn type_id(&self) -> TypeId {
			TypeId::of::<u32>()
		}
	}

	struct MockStateBackend;

	// Mock runtime api (required to implement ProvideRuntimeApi)
	struct MockRuntimeApi;
	// Implement ApiExt for MockRuntimeApi
	impl sp_api::ApiExt<TestBlock> for MockRuntimeApi {
		// type StateBackend = sp_state_machine::InMemoryBackend<sp_runtime::traits::BlakeTwo256>;

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

		fn set_call_context(&mut self, call_context: sp_api::CallContext) {
			unimplemented!("set_call_context not needed for tests")
		}
		fn register_extension<MockExtension>(&mut self, extension: MockExtension) {
			unimplemented!("register_extension not needed for tests")
		}
	}

	// Mock Transaction Pool
	struct MockTransactionPool {
		submitted: Arc<Mutex<Vec<<TestBlock as BlockT>::Extrinsic>>>,
	}

	impl MockTransactionPool {
		fn new() -> Self {
			Self { submitted: Arc::new(Mutex::new(Vec::new())) }
		}

		fn get_submitted(&self) -> Vec<<TestBlock as BlockT>::Extrinsic> {
			self.submitted.lock().clone()
		}
	}

	// Create a minimal InPoolTransaction implementation
	struct MockInPoolTransaction;

	impl sc_transaction_pool_api::InPoolTransaction for MockInPoolTransaction {
		type Transaction = Arc<<TestBlock as BlockT>::Extrinsic>;
		type Hash = <TestBlock as BlockT>::Hash;

		fn data(&self) -> &Self::Transaction {
			unimplemented!()
		}

		fn hash(&self) -> &Self::Hash {
			unimplemented!()
		}

		fn priority(&self) -> &u64 {
			unimplemented!()
		}

		fn longevity(&self) -> &u64 {
			unimplemented!()
		}

		fn requires(&self) -> &[Vec<u8>] {
			unimplemented!()
		}

		fn provides(&self) -> &[Vec<u8>] {
			unimplemented!()
		}

		fn is_propagable(&self) -> bool {
			false
		}
	}

	#[async_trait]
	impl sc_transaction_pool_api::TransactionPool for MockTransactionPool {
		type Block = TestBlock;
		type Hash = <TestBlock as BlockT>::Hash;
		type InPoolTransaction = MockInPoolTransaction;
		type Error = sc_transaction_pool_api::error::Error;

		fn submit_at(
			&self,
			_at: <Self::Block as BlockT>::Hash,
			_source: sc_transaction_pool_api::TransactionSource,
			xts: Vec<<Self::Block as BlockT>::Extrinsic>,
		) -> std::pin::Pin<
			Box<
				dyn std::future::Future<
						Output = Result<Vec<Result<Self::Hash, Self::Error>>, Self::Error>,
					> + Send,
			>,
		> {
			self.submitted.lock().extend(xts.clone());
			let results: Vec<Result<Self::Hash, Self::Error>> =
				xts.iter().map(|_| Ok(Default::default())).collect();
			Box::pin(async move { Ok(results) })
		}

		fn submit_one(
			&self,
			_at: <Self::Block as BlockT>::Hash,
			_source: sc_transaction_pool_api::TransactionSource,
			xt: <Self::Block as BlockT>::Extrinsic,
		) -> std::pin::Pin<
			Box<dyn std::future::Future<Output = Result<Self::Hash, Self::Error>> + Send>,
		> {
			self.submitted.lock().push(xt);
			Box::pin(async move { Ok(Default::default()) })
		}

		fn submit_and_watch(
			&self,
			_at: <Self::Block as BlockT>::Hash,
			_source: sc_transaction_pool_api::TransactionSource,
			_xt: <Self::Block as BlockT>::Extrinsic,
		) -> std::pin::Pin<
			Box<
				dyn std::future::Future<
						Output = Result<
							std::pin::Pin<
								Box<sc_transaction_pool_api::TransactionStatusStreamFor<Self>>,
							>,
							Self::Error,
						>,
					> + Send,
			>,
		> {
			Box::pin(async move {
				Err(sc_transaction_pool_api::error::Error::ImmediatelyDropped.into())
			})
		}

		fn ready_at(
			&self,
			_at: <Self::Block as BlockT>::Hash,
		) -> std::pin::Pin<
			Box<
				dyn std::future::Future<
						Output = Box<
							dyn sc_transaction_pool_api::ReadyTransactions<
									Item = Arc<Self::InPoolTransaction>,
								> + Send,
						>,
					> + Send,
			>,
		> {
			Box::pin(async move {
				Box::new(std::iter::empty())
					as Box<
						dyn sc_transaction_pool_api::ReadyTransactions<
								Item = Arc<Self::InPoolTransaction>,
							> + Send,
					>
			})
		}

		fn ready(
			&self,
		) -> Box<
			dyn sc_transaction_pool_api::ReadyTransactions<Item = Arc<Self::InPoolTransaction>>
				+ Send,
		> {
			Box::new(std::iter::empty())
		}

		async fn report_invalid(
			&self,
			_at: Option<Self::Hash>,
			_invalid_tx_errors: TxInvalidityReportMap<TxHash<Self>>,
		) -> Vec<Arc<Self::InPoolTransaction>> {
			Default::default()
		}

		fn futures(&self) -> Vec<Self::InPoolTransaction> {
			Vec::new()
		}

		fn status(&self) -> sc_transaction_pool_api::PoolStatus {
			sc_transaction_pool_api::PoolStatus {
				ready: 0,
				ready_bytes: 0,
				future: 0,
				future_bytes: 0,
			}
		}

		fn import_notification_stream(
			&self,
		) -> sc_transaction_pool_api::ImportNotificationStream<Self::Hash> {
			let (tx, rx) = sc_utils::mpsc::tracing_unbounded("mock_import_notif", 100);
			drop(tx); // Close immediately
			rx
		}

		fn on_broadcasted(
			&self,
			_propagations: std::collections::HashMap<Self::Hash, Vec<String>>,
		) {
		}

		fn hash_of(&self, _xt: &<Self::Block as BlockT>::Extrinsic) -> Self::Hash {
			Default::default()
		}

		fn ready_transaction(&self, _hash: &Self::Hash) -> Option<Arc<Self::InPoolTransaction>> {
			None
		}

		fn ready_at_with_timeout(
			&self,
			_at: <Self::Block as BlockT>::Hash,
			_timeout: std::time::Duration,
		) -> std::pin::Pin<
			Box<
				dyn std::future::Future<
						Output = Box<
							dyn sc_transaction_pool_api::ReadyTransactions<
									Item = Arc<Self::InPoolTransaction>,
								> + Send,
						>,
					> + Send,
			>,
		> {
			Box::pin(async move {
				Box::new(std::iter::empty())
					as Box<
						dyn sc_transaction_pool_api::ReadyTransactions<
								Item = Arc<Self::InPoolTransaction>,
							> + Send,
					>
			})
		}
	}

	// Mock Extrinsic Constructor
	struct MockExtrinsicConstructor {
		should_fail: Mutex<bool>,
	}

	impl MockExtrinsicConstructor {
		fn new() -> Self {
			Self { should_fail: Mutex::new(false) }
		}

		fn set_should_fail(&self, fail: bool) {
			*self.should_fail.lock() = fail;
		}
	}

	impl ExtrinsicConstructor<TestBlock> for MockExtrinsicConstructor {
		fn construct_pulse_extrinsic(
			&self,
			_signer: sr25519::Public,
			_asig: OpaqueSignature,
			_start: u64,
			_end: u64,
		) -> Result<<TestBlock as BlockT>::Extrinsic, Box<dyn std::error::Error + Send + Sync>> {
			if *self.should_fail.lock() {
				return Err("Construction failed".into());
			}
			// Return a dummy opaque extrinsic
			Ok(sp_runtime::OpaqueExtrinsic::from_bytes(&[]).unwrap())
		}
	}

	async fn create_test_keystore_with_key() -> KeystorePtr {
		let keystore = MemoryKeystore::new();
		// generate an aura key
		keystore
			.sr25519_generate_new(AuthorityPair::ID, Some("//Alice"))
			.expect("Failed to generate key");

		Arc::new(keystore)
	}

	fn create_test_keystore_empty() -> KeystorePtr {
		Arc::new(MemoryKeystore::new())
	}

	// // Tests
	// #[tokio::test]
	// async fn test_can_construct_pulse_worker() {
	// 	let keystore = create_test_keystore_with_key().await;
	// 	let client = Arc::new(MockClient::new());
	// 	let pool = Arc::new(MockTransactionPool::new());
	// 	let constructor = Arc::new(MockExtrinsicConstructor::new());

	// 	let _worker = PulseWorker::new(client, keystore, pool, constructor);

	// 	// If we get here, construction succeeded
	// }

	// #[tokio::test]
	// async fn test_submit_pulse_success() {
	// 	let keystore = create_test_keystore_with_key().await;
	// 	let client = Arc::new(MockClient::new());
	// 	let pool = Arc::new(MockTransactionPool::new());
	// 	let constructor = Arc::new(MockExtrinsicConstructor::new());

	// 	let worker = PulseWorker::new(client.clone(), keystore, pool.clone(), constructor);

	// 	// Create a test signature
	// 	let asig: OpaqueSignature = vec![0u8; 48].try_into().unwrap();

	// 	// Submit pulse
	// 	let result = worker.submit_pulse(asig, 100, 101).await;

	// 	assert!(result.is_ok(), "Pulse submission should succeed");

	// 	// Verify transaction was submitted to pool
	// 	let submitted = pool.get_submitted();
	// 	assert_eq!(submitted.len(), 1, "Should have submitted one transaction");
	// }

	// #[tokio::test]
	// async fn test_submit_pulse_no_authority_key() {
	// 	let keystore = create_test_keystore_empty();
	// 	let client = Arc::new(MockClient::new());
	// 	let pool = Arc::new(MockTransactionPool::new());
	// 	let constructor = Arc::new(MockExtrinsicConstructor::new());

	// 	let worker = PulseWorker::new(client, keystore, pool, constructor);

	// 	let asig: OpaqueSignature = vec![0u8; 48].try_into().unwrap();

	// 	// Should fail because no authority key exists
	// 	let result = worker.submit_pulse(asig, 100, 101).await;

	// 	assert!(result.is_err(), "Should fail when no authority key found");
	// 	assert!(
	// 		result.unwrap_err().to_string().contains("No authority key"),
	// 		"Error should mention missing authority key"
	// 	);
	// }

	// #[tokio::test]
	// async fn test_submit_pulse_extrinsic_construction_fails() {
	// 	let keystore = create_test_keystore_with_key().await;
	// 	let client = Arc::new(MockClient::new());
	// 	let pool = Arc::new(MockTransactionPool::new());
	// 	let constructor = Arc::new(MockExtrinsicConstructor::new());

	// 	// Make constructor fail
	// 	constructor.set_should_fail(true);

	// 	let worker = PulseWorker::new(client, keystore, pool.clone(), constructor);

	// 	let asig: OpaqueSignature = vec![0u8; 48].try_into().unwrap();

	// 	// Should fail at extrinsic construction
	// 	let result = worker.submit_pulse(asig, 100, 101).await;

	// 	assert!(result.is_err(), "Should fail when extrinsic construction fails");

	// 	// Verify nothing was submitted
	// 	assert_eq!(pool.get_submitted().len(), 0, "Should not submit if construction fails");
	// }

	// #[tokio::test]
	// async fn test_get_authority_key_success() {
	// 	let keystore = create_test_keystore_with_key().await;
	// 	let client = Arc::new(MockClient::new());
	// 	let pool = Arc::new(MockTransactionPool::new());
	// 	let constructor = Arc::new(MockExtrinsicConstructor::new());

	// 	let worker = PulseWorker::new(client, keystore, pool, constructor);

	// 	let key = worker.get_authority_key();
	// 	assert!(key.is_ok(), "Should successfully retrieve authority key");
	// }

	// #[tokio::test]
	// async fn test_get_authority_key_not_found() {
	// 	let keystore = create_test_keystore_empty();
	// 	let client = Arc::new(MockClient::new());
	// 	let pool = Arc::new(MockTransactionPool::new());
	// 	let constructor = Arc::new(MockExtrinsicConstructor::new());

	// 	let worker = PulseWorker::new(client, keystore, pool, constructor);

	// 	let key = worker.get_authority_key();
	// 	assert!(key.is_err(), "Should fail when no authority key exists");
	// }

	// #[tokio::test]
	// async fn test_submit_multiple_pulses() {
	// 	let keystore = create_test_keystore_with_key().await;
	// 	let client = Arc::new(MockClient::new());
	// 	let pool = Arc::new(MockTransactionPool::new());
	// 	let constructor = Arc::new(MockExtrinsicConstructor::new());

	// 	let worker = PulseWorker::new(client, keystore, pool.clone(), constructor);

	// 	// Submit multiple pulses
	// 	for i in 0..3 {
	// 		let asig: OpaqueSignature = vec![0u8; 48].try_into().unwrap();
	// 		let result = worker.submit_pulse(asig, 100 + i, 100 + i).await;
	// 		assert!(result.is_ok(), "Pulse submission {} should succeed", i);
	// 	}

	// 	// Verify all transactions were submitted
	// 	let submitted = pool.get_submitted();
	// 	assert_eq!(submitted.len(), 3, "Should have submitted three transactions");
	// }
}
