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
use parking_lot::Mutex;
use sc_client_api::HeaderBackend;
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver};
use sp_api::ProvideRuntimeApi;
use sp_consensus_randomness_beacon::types::SERIALIZED_SIG_SIZE;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT};
use std::{pin::Pin, sync::Arc};
use tokio::sync::Mutex as TokioMutex;

const LOG_TARGET: &str = "rand-beacon-gadget";

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
	next_round: Arc<Mutex<u64>>,
	submission_lock: Arc<TokioMutex<()>>,
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
		let next_round = Arc::new(Mutex::new(0u64));
		let submission_lock = Arc::new(TokioMutex::new(()));
		Self {
			client,
			pulse_submitter,
			pulse_receiver,
			next_round,
			submission_lock,
			_phantom: Default::default(),
		}
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
		// acquire a lock to ensure only one submission at a time since finality notifications
		// can come in rapid succession sometimes
		let _guard = self.submission_lock.lock().await;
		// get 'finalized' round from the runtime
		let at_hash = self.client.info().best_hash;
		// the network does not finalize blocks immediately (e.g. around when block #7 is authored)
		// thus, in the initial few rounds, the client will always read '0' from the runtime as the
		// latest round
		let runtime_round = self.client.runtime_api().next_round(at_hash).unwrap_or(0);
		// this represents the NEXT smallest round we can ingest
		let local_round = *self.next_round.lock();
		let next_round = runtime_round.max(local_round);

		let max_rounds = self
			.client
			.runtime_api()
			.max_rounds(at_hash)
			.expect("The max rounds is defined. qed.");
		// get fresh pulses
		let pulses = self.pulse_receiver.consume(max_rounds.into()).await;

		log::debug!(
			target: LOG_TARGET,
			"Consumed {} pulses from queue, next_round={}",
			pulses.len(),
			next_round
		);
		// only take up to as many pulses that we know will be valid in the runtime
		// this allows the node to hold a 'backlog' or queue of pulses in the case that
		// block proposal or block finality significantly lags.
		// This also filters any expired pulses that are lingering in the drand receiver.
		let new_pulses: Vec<_> = pulses
			.clone()
			.into_iter()
			.filter(|p| p.round >= next_round)
			.rev()
			// just always get the latest pulse for now
			// see: https://github.com/ideal-lab5/idn-sdk/issues/392
			.take(1)
			.collect();

		// since we reversed the list
		if let (Some(first), Some(last)) = (new_pulses.last(), new_pulses.first()) {
			let start = first.round;
			let end = last.round;

			log::info!(
				target: LOG_TARGET,
				"Block #{:?} finalized (round {}), submitting pulses for rounds {} - {}",
				notification.header.number(),
				next_round,
				start,
				end,
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
			*self.next_round.lock() = end + 1;
		} else {
			log::info!(
				target: LOG_TARGET,
				"Block #{:?} finalized (round {}), No new pulses.",
				notification.header.number(),
				next_round,
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
	async fn test_gadget_submits_single_latest_pulse_on_finality() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		// Submit multiple pulses - only the latest should be taken
		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(101)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(102)).unwrap();

		let gadget = PulseFinalizationGadget::new(client, mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1, "Expected exactly one submission");
		// Due to .take(1), only the latest pulse (102) should be submitted
		assert_eq!(submissions[0].1, 102, "Start round should be 102 (latest pulse)");
		assert_eq!(submissions[0].2, 102, "End round should be 102 (single pulse)");
	}

	#[tokio::test]
	async fn test_gadget_skips_old_pulses() {
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
		*client.runtime_api_state.next_round.lock() = 101;

		tokio::time::sleep(Duration::from_millis(50)).await;

		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 0, "Should not submit old pulses");
	}

	#[tokio::test]
	async fn test_gadget_filters_mixed_old_and_new_pulses() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		// Mix of old and new pulses
		pulse_tx.unbounded_send(create_test_pulse(50)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(105)).unwrap();

		let gadget =
			PulseFinalizationGadget::new(client.clone(), mock_submitter.clone(), pulse_receiver);

		*client.runtime_api_state.next_round.lock() = 100;

		tokio::time::sleep(Duration::from_millis(50)).await;

		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1, "Should submit one pulse");
		// Should get the latest valid pulse (105)
		assert_eq!(submissions[0].1, 105);
		assert_eq!(submissions[0].2, 105);
	}

	#[tokio::test]
	async fn test_gadget_processes_multiple_finality_notifications() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();

		let gadget =
			PulseFinalizationGadget::new(client.clone(), mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		// First finality notification
		let notification1 = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification1).await.unwrap();

		// Verify next_round was updated
		assert_eq!(*gadget.next_round.lock(), 101);

		tokio::time::sleep(Duration::from_millis(50)).await;

		// Add new pulse after next_round updated
		pulse_tx.unbounded_send(create_test_pulse(102)).unwrap();
		tokio::time::sleep(Duration::from_millis(50)).await;

		// Second finality notification
		let notification2 = create_unpinned_notification(2);
		gadget.handle_finality_notification(&notification2).await.unwrap();

		// Verify both submissions
		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 2, "Expected two submissions");
		assert_eq!(submissions[0].1, 100);
		assert_eq!(submissions[0].2, 100);
		assert_eq!(submissions[1].1, 102);
		assert_eq!(submissions[1].2, 102);
	}

	#[tokio::test]
	async fn test_gadget_handles_empty_pulse_queue() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (_pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		let gadget = PulseFinalizationGadget::new(client, mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 0, "Should not submit when queue is empty");
	}

	#[tokio::test]
	async fn test_gadget_updates_next_round_after_submission() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();

		let gadget =
			PulseFinalizationGadget::new(client.clone(), mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		// Initial state
		assert_eq!(*gadget.next_round.lock(), 0);

		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		// Verify next_round was updated to the end round
		assert_eq!(*gadget.next_round.lock(), 101);

		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1);
		assert_eq!(submissions[0].1, 100);
		assert_eq!(submissions[0].2, 100);
	}

	#[tokio::test]
	async fn test_gadget_continues_after_submission_error() {
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

		// Verify next_round was NOT updated on failure
		assert_eq!(*gadget.next_round.lock(), 0);

		// Add new pulse and fix submitter
		pulse_tx.unbounded_send(create_test_pulse(101)).unwrap();
		mock_submitter.set_should_fail(false);
		tokio::time::sleep(Duration::from_millis(50)).await;

		let notification2 = create_unpinned_notification(2);
		gadget.handle_finality_notification(&notification2).await.unwrap();

		// Verify gadget recovered
		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1, "Gadget should recover and process after error");
		assert_eq!(submissions[0].1, 101);
		assert_eq!(*gadget.next_round.lock(), 102);
	}

	#[tokio::test]
	async fn test_gadget_uses_max_of_runtime_and_local_round() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		// Add pulses around the boundary
		pulse_tx.unbounded_send(create_test_pulse(95)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(105)).unwrap();

		let gadget =
			PulseFinalizationGadget::new(client.clone(), mock_submitter.clone(), pulse_receiver);

		// Set runtime round to 90 but local next_round to 100
		*client.runtime_api_state.next_round.lock() = 90;
		*gadget.next_round.lock() = 100;

		tokio::time::sleep(Duration::from_millis(50)).await;

		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1);
		// Should only submit pulse 105 (>= max(90, 100) = 100)
		assert_eq!(submissions[0].1, 105);
	}

	#[tokio::test]
	async fn test_gadget_serializes_signature_correctly() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();

		let gadget = PulseFinalizationGadget::new(client, mock_submitter.clone(), pulse_receiver);

		tokio::time::sleep(Duration::from_millis(50)).await;

		let notification = create_unpinned_notification(1);
		gadget.handle_finality_notification(&notification).await.unwrap();

		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1);
		assert_eq!(
			submissions[0].0.len(),
			SERIALIZED_SIG_SIZE,
			"Signature should be properly serialized"
		);
	}

	#[tokio::test]
	async fn test_gadget_respects_submission_lock() {
		let mock_submitter = Arc::new(MockPulseSubmitter::new());
		let client = Arc::new(MockClient::new());

		let (pulse_tx, pulse_rx) = sc_utils::mpsc::tracing_unbounded("test-pulses", 100);
		let pulse_receiver = DrandReceiver::<MAX_QUEUE_SIZE>::new(pulse_rx);

		pulse_tx.unbounded_send(create_test_pulse(100)).unwrap();
		pulse_tx.unbounded_send(create_test_pulse(101)).unwrap();

		let gadget = Arc::new(PulseFinalizationGadget::new(
			client.clone(),
			mock_submitter.clone(),
			pulse_receiver,
		));

		tokio::time::sleep(Duration::from_millis(50)).await;

		// Spawn multiple concurrent finality handling attempts
		let gadget1 = Arc::clone(&gadget);
		let gadget2 = Arc::clone(&gadget);

		let notification = create_unpinned_notification(1);
		let notification_clone = notification.clone();

		let handle1 =
			tokio::spawn(async move { gadget1.handle_finality_notification(&notification).await });

		let handle2 =
			tokio::spawn(
				async move { gadget2.handle_finality_notification(&notification_clone).await },
			);

		let (result1, result2) = tokio::join!(handle1, handle2);

		// Both should succeed (lock ensures sequential execution)
		assert!(result1.unwrap().is_ok());
		assert!(result2.unwrap().is_ok());

		// Should have exactly one submission (second one would see next_round already updated)
		let submissions = mock_submitter.get_submissions();
		assert_eq!(submissions.len(), 1, "Lock should prevent duplicate submissions");
	}
}
