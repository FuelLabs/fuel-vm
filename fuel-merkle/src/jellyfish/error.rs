use crate::common::Bytes32;

#[derive(Debug, derive_more::Display)]
pub enum MerkleTreeError<StorageError> {
    #[display(fmt = "{}", _0)]
    StorageError(StorageError),
    #[display(
        fmt = "Error arising from the jmt::TreeReader trait implementation: {}",
        _0
    )]
    TreeReaderError(anyhow::Error),
    #[display(
        fmt = "Error arising from the jmt::TreeWriter trait implementation: {}",
        _0
    )]
    TreeWriterError(anyhow::Error),
    #[display(
        fmt = "Error arising from the jmt::HasPreimage trait implementation: {}",
        _0
    )]
    HasPreimageError(anyhow::Error),
    #[display(fmt = "Error propagated from the jmt crate: {}", _0)]
    JmtError(anyhow::Error),
    #[display(fmt = "The tree has no version")]
    NoVersion,
    #[display(fmt = "Expected root hash: {:?}, Actual root hash: {:?} ", _0, _1)]
    RootHashMismatch(Bytes32, Bytes32),
}

impl<StorageError> From<StorageError> for MerkleTreeError<StorageError> {
    fn from(err: StorageError) -> MerkleTreeError<StorageError> {
        MerkleTreeError::StorageError(err)
    }
}
