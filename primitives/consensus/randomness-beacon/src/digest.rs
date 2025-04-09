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

use codec::{Decode, Encode};
use sp_core::RuntimeDebug;
use sp_runtime::generic::DigestItem;

/// Custom header digest items, inserted as DigestItem::Other
#[derive(Encode, Decode, Copy, Clone, Eq, PartialEq, RuntimeDebug)]
pub enum ConsensusLog {
	#[codec(index = 0)]
	/// Provides information about the latest drand round number observed by the network
	LatestRoundNumber(u64),
}

/// Convert custom application digest item into a concrete digest item
impl From<ConsensusLog> for DigestItem {
	fn from(val: ConsensusLog) -> Self {
		DigestItem::Other(val.encode())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn encode_decode_latest_round_number() {
		let original_log = ConsensusLog::LatestRoundNumber(42);
		let encoded = original_log.encode();
		let decoded = ConsensusLog::decode(&mut &encoded[..])
			.expect("Decoding should succeed");

		assert_eq!(original_log, decoded);
	}

	#[test]
	fn consensus_log_to_digest_item_and_back() {
		let original_log = ConsensusLog::LatestRoundNumber(123456789);
		let digest_item: DigestItem = original_log.into();

		// Ensure it's a DigestItem::Other and decode it back
		if let DigestItem::Other(data) = digest_item {
			let decoded_log = ConsensusLog::decode(&mut &data[..])
				.expect("Should decode back to ConsensusLog");
			assert_eq!(original_log, decoded_log);
		} else {
			panic!("Expected DigestItem::Other variant");
		}
	}
}