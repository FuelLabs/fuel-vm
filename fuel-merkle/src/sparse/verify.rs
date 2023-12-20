use crate::{
    common::{
        path::{
            Instruction,
            Path,
        },
        Bytes32,
        Prefix,
        ProofSet,
    },
    sparse::Node,
};

pub fn _verify<T: AsRef<[u8]>>(
    _key: &Bytes32,
    _value: &T,
    _root: &Bytes32,
    _proof_set: &ProofSet,
) -> bool {
    true
}

pub fn verify_inclusion<T: AsRef<[u8]>>(
    key: &Bytes32,
    value: &T,
    root: &Bytes32,
    proof_set: &ProofSet,
) -> bool {
    let leaf = Node::create_leaf(key, value);
    let path = leaf.leaf_key();
    let mut current = *leaf.hash();
    for (i, side_hash) in proof_set.iter().enumerate() {
        let index = proof_set.len() - 1 - i;
        let prefix = Prefix::Node;
        current = match path.get_instruction(index as u32).unwrap() {
            Instruction::Left => Node::calculate_hash(&prefix, &current, side_hash),
            Instruction::Right => Node::calculate_hash(&prefix, side_hash, &current),
        };
    }
    current == *root
}

#[cfg(test)]
mod test {
    use crate::{
        common::{
            Bytes32,
            StorageMap,
        },
        sparse::{
            hash::sum,
            verify::verify_inclusion,
            MerkleTree,
            MerkleTreeKey,
            Node,
            Primitive,
        },
    };
    use fuel_storage::Mappable;
    use rand::{
        prelude::StdRng,
        SeedableRng,
    };

    #[derive(Debug)]
    struct TestTable;

    impl Mappable for TestTable {
        type Key = Self::OwnedKey;
        type OwnedKey = Bytes32;
        type OwnedValue = Primitive;
        type Value = Self::OwnedValue;
    }

    fn random_bytes32<R>(rng: &mut R) -> Bytes32
    where
        R: rand::Rng + ?Sized,
    {
        let mut bytes = [0u8; 32];
        rng.fill(bytes.as_mut());
        bytes
    }

    #[test]
    fn verify_inclusion_returns_true_for_included_key_value() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let k0 = [0u8; 32];
        let v0 = sum(b"DATA");
        tree.update(MerkleTreeKey::new_without_hash(k0), &v0)
            .expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let v1 = sum(b"DATA");
        tree.update(MerkleTreeKey::new_without_hash(k1), &v1)
            .expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let v2 = sum(b"DATA");
        tree.update(MerkleTreeKey::new_without_hash(k2), &v2)
            .expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let v3 = sum(b"DATA");
        tree.update(MerkleTreeKey::new_without_hash(k3), &v3)
            .expect("Expected successful update");

        // 256:           N4
        //               /  \
        // 255:         N3   \
        //             /  \   \
        // 254:       /   N2   \
        //           /   /  \   \
        // 253:     /   N1   \   \
        //         /   /  \   \   \
        // 252:   /   N0   \   \   \
        // ...   /   /  \   \   \   \
        //   0: L0  L1  L3  P1  L2  P0
        //      K0  K1  K3      K2

        let (root, proof_set) = tree.generate_proof(k2).unwrap();
        let inclusion = verify_inclusion(&k2, &v2, &root, &proof_set);
        assert!(inclusion);
    }

    #[test]
    fn verify_inclusion_returns_false_for_excluded_key_value() {
        let mut rng = StdRng::seed_from_u64(0xBAADF00D);
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let k0 = [0u8; 32];
        let v0 = sum(b"DATA");
        tree.update(MerkleTreeKey::new_without_hash(k0), &v0)
            .expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let v1 = sum(b"DATA");
        tree.update(MerkleTreeKey::new_without_hash(k1), &v1)
            .expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let v2 = sum(b"DATA");
        tree.update(MerkleTreeKey::new_without_hash(k2), &v2)
            .expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let v3 = sum(b"DATA");
        tree.update(MerkleTreeKey::new_without_hash(k3), &v3)
            .expect("Expected successful update");

        // 256:           N4
        //               /  \
        // 255:         N3   \
        //             /  \   \
        // 254:       /   N2   \
        //           /   /  \   \
        // 253:     /   N1   \   \
        //         /   /  \   \   \
        // 252:   /   N0   \   \   \
        // ...   /   /  \   \   \   \
        //   0: L0  L1  L3  P1  L2  P0
        //      K0  K1  K3      K2

        let (root, proof_set) = tree.generate_proof(k2).unwrap();
        let erroneous_v = random_bytes32(&mut rng);
        let inclusion = verify_inclusion(&k2, &erroneous_v, &root, &proof_set);
        assert!(!inclusion);
    }

    #[test]
    fn verify_inclusion_test() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let mut rng = StdRng::seed_from_u64(0xBAADF00D);

        let key = random_bytes32(&mut rng);
        let value = random_bytes32(&mut rng);
        tree.update(MerkleTreeKey::new_without_hash(key), &value)
            .unwrap();

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng);
            let value = random_bytes32(&mut rng);
            tree.update(MerkleTreeKey::new(key), &value).unwrap();
        }

        let (root, proof_set) = tree.generate_proof(key).unwrap();
        let inclusion = verify_inclusion(&key, &value, &root, &proof_set);
        assert!(inclusion);
    }
}
