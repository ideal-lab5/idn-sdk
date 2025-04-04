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

use crate::gossipsub::DrandReceiver;
use sc_consensus::block_import::{BlockImport, BlockImportParams};
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use sp_consensus::Error as ClientError;
use sp_consensus_randomness_beacon::digest::ConsensusLog;
use sp_runtime::{
	generic::OpaqueDigestItemId,
	traits::{Block as BlockT, Header as HeaderT},
};

const LATEST_ROUND_IMPORT_CHANNEL: &str = "LatestRoundImportChannel";

/// Custom wrapper to prune local storage based on digest log
pub struct PruningBlockImport<BI> {
	inner: BI,
	sender: TracingUnboundedSender<u64>,
}

impl<BI> PruningBlockImport<BI> {
	/// Create a new instance
	pub fn new(inner: BI) -> (Self, TracingUnboundedReceiver<u64>) {
		let (sender, receiver) = tracing_unbounded(LATEST_ROUND_IMPORT_CHANNEL, 1000);
		(Self { inner, sender }, receiver)
	}
}

#[async_trait::async_trait]
impl<Block, BI> BlockImport<Block> for PruningBlockImport<BI>
where
	Block: BlockT,
	BI: BlockImport<Block> + Send + Sync,
	BI::Error: Into<ClientError>,
{
	type Error = ClientError;

	async fn check_block(
		&self,
		block: sc_consensus::BlockCheckParams<Block>,
	) -> Result<sc_consensus::ImportResult, Self::Error> {
		self.inner.check_block(block).await.map_err(Into::into)
	}

	async fn import_block(
		&self,
		params: BlockImportParams<Block>,
	) -> Result<sc_consensus::ImportResult, Self::Error> {
		let header = params.header.clone();

		if let Some(round) = header.digest().convert_first(|l| {
			l.try_to(OpaqueDigestItemId::Other).and_then(|log: ConsensusLog| match log {
				ConsensusLog::LatestRoundNumber(round) => Some(round),
			})
		}) {
			let _ = self.sender.unbounded_send(round);
		}

		// Proceed with the inner block import
		self.inner.import_block(params).await.map_err(Into::into)
	}
}
