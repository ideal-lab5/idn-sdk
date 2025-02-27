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

//! # Inherent-Data Sync based consensus mechanism
//!
//! This consensus mechanism produces blocks whenever it has the necessary data as required by inherent data providers.
//! Specifically, the goal is to author a block whenever there are N new messages in a queue
//!

// use futures::prelude::*;
// use futures_timer::Delay;
// use prometheus_endpoint::Registry;
// use sc_client_api::{
// 	backend::{Backend as ClientBackend, Finalizer},
// 	client::BlockchainEvents,
// };
// use sc_consensus::{
// 	block_import::{BlockImport, BlockImportParams, ForkChoiceStrategy},
// 	import_queue::{BasicQueue, BoxBlockImport, Verifier},
// };
use crate::gossipsub::{GossipsubNetwork, GossipsubState, SharedState};
use libp2p::{gossipsub::Config as GossipsubConfig, identity::Keypair};
use sc_consensus_manual_seal::{
	consensus::ConsensusDataProvider, run_manual_seal, EngineCommand, ManualSealParams,
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
// use sp_consensus::{Environment, Proposer, SelectChain};
// use sp_core::traits::SpawnNamed;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::{traits::Block as BlockT, ConsensusEngineId};
use std::{
	marker::PhantomData,
	sync::{Arc, Mutex},
	time::Duration,
};

const LOG_TARGET: &str = "drand-lock-step";

// / The `ConsensusEngineId` of Manual Seal.
// pub const MANUAL_SEAL_ENGINE_ID: ConsensusEngineId = [b'm', b'a', b'n', b'l'];

/// Params required to start the data-sync sealing authorship task.
pub struct ConsensusParams<B: BlockT, BI, E, C: ProvideRuntimeApi<B>, TP, SC, CIDP, P> {
	/// Block import instance for well. importing blocks.
	pub block_import: BI,

	/// The environment we are producing blocks for.
	pub env: E,

	/// Client instance
	pub client: Arc<C>,

	/// Shared reference to the transaction pool.
	pub pool: Arc<TP>,

	/// SelectChain strategy.
	pub select_chain: SC,

	/// Digest provider for inclusion in blocks.
	pub consensus_data_provider: Option<Box<dyn ConsensusDataProvider<B, Proof = P>>>,

	/// Something that can create the inherent data providers.
	pub create_inherent_data_providers: CIDP,
}

// /// runs the background authorship task for the randomness-beacon pulse-based seal engine.
// /// It creates a new block for every N pulses from a randomness beacon ingested into a queue
// pub async fn run<B, BI, CB, E, C, TP, SC, CIDP, P>(
// 	ConsensusParams {
// 		block_import,
// 		env,
// 		client,
// 		pool,
// 		select_chain,
// 		consensus_data_provider,
// 		create_inherent_data_providers,
// 	}: ConsensusParams<B, BI, E, C, TP, SC, CIDP, P>,
// ) where
// 	B: BlockT + 'static,
// 	BI: BlockImport<B, Error = sp_consensus::Error> + Send + Sync + 'static,
// 	C: HeaderBackend<B> + Finalizer<B, CB> + ProvideRuntimeApi<B> + 'static,
// 	CB: ClientBackend<B> + 'static,
// 	E: Environment<B> + 'static,
// 	E::Proposer: Proposer<B, Proof = P>,
// 	SC: SelectChain<B> + 'static,
// 	TP: TransactionPool<Block = B>,
// 	CIDP: CreateInherentDataProviders<B, ()>,
// 	P: codec::Encode + Send + Sync + 'static,
// {
// 	// instant-seal creates blocks as soon as transactions are imported
// 	// into the transaction pool.
// 	let commands_stream = pool.import_notification_stream().map(|_| EngineCommand::SealNewBlock {
// 		create_empty: true,
// 		finalize: false,
// 		parent_hash: None,
// 		sender: None,
// 	});

// 	run_manual_seal(ManualSealParams {
// 		block_import,
// 		env,
// 		client,
// 		pool,
// 		commands_stream,
// 		select_chain,
// 		consensus_data_provider,
// 		create_inherent_data_providers,
// 	})
// 	.await
// }

#[cfg(test)]
mod tests {
	use super::*;
	use futures::StreamExt;
	use sc_basic_authorship::ProposerFactory;
	use sc_consensus::{BlockImportParams, ImportedAux};
	use sc_consensus_manual_seal::{CreatedBlock, Error};
	use sc_transaction_pool::{BasicPool, FullChainApi, Options, RevalidationType};
	use sc_transaction_pool_api::{MaintainedTransactionPool, TransactionPool, TransactionSource};
	use sp_inherents::InherentData;
	use sp_runtime::generic::{Digest, DigestItem};
	use substrate_test_runtime_client::AccountKeyring::Alice;
	use substrate_test_runtime_client::{
		DefaultTestClientBuilderExt, TestClientBuilder, TestClientBuilderExt,
	};
	use substrate_test_runtime_transaction_pool::{uxt, TestApi};

	fn api() -> Arc<TestApi> {
		Arc::new(TestApi::empty())
	}

	const SOURCE: TransactionSource = TransactionSource::External;

	struct TestDigestProvider<C> {
		_client: Arc<C>,
	}
	impl<B, C> ConsensusDataProvider<B> for TestDigestProvider<C>
	where
		B: BlockT,
		C: ProvideRuntimeApi<B> + Send + Sync,
	{
		type Proof = ();

		fn create_digest(
			&self,
			_parent: &B::Header,
			_inherents: &InherentData,
		) -> Result<Digest, Error> {
			Ok(Digest { logs: vec![] })
		}

		fn append_block_import(
			&self,
			_parent: &B::Header,
			params: &mut BlockImportParams<B>,
			_inherents: &InherentData,
			_proof: Self::Proof,
		) -> Result<(), Error> {
			params.post_digests.push(DigestItem::Other(vec![1]));
			Ok(())
		}
	}

	async fn run_gossipsub() -> SharedState {
		let topic_str: &str =
			"/drand/pubsub/v0.0.0/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";

		let maddr1: libp2p::Multiaddr =
			"/ip4/184.72.27.233/tcp/44544/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG"
				.parse()
				.expect("The string is a well-formatted multiaddress. qed.");

		let maddr2: libp2p::Multiaddr =
        "/ip4/54.193.191.250/tcp/44544/p2p/12D3KooWQqDi3D3KLfDjWATQUUE4o5aSshwBFi9JM36wqEPMPD5y"
            .parse()
            .expect("The string is a well-formatted multiaddress. qed.");

		let local_identity: Keypair = Keypair::generate_ed25519();
		let state = Arc::new(Mutex::new(GossipsubState { pulses: vec![] }));
		let gossipsub_config = GossipsubConfig::default();
		let mut gossipsub =
			GossipsubNetwork::new(&local_identity, state.clone(), gossipsub_config).unwrap();

		tokio::spawn(async move {
			if let Err(e) = gossipsub.run(topic_str, vec![&maddr1, &maddr2], None).await {
				log::error!("Failed to run gossipsub network: {:?}", e);
			}
		});

		state
	}

	#[tokio::test]
	async fn instant_seal() {
		let builder = TestClientBuilder::new();
		let (client, select_chain) = builder.build_with_longest_chain();
		let client = Arc::new(client);
		let spawner = sp_core::testing::TaskExecutor::new();
		let genesis_hash = client.info().genesis_hash;
		let pool_api = Arc::new(FullChainApi::new(client.clone(), None, &spawner.clone()));
		let pool = Arc::new(BasicPool::with_revalidation_type(
			Options::default(),
			true.into(),
			pool_api,
			None,
			RevalidationType::Full,
			spawner.clone(),
			0,
			genesis_hash,
			genesis_hash,
		));

		let env = ProposerFactory::new(spawner.clone(), client.clone(), pool.clone(), None, None);
		// this test checks that blocks are created as soon as transactions are imported into the
		// pool.
		let (sender, receiver) = futures::channel::oneshot::channel();
		let mut sender = Arc::new(Some(sender));

		// so we want to redefine the commands stream
		let mut shared_state = run_gossipsub().await;
		// shared_state. 


		let commands_stream =
			pool.pool().validated_pool().import_notification_stream().map(move |_| {
				// we're only going to submit one tx so this fn will only be called once.
				let mut_sender = Arc::get_mut(&mut sender).unwrap();
				let sender = std::mem::take(mut_sender);
				EngineCommand::SealNewBlock {
					create_empty: false,
					finalize: true,
					parent_hash: None,
					sender,
				}
			});

		// spawn the background authorship task
		tokio::spawn(run_manual_seal(ManualSealParams {
			block_import: client.clone(),
			env,
			client: client.clone(),
			pool: pool.clone(),
			commands_stream,
			select_chain,
			create_inherent_data_providers: |_, _| async { Ok(()) },
			consensus_data_provider: None,
		}));

		// submit a transaction to pool.
		let result = pool.submit_one(genesis_hash, SOURCE, uxt(Alice, 0)).await;
		// assert that it was successfully imported
		assert!(result.is_ok());
		// assert that the background task returns ok
		let created_block = receiver.await.unwrap().unwrap();
		assert_eq!(
			created_block,
			CreatedBlock {
				hash: created_block.hash,
				aux: ImportedAux {
					header_only: false,
					clear_justification_requests: false,
					needs_justification: false,
					bad_justification: false,
					is_new_best: true,
				},
				proof_size: 0
			}
		);
		// assert that there's a new block in the db.
		assert!(client.header(created_block.hash).unwrap().is_some());
		assert_eq!(client.header(created_block.hash).unwrap().unwrap().number, 1)
	}
}
