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

use sp_application_crypto::AppCrypto;
use sp_runtime::traits::Block as BlockT;
use crate::{
    gadget::PulseSubmitter,
};
use sp_consensus_randomness_beacon::types::OpaqueSignature;
use sc_client_api::HeaderBackend;
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_api::ProvideRuntimeApi;
use sp_core::sr25519;
use sp_keystore::KeystorePtr;
use std::sync::Arc;

const LOG_TARGET: &str = "pulse-worker";

/// Trait for constructing runtime-specific extrinsics
pub trait ExtrinsicConstructor<Block: BlockT>: Send + Sync {
    /// Construct a signed extrinsic for pulse submission
    fn construct_pulse_extrinsic(
        &self,
        signer: sr25519::Public,
        asig: OpaqueSignature,
        start: u64,
        end: u64,
    ) -> Result<Block::Extrinsic, Box<dyn std::error::Error + Send + Sync>>;
}

/// The worker responsible for submitting pulses to the runtime
pub struct PulseWorker<Block, Client, Pool, Constructor>
where
    Block: BlockT,
{
    client: Arc<Client>,
    keystore: KeystorePtr,
    transaction_pool: Arc<Pool>,
    extrinsic_constructor: Arc<Constructor>,
    _phantom: std::marker::PhantomData<Block>,
}

impl<Block, Client, Pool, Constructor> PulseWorker<Block, Client, Pool, Constructor>
where
    Block: BlockT,
{
    pub fn new(
        client: Arc<Client>,
        keystore: KeystorePtr,
        transaction_pool: Arc<Pool>,
        extrinsic_constructor: Arc<Constructor>,
    ) -> Self {
        Self {
            client,
            keystore,
            transaction_pool,
            extrinsic_constructor,
            _phantom: Default::default(),
        }
    }

    /// get aura keys (for now: TODO)
    fn get_authority_key(&self) -> Result<sr25519::Public, Box<dyn std::error::Error + Send + Sync>> {
        self.keystore
            .sr25519_public_keys(sp_consensus_aura::sr25519::AuthorityPair::ID)
            .first()
            .cloned()
            .ok_or_else(|| "No authority key found in keystore".into())
    }
}

impl<Block, Client, Pool, Constructor> PulseSubmitter<Block>
    for PulseWorker<Block, Client, Pool, Constructor>
where
    Block: BlockT,
    Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
    Pool: TransactionPool<Block = Block> + 'static,
    Constructor: ExtrinsicConstructor<Block> + 'static,
{
    async fn submit_pulse(
        &self,
        asig: OpaqueSignature,
        start: u64,
        end: u64,
    ) -> Result<Block::Hash, Box<dyn std::error::Error + Send + Sync>> {
        let public = self.get_authority_key()?;

        log::info!(
            target: LOG_TARGET,
            "Constructing pulse extrinsic for rounds {}-{}",
            start,
            end
        );

        let extrinsic = self.extrinsic_constructor.construct_pulse_extrinsic(
            public, 
            asig, 
            start,
            end,
        ).unwrap();

        // Submit to transaction pool
        let best_hash = self.client.info().best_hash;
        let _hash = self
            .transaction_pool
            .submit_one(best_hash, TransactionSource::Local, extrinsic)
            .await?;

        log::info!(
            target: LOG_TARGET,
            "Submitted pulse extrinsic",
        );

        Ok(best_hash)
    }
}