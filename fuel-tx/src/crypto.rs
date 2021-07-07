use crate::Bytes32;
use sha2::{Digest, Sha256};

pub fn hash(data: &[u8]) -> Bytes32 {
    let mut hasher = Sha256::new();

    hasher.update(data);

    <[u8; Bytes32::size_of()]>::from(hasher.finalize()).into()
}
