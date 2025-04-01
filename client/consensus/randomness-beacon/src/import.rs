// use sp_runtime::traits::Block as BlockT;
// use sc_consensus::BlockImport;
// use sp_consensus::ImportResult;
// use sp_blockchain::Error as BlockchainError;
// use std::sync::Arc;
// use log::info;

// pub struct LoggingBlockImport<I, Block>
// where
//     I: BlockImport<Block>,
//     Block: BlockT,
// {
//     inner: Arc<I>,
// }

// impl<I, Block> LoggingBlockImport<I, Block>
// where
//     I: BlockImport<Block>,
//     Block: BlockT,
// {
//     pub fn new(inner: Arc<I>) -> Self {
//         Self { inner }
//     }
// }

// impl<I, Block> BlockImport<Block> for LoggingBlockImport<I, Block>
// where
//     I: BlockImport<Block>,
//     Block: BlockT,
// {
//     fn import_block(
//         &mut self,
//         block: sp_consensus::BlockImportParams<Block, ()>,
//         cache: std::collections::HashMap<sp_consensus::CacheKeyId, Vec<u8>>,
//     ) -> Result<ImportResult, BlockchainError> {
//         // Log block details
//         info!("Block imported: #{:?}, Hash: {:?}", block.header.number(), block.header.hash());
        
//         // Proceed with normal import
//         self.inner.import_block(block, cache)
//     }
// }

// use sc_consensus::{
// 	block_import::{BlockImport, BlockImportParams, ForkChoiceStrategy},
// 	import_queue::{BasicQueue, BoxBlockImport, Verifier},
// };

// /// Instantiate the import queue for the manual seal consensus engine.
// pub fn import_queue<Block>(
// 	block_import: BoxBlockImport<Block>,
// 	spawner: &impl sp_core::traits::SpawnEssentialNamed,
// 	registry: Option<&Registry>,
// ) -> BasicQueue<Block>
// where
// 	Block: BlockT,
// {
//     log::info!("###################################################################################################### IMPORTED A NEW BLOCK!!");   
// 	BasicQueue::new(ManualSealVerifier, block_import, None, spawner, registry)
// }
