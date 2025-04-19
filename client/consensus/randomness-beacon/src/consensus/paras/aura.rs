// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus. If not, see <https://www.gnu.org/licenses/>.

/// An import queue which provides some equivocation resistance with lenient trait bounds.
///
/// Equivocation resistance in general is a hard problem, as different nodes in the network
/// may see equivocations in a different order, and therefore may not agree on which blocks
/// should be thrown out and which ones should be kept.
use crate::block_import::LatestRoundNotifier;
use codec::Codec;
use cumulus_client_consensus_aura::equivocation_import_queue::Verifier;
use cumulus_client_consensus_common::ParachainBlockImportMarker;

use sc_consensus::{import_queue::BasicQueue, BlockImport, DefaultImportQueue};
use sc_telemetry::TelemetryHandle;
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder as BlockBuilderApi;
use sp_consensus::error::Error as ConsensusError;
use sp_consensus_aura::AuraApi;
use sp_core::crypto::Pair;
use sp_inherents::CreateInherentDataProviders;
use sp_runtime::traits::Block as BlockT;
use std::{fmt::Debug, sync::Arc};

/// Start an import queue for a Cumulus node which checks blocks' seals and inherent data.
///
/// Pass in only inherent data providers which don't include aura or parachain consensus inherents,
/// e.g. things like timestamp and custom inherents for the runtime.
///
/// The others are generated explicitly internally.
///
/// This should only be used for runtimes where the runtime does not check all inherents and
/// seals in `execute_block` (see <https://github.com/paritytech/cumulus/issues/2436>)
pub fn fully_verifying_import_queue<P, Client, Block: BlockT, I, CIDP>(
	client: Arc<Client>,
	block_import: I,
	create_inherent_data_providers: CIDP,
	spawner: &impl sp_core::traits::SpawnEssentialNamed,
	registry: Option<&prometheus_endpoint::Registry>,
	telemetry: Option<TelemetryHandle>,
) -> (DefaultImportQueue<Block>, TracingUnboundedReceiver<u64>)
where
	P: Pair + 'static,
	P::Signature: Codec,
	P::Public: Codec + Debug,
	I: BlockImport<Block, Error = ConsensusError>
		+ ParachainBlockImportMarker
		+ Send
		+ Sync
		+ 'static,
	Client: ProvideRuntimeApi<Block> + Send + Sync + 'static,
	<Client as ProvideRuntimeApi<Block>>::Api: BlockBuilderApi<Block> + AuraApi<Block, P::Public>,
	CIDP: CreateInherentDataProviders<Block, ()> + 'static,
{
	let verifier: Verifier<P, _, _, _> =
		Verifier::new(client, create_inherent_data_providers, telemetry);
	let (block_import, rx) = LatestRoundNotifier::new(block_import);

	(BasicQueue::new(verifier, Box::new(block_import), None, spawner, registry), rx)
}
