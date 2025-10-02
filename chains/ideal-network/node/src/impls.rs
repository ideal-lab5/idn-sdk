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

use crate::service::ParachainClient;
use idn_runtime::{UncheckedExtrinsic, opaque::Block};
use sc_client_api::HeaderBackend;
use sc_consensus_randomness_beacon::worker::ExtrinsicConstructor;
use sp_application_crypto::AppCrypto;
use sp_api::ProvideRuntimeApi;
use sp_consensus_randomness_beacon::{types::OpaqueSignature, api::ExtrinsicBuilderApi};
use sp_core::sr25519;
use sp_keystore::KeystorePtr;
use sp_runtime::{
    traits::{Block as BlockT, IdentifyAccount},
    MultiSignature, MultiSigner,
};
use std::sync::Arc;
use substrate_frame_rpc_system::AccountNonceApi;

pub(crate) struct RuntimeExtrinsicConstructor {
    pub(crate) client: Arc<ParachainClient>,
    pub(crate) keystore: KeystorePtr,
}

impl ExtrinsicConstructor<Block> for RuntimeExtrinsicConstructor {
    fn construct_pulse_extrinsic(
        &self,
        signer: sr25519::Public,
        asig: OpaqueSignature,
        start: u64,
        end: u64,
    ) -> Result<<Block as BlockT>::Extrinsic, Box<dyn std::error::Error + Send + Sync>> {
 		let account: idn_runtime::AccountId = MultiSigner::Sr25519(signer).into_account();
        let at_hash = self.client.info().best_hash;

		// we only need to proceed if it is our turn (round robin aura style)
		// Get our authority ID
		// TODO: eventually we should designate named keys for this (like aura does)
		// let our_authority = match get_authority_id(&keystore).await {
		// 	Some(id) => id,
		// 	None => {
		// 		log::warn!("Not an authority, skipping submission");
		// 		return Ok(Default::default());
		// 	}
		// };
		

		// let local_id = keystore
        // .sr25519_public_keys(RANDOMNESS_BEACON_KEY_TYPE)
        // .into_iter()
        // .next()
        // .ok_or("No authority key found in keystore")?;
		// Calculate designated submitter
		// let submitter_index = (round as usize) % authorities.len();
		// let designated = &authorities[submitter_index];
		
		// // Only designated authority submits
		// if designated != &account {
		// 	log::debug!("Not designated submitter for round {}", round);
		// 	return Ok(Default::default());
		// }
		
		// log::info!("🎯 Designated submitter for round {}, submitting...", round);
        
        // let version = self.client.runtime_api().version(at_hash)?;
        let nonce = self.client.runtime_api().account_nonce(at_hash, account.clone())?;
   
		let (payload, call, tx_ext) = self.client.runtime_api().construct_pulse_payload(
			at_hash,
			asig,
			start,
			end,
			nonce,
		).unwrap();

		let signature = self.keystore
			.sr25519_sign(
				sp_consensus_aura::sr25519::AuthorityPair::ID,
				&signer.into(),
				&payload,
			)?
			.ok_or("Failed to sign")?;

        Ok(UncheckedExtrinsic::new_signed(
            call,
            account.into(),
            MultiSignature::Sr25519(signature),
            tx_ext,
        ).into())
    }
}