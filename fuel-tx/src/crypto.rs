use crate::Hash;
use sha2::{Digest, Sha256};

pub fn hash(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();

    hasher.update(data);

    <[u8; Hash::size_of()]>::from(hasher.finalize()).into()
}
