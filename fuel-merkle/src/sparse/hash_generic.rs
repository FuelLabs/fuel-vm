use crate::common::{
    Bytes,
    Zero,
};
use std::{
    convert::TryInto,
    sync::OnceLock,
};

use digest::Digest;
use sha2::Sha256;

pub(crate) type Hash = Sha256;

// pub fn zero_sum<T: Zero>() -> &'static T {
//     static COMPUTATION: OnceLock<T> = OnceLock::new();
//     COMPUTATION.get_or_init(|| T::zero())
// }

pub fn zero_sum<const N: usize>() -> [u8; N] {
    [0u8; N]
}

pub fn sum<I, const N: usize>(data: I) -> Bytes<N>
where
    I: AsRef<[u8]>,
{
    let mut hash = crate::sparse::hash::Hash::new();
    hash.update(data);
    let h = hash.finalize();
    let mut vec = h.as_slice().to_vec();
    vec.truncate(N);
    vec.try_into().unwrap()
}
