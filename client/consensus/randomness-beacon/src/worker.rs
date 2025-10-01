use sp_application_crypto::AppCrypto;
use sp_runtime::{
    generic::{Era, UncheckedExtrinsic},
    traits::{Block as BlockT, Header, IdentifyAccount, Verify},
    MultiSignature, MultiSigner,
};
use crate::{
    gadget::PulseSubmitter,
};
use sp_consensus_randomness_beacon::types::OpaqueSignature;

use codec::Encode;
use sc_client_api::{Backend, HeaderBackend};
use sc_transaction_pool_api::{TransactionPool, TransactionSource};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_core::{sr25519, Pair};
use sp_keystore::KeystorePtr;
use sp_runtime::traits::TransactionExtension;
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
        // Get authority key
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
        log::info!("Submitting extrinsic!");
        let _hash = self
            .transaction_pool
            .submit_one(best_hash, TransactionSource::Local, extrinsic)
            .await?;

        log::info!(
            target: LOG_TARGET,
            "✅ Submitted pulse extrinsic",
        );

        Ok(best_hash)
    }
}