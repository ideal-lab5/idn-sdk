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

use crate::block_import::PruningBlockImport;
use prometheus_endpoint::Registry;
use sc_consensus::{
	block_import::{BlockImportParams, ForkChoiceStrategy},
	import_queue::{BasicQueue, BoxBlockImport, Verifier},
};
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_consensus_randomness_beacon::types::OpaquePulse;
use sp_runtime::traits::Block as BlockT;

/// The verifier for the manual seal engine; instantly finalizes.
struct ManualSealVerifier;

#[async_trait::async_trait]
impl<B: BlockT> Verifier<B> for ManualSealVerifier {
	async fn verify(
		&self,
		mut block: BlockImportParams<B>,
	) -> Result<BlockImportParams<B>, String> {
		block.finalized = false;
		block.fork_choice = Some(ForkChoiceStrategy::LongestChain);
		Ok(block)
	}
}

/// Instantiate the import queue for the manual seal consensus engine.
pub fn import_queue<Block>(
	block_import: BoxBlockImport<Block>,
	spawner: &impl sp_core::traits::SpawnEssentialNamed,
	registry: Option<&Registry>,
) -> (BasicQueue<Block>, TracingUnboundedReceiver<u64>)
where
	Block: BlockT,
{
	let (block_import, rx) = PruningBlockImport::new(block_import);
	(BasicQueue::new(ManualSealVerifier, Box::new(block_import), None, spawner, registry), rx)
}
