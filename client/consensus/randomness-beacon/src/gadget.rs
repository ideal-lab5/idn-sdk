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

//! Pulse Finalization Gadget
//!
//! Submits signed extrinsics containing verified randomness pulses to the chain.

use crate::{gossipsub::DrandReceiver};
use ark_bls12_381::G1Affine;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use codec::Decode;
use futures::{stream::Fuse, Future, FutureExt, Stream, StreamExt};
use sc_client_api::{FinalityNotification, HeaderBackend};
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver};
use sp_api::ProvideRuntimeApi;
use sp_consensus_randomness_beacon::types::OpaqueSignature;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};
use std::pin::Pin;
use std::{sync::Arc, time::Duration};

const LOG_TARGET: &str = "rand-beacon-gadget";

const SERIALIZED_SIG_SIZE: usize = 48;

/// Trait for submitting pulses to the chain.
///
/// This trait abstracts away runtime-specific details, allowing the gadget
/// to remain independent of any particular runtime implementation.
pub trait PulseSubmitter<Block: BlockT>: Send + Sync {
	/// Submit a pulse extrinsic to the transaction pool.
	///
	/// # Parameters
	/// - `asig`: The aggregated signature as bytes
	/// - `start`: The starting round number
	/// - `end`: The ending round number
	///
	/// # Returns
	/// The hash of the submitted extrinsic
	fn submit_pulse(
		&self,
		asig: OpaqueSignature,
		start: u64,
		end: u64,
	) -> impl std::future::Future<
		Output = Result<Block::Hash, Box<dyn std::error::Error + Send + Sync>>,
	> + Send;
}

/// Finality notification without pinned block references
#[derive(Clone, Debug)]
pub struct UnpinnedFinalityNotification<Block: BlockT> {
	pub hash: Block::Hash,
	pub header: Block::Header,
}

impl<Block: BlockT> From<sc_client_api::FinalityNotification<Block>>
	for UnpinnedFinalityNotification<Block>
{
	fn from(notification: sc_client_api::FinalityNotification<Block>) -> Self {
		Self { hash: notification.hash, header: notification.header }
	}
}

/// Produce a future that transformes finality notifications into a struct that does not keep blocks
/// pinned.
///
/// # Parameters
/// - `finality_notification`: A mutable [`sc_client_api::FinalityNotifications`]
///
///
fn finality_notification_transformer_future<B>(
	mut finality_notifications: sc_client_api::FinalityNotifications<B>,
) -> (
	Pin<Box<futures::future::Fuse<impl Future<Output = ()> + Sized>>>,
	Fuse<TracingUnboundedReceiver<UnpinnedFinalityNotification<B>>>,
)
where
	B: BlockT,
{
	let (tx, rx) = tracing_unbounded("pulse-ingestion-notification-transformer-channel", 10000);
	let transformer_fut = async move {
		while let Some(notification) = finality_notifications.next().await {
			log::debug!(
				target: LOG_TARGET,
				"Finality notification: #{:?}({:?})",
				notification.header.number(),
				notification.hash
			);

			if tx.unbounded_send(UnpinnedFinalityNotification::from(notification)).is_err() {
				log::error!(target: LOG_TARGET, "Finality transformer channel closed, shutting down");
				return;
			}
		}
	};
	(Box::pin(transformer_fut.fuse()), rx.fuse())
}

/// Runs as a background task, monitoring for new randomness pulses and
/// submitting them to the chain via signed extrinsics.
pub struct PulseFinalizationGadget<Block: BlockT, S, Client, const MAX_QUEUE_SIZE: usize> {
	client: Arc<Client>,
	pulse_submitter: Arc<S>,
	pulse_receiver: DrandReceiver<MAX_QUEUE_SIZE>, // Add this
	_phantom: std::marker::PhantomData<Block>,
}

impl<Block: BlockT, S: PulseSubmitter<Block>, Client, const MAX_QUEUE_SIZE: usize>
	PulseFinalizationGadget<Block, S, Client, MAX_QUEUE_SIZE>
where
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
{
	pub fn new(
		client: Arc<Client>,
		pulse_submitter: Arc<S>,
		pulse_receiver: DrandReceiver<MAX_QUEUE_SIZE>,
	) -> Self {
		Self { client, pulse_submitter, pulse_receiver, _phantom: Default::default() }
	}

	/// Run the main loop indefinitely.
	///
	/// This function checks for new pulses whenever it receives a finality notification
	/// and submits their aggregated signature to the chain.
	pub async fn run(
		self,
		finality_notifications: sc_client_api::FinalityNotifications<Block>,
	) {
		log::info!(target: LOG_TARGET, "🎲 Starting Pulse Finalization Gadget");
		// Subscribe to finality notifications and justifications before waiting for runtime pallet and
		// reuse the streams, so we don't miss notifications while waiting for pallet to be available.
		// let finality_notifications = self.client.finality_notification_stream();
		let (transformer, mut finality_notifications) =
			finality_notification_transformer_future(finality_notifications);

		// Spawn transformer as background task
		tokio::spawn(transformer);

		while let Some(notification) = finality_notifications.next().await {
			if let Err(e) = self.handle_finality_notification(&notification).await {
				log::error!(target: LOG_TARGET, "Error handling finality: {:?}", e);
			}
		}

		log::error!(target: LOG_TARGET, "Finality notification stream ended");
	}

    /// Attempts to submit new pulses when it receives a notification with a valid header
	async fn handle_finality_notification(
		&self,
		notification: &UnpinnedFinalityNotification<Block>,
	) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
		// get latest round from header
		let latest_round = Self::extract_latest_round(&notification.header).unwrap_or(0);
		// get fresh pulses
		let pulses = self.pulse_receiver.read().await;
		let new_pulses: Vec<_> = pulses.into_iter().filter(|p| p.round > latest_round).collect();

		if new_pulses.is_empty() {
			return Ok(());
		}

		log::info!(
			target: LOG_TARGET,
			"Block #{:?} finalized (round {}), submitting {} new pulses",
			notification.header.number(),
			latest_round,
			new_pulses.len()
		);

		let start = new_pulses.first().unwrap().round;
		let end = new_pulses.last().unwrap().round;
		let start = new_pulses.first().unwrap().round;
		let end = new_pulses.last().unwrap().round;
		// aggregate sigs
		let asig = new_pulses
			.into_iter()
			.filter_map(|pulse| {
				let bytes = pulse.signature;
				G1Affine::deserialize_compressed(&mut bytes.as_ref()).ok()
			})
			.fold(sp_idn_crypto::bls12_381::zero_on_g1(), |acc, sig| (acc + sig).into());

		let mut asig_bytes = Vec::with_capacity(SERIALIZED_SIG_SIZE);
		// [SRLABS]: This error is untestable since we know the signature is correct here.
		//  Is it reasonable to use an expect?
		asig.serialize_compressed(&mut asig_bytes)
			.expect("The signature is well formatted. qed.");

		self.pulse_submitter.submit_pulse(asig_bytes.try_into().unwrap(), start, end).await?;
		Ok(())
	}

	/// extract the latest round number from block header digest logs
	///
	/// * `header`: The block header to extract digest logs from
	///
	fn extract_latest_round(header: &Block::Header) -> Option<u64> {
		use sp_consensus_randomness_beacon::digest::ConsensusLog;

		for log in header.digest().logs() {
			if let Some((pre, _)) = log.as_consensus() {
				if let Ok(ConsensusLog::LatestRoundNumber(round)) =
					ConsensusLog::decode(&mut &pre[..])
				{
					return Some(round);
				}
			}
		}
		None
	}
}

#[cfg(test)]
mod tests {
    use super::*;
    use codec::Encode;
    use futures::channel::mpsc;
    use sc_client_api::FinalityNotification;
    use sp_consensus_randomness_beacon::{
        digest::ConsensusLog,
        types::CanonicalPulse,
        RANDOMNESS_BEACON_ID,
    };
    use sp_runtime::{
        generic::Header,
        traits::{BlakeTwo256, Block as BlockT},
        Digest, DigestItem,
    };
    use std::sync::{Arc, Mutex};

    const MAX_QUEUE_SIZE: usize = 100;

    // Define a minimal test block type
    type TestBlock = sp_runtime::generic::Block<
        Header<u64, BlakeTwo256>,
        sp_runtime::OpaqueExtrinsic,
    >;

    // Mock pulse submitter
    struct MockPulseSubmitter {
        submissions: Arc<Mutex<Vec<(Vec<u8>, u64, u64)>>>,
    }

    impl MockPulseSubmitter {
        fn new() -> Self {
            Self { submissions: Arc::new(Mutex::new(Vec::new())) }
        }

        fn get_submissions(&self) -> Vec<(Vec<u8>, u64, u64)> {
            self.submissions.lock().unwrap().clone()
        }
    }

    impl PulseSubmitter<TestBlock> for MockPulseSubmitter {
        async fn submit_pulse(
            &self,
            asig: Vec<u8>,
            start: u64,
            end: u64,
        ) -> Result<<TestBlock as BlockT>::Hash, Box<dyn std::error::Error + Send + Sync>> {
            self.submissions.lock().unwrap().push((asig, start, end));
            Ok(Default::default())
        }
    }

    // Minimal mock client
    struct MockClient;


	


    impl sc_client_api::HeaderBackend<TestBlock> for MockClient {
        fn header(
            &self,
            _hash: <TestBlock as BlockT>::Hash,
        ) -> sp_blockchain::Result<Option<<TestBlock as BlockT>::Header>> {
            Ok(None)
        }

        fn info(&self) -> sc_client_api::blockchain::Info<TestBlock> {
            unimplemented!("Not needed for this test")
        }

        fn status(
            &self,
            _hash: <TestBlock as BlockT>::Hash,
        ) -> sp_blockchain::Result<sc_client_api::blockchain::BlockStatus> {
            unimplemented!("Not needed for this test")
        }

        fn number(
            &self,
            _hash: <TestBlock as BlockT>::Hash,
        ) -> sp_blockchain::Result<Option<u64>> {
            Ok(None)
        }

        fn hash(&self, _number: u64) -> sp_blockchain::Result<Option<<TestBlock as BlockT>::Hash>> {
            Ok(None)
        }
    }

	// #[derive(Clone)]
	// pub(crate) struct TestApi {
	// 	pub(crate) authorities: Vec<AuthorityId>,
	// }

	// impl ProvideRuntimeApi<Block> for MockClient {
	// 	type Api = RuntimeApi;

	// 	fn runtime_api(&self) -> ApiRef<'_, Self::Api> {
	// 		RuntimeApi { authorities: self.authorities.clone() }.into()
	// 	}
	// }

    // impl sp_api::ProvideRuntimeApi<TestBlock> for MockClient {
    //     type Api = ();
    //     fn runtime_api(&self) -> sp_api::ApiRef<Self::Api> {
    //         unimplemented!("Not needed for this test")
    //     }
    // }

    fn create_header_with_round(number: u64, round: u64) -> Header<u64, BlakeTwo256> {
        let mut digest = Digest::default();
        let log = ConsensusLog::LatestRoundNumber(round);
        let digest_item = DigestItem::Consensus(RANDOMNESS_BEACON_ID, log.encode());
        digest.push(digest_item);

        Header::new(number, Default::default(), Default::default(), Default::default(), digest)
    }

    fn create_test_pulse(round: u64) -> CanonicalPulse {
        CanonicalPulse { round, signature: vec![0u8; SERIALIZED_SIG_SIZE] }
    }

    #[tokio::test]
    async fn test_gadget_submits_new_pulses_on_finality() {
        let mock_submitter = Arc::new(MockPulseSubmitter::new());
        let mock_client = Arc::new(MockClient);

        let (tx, rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
        let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(rx);

        // Add test pulses
        tx.unbounded_send(create_test_pulse(100)).unwrap();
        tx.unbounded_send(create_test_pulse(101)).unwrap();

        let gadget = PulseFinalizationGadget::new(
            mock_client,
            mock_submitter.clone(),
            pulse_receiver,
        );

        let (mut finality_tx, finality_rx) = mpsc::unbounded();

        tokio::spawn(async move {
            gadget.run(finality_rx).await;
        });

        // Send finality notification
        let header = create_header_with_round(1, 99);
        finality_tx
            .unbounded_send(FinalityNotification {
                hash: Default::default(),
                header,
                tree_route: Arc::new(vec![]),
                stale_heads: Arc::new(vec![]),
            })
            .unwrap();

        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify submission
        let submissions = mock_submitter.get_submissions();
        assert_eq!(submissions.len(), 1);
        assert_eq!(submissions[0].1, 100); // start
        assert_eq!(submissions[0].2, 101); // end
    }
}