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
use sc_transaction_pool_api::{
	TransactionPool, TransactionSource,
};
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
	use async_trait::async_trait;
	use parking_lot::Mutex;
	use sc_client_api::blockchain::{BlockStatus, Info};
	use sc_transaction_pool_api::{
		ImportNotificationStream, PoolStatus, ReadyTransactions, TransactionFor, TransactionStatusStreamFor, TxHash, TxInvalidityReportMap,
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

	struct MockClient {
		best_hash: Mutex<Option<<TestBlock as BlockT>::Hash>>,
		best_number: Mutex<u64>,
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

	#[allow(dead_code)] // this struct never gets constructed
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

	#[derive(Clone, Debug)]
	pub struct PoolTransaction {
		data: Arc<OpaqueExtrinsic>,
		hash: <TestBlock as BlockT>::Hash,
	}

	impl From<OpaqueExtrinsic> for PoolTransaction {
		fn from(e: OpaqueExtrinsic) -> Self {
			PoolTransaction { data: Arc::from(e), hash: <TestBlock as BlockT>::Hash::zero() }
		}
	}

	impl sc_transaction_pool_api::InPoolTransaction for PoolTransaction {
		type Transaction = Arc<OpaqueExtrinsic>;
		type Hash = <TestBlock as BlockT>::Hash;

		fn data(&self) -> &Self::Transaction {
			&self.data
		}

		fn hash(&self) -> &Self::Hash {
			&self.hash
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
			unimplemented!()
		}
	}

	#[derive(Clone, Debug)]
	pub struct MockTransactionPool { 
		pool: Arc<Mutex<Vec<PoolTransaction>>>,
		should_fail: Arc<Mutex<bool>>,
	}

	impl MockTransactionPool {
		fn new() -> Self {
			let pool = Arc::new(Mutex::new(vec![]));
			let should_fail = Arc::new(Mutex::new(false));
			
			Self { pool, should_fail }
		}

		fn set_should_fail(&self, do_fail: bool) {
			*self.should_fail.lock() = do_fail;
		}
	}

	pub struct TransactionsIterator(std::vec::IntoIter<Arc<PoolTransaction>>);

	impl Iterator for TransactionsIterator {
		type Item = Arc<PoolTransaction>;

		fn next(&mut self) -> Option<Self::Item> {
			self.0.next()
		}
	}

	impl ReadyTransactions for TransactionsIterator {
		fn report_invalid(&mut self, _tx: &Self::Item) {}
	}

	#[async_trait]
	impl sc_transaction_pool_api::TransactionPool for MockTransactionPool {
		type Block = TestBlock;
		type Hash = <TestBlock as BlockT>::Hash;
		type InPoolTransaction = PoolTransaction;
		type Error = GadgetError;

		/// Asynchronously imports a bunch of unverified transactions to the pool.
		async fn submit_at(
			&self,
			_at: Self::Hash,
			_source: TransactionSource,
			_xts: Vec<TransactionFor<Self>>,
		) -> Result<Vec<Result<<TestBlock as BlockT>::Hash, Self::Error>>, Self::Error> {
			unimplemented!()
		}

		/// Asynchronously imports one unverified transaction to the pool.
		async fn submit_one(
			&self,
			_at: Self::Hash,
			_source: TransactionSource,
			xt: TransactionFor<Self>,
		) -> Result<TxHash<Self>, Self::Error> {

			if *self.should_fail.lock() {
				return Err(GadgetError::TransactionSubmissionFailed);
			}

			self.pool.lock().push(PoolTransaction::from(xt));
			let tx_hash: TxHash<Self> = Default::default();
			Ok(tx_hash)
		}

		async fn submit_and_watch(
			&self,
			_at: Self::Hash,
			_source: TransactionSource,
			_xt: TransactionFor<Self>,
		) -> Result<Pin<Box<TransactionStatusStreamFor<Self>>>, Self::Error> {
			unimplemented!()
		}

		async fn ready_at(
			&self,
			_at: Self::Hash,
		) -> Box<dyn ReadyTransactions<Item = Arc<Self::InPoolTransaction>> + Send> {
			unimplemented!()
		}

		fn ready(&self) -> Box<dyn ReadyTransactions<Item = Arc<Self::InPoolTransaction>> + Send> {
			unimplemented!()
		}

		fn report_invalid(
			&self,
			_at: Option<Self::Hash>,
			_invalid_tx_errors: TxInvalidityReportMap<TxHash<Self>>,
		) -> Vec<Arc<Self::InPoolTransaction>> {
			Default::default()
		}

		fn futures(&self) -> Vec<Self::InPoolTransaction> {
			unimplemented!()
		}

		fn status(&self) -> PoolStatus {
			unimplemented!()
		}

		fn import_notification_stream(&self) -> ImportNotificationStream<TxHash<Self>> {
			unimplemented!()
		}

		fn on_broadcasted(&self, _propagations: HashMap<TxHash<Self>, Vec<String>>) {
			unimplemented!()
		}

		fn hash_of(&self, _xt: &TransactionFor<Self>) -> TxHash<Self> {
			unimplemented!()
		}

		fn ready_transaction(&self, _hash: &TxHash<Self>) -> Option<Arc<Self::InPoolTransaction>> {
			unimplemented!()
		}

		async fn ready_at_with_timeout(
			&self,
			_at: Self::Hash,
			_timeout: std::time::Duration,
		) -> Box<dyn sc_transaction_pool_api::ReadyTransactions<Item = Arc<Self::InPoolTransaction>> + Send> {
			unimplemented!()
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
			_asig: Vec<u8>,
			_start: u64,
			_end: u64,
		) -> Result<<TestBlock as BlockT>::Extrinsic, GadgetError> {
			if *self.should_fail.lock() {
				return Err(GadgetError::ExtrinsicConstructionFailed);
			}
			// Return a dummy opaque extrinsic
			Ok(sp_runtime::OpaqueExtrinsic::from_bytes([0u8; 32].as_slice()).unwrap())
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
