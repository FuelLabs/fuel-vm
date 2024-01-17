use crate::common::Bytes;
use std::{
    convert::TryInto,
    sync::OnceLock,
};

use digest::Digest;
use sha2::Sha256;
pub(crate) type Hash = Sha256;

pub fn zero_sum<const N: usize>() -> &'static [u8; N] {
    static ZERO: OnceLock<Vec<u8>> = OnceLock::new();
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
    let h = hash.finalize();
    let mut vec = h.as_slice().to_vec();
    vec.truncate(N);
    vec.try_into().unwrap()
}
