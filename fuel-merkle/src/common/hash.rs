use super::Bytes32;

pub fn sum<T: AsRef<[u8]>>(data: T) -> Bytes32 {
    use digest::Digest;
    let mut hash = sha2::Sha256::new();
    hash.update(data.as_ref());
    hash.finalize().into()
}

pub fn sum_iter<I: IntoIterator<Item = T>, T: AsRef<[u8]>>(iterator: I) -> Bytes32 {
    use digest::Digest;
    let mut hash = sha2::Sha256::new();
    for data in iterator {
        hash.update(data.as_ref());
    }
    hash.finalize().into()
}
