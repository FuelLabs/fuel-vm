use crate::common::Bytes;
use alloc::{
    vec,
    vec::Vec,
};
use once_cell::sync::OnceCell;

use digest::Digest;
use sha2::Sha256 as Hash;

pub fn zero_sum<const N: usize>() -> &'static [u8; N] {
    static ZERO: OnceCell<Vec<u8>> = OnceCell::new();
    ZERO.get_or_init(|| vec![0; N])
        .as_slice()
        .try_into()
        .expect("Expected valid zero sum")
}

pub fn sum<I, const N: usize>(data: I) -> Bytes<N>
where
    I: AsRef<[u8]>,
{
    let mut hash = Hash::new();
    hash.update(data);
    let hash = hash.finalize();
    let mut vec = hash.as_slice().to_vec();
    vec.truncate(N);
    vec.try_into().unwrap()
}
