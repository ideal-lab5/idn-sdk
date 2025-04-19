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

//! Manual-seal block import logic
//! > Note: This should be used for **testing purposes only**!
//!
//! # Overview
//! This is the manual seal block import and verification logic using the `LatestRoundNotifier`
//! block import wrapper.

use crate::block_import::LatestRoundNotifier;
use prometheus_endpoint::Registry;
use sc_consensus::{
	block_import::{BlockImportParams, ForkChoiceStrategy},
	import_queue::{BasicQueue, BoxBlockImport, Verifier},
};
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_runtime::traits::Block as BlockT;

/// The verifier for the manual seal engine; instantly finalizes.
struct ManualSealVerifier;

/// The basic manual seal verifier
/// The impl is not public, so we copy it here.
/// see: https://github.com/paritytech/polkadot-sdk/blob/c31bf227ebd5d8c2f52bb376d8ef28cb2d0e946c/substrate/client/consensus/manual-seal/src/lib.rs#L65
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
	let (block_import, rx) = LatestRoundNotifier::new(block_import);
	(BasicQueue::new(ManualSealVerifier, Box::new(block_import), None, spawner, registry), rx)
}

#[cfg(test)]
mod tests {

	use super::*;
	use crate::tests::tests::{Block, DummyBlockImport};
	use sc_consensus::block_import::BlockImportParams;
	use sp_consensus::BlockOrigin;
	use sp_core::testing::TaskExecutor;
	use sp_runtime::testing::Header;

	#[tokio::test]
	async fn test_can_verify_block() {
		let header = Header::new_from_number(1);
		let params: BlockImportParams<Block> = BlockImportParams::new(BlockOrigin::Own, header);

		let verified = ManualSealVerifier.verify(params).await.expect("verifier should succeed");

		assert_eq!(verified.finalized, false);
		assert_eq!(verified.fork_choice, Some(ForkChoiceStrategy::LongestChain));
	}

	#[test]
	fn test_can_build_import_queue() {
		let block_import = Box::new(DummyBlockImport) as BoxBlockImport<Block>;
		let spawner = TaskExecutor::new();

		let (queue, rx) = import_queue::<Block>(block_import, &spawner, None);

		assert!(std::mem::size_of_val(&queue) > 0);
		assert!(rx.len() == 0);
	}
}
