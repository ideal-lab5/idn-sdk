use std::fmt::Debug;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("Extrinsic construction failed.")]
    ExtrinsicConstructionFailed,
    #[error("There are no authority keys available in the keystore.")]
    NoAuthorityKeys,
    #[error("The transaction failed to be included in the transaction pool.")]
    TransactionSubmissionFailed,
}