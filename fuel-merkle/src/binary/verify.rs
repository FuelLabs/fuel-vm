use crate::{
    binary::{
        leaf_sum,
        node_sum,
    },
    common::{
        Bytes32,
        ProofSet,
    },
};

/// Returns None if:
/// - `num_leaves` is 0
/// - the result doens't fit in an usize
fn path_length_from_key(key: u64, num_leaves: u64) -> Option<usize> {
    if num_leaves == 0 {
        return None;
    }

    #[allow(clippy::arithmetic_side_effects)] // ilog2(..) < 64
    let path_length = if num_leaves.is_power_of_two() {
        num_leaves.ilog2()
    } else {
        num_leaves.ilog2() + 1
    };

    #[allow(clippy::arithmetic_side_effects)] // ilog2(..) > 0
    let num_leaves_left_subtree = 1 << (path_length - 1);

    let subtree_leaves = num_leaves.saturating_sub(num_leaves_left_subtree);

    let Some(subtree_key) = key.checked_sub(num_leaves_left_subtree) else {
        // If leaf is in left subtree, path length is full height of left subtree
        return path_length.try_into().ok();
    };

    // Otherwise, if left or right subtree has only one leaf, path has one additional step
    if num_leaves_left_subtree == 1 || subtree_leaves <= 1 {
        return Some(1);
    }

    // Otherwise, add 1 to height and recurse into right subtree
    path_length_from_key(subtree_key, subtree_leaves)?.checked_add(1)
}

pub fn verify<T: AsRef<[u8]>>(
    root: &Bytes32,
    data: &T,
    proof_set: &ProofSet,
    proof_index: u64,
    num_leaves: u64,
) -> bool {
    if num_leaves <= 1 {
        if !proof_set.is_empty() {
            return false;
        }
    } else if Some(proof_set.len()) != path_length_from_key(proof_index, num_leaves) {
        return false;
    }

    if proof_index >= num_leaves {
        return false;
    }

    let mut sum = leaf_sum(data.as_ref());
    if proof_set.is_empty() {
        return if num_leaves == 1 { *root == sum } else { false }
    }
    #[allow(clippy::arithmetic_side_effects)] // checked above
    let last_leaf = num_leaves - 1;

    let mut parent = 0usize;
    let mut stable_end = proof_index;

    loop {
        #[allow(clippy::arithmetic_side_effects)] // path_length_from_key checks
        let height = parent + 1;

        let subtree_size = 1u64 << height;
        #[allow(clippy::arithmetic_side_effects)] // floor(a / b) * b <= a
        let subtree_start_index = proof_index / subtree_size * subtree_size;
        #[allow(clippy::arithmetic_side_effects)]
        let subtree_end_index = subtree_start_index + subtree_size - 1;

        if subtree_end_index >= num_leaves {
            break
        }

        stable_end = subtree_end_index;

        if proof_set.len() < height {
            return false
        }

        let proof_data = proof_set[parent];
        #[allow(clippy::arithmetic_side_effects)] // proof_index > subtree_start_index
        if proof_index - subtree_start_index < (1 << parent) {
            sum = node_sum(&sum, &proof_data);
        } else {
            sum = node_sum(&proof_data, &sum);
        }

        #[allow(clippy::arithmetic_side_effects)] // path_length_from_key checks
        {
            parent += 1;
        }
    }

    if stable_end != last_leaf {
        if proof_set.len() <= parent {
            return false
        }
        let proof_data = proof_set[parent];
        sum = node_sum(&sum, &proof_data);
        #[allow(clippy::arithmetic_side_effects)] // path_length_from_key checks
        {
            parent += 1;
        }
    }

    while parent < proof_set.len() {
        let proof_data = proof_set[parent];
        sum = node_sum(&proof_data, &sum);
        #[allow(clippy::arithmetic_side_effects)] // path_length_from_key checks
        {
            parent += 1;
        }
    }

    sum == *root
}

#[cfg(test)]
mod test {
    use super::verify;
    use crate::{
        binary::{
            MerkleTree,
            Primitive,
        },
        common::StorageMap,
    };
    use fuel_merkle_test_helpers::TEST_DATA;
    use fuel_storage::Mappable;

    #[derive(Debug)]
    struct TestTable;

    impl Mappable for TestTable {
        type Key = Self::OwnedKey;
        type OwnedKey = u64;
        type OwnedValue = Primitive;
        type Value = Self::OwnedValue;
    }

    #[test]
    fn verify_returns_true_when_the_given_proof_set_matches_the_given_merkle_root() {
        let mut storage_map = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        const PROOF_INDEX: usize = 2;
        const LEAVES_COUNT: usize = 5;

        let data = &TEST_DATA[0..LEAVES_COUNT]; // 5 leaves
        for datum in data.iter() {
            tree.push(datum).unwrap();
        }

        let (root, proof_set) = tree.prove(PROOF_INDEX as u64).unwrap();
        let verification = verify(
            &root,
            &TEST_DATA[PROOF_INDEX],
            &proof_set,
            PROOF_INDEX as u64,
            LEAVES_COUNT as u64,
        );
        assert!(verification);
    }

    #[test]
    fn verify_returns_false_when_the_given_proof_set_does_not_match_the_given_merkle_root()
     {
        // Check the Merkle root of one tree against the computed Merkle root of
        // another tree's proof set: because the two roots come from different
        // trees, the comparison should fail.

        // Generate the first Merkle tree and get its root
        let mut storage_map = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        const PROOF_INDEX: usize = 2;
        const LEAVES_COUNT: usize = 5;

        let data = &TEST_DATA[0..LEAVES_COUNT - 1];
        for datum in data.iter() {
            tree.push(datum).unwrap();
        }
        let proof = tree.prove(PROOF_INDEX as u64).unwrap();
        let root = proof.0;

        // Generate the second Merkle tree and get its proof set
        let mut storage_map = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        let data = &TEST_DATA[5..10];
        for datum in data.iter() {
            tree.push(datum).unwrap();
        }
        let proof = tree.prove(PROOF_INDEX as u64).unwrap();
        let set = proof.1;

        let verification = verify(
            &root,
            &TEST_DATA[PROOF_INDEX],
            &set,
            PROOF_INDEX as u64,
            LEAVES_COUNT as u64,
        );
        assert!(!verification);
    }

    #[test]
    fn verify_returns_false_when_the_proof_set_is_empty() {
        const PROOF_INDEX: usize = 0;
        const LEAVES_COUNT: usize = 0;

        let verification = verify(
            &Default::default(),
            &TEST_DATA[PROOF_INDEX],
            &vec![],
            PROOF_INDEX as u64,
            LEAVES_COUNT as u64,
        );
        assert!(!verification);
    }

    #[test]
    fn verify_returns_false_when_the_proof_index_is_invalid() {
        let mut storage_map = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage_map);

        const PROOF_INDEX: usize = 0;
        const LEAVES_COUNT: usize = 5;

        let data = &TEST_DATA[0..LEAVES_COUNT - 1];
        for datum in data.iter() {
            tree.push(datum).unwrap();
        }

        let proof = tree.prove(PROOF_INDEX as u64).unwrap();
        let root = proof.0;
        let set = proof.1;

        let verification = verify(
            &root,
            &TEST_DATA[PROOF_INDEX],
            &set,
            PROOF_INDEX as u64 + 15,
            LEAVES_COUNT as u64,
        );
        assert!(!verification);
    }
}
