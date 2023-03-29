use crate::binary::{leaf_sum, node_sum};
use crate::common::Bytes32;
use crate::common::ProofSet;

pub fn verify<T: AsRef<[u8]>>(
    root: &Bytes32,
    digest: T,
    proof_set: &ProofSet,
    proof_index: u64,
    num_leaves: u64,
) -> bool {
    if proof_index >= num_leaves {
        return false;
    }

    if proof_set.is_empty() {
        return false;
    }

    let mut height = 0usize;
    let mut sum = proof_set[height];
    height += 1;

    let digest = leaf_sum(digest.as_ref());
    if digest != sum {
        return false;
    }

    let mut stable_end = proof_index;

    loop {
        let subtree_start_index = proof_index / (1 << height) * (1 << height);
        let subtree_end_index = subtree_start_index + (1 << height) - 1;
        if subtree_end_index >= num_leaves {
            break;
        }

        stable_end = subtree_end_index;

        if proof_set.len() <= height {
            return false;
        }

        let proof_data = proof_set[height];
        if proof_index - subtree_start_index < 1 << (height - 1) {
            sum = node_sum(&sum, &proof_data);
        } else {
            sum = node_sum(&proof_data, &sum);
        }

        height += 1;
    }

    if stable_end != num_leaves - 1 {
        if proof_set.len() <= height {
            return false;
        }
        let proof_data = proof_set[height];
        sum = node_sum(&sum, &proof_data);
        height += 1;
    }

    while height < proof_set.len() {
        let proof_data = proof_set[height];
        sum = node_sum(&proof_data, &sum);
        height += 1;
    }

    sum == *root
}

#[cfg(test)]
mod test {
    use super::verify;
    use crate::binary::MerkleTree;
    use crate::binary::Primitive;
    use crate::common::StorageMap;
    use fuel_merkle_test_helpers::TEST_DATA;
    use fuel_storage::Mappable;

    #[derive(Debug)]
    struct TestTable;

    impl Mappable for TestTable {
        type Key = Self::OwnedKey;
        type OwnedKey = u64;
        type Value = Self::OwnedValue;
        type OwnedValue = Primitive;
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

        let proof = tree.prove(PROOF_INDEX as u64).unwrap();
        let root = proof.0;
        let set = proof.1;

        let verification = verify(
            &root,
            TEST_DATA[PROOF_INDEX],
            &set,
            PROOF_INDEX as u64,
            LEAVES_COUNT as u64,
        );
        assert!(verification);
    }

    #[test]
    fn verify_returns_false_when_the_given_proof_set_does_not_match_the_given_merkle_root() {
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
            TEST_DATA[PROOF_INDEX],
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
            TEST_DATA[PROOF_INDEX],
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
            TEST_DATA[PROOF_INDEX],
            &set,
            PROOF_INDEX as u64 + 15,
            LEAVES_COUNT as u64,
        );
        assert!(!verification);
    }
}
