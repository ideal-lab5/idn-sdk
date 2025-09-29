// Copyright 2025 by Ideal Labs, LLC
// SPDX-License-Identifier: Apache-2.0

//! Pulse Finalization Gadget
//!
//! Submits signed extrinsics containing verified randomness pulses to the chain.

use sp_runtime::traits::Block as BlockT;
use std::{sync::Arc, time::Duration};

const LOG_TARGET: &str = "rand-beacon-gadget";

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
        asig: Vec<u8>,
        start: u64,
        end: u64,
    ) -> impl std::future::Future<Output = Result<Block::Hash, Box<dyn std::error::Error + Send + Sync>>> + Send;
}

/// The Pulse Finalization Gadget.
///
/// Runs as a background task, monitoring for new randomness pulses and
/// submitting them to the chain via signed extrinsics.
pub struct PulseFinalizationGadget<Block: BlockT, S> {
    pulse_submitter: Arc<S>,
    _phantom: std::marker::PhantomData<Block>,
}

impl<Block: BlockT, S: PulseSubmitter<Block>> PulseFinalizationGadget<Block, S> {
    /// Create a new Pulse Finalization Gadget.
    pub fn new(pulse_submitter: Arc<S>) -> Self {
        Self {
            pulse_submitter,
            _phantom: Default::default(),
        }
    }

    /// Run the gadget's main loop.
    ///
    /// This function runs indefinitely, periodically checking for new pulses
    /// and submitting them to the chain.
    pub async fn run(self) {
        log::info!(target: LOG_TARGET, "🎲 Starting Pulse Finalization Gadget");

        let mut interval = tokio::time::interval(Duration::from_secs(12));
        let mut counter = 0u32;

        loop {
            interval.tick().await;

            // For now, just submit test remarks
            // TODO: Read from DrandReceiver and aggregate pulses
            match self.submit_test_remark(counter).await {
                Ok(hash) => {
                    log::info!(
                        target: LOG_TARGET,
                        "✅ PFG submitted test remark #{} with hash: {:?}",
                        counter,
                        hash
                    );
                    counter += 1;
                }
                Err(e) => {
                    log::error!(target: LOG_TARGET, "❌ PFG submission failed: {:?}", e);
                }
            }
        }
    }

    /// Submit a test remark (placeholder for actual pulse submission).
    async fn submit_test_remark(
        &self,
        counter: u32,
    ) -> Result<Block::Hash, Box<dyn std::error::Error + Send + Sync>> {
        // For now, use dummy values
        // TODO: Replace with actual pulse aggregation
        let asig = format!("test-sig-{}", counter).into_bytes();
        let start = counter as u64;
        let end = counter as u64;

        self.pulse_submitter.submit_pulse(asig, start, end).await
    }
}