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

// External crates imports
use alloc::vec;
use codec::Encode;
use frame_support::{
	dispatch::DispatchInfo,
	genesis_builder_helper::{build_state, get_preset},
	weights::Weight,
};
use frame_system::limits::BlockWeights;
use pallet_aura::Authorities;
use pallet_revive::AddressMapper;
use sp_api::impl_runtime_apis;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, U256};
use sp_runtime::{
	traits::{Block as BlockT, TransactionExtension},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult,
};
use sp_std::prelude::Vec;
use sp_version::RuntimeVersion;

// Local module imports
use super::{
	configs::RuntimeBlockWeights, AccountId, Balance, Block, BlockNumber, ConsensusHook, Contracts,
	EventRecord, Executive, Hash, InherentDataExt, Nonce, ParachainSystem, Revive, Runtime,
	RuntimeCall, RuntimeGenesisConfig, RuntimeOrigin, SessionKeys, System, TransactionPayment,
	UncheckedExtrinsic, CONTRACTS_DEBUG_OUTPUT, CONTRACTS_EVENTS, SLOT_DURATION, VERSION,
};
#[cfg(feature = "runtime-benchmarks")]
use crate::benchmarks::*;

impl_runtime_apis! {
	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
		}

		fn authorities() -> Vec<AuraId> {
			Authorities::<Runtime>::get().into_inner()
		}
	}

	impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
		fn can_build_upon(
			included_hash: <Block as BlockT>::Hash,
			slot: cumulus_primitives_aura::Slot
		) -> bool {
			ConsensusHook::can_build_upon(included_hash, slot)
		}
	}

	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Runtime::metadata().into())
		}

		fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData,
		) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx, block_hash)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
			SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
		fn account_nonce(account: AccountId) -> Nonce {
			System::account_nonce(account)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
		fn query_info(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_info(uxt, len)
		}
		fn query_fee_details(
			uxt: <Block as BlockT>::Extrinsic,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_fee_details(uxt, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
		fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

	impl pallet_revive::ReviveApi<Block, AccountId, Balance, Nonce, BlockNumber> for Runtime
	{
		fn balance(address: H160) -> U256 {
			Revive::evm_balance(&address)
		}

		fn block_gas_limit() -> U256 {
			Revive::evm_block_gas_limit()
		}

		fn gas_price() -> U256 {
			Revive::evm_gas_price()
		}

		fn nonce(address: H160) -> Nonce {
			let account = <Runtime as pallet_revive::Config>::AddressMapper::to_account_id(&address);
			System::account_nonce(account)
		}

		fn eth_transact(tx: pallet_revive::evm::GenericTransaction) -> Result<pallet_revive::EthTransactInfo<Balance>, pallet_revive::EthTransactError>
		{
			let blockweights: BlockWeights = <Runtime as frame_system::Config>::BlockWeights::get();
			let tx_fee = |pallet_call, mut dispatch_info: DispatchInfo| {
				let call = RuntimeCall::Revive(pallet_call);
				let extension = (
					frame_system::CheckNonZeroSender::<Runtime>::new(),
					frame_system::CheckSpecVersion::<Runtime>::new(),
					frame_system::CheckTxVersion::<Runtime>::new(),
					frame_system::CheckGenesis::<Runtime>::new(),
					frame_system::CheckEra::<Runtime>::from(crate::generic::Era::Immortal),
					frame_system::CheckNonce::<Runtime>::from(0), // Using 0 since nonce is not defined in this scope
					frame_system::CheckWeight::<Runtime>::new(),
					pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(0u32.into()),
					frame_metadata_hash_extension::CheckMetadataHash::<Runtime>::new(false),
					frame_system::WeightReclaim::<Runtime>::new(),
				);
				dispatch_info.extension_weight = extension.weight(&call);
				let uxt: UncheckedExtrinsic = sp_runtime::generic::UncheckedExtrinsic::new_bare(call);

				pallet_transaction_payment::Pallet::<Runtime>::compute_fee(
					uxt.encoded_size() as u32,
					&dispatch_info,
					0u32.into(),
				)
			};

			Revive::bare_eth_transact(tx, blockweights.max_block, tx_fee)
		}

		fn call(
			origin: AccountId,
			dest: H160,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_revive::ContractResult<pallet_revive::ExecReturnValue, Balance> {
			Revive::bare_call(
				RuntimeOrigin::signed(origin),
				dest,
				value,
				gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block),
				pallet_revive::DepositLimit::Balance(storage_deposit_limit.unwrap_or(u128::MAX)),
				input_data,
			)
		}

		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			code: pallet_revive::Code,
			data: Vec<u8>,
			salt: Option<[u8; 32]>,
		) -> pallet_revive::ContractResult<pallet_revive::InstantiateReturnValue, Balance>
		{
			Revive::bare_instantiate(
				RuntimeOrigin::signed(origin),
				value,
				gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block),
				pallet_revive::DepositLimit::Balance(storage_deposit_limit.unwrap_or(u128::MAX)),
				code,
				data,
				salt,
			)
		}

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
		) -> pallet_revive::CodeUploadResult<Balance>
		{
			Revive::bare_upload_code(
				RuntimeOrigin::signed(origin),
				code,
				storage_deposit_limit.unwrap_or(u128::MAX),
			)
		}

		fn get_storage(
			address: H160,
			key: [u8; 32],
		) -> pallet_revive::GetStorageResult {
			Revive::get_storage(
				address,
				key
			)
		}

		fn trace_block(
			block: Block,
			tracer_type: pallet_revive::evm::TracerType,
		) -> Vec<(u32, pallet_revive::evm::Trace)> {
			use pallet_revive::tracing::trace;
			let mut tracer = Revive::evm_tracer(tracer_type);
			let mut traces = vec![];
			let (header, extrinsics) = block.deconstruct();
			Executive::initialize_block(&header);
			for (index, ext) in extrinsics.into_iter().enumerate() {
				trace(tracer.as_tracing(), || {
					let _ = Executive::apply_extrinsic(ext);
				});

				if let Some(tx_trace) = tracer.collect_trace() {
					traces.push((index as u32, tx_trace));
				}
			}

			traces
		}

		fn trace_tx(
			block: Block,
			tx_index: u32,
			tracer_type: pallet_revive::evm::TracerType,
		) -> Option<pallet_revive::evm::Trace> {
			use pallet_revive::tracing::trace;
			let mut tracer = Revive::evm_tracer(tracer_type);
			let (header, extrinsics) = block.deconstruct();

			Executive::initialize_block(&header);
			for (index, ext) in extrinsics.into_iter().enumerate() {
				if index as u32 == tx_index {
				trace(tracer.as_tracing(), || {
						let _ = Executive::apply_extrinsic(ext);
					});
					break;
				} else {
					let _ = Executive::apply_extrinsic(ext);
				}
			}

			tracer.collect_trace()
		}

		fn trace_call(
			tx: pallet_revive::evm::GenericTransaction,
			tracer_type: pallet_revive::evm::TracerType,
			)
			-> Result<pallet_revive::evm::Trace, pallet_revive::EthTransactError>
		{
			use pallet_revive::tracing::trace;
			let mut tracer = Revive::evm_tracer(tracer_type);
			let result = trace(tracer.as_tracing(), || Self::eth_transact(tx));

			if let Some(trace) = tracer.collect_trace() {
				Ok(trace)
			} else if let Err(err) = result {
				Err(err)
			} else {
				Ok(tracer.empty_trace())
			}
		}
	}

	impl pallet_contracts::ContractsApi<Block, AccountId, Balance, BlockNumber, Hash, EventRecord>
		for Runtime
	{
		fn call(
			origin: AccountId,
			dest: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			input_data: Vec<u8>,
		) -> pallet_contracts::ContractExecResult<Balance, EventRecord> {
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
			Contracts::bare_call(
				origin,
				dest,
				value,
				gas_limit,
				storage_deposit_limit,
				input_data,
				CONTRACTS_DEBUG_OUTPUT,
				CONTRACTS_EVENTS,
				pallet_contracts::Determinism::Enforced,
			)
		}

		fn instantiate(
			origin: AccountId,
			value: Balance,
			gas_limit: Option<Weight>,
			storage_deposit_limit: Option<Balance>,
			code: pallet_contracts::Code<Hash>,
			data: Vec<u8>,
			salt: Vec<u8>,
		) -> pallet_contracts::ContractInstantiateResult<AccountId, Balance, EventRecord>
		{
			let gas_limit = gas_limit.unwrap_or(RuntimeBlockWeights::get().max_block);
			Contracts::bare_instantiate(
				origin,
				value,
				gas_limit,
				storage_deposit_limit,
				code,
				data,
				salt,
				CONTRACTS_DEBUG_OUTPUT,
				CONTRACTS_EVENTS,
			)
		}

		fn upload_code(
			origin: AccountId,
			code: Vec<u8>,
			storage_deposit_limit: Option<Balance>,
			determinism: pallet_contracts::Determinism,
		) -> pallet_contracts::CodeUploadResult<Hash, Balance>
		{
			Contracts::bare_upload_code(origin, code, storage_deposit_limit, determinism)
		}

		fn get_storage(
			address: AccountId,
			key: Vec<u8>,
		) -> pallet_contracts::GetStorageResult {
			Contracts::get_storage(address, key)
		}
	}


	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info(header)
		}
	}

	#[cfg(feature = "try-runtime")]
	impl frame_try_runtime::TryRuntime<Block> for Runtime {
		fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			use super::configs::RuntimeBlockWeights;

			let weight = Executive::try_runtime_upgrade(checks).unwrap();
			(weight, RuntimeBlockWeights::get().max_block)
		}

		fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect,
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	impl frame_benchmarking::Benchmark<Block> for Runtime {
		fn benchmark_metadata(extra: bool) -> (
			Vec<frame_benchmarking::BenchmarkList>,
			Vec<frame_support::traits::StorageInfo>,
		) {
			let mut list = Vec::<BenchmarkList>::new();
			list_benchmarks!(list, extra);

			let storage_info = AllPalletsWithSystem::storage_info();
			(list, storage_info)
		}

		fn dispatch_benchmark(
			config: frame_benchmarking::BenchmarkConfig
		) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, alloc::string::String> {
			let whitelist = AllPalletsWithSystem::whitelisted_storage_keys();
			let mut batches = Vec::<BenchmarkBatch>::new();
			let params = (&config, &whitelist);
			add_benchmarks!(params, batches);

			if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
			Ok(batches)
		}
	}

	impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
		fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
			build_state::<RuntimeGenesisConfig>(config)
		}

		fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
			get_preset::<RuntimeGenesisConfig>(id, crate::genesis_config_presets::get_preset)
		}

		fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
			crate::genesis_config_presets::preset_names()
		}
	}
}
