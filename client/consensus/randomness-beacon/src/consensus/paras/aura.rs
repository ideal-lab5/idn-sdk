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

//! An import queue which provides some equivocation resistance with lenient trait bounds.
//!
//! Equivocation resistance in general is a hard problem, as different nodes in the network
//! may see equivocations in a different order, and therefore may not agree on which blocks
//! should be thrown out and which ones should be kept.

use crate::block_import::LatestRoundNotifier;
use codec::Codec;
use cumulus_client_consensus_aura::equivocation_import_queue::Verifier;
use cumulus_client_consensus_common::ParachainBlockImportMarker;

use sc_consensus::{import_queue::BasicQueue, BlockImport, DefaultImportQueue};
use sc_telemetry::TelemetryHandle;
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_consensus::error::Error as ConsensusError;
use sp_consensus_aura::AuraApi;
use sp_core::crypto::Pair;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::traits::Block as BlockT;
use std::{fmt::Debug, sync::Arc};

/// Start an import queue for a Cumulus node which checks blocks' seals and inherent data.
///
/// Pass in only inherent data providers which don't include aura or parachain consensus inherents,
/// e.g. things like timestamp and custom inherents for the runtime.
///
/// The others are generated explicitly internally.
///
/// This should only be used for runtimes where the runtime does not check all inherents and
/// seals in `execute_block` (see <https://github.com/paritytech/cumulus/issues/2436>)
pub fn fully_verifying_import_queue<P, Client, Block: BlockT, I, CIDP>(
	client: Arc<Client>,
	block_import: I,
	create_inherent_data_providers: CIDP,
	spawner: &impl sp_core::traits::SpawnEssentialNamed,
	registry: Option<&prometheus_endpoint::Registry>,
	telemetry: Option<TelemetryHandle>,
) -> (DefaultImportQueue<Block>, TracingUnboundedReceiver<u64>)
where
	P: Pair + 'static,
	P::Signature: Codec,
	P::Public: Codec + Debug,
	I: BlockImport<Block, Error = ConsensusError>
		+ ParachainBlockImportMarker
		+ Send
		+ Sync
		+ 'static,
	Client: ProvideRuntimeApi<Block> + Send + Sync + 'static,
	<Client as ProvideRuntimeApi<Block>>::Api: BlockBuilderApi<Block> + AuraApi<Block, P::Public>,
	CIDP: CreateInherentDataProviders<Block, ()> + 'static,
{
	let verifier: Verifier<P, _, _, _> =
		Verifier::new(client, create_inherent_data_providers, telemetry);
	let (block_import, rx) = LatestRoundNotifier::new(block_import);

	(BasicQueue::new(verifier, Box::new(block_import), None, spawner, registry), rx)
}
