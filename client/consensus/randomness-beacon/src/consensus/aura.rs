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

use crate::{block_import::LatestRoundNotifier, gossipsub::DrandReceiver};
use prometheus_endpoint::Registry;
use sc_consensus::{
	block_import::{BlockImportParams, ForkChoiceStrategy},
	import_queue::{BasicQueue, BoxBlockImport, DefaultImportQueue, Verifier},
};
use sc_consensus_slots::InherentDataProviderExt;
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::Error as ConsensusError;
use sp_consensus_aura::{
	build_verifier, inherents::AuraInherentData, AuraApi, BuildVerifierParams,
};
use sp_consensus_randomness_beacon::types::OpaquePulse;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::traits::Block as BlockT;

/// Start an import queue for the Aura consensus algorithm.
pub fn import_queue<P, Block, I, C, S, CIDP>(
	ImportQueueParams {
		block_import,
		justification_import,
		client,
		create_inherent_data_providers,
		spawner,
		registry,
		check_for_equivocation,
		telemetry,
		compatibility_mode,
	}: ImportQueueParams<Block, I, C, S, CIDP>,
) -> Result<DefaultImportQueue<Block>, sp_consensus::Error>
where
	Block: BlockT,
	C::Api: BlockBuilderApi<Block> + AuraApi<Block, AuthorityId<P>> + ApiExt<Block>,
	C: 'static
		+ ProvideRuntimeApi<Block>
		+ BlockOf
		+ Send
		+ Sync
		+ AuxStore
		+ UsageProvider<Block>
		+ HeaderBackend<Block>,
	I: BlockImport<Block, Error = ConsensusError> + Send + Sync + 'static,
	P: Pair + 'static,
	P::Public: Codec + Debug,
	P::Signature: Codec,
	S: sp_core::traits::SpawnEssentialNamed,
	CIDP: CreateInherentDataProviders<Block, ()> + Sync + Send + 'static,
	CIDP::InherentDataProviders: InherentDataProviderExt + Send + Sync,
{
	let verifier = build_verifier::<P, _, _, _>(BuildVerifierParams {
		client,
		create_inherent_data_providers,
		check_for_equivocation,
		telemetry,
		compatibility_mode,
	});

	Ok(BasicQueue::new(verifier, Box::new(block_import), justification_import, spawner, registry))
}
