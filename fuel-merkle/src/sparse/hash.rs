use crate::common::{
    Bytes32,
    Prefix,
    sum_iter,
};

pub const fn zero_sum() -> &'static Bytes32 {
    const ZERO_SUM: Bytes32 = [0; 32];

    &ZERO_SUM
}

pub fn calculate_hash(
    prefix: &Prefix,
    bytes_lo: &Bytes32,
    bytes_hi: &Bytes32,
) -> Bytes32 {
    let input = [prefix.as_ref(), bytes_lo.as_ref(), bytes_hi.as_ref()];
    sum_iter(input)
}

pub fn calculate_leaf_hash(leaf_key: &Bytes32, leaf_value: &Bytes32) -> Bytes32 {
    calculate_hash(&Prefix::Leaf, leaf_key, leaf_value)
}

pub fn calculate_node_hash(left_child: &Bytes32, right_child: &Bytes32) -> Bytes32 {
    calculate_hash(&Prefix::Node, left_child, right_child)
}
