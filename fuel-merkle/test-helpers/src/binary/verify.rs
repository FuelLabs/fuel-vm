use crate::binary::{
    Data,
    leaf_sum,
    node_sum,
};

pub fn verify<T: AsRef<[u8]>>(
    root: &Data,
    data: &T,
    proof_set: &[Data],
    proof_index: u64,
    num_leaves: u64,
) -> bool {
    let mut sum = leaf_sum(data.as_ref());

    if proof_index >= num_leaves {
        return false
    }

    if proof_set.is_empty() {
        return if num_leaves == 1 { *root == sum } else { false }
    }

    let mut height = 1usize;
    let mut stable_end = proof_index;

    loop {
        let subtree_start_index = proof_index / (1 << height) * (1 << height);
        let subtree_end_index = subtree_start_index + (1 << height) - 1;

        if subtree_end_index >= num_leaves {
            break
        }

        stable_end = subtree_end_index;

        if proof_set.len() < height {
            return false
        }

        let proof_data = proof_set[height - 1];
        if proof_index - subtree_start_index < 1 << (height - 1) {
            sum = node_sum(&sum, &proof_data);
        } else {
            sum = node_sum(&proof_data, &sum);
        }

        height += 1;
    }

    if stable_end != num_leaves - 1 {
        if proof_set.len() < height {
            return false
        }
        let proof_data = proof_set[height - 1];
        sum = node_sum(&sum, &proof_data);
        height += 1;
    }

    while height - 1 < proof_set.len() {
        let proof_data = proof_set[height - 1];
        sum = node_sum(&proof_data, &sum);
        height += 1;
    }

    sum == *root
}

#[cfg(test)]
mod test {
    use super::verify;
    use crate::{
        TEST_DATA,
        binary::{
            Data,
            MerkleTree,
        },
    };

    #[test]
    fn verify_returns_true_when_the_given_proof_set_matches_the_given_merkle_root() {
        let proof_index = 2;

        let mut mt = MerkleTree::new();
        mt.set_proof_index(proof_index);

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            mt.push(datum);
        }

        let (root, proof_set) = mt.prove();
        let verification = verify(
            &root,
            &data[proof_index as usize],
            &proof_set,
            proof_index,
            data.len() as u64,
        );
        assert!(verification);
    }

    #[test]
    fn verify_returns_false_when_the_given_proof_set_does_not_match_the_given_merkle_root()
     {
        // Check the Merkle root of one tree against the computed Merkle root of
        // another tree's proof set: because the two roots come from different
        // trees, the comparison should fail.

        let proof_index = 2;

        // Generate the first Merkle tree and get its root
        let mut mt = MerkleTree::new();
        mt.set_proof_index(proof_index);

        let data = &TEST_DATA[0..4];
        for datum in data.iter() {
            mt.push(datum)
        }
        let (root, _proof_set) = mt.prove();

        // Generate the second Merkle tree and get its proof set
        let mut mt = MerkleTree::new();
        mt.set_proof_index(proof_index);

        let data = &TEST_DATA[5..10];
        for datum in data.iter() {
            mt.push(datum);
        }
        let proof_index = 2;
        let (_, proof_set) = mt.prove();

        let verification = verify(
            &root,
            &data[proof_index],
            &proof_set,
            proof_index as u64,
            data.len() as u64,
        );
        assert!(!verification);
    }

    #[test]
    fn verify_returns_false_when_the_proof_set_is_empty() {
        let proof_index = 2;

        let mut mt = MerkleTree::new();
        mt.set_proof_index(proof_index);

        let (root, proof_set) = mt.prove();
        let verification = verify(&root, &Data::default(), &proof_set, 0, 0);
        assert!(!verification);
    }

    #[test]
    fn verify_returns_false_when_the_proof_index_is_invalid() {
        let proof_index = 2;

        let mut mt = MerkleTree::new();
        mt.set_proof_index(proof_index);

        let data = &TEST_DATA[0..4];
        for datum in data.iter() {
            mt.push(datum);
        }

        let (root, proof_set) = mt.prove();
        let verification = verify(&root, &data[proof_index as usize], &proof_set, 15, 5);
        assert!(!verification);
    }
}
