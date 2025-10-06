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

// ! impls for constructing extrinsics
use idn_runtime::UncheckedExtrinsic;
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
use std::sync::Arc;
use substrate_frame_rpc_system::AccountNonceApi;

pub(crate) struct RuntimeExtrinsicConstructor<B, C> {
	pub(crate) client: Arc<C>,
	pub(crate) keystore: KeystorePtr,
	_phantom: std::marker::PhantomData<B>,
}

impl<B, C> RuntimeExtrinsicConstructor<B, C>
where
	B: BlockT,
{
	pub fn new(client: Arc<C>, keystore: KeystorePtr) -> Self {
		Self { client, keystore, _phantom: Default::default() }
	}
}

impl<B, C> ExtrinsicConstructor<B> for RuntimeExtrinsicConstructor<B, C>
where
	B: BlockT,
	B::Extrinsic: From<UncheckedExtrinsic>,
	C: HeaderBackend<B> + ProvideRuntimeApi<B>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<B, idn_runtime::AccountId, idn_runtime::Nonce>
		+ pallet_randomness_beacon::ExtrinsicBuilderApi<
			B,
			idn_runtime::AccountId,
			idn_runtime::RuntimeCall,
			OpaqueSignature,
			idn_runtime::TxExtension,
			idn_runtime::Nonce,
		>,
{
	fn construct_pulse_extrinsic(
		&self,
		signer: sr25519::Public,
		asig: Vec<u8>,
		start: u64,
		end: u64,
	) -> Result<<B as BlockT>::Extrinsic, GadgetError> {
		let account: idn_runtime::AccountId = MultiSigner::Sr25519(signer).into_account();
		let at_hash = self.client.info().best_hash;

		let nonce = self
			.client
			.runtime_api()
			.account_nonce(at_hash, account.clone())
			.map_err(|e| GadgetError::RuntimeApiError(e.to_string()))?;

		let formatted: [u8; SERIALIZED_SIG_SIZE] = asig.clone().try_into().map_err(|_| {
			GadgetError::InvalidSignatureSize(asig.len() as u8, SERIALIZED_SIG_SIZE as u8)
		})?;

		let (payload, call, tx_ext) = self
			.client
			.runtime_api()
			.construct_pulse_payload(at_hash, formatted, start, end, nonce)
			.map_err(|e| GadgetError::RuntimeApiError(e.to_string()))?;

		let signature = self
			.keystore
			.sr25519_sign(AuthorityPair::ID, &signer, &payload)
			.map_err(|e| GadgetError::KeystoreError(e.to_string()))?
			.ok_or(GadgetError::SigningFailed)?;

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
	use parking_lot::Mutex;
	use sc_client_api::blockchain::{BlockStatus, Info};
	use sp_api::{ApiError, ApiRef, ProvideRuntimeApi};
	use sp_blockchain::Result as BlockchainResult;
	use sp_consensus_aura::sr25519::AuthorityPair;
	use sp_keystore::{testing::MemoryKeystore, Keystore, KeystorePtr};
	use sp_runtime::{
		generic::Header,
		traits::{BlakeTwo256, Block as BlockT},
	};
	use std::sync::Arc;

	// Test block type
	type TestBlock =
		sp_runtime::generic::Block<Header<u64, BlakeTwo256>, sp_runtime::OpaqueExtrinsic>;

	fn create_test_keystore_with_key() -> KeystorePtr {
		let keystore = MemoryKeystore::new();
		keystore
			.sr25519_generate_new(AuthorityPair::ID, Some("//Alice"))
			.expect("Failed to generate key");
		Arc::new(keystore)
	}

	fn create_test_keystore_empty() -> KeystorePtr {
		Arc::new(MemoryKeystore::new())
	}

	// Mock client for testing
	struct MockParachainClient {
		best_hash: Mutex<Option<<TestBlock as BlockT>::Hash>>,
		best_number: Mutex<u64>,
	}

	impl MockParachainClient {
		fn new() -> Self {
			Self { best_hash: Mutex::new(Some(Default::default())), best_number: Mutex::new(0) }
		}
	}

	impl HeaderBackend<TestBlock> for MockParachainClient {
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

	// Mock Runtime API - just provide enough to make tests compile
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
		) -> Result<bool, ApiError>
		where
			Self: Sized,
		{
			Ok(false)
		}

		fn has_api_with<A: sp_api::RuntimeApiInfo + ?Sized, P: Fn(u32) -> bool>(
			&self,
			_at: <TestBlock as BlockT>::Hash,
			_pred: P,
		) -> Result<bool, ApiError>
		where
			Self: Sized,
		{
			Ok(false)
		}

		fn api_version<A: sp_api::RuntimeApiInfo + ?Sized>(
			&self,
			_at: <TestBlock as BlockT>::Hash,
		) -> Result<Option<u32>, ApiError>
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
	impl sp_api::Core<TestBlock> for MockRuntimeApi {
		fn version(
			&self,
			_at: <TestBlock as BlockT>::Hash,
		) -> Result<sp_version::RuntimeVersion, sp_api::ApiError> {
			Ok(sp_version::RuntimeVersion {
				spec_name: "test".into(),
				impl_name: "test".into(),
				apis: sp_version::create_apis_vec![[]],
				authoring_version: 1,
				spec_version: 0,
				impl_version: 1,
				transaction_version: 1,
				system_version: 1,
			})
		}

		fn execute_block(
			&self,
			_at: <TestBlock as BlockT>::Hash,
			_block: TestBlock,
		) -> Result<(), sp_api::ApiError> {
			unimplemented!()
		}

		fn initialize_block(
			&self,
			_at: <TestBlock as BlockT>::Hash,
			_header: &<TestBlock as BlockT>::Header,
		) -> Result<sp_runtime::ExtrinsicInclusionMode, sp_api::ApiError> {
			unimplemented!()
		}

		fn initialize_block_before_version_5(
			&self,
			__runtime_api_at_param__: <Block as BlockT>::Hash,
			_header: &sp_runtime::generic::Header<u64, BlakeTwo256>,
		) -> Result<(), sp_api::ApiError> {
			unimplemented!()
		}

		fn __runtime_api_internal_call_api_at(
			&self,
			_: <sp_runtime::generic::Block<
				sp_runtime::generic::Header<u64, BlakeTwo256>,
				sp_runtime::OpaqueExtrinsic,
			> as BlockT>::Hash,
			_: Vec<u8>,
			_: &dyn Fn(sp_version::RuntimeVersion) -> &'static str,
		) -> Result<Vec<u8>, sp_api::ApiError> {
			todo!()
		}
	}

	impl
		substrate_frame_rpc_system::AccountNonceApi<
			TestBlock,
			idn_runtime::AccountId,
			idn_runtime::Nonce,
		> for MockRuntimeApi
	{
		fn account_nonce(
			&self,
			_runtime_api_at_param: <TestBlock as BlockT>::Hash,
			acct: idn_runtime::AccountId,
		) -> Result<idn_runtime::Nonce, sp_api::ApiError> {
			if acct == sp_runtime::AccountId32::new([0u8; 32]) {
				return Err(sp_api::ApiError::UsingSameInstanceForDifferentBlocks);
			}

			Ok(0)
		}

		fn __runtime_api_internal_call_api_at(
			&self,
			_: <sp_runtime::generic::Block<
				sp_runtime::generic::Header<u64, BlakeTwo256>,
				sp_runtime::OpaqueExtrinsic,
			> as BlockT>::Hash,
			_: Vec<u8>,
			_: &dyn Fn(sp_version::RuntimeVersion) -> &'static str,
		) -> Result<Vec<u8>, sp_api::ApiError> {
			todo!()
		}
	}

	impl ProvideRuntimeApi<TestBlock> for MockParachainClient {
		type Api = MockRuntimeApi;

		fn runtime_api(&self) -> ApiRef<'_, Self::Api> {
			MockRuntimeApi.into()
		}
	}

	impl
		pallet_randomness_beacon::ExtrinsicBuilderApi<
			TestBlock,
			idn_runtime::AccountId,
			idn_runtime::RuntimeCall,
			OpaqueSignature,
			idn_runtime::TxExtension,
			idn_runtime::Nonce,
		> for MockRuntimeApi
	{
		fn construct_pulse_payload(
			&self,
			_at: <TestBlock as BlockT>::Hash,
			_asig: OpaqueSignature,
			start: u64,
			_end: u64,
			_nonce: idn_runtime::Nonce,
		) -> Result<(Vec<u8>, idn_runtime::RuntimeCall, idn_runtime::TxExtension), ApiError> {
			// mock failure
			if start == 101 {
				return Err(ApiError::UsingSameInstanceForDifferentBlocks);
			}

			let payload = vec![0u8; 32];
			let call =
				idn_runtime::RuntimeCall::System(frame_system::Call::remark { remark: vec![] });
			let tx_ext: idn_runtime::TxExtension = (
				frame_system::CheckNonZeroSender::<idn_runtime::Runtime>::new(),
				frame_system::CheckSpecVersion::<idn_runtime::Runtime>::new(),
				frame_system::CheckTxVersion::<idn_runtime::Runtime>::new(),
				frame_system::CheckGenesis::<idn_runtime::Runtime>::new(),
				frame_system::CheckEra::<idn_runtime::Runtime>::from(
					sp_runtime::generic::Era::immortal(),
				),
				frame_system::CheckNonce::<idn_runtime::Runtime>::from(0),
				frame_system::CheckWeight::<idn_runtime::Runtime>::new(),
				pallet_transaction_payment::ChargeTransactionPayment::<idn_runtime::Runtime>::from(
					0,
				),
				frame_metadata_hash_extension::CheckMetadataHash::<idn_runtime::Runtime>::new(
					false,
				),
				frame_system::WeightReclaim::<idn_runtime::Runtime>::new(),
			);

			Ok((payload, call, tx_ext))
		}

		fn __runtime_api_internal_call_api_at(
			&self,
			_: <sp_runtime::generic::Block<
				sp_runtime::generic::Header<u64, BlakeTwo256>,
				sp_runtime::OpaqueExtrinsic,
			> as BlockT>::Hash,
			_: Vec<u8>,
			_: &dyn Fn(sp_version::RuntimeVersion) -> &'static str,
		) -> Result<Vec<u8>, sp_api::ApiError> {
			todo!()
		}
	}

	#[test]
	fn test_error_on_invalid_sig_size() {
		let client = Arc::new(MockParachainClient::new());
		let keystore = create_test_keystore_with_key();
		let constructor = RuntimeExtrinsicConstructor::new(client, keystore);

		// invalid signature size - buffer too small
		let signer = sr25519::Public::from_raw([1u8; 32]);
		let invalid_sig = vec![0u8; 32];

		let result = constructor.construct_pulse_extrinsic(signer, invalid_sig.clone(), 100, 101);

		assert!(result.is_err());
		assert!(matches!(result, Err(GadgetError::InvalidSignatureSize(32, 48))));
	}

	#[test]
	fn test_error_on_invalid_sig_size_too_long() {
		let client = Arc::new(MockParachainClient::new());
		let keystore = create_test_keystore_with_key();

		let constructor = RuntimeExtrinsicConstructor::new(client, keystore);

		let signer = sr25519::Public::from_raw([1u8; 32]);
		let invalid_sig = vec![0u8; 64]; // Too long

		let result = constructor.construct_pulse_extrinsic(signer, invalid_sig.clone(), 100, 101);
		assert!(result.is_err());
		assert!(matches!(result, Err(GadgetError::InvalidSignatureSize(64, 48))));
	}

	#[test]
	fn test_correct_sig_size_accepted() {
		let client = Arc::new(MockParachainClient::new());
		let keystore = create_test_keystore_with_key();

		let constructor = RuntimeExtrinsicConstructor::new(client, keystore.clone());

		let signer = keystore
			.sr25519_public_keys(AuthorityPair::ID)
			.into_iter()
			.next()
			.expect("Should have a key in keystore");

		let valid_sig = vec![0u8; SERIALIZED_SIG_SIZE]; // Correct size
		let result = constructor.construct_pulse_extrinsic(signer, valid_sig, 100, 101);
		assert!(result.is_ok());
	}

	#[test]
	fn test_account_nonce_api_failure() {
		let client = Arc::new(MockParachainClient::new());
		let keystore = create_test_keystore_with_key();
		let constructor = RuntimeExtrinsicConstructor::new(client, keystore.clone());
		// should force the account nonce api to fail
		let signer = sr25519::Public::from_raw([0u8; 32]);

		let valid_sig = vec![0u8; SERIALIZED_SIG_SIZE]; // Correct size
		let result = constructor.construct_pulse_extrinsic(signer, valid_sig, 100, 101);

		assert!(result.is_err());
		// actual inner error is arbitrary
		assert!(matches!(result, Err(GadgetError::RuntimeApiError(_))));
	}

	#[test]
	fn test_construct_pulse_payload_failure() {
		let client = Arc::new(MockParachainClient::new());
		let keystore = create_test_keystore_with_key();
		let constructor = RuntimeExtrinsicConstructor::new(client, keystore.clone());
		// should force the account nonce api to fail
		let signer = sr25519::Public::from_raw([1u8; 32]);

		let valid_sig = vec![0u8; SERIALIZED_SIG_SIZE];
		// start = 101 is the trigger to mock an error
		let result = constructor.construct_pulse_extrinsic(signer, valid_sig, 101, 101);

		assert!(result.is_err());
		// actual inner error is arbitrary
		assert!(matches!(result, Err(GadgetError::RuntimeApiError(_))));
	}

	#[test]
	fn test_keystore_signing_failure() {
		let client = Arc::new(MockParachainClient::new());
		let keystore = create_test_keystore_with_key();
		let constructor = RuntimeExtrinsicConstructor::new(client, keystore.clone());
		// Use a wrong public key
		let wrong_signer = sr25519::Public::from_raw([1u8; 32]);

		let valid_sig = vec![0u8; SERIALIZED_SIG_SIZE];
		let result = constructor.construct_pulse_extrinsic(wrong_signer, valid_sig, 100, 101);

		assert!(result.is_err());
		assert!(matches!(result, Err(GadgetError::SigningFailed)));
	}

	#[test]
	fn test_keystore_signing_failure_with_empty_keystore() {
		let client = Arc::new(MockParachainClient::new());
		let keystore = create_test_keystore_empty();
		let constructor = RuntimeExtrinsicConstructor::new(client, keystore.clone());
		// Use a wrong public key (since we can't get one from the keystore)
		let wrong_signer = sr25519::Public::from_raw([1u8; 32]);

		let valid_sig = vec![0u8; SERIALIZED_SIG_SIZE];
		let result = constructor.construct_pulse_extrinsic(wrong_signer, valid_sig, 100, 101);

		assert!(result.is_err());
		assert!(matches!(result, Err(GadgetError::SigningFailed)));
	}
}
