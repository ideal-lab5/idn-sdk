use std::fmt::Debug;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("Extrinsic construction failed.")]
    ExtrinsicConstructionFailed,
    #[error("The signature is incorrectly sized: has {0} bytes, but must be {1}.")]
    InvalidSignatureSize(u8, u8),
    #[error("There are no authority keys available in the keystore.")]
    NoAuthorityKeys,
    #[error("The transaction failed to be included in the transaction pool.")]
    TransactionSubmissionFailed,
}