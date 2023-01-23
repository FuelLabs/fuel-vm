use crate::binary::{node_sum, Data};

pub fn verify(root: &Data, proof_set: &Vec<Data>, proof_index: u64, num_leaves: u64) -> bool {
    if proof_index >= num_leaves {
        return false;
    }

    if proof_set.is_empty() {
        return false;
    }

    let mut height = 0usize;
    let mut sum = proof_set[height];
    height += 1;

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
    use crate::TEST_DATA;

    #[test]
    fn verify_returns_true_when_the_given_proof_set_matches_the_given_merkle_root() {
        let mut mt = MerkleTree::new();
        mt.set_proof_index(2);

        let data = &TEST_DATA[0..5]; // 5 leaves
        for datum in data.iter() {
            mt.push(datum);
        }

        let proof = mt.prove();
        let root = proof.0;
        let set = proof.1;

        let verification = verify(&root, &set, 2, 5);
        assert_eq!(verification, true);
    }

    #[test]
    fn verify_returns_false_when_the_given_proof_set_does_not_match_the_given_merkle_root() {
        // Check the Merkle root of one tree against the computed Merkle root of
        // another tree's proof set: because the two roots come from different
        // trees, the comparison should fail.

        // Generate the first Merkle tree and get its root
        let mut mt = MerkleTree::new();
        mt.set_proof_index(2);

        let data = &TEST_DATA[0..4];
        for datum in data.iter() {
            mt.push(datum)
        }
        let proof = mt.prove();
        let root = proof.0;

        // Generate the second Merkle tree and get its proof set
        let mut mt = MerkleTree::new();
        mt.set_proof_index(2);

        let data = &TEST_DATA[5..10];
        for datum in data.iter() {
            mt.push(datum);
        }
        let proof = mt.prove();
        let set = proof.1;

        let verification = verify(&root, &set, 2, 5);
        assert_eq!(verification, false);
    }

    #[test]
    fn verify_returns_false_when_the_proof_set_is_empty() {
        let mut mt = MerkleTree::new();
        mt.set_proof_index(0);

        let proof = mt.prove();
        let root = proof.0;
        let set = proof.1;

        let verification = verify(&root, &set, 0, 0);
        assert_eq!(verification, false);
    }

    #[test]
    fn verify_returns_false_when_the_proof_index_is_invalid() {
        let mut mt = MerkleTree::new();
        mt.set_proof_index(0);

        let data = &TEST_DATA[0..4];
        for datum in data.iter() {
            mt.push(datum);
        }

        let proof = mt.prove();
        let root = proof.0;
        let set = proof.1;

        let verification = verify(&root, &set, 15, 5);
        assert_eq!(verification, false);
    }
}
