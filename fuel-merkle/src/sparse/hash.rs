use crate::common::{
    sum,
    Bytes,
    Prefix,
};
use std::sync::OnceLock;

pub fn zero_sum<const N: usize>() -> &'static [u8; N] {
    static ZERO: OnceLock<Vec<u8>> = OnceLock::new();
    ZERO.get_or_init(|| vec![0; N])
        .as_slice()
        .try_into()
        .expect("Expected valid zero sum")
}

pub fn sum_truncated<const N: usize, T: AsRef<[u8]>>(data: T) -> Bytes<N> {
    let hash = sum(data);
    let mut vec = hash.as_slice().to_vec();
    vec.truncate(N);
    vec.try_into().unwrap()
}

pub fn calculate_hash<const N: usize>(
    prefix: &Prefix,
    bytes_lo: &Bytes<N>,
    bytes_hi: &Bytes<N>,
) -> Bytes<N> {
    let input = [prefix.as_ref(), bytes_lo.as_ref(), bytes_hi.as_ref()]
        .into_iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    sum_truncated(input)
}

pub fn calculate_leaf_hash<const N: usize>(
    leaf_key: &Bytes<N>,
    leaf_value: &Bytes<N>,
) -> Bytes<N> {
    calculate_hash(&Prefix::Leaf, leaf_key, leaf_value)
}

pub fn calculate_node_hash<const N: usize>(
    left_child: &Bytes<N>,
    right_child: &Bytes<N>,
) -> Bytes<N> {
    calculate_hash(&Prefix::Node, left_child, right_child)
}
