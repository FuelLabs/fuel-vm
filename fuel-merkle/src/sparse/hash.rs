use crate::common::Bytes32;

use digest::Digest;
use sha2::Sha256;

pub(crate) type Hash = Sha256;

pub fn zero_sum() -> &'static Bytes32 {
    const ZERO_SUM: Bytes32 = [0; 32];

    &ZERO_SUM
}

pub fn sum<I>(data: I) -> Bytes32
where
    I: AsRef<[u8]>,
{
    let mut hash = Hash::new();
    hash.update(data);
    hash.finalize().try_into().unwrap()
}

pub fn sum_all<I>(data: I) -> Bytes32
where
    I: IntoIterator,
    I::Item: AsRef<[u8]>,
{
    let mut hash = Hash::new();
    for datum in data.into_iter() {
        hash.update(datum)
    }
    hash.finalize().try_into().unwrap()
}
