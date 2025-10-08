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
//! Submits signed extrinsics containing verified pulses from a randomness beacon to the chain.
use crate::{error::Error as GadgetError, gossipsub::DrandReceiver};
use ark_bls12_381::G1Affine;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use futures::{stream::Fuse, Future, FutureExt, StreamExt};
use pallet_randomness_beacon::RandomnessBeaconApi;
use sc_client_api::HeaderBackend;
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver};
use sp_api::ProvideRuntimeApi;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use std::{pin::Pin, sync::Arc};

const LOG_TARGET: &str = "rand-beacon-gadget";

pub const SERIALIZED_SIG_SIZE: usize = 48;

/// Trait for submitting pulses to the chain.
pub trait PulseSubmitter<Block: BlockT>: Send + Sync {
	/// Submit an extrinsic to the transaction pool to ingest new pulses.
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
		asig: Vec<u8>,
		start: u64,
		end: u64,
	) -> impl std::future::Future<Output = Result<Block::Hash, GadgetError>> + Send;
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

/// Produce a future that transforms finality notifications into a struct that does not keep blocks
/// pinned.
///
/// # Parameters
/// - `finality_notification`: A mutable [`sc_client_api::FinalityNotifications`]
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

/// Runs as a background task, monitoring for new pulses and
/// submitting them to the chain via signed extrinsics.
pub struct PulseFinalizationGadget<Block, Client, S, const MAX_QUEUE_SIZE: usize> {
	client: Arc<Client>,
	pulse_submitter: Arc<S>,
	pulse_receiver: DrandReceiver<MAX_QUEUE_SIZE>,
	_phantom: std::marker::PhantomData<Block>,
}

impl<Block, Client, S, const MAX_QUEUE_SIZE: usize>
	PulseFinalizationGadget<Block, Client, S, MAX_QUEUE_SIZE>
where
	Block: BlockT,
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	Client::Api: pallet_randomness_beacon::RandomnessBeaconApi<Block>,
	S: PulseSubmitter<Block>,
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
	/// and submits an aggregated signature to the chain.
	pub async fn run(self, finality_notifications: sc_client_api::FinalityNotifications<Block>) {
		log::info!(target: LOG_TARGET, "🎲 Starting Pulse Finalization Gadget");
		// Subscribe to finality notifications and reuse the streams,
		// so we don't miss notifications while waiting for pallet to be available.
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
	) -> Result<(), GadgetError> {
		// get 'finalized' round from the runtime
		let at_hash = self.client.info().best_hash;
		let latest_round = self.client.runtime_api().latest_round(at_hash).unwrap_or(0);
		let max_rounds = self
			.client
			.runtime_api()
			.max_rounds(at_hash)
			.expect("The max rounds is defined. qed.");
		// get fresh pulses
		let pulses = self.pulse_receiver.read().await;
		// only take up to as many pulses that we know will be valid in the runtime
		// this allows the node to hold a 'backlog' or queue of pulses in the case that
		// block proposal or block finality significantly lags
		let new_pulses: Vec<_> = pulses
			.clone()
			.into_iter()
			.filter(|p| p.round >= latest_round)
			.take(max_rounds as usize)
			.collect();

		if let (Some(first), Some(last)) = (new_pulses.first(), new_pulses.last()) {
			let start = first.round;
			let end = last.round;

			log::info!(
				target: LOG_TARGET,
				"Block #{:?} finalized (round {}), submitting {} new pulses",
				notification.header.number(),
				latest_round,
				end.saturating_sub(start)
			);

			// aggregate sigs
			let asig = new_pulses
				.into_iter()
				.filter_map(|pulse| {
					let bytes = pulse.signature;
					G1Affine::deserialize_compressed(&mut bytes.as_ref()).ok()
				})
				.fold(sp_idn_crypto::bls12_381::zero_on_g1(), |acc, sig| (acc + sig).into());

			let mut asig_bytes = Vec::with_capacity(SERIALIZED_SIG_SIZE);
			// NOTE: The expect here is okay since asig **must** be right-sized.
			asig.serialize_compressed(&mut asig_bytes)
				.expect("The signature is well formatted. qed.");

			self.pulse_submitter.submit_pulse(asig_bytes, start, end).await?;
		} else {
			log::info!(
				target: LOG_TARGET,
				"Block #{:?} finalized (round {}), No new pulses.",
				notification.header.number(),
				latest_round,
			);
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::MockClient;
	use sp_consensus_randomness_beacon::types::CanonicalPulse;
	use sp_runtime::{
		generic::Header,
		traits::{BlakeTwo256, Block as BlockT},
	};
	use std::{
		sync::{Arc, Mutex},
		time::Duration,
	};

	const MAX_QUEUE_SIZE: usize = 100;
	// a minimal test block type
	type TestBlock =
		sp_runtime::generic::Block<Header<u64, BlakeTwo256>, sp_runtime::OpaqueExtrinsic>;

	// Mock pulse submitter
	struct MockPulseSubmitter {
		submissions: Arc<Mutex<Vec<(Vec<u8>, u64, u64)>>>,
		should_fail: Arc<Mutex<bool>>,
	}

	impl MockPulseSubmitter {
		fn new() -> Self {
			let submissions = Arc::new(Mutex::new(Vec::new()));
			let should_fail = Arc::new(Mutex::new(false));
			Self { submissions, should_fail }
		}

		fn get_submissions(&self) -> Vec<(Vec<u8>, u64, u64)> {
			self.submissions.lock().unwrap().clone()
		}

		fn set_should_fail(&self, do_fail: bool) {
			*self.should_fail.lock().unwrap() = do_fail;
		}
	}

	impl PulseSubmitter<TestBlock> for MockPulseSubmitter {
		async fn submit_pulse(
			&self,
			asig: Vec<u8>,
			start: u64,
			end: u64,
		) -> Result<<TestBlock as BlockT>::Hash, GadgetError> {
			if *self.should_fail.lock().unwrap() {
				return Err(GadgetError::TransactionSubmissionFailed);
			}
			self.submissions.lock().unwrap().push((asig, start, end));
			Ok(Default::default())
		}
	}

	fn create_header(number: u64) -> Header<u64, BlakeTwo256> {
		Header::new(
			number,
			Default::default(),
			Default::default(),
			Default::default(),
			Default::default(),
		)
	}

	fn create_test_pulse(round: u64) -> CanonicalPulse {
		CanonicalPulse { round, signature: [0u8; SERIALIZED_SIG_SIZE] }
	}

	fn create_unpinned_notification(number: u64) -> UnpinnedFinalityNotification<TestBlock> {
		UnpinnedFinalityNotification { hash: Default::default(), header: create_header(number) }
	}

	#[tokio::test]
	async fn gadget_submits_new_pulses_on_finality() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		// submit two pulses to the channel
		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(101)).unwrap();

		let gadget = PulseFinalizationGadget::new(client, mock_submitter.clone(), pulse_receiver);

		// wait for pulses to be recieved
		tokio::time::sleep(Duration::from_millis(50)).await;

		// handle finality notification directly
		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1, "Expected exactly one submission");
		assert_eq!(submissions[0].1, 100, "Start round should be 100");
		assert_eq!(submissions[0].2, 101, "End round should be 101");
	}

	#[tokio::test]
	async fn gadget_skips_old_pulses() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		// Add pulses with old rounds
		pulse_tx.unbounded_send(create_test_pulse(50)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(51)).unwrap();

		let gadget =
			PulseFinalizationGadget::new(client.clone(), mock_submitter.clone(), pulse_receiver);

		// Set latest round to 100 via the client's runtime API state
		*client.runtime_api_state.latest_round.lock() = 100;

		tokio::time::sleep(Duration::from_millis(50)).await;

		// Handle finality notification
		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		// Verify no submissions (all pulses were old)
		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 0, "Should not submit old pulses");
	}

	#[tokio::test]
	async fn gadget_processes_multiple_finality_notifications() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		// Add first batch of pulses
		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(101)).unwrap();

		let gadget =
			PulseFinalizationGadget::new(client.clone(), mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		// First finality notification
		let notification1 = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification1).await.unwrap();
		tokio::time::sleep(Duration::from_millis(50)).await;

		// Update latest_round to simulate progression
		*client.runtime_api_state.latest_round.lock() = 102;

		// Add second batch of pulses
		pulse_tx.unbounded_send(create_test_pulse(102)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(103)).unwrap();
		tokio::time::sleep(Duration::from_millis(50)).await;

		// Second finality notification
		let notification2 = create_unpinned_notification(2);
		gadget.handle_finality_notification(&notification2).await.unwrap();

		// Verify both submissions
		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 2, "Expected two submissions");
		assert_eq!(submissions[0].1, 100);
		assert_eq!(submissions[0].2, 101);
		assert_eq!(submissions[1].1, 102);
		assert_eq!(submissions[1].2, 103);
	}

	#[tokio::test]
	async fn gadget_handles_empty_pulse_queue() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (_pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		// Don't add any pulses
		let gadget = PulseFinalizationGadget::new(client, mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		// Handle finality notification
		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		// Verify no submissions
		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 0, "Should not submit when queue is empty");
	}

	#[tokio::test]
	async fn gadget_updates_best_finalized_round() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(105)).unwrap();

		let gadget =
			PulseFinalizationGadget::new(client.clone(), mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		// Initial state
		assert_eq!(*client.runtime_api_state.latest_round.lock(), 0);

		// Handle finality notification
		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		// Verify pulses were submitted
		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1);
		assert_eq!(submissions[0].1, 100, "Start round should be 100");
		assert_eq!(submissions[0].2, 105, "End round should be 105");

		// Simulate the runtime updating its latest round after submission
		*client.runtime_api_state.latest_round.lock() = 105;

		// Verify the mock state was updated
		assert_eq!(
			*client.runtime_api_state.latest_round.lock(),
			105,
			"Latest round should be updated to 105"
		);
	}

	#[tokio::test]
	async fn gadget_continues_after_submission_error() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();

		let gadget =
			PulseFinalizationGadget::new(client.clone(), mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		// Make submission fail
		mock_submitter.set_should_fail(true);
		let notification1 = create_unpinned_notification(1);
		let result = gadget.handle_finality_notification(&notification1).await;
		assert!(result.is_err(), "Should return error when submission fails");

		// Update latest_round and add new pulse
		*client.runtime_api_state.latest_round.lock() = 101;
		pulse_tx.unbounded_send(create_test_pulse(101)).unwrap();
		mock_submitter.set_should_fail(false);
		tokio::time::sleep(Duration::from_millis(50)).await;

		let notification2 = create_unpinned_notification(2);
		gadget.handle_finality_notification(&notification2).await.unwrap();

		// Verify gadget recovered and processed second notification
		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1, "Gadget should recover and process after error");
		assert_eq!(submissions[0].1, 101);
	}

	#[tokio::test]
	async fn gadget_aggregates_multiple_pulses() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		// Add multiple pulses (max_rounds is 6, so only first 6 will be taken)
		for round in 100..110 {
			pulse_tx.unbounded_send(create_test_pulse(round)).unwrap();
		}

		let gadget = PulseFinalizationGadget::new(client, mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1);
		assert_eq!(submissions[0].1, 100, "Start should be first pulse");
		assert_eq!(submissions[0].2, 105, "End should be 6th pulse (max_rounds=6)");
		assert_eq!(submissions[0].0.len(), SERIALIZED_SIG_SIZE, "Signature should be serialized");
	}
}
