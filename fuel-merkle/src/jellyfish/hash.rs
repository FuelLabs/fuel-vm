use hashbrown::HashMap;

use crate::common::{
    sum_iter,
    Bytes32,
    Prefix,
};

use super::merkle_tree::node::{
    Nibble,
    NibblePath,
    Version,
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

pub fn calculate_leaf_hash(leaf_key: &NibblePath, leaf_value: &Bytes32) -> Bytes32 {
    let leaf_key: &[u8] = leaf_key.as_ref();
    let leaf_value: &[u8] = leaf_value.as_ref();
    let input = [leaf_key, leaf_value];
    sum_iter(input)
}

// A node has at most 16 children, but we treat it as if it were a Merkle Tree with 16
// nodes when calculating its hash.
pub fn calculate_node_hash(
    children: &HashMap<Nibble, (Version, Bytes32)>,
    placeholder_hash: Bytes32,
) -> Bytes32 {
    // TODO: Optimise this

    let mut sorted_children: Vec<_> = (0x00..=0x0F)
        .map(|i| {
            children
                .get(&Nibble::new(i))
                .map(|(_version, hash)| hash.clone())
                .unwrap_or(placeholder_hash)
        })
        .collect();
    let mut length = 16;
    while length != 1 {
        let mut current_index = 0;
        while current_index < length {
            sorted_children[current_index / 2] = sum_iter(&[
                sorted_children[current_index],
                sorted_children[current_index + 1],
            ]);
            current_index += 2;
        }
        length /= 2;
    }
    sorted_children[0].clone()
}
