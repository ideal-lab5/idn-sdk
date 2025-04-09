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

#[cfg(test)]
pub(crate) mod tests {
	use crate::block_import::*;
	use sc_consensus::{
		block_import::{BlockImport, BlockImportParams},
		BlockCheckParams, ImportResult, ImportedAux,
	};
	use sp_consensus::{BlockOrigin, Error as ClientError};
	use sp_runtime::testing::{H256, Header};

	use sp_consensus_randomness_beacon::digest::ConsensusLog;
	use sp_runtime::generic::DigestItem;

	use futures::StreamExt;
	use std::sync::Arc;
	use tokio::{
		sync::Mutex,
		time::{sleep, Duration},
	};

	pub(crate) type Block = sp_runtime::generic::Block<Header, sp_runtime::OpaqueExtrinsic>;

	#[derive(Clone)]
	pub(crate) struct DummyBlockImport;

	#[async_trait::async_trait]
	impl BlockImport<Block> for DummyBlockImport {
		type Error = ClientError;

		async fn import_block(
			&self,
			_block: BlockImportParams<Block>,
		) -> Result<ImportResult, Self::Error> {
			let res = ImportResult::Imported(ImportedAux {
				header_only: false,
				clear_justification_requests: false,
				needs_justification: false,
				bad_justification: false,
				is_new_best: true,
			});
			Ok(res)
		}

		async fn check_block(
			&self,
			_block: BlockCheckParams<Block>,
		) -> Result<ImportResult, Self::Error> {
			let res = ImportResult::Imported(ImportedAux {
				header_only: false,
				clear_justification_requests: false,
				needs_justification: false,
				bad_justification: false,
				is_new_best: true,
			});
			Ok(res)
		}
	}

	#[tokio::test]
	async fn can_check_block() {
		let (block_import, _round_receiver) = LatestRoundNotifier::new(DummyBlockImport);
		let hash = H256::repeat_byte(1);
		let parent_hash = H256::repeat_byte(2);

		let params = BlockCheckParams {
			hash: hash,
			number: 1,
			parent_hash: parent_hash,
			allow_missing_state: false,
			allow_missing_parent: false,
			import_existing: false,
		};

		let result = block_import.check_block(params).await;
		assert!(result.is_ok());
	}

	#[tokio::test]
	async fn can_import_block_without_digest_log() {
		let (block_import, mut round_receiver) = LatestRoundNotifier::new(DummyBlockImport);
		let header = Header::new_from_number(1);
		let params = BlockImportParams::new(BlockOrigin::Own, header);

		let result = block_import.import_block(params).await;
		assert!(result.is_ok());

		let latest = Arc::new(Mutex::new(0u64));
		let latest_clone = latest.clone();

		tokio::spawn(async move {
			while let Some(rn) = round_receiver.next().await {
				let mut latest = latest_clone.lock().await;
				*latest = rn;
			}
		});

		sleep(Duration::from_secs(1)).await;

		let latest = latest.lock().await;
		assert!(*latest == 0)
	}

	#[tokio::test]
	async fn can_import_block_with_digest_log() {
		let (block_import, mut round_receiver) = LatestRoundNotifier::new(DummyBlockImport);

		let round_number = 777u64;
		let mut header = Header::new_from_number(1);
		let digest_item: DigestItem = ConsensusLog::LatestRoundNumber(round_number).into();
		header.digest.push(digest_item);

		let params = BlockImportParams::new(BlockOrigin::Own, header);

		let result = block_import.import_block(params).await;
		assert!(result.is_ok());

		let latest = Arc::new(Mutex::new(0u64));
		let latest_clone = latest.clone();

		tokio::spawn(async move {
			while let Some(rn) = round_receiver.next().await {
				let mut latest = latest_clone.lock().await;
				*latest = rn;
			}
		});

		sleep(Duration::from_secs(1)).await;

		let latest = latest.lock().await;
		assert!(*latest == round_number)
	}
}
