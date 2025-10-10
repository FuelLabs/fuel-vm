use crate::{
    common::{
        Bytes32,
        ProofSet,
        path::{
            Path,
            Side,
        },
        sum,
    },
    sparse::{
        MerkleTreeKey,
        hash::{
            calculate_leaf_hash,
            calculate_node_hash,
        },
        zero_sum,
    },
};

use alloc::vec::Vec;
use core::{
    fmt,
    fmt::Debug,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Proof {
    Inclusion(InclusionProof),
    Exclusion(ExclusionProof),
}

impl Proof {
    pub fn proof_set(&self) -> &ProofSet {
        match self {
            Proof::Inclusion(proof) => &proof.proof_set,
            Proof::Exclusion(proof) => &proof.proof_set,
        }
    }

    pub fn is_inclusion(&self) -> bool {
        match self {
            Proof::Inclusion(_) => true,
            Proof::Exclusion(_) => false,
        }
    }

    pub fn is_exclusion(&self) -> bool {
        !self.is_inclusion()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct InclusionProof {
    pub proof_set: ProofSet,
}

impl InclusionProof {
    pub fn verify(&self, root: &Bytes32, key: &MerkleTreeKey, value: &[u8]) -> bool {
        let Self { proof_set } = self;

        if proof_set.len() > 256usize {
            return false;
        }

        let mut current = calculate_leaf_hash(key, &sum(value));
        for (i, side_hash) in proof_set.iter().enumerate() {
            #[allow(clippy::arithmetic_side_effects)] // Cannot underflow
            let index =
                u32::try_from(proof_set.len() - 1 - i).expect("We've checked it above");
            current = match key.get_instruction(index).expect("Infallible") {
                Side::Left => calculate_node_hash(&current, side_hash),
                Side::Right => calculate_node_hash(side_hash, &current),
            };
        }
        current == *root
    }
}

impl Debug for InclusionProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let proof_set = self.proof_set.iter().map(hex::encode).collect::<Vec<_>>();
        f.debug_struct("InclusionProof")
            .field("Proof set", &proof_set)
            .finish()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ExclusionLeaf {
    Leaf(ExclusionLeafData),
    Placeholder,
}

#[derive(Clone, Eq, PartialEq)]
pub struct ExclusionLeafData {
    /// The leaf key.
    pub leaf_key: Bytes32,
    /// Hash of the value of the leaf.
    pub leaf_value: Bytes32,
}

impl Debug for ExclusionLeafData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExclusionLeafData")
            .field("Leaf key", &hex::encode(self.leaf_key))
            .field("Leaf value", &hex::encode(self.leaf_value))
            .finish()
    }
}

impl ExclusionLeaf {
    fn hash(&self) -> Bytes32 {
        match self {
            ExclusionLeaf::Leaf(data) => {
                calculate_leaf_hash(&data.leaf_key, &data.leaf_value)
            }
            ExclusionLeaf::Placeholder => *zero_sum(),
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct ExclusionProof {
    pub proof_set: ProofSet,
    pub leaf: ExclusionLeaf,
}

impl ExclusionProof {
    pub fn verify(&self, root: &Bytes32, key: &MerkleTreeKey) -> bool {
        let Self { proof_set, leaf } = self;

        if let ExclusionLeaf::Leaf(data) = leaf
            && data.leaf_key == key.as_ref()
        {
            return false;
        }

        if proof_set.len() > 256usize {
            return false;
        }

        let mut current = leaf.hash();
        for (i, side_hash) in proof_set.iter().enumerate() {
            #[allow(clippy::arithmetic_side_effects)] // Cannot underflow
            let index =
                u32::try_from(proof_set.len() - 1 - i).expect("We've checked it above");
            current = match key.get_instruction(index).expect("Infallible") {
                Side::Left => calculate_node_hash(&current, side_hash),
                Side::Right => calculate_node_hash(side_hash, &current),
            };
        }
        current == *root
    }
}

impl Debug for ExclusionProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let proof_set = self.proof_set.iter().map(hex::encode).collect::<Vec<_>>();
        f.debug_struct("ExclusionProof")
            .field("Proof set", &proof_set)
            .field("Leaf", &self.leaf)
            .finish()
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod test {
    use crate::{
        common::{
            Bytes32,
            StorageMap,
        },
        sparse::{
            MerkleTree,
            Primitive,
            proof::Proof,
        },
    };
    use fuel_storage::Mappable;

    #[derive(Debug)]
    struct TestTable;

    impl Mappable for TestTable {
        type Key = Self::OwnedKey;
        type OwnedKey = Bytes32;
        type OwnedValue = Primitive;
        type Value = Self::OwnedValue;
    }

    #[test]
    fn inclusion_proof__verify__returns_true_for_correct_key_and_correct_value() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

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

        let k0 = [0u8; 32].into();
        let v0 = b"DATA_0";
        tree.insert(k0, v0).expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let k1 = k1.into();
        let v1 = b"DATA_1";
        tree.insert(k1, v1).expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let k2 = k2.into();
        let v2 = b"DATA_2";
        tree.insert(k2, v2).expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let k3 = k3.into();
        let v3 = b"DATA_3";
        tree.insert(k3, v3).expect("Expected successful update");

        let root = tree.root();

        {
            // Given
            let proof = tree.generate_proof(&k0).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(&root, &k0, b"DATA_0"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(&k1).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(&root, &k1, b"DATA_1"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(&k2).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(&root, &k2, b"DATA_2"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(&k3).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(&root, &k3, b"DATA_3"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(inclusion);
        }
    }

    #[test]
    fn inclusion_proof__verify__returns_false_for_correct_key_and_incorrect_value() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

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

        let k0 = [0u8; 32].into();
        let v0 = b"DATA_0";
        tree.insert(k0, v0).expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let k1 = k1.into();
        let v1 = b"DATA_1";
        tree.insert(k1, v1).expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let k2 = k2.into();
        let v2 = b"DATA_2";
        tree.insert(k2, v2).expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let k3 = k3.into();
        let v3 = b"DATA_3";
        tree.insert(k3, v3).expect("Expected successful update");

        let root = tree.root();

        {
            // Given
            let proof = tree.generate_proof(&k0).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(&root, &k0, b"DATA_100"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(!inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(&k1).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(&root, &k1, b"DATA_100"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(!inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(&k2).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(&root, &k2, b"DATA_100"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(!inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(&k3).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(&root, &k3, b"DATA_100"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };
            // Then
            assert!(!inclusion);
        }
    }

    #[test]
    fn inclusion_proof__verify__returns_false_for_incorrect_key() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

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

        let k0 = [0u8; 32].into();
        let v0 = b"DATA_0";
        tree.insert(k0, v0).expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let k1 = k1.into();
        let v1 = b"DATA_1";
        tree.insert(k1, v1).expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let k2 = k2.into();
        let v2 = b"DATA_2";
        tree.insert(k2, v2).expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let k3 = k3.into();
        let v3 = b"DATA_3";
        tree.insert(k3, v3).expect("Expected successful update");

        let root = tree.root();

        // Given
        let proof = tree.generate_proof(&k3).unwrap();

        // When
        let key = [1u8; 32].into();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(&root, &key, b"DATA_3"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };

        // Then
        assert!(!inclusion);
    }

    #[test]
    fn exclusion_proof__verify__returns_true_for_correct_key() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

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

        let k0 = [0u8; 32];
        let v0 = b"DATA_0";
        tree.insert(k0.into(), v0)
            .expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let v1 = b"DATA_1";
        tree.insert(k1.into(), v1)
            .expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let v2 = b"DATA_2";
        tree.insert(k2.into(), v2)
            .expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let v3 = b"DATA_3";
        tree.insert(k3.into(), v3)
            .expect("Expected successful update");

        let root = tree.root();

        // Given
        let key = [0xffu8; 32].into();
        let proof = tree.generate_proof(&key).unwrap();

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(&root, &key),
        };

        // Then
        assert!(exclusion);
    }

    #[test]
    fn exclusion_proof__verify__returns_false_for_incorrect_key() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

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

        let k0 = [0u8; 32].into();
        let v0 = b"DATA_0";
        tree.insert(k0, v0).expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let k1 = k1.into();
        let v1 = b"DATA_1";
        tree.insert(k1, v1).expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let k2 = k2.into();
        let v2 = b"DATA_2";
        tree.insert(k2, v2).expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let k3 = k3.into();
        let v3 = b"DATA_3";
        tree.insert(k3, v3).expect("Expected successful update");

        let root = tree.root();

        // Given
        let key = [0xffu8; 32].into();
        let proof = tree.generate_proof(&key).unwrap();

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(&root, &k1),
        };

        // Then
        assert!(!exclusion);
    }

    #[test]
    fn exclusion_proof__verify__returns_true_for_placeholder() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

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
        //   0: P1  L0  L2  P0  L1  P2
        //          K0  K2      K1

        let mut k0 = [0u8; 32];
        k0[0] = 0b01000000;
        let k0 = k0.into();
        let v0 = b"DATA_0";
        tree.insert(k0, v0).expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01100000;
        let k1 = k1.into();
        let v1 = b"DATA_1";
        tree.insert(k1, v1).expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01001000;
        let k2 = k2.into();
        let v2 = b"DATA_2";
        tree.insert(k2, v2).expect("Expected successful update");

        let root = tree.root();

        // Given
        let key = [0b00000000; 32].into();
        let proof = tree.generate_proof(&key).unwrap();

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(&root, &key),
        };

        // Then
        assert!(exclusion);
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod test_random {
    use crate::{
        common::{
            Bytes32,
            StorageMap,
        },
        sparse::{
            MerkleTree,
            MerkleTreeKey,
            Primitive,
            proof::Proof,
        },
    };
    use fuel_storage::Mappable;

    use rand::{
        SeedableRng,
        prelude::StdRng,
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
    fn inclusion_proof__verify__returns_true_for_correct_key_and_correct_value() {
        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let key = random_bytes32(&mut rng).into();
        let value = random_bytes32(&mut rng);
        tree.insert(key, &value).unwrap();

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng).into();
            let value = random_bytes32(&mut rng);
            tree.insert(key, &value).unwrap();
        }

        let root = tree.root();

        // Given
        let proof = tree.generate_proof(&key).unwrap();

        // When
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(&root, &key, &value),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };

        // Then
        assert!(inclusion);
    }

    #[test]
    fn inclusion_proof__verify__returns_false_for_correct_key_and_incorrect_value() {
        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let key = random_bytes32(&mut rng).into();
        let value = random_bytes32(&mut rng);
        tree.insert(key, &value).unwrap();

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng).into();
            let value = random_bytes32(&mut rng);
            tree.insert(key, &value).unwrap();
        }

        let root = tree.root();

        // Given
        let proof = tree.generate_proof(&key).unwrap();

        // When
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(&root, &key, b"DATA"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };

        // Then
        assert!(!inclusion);
    }

    #[test]
    fn inclusion_proof__verify__returns_false_for_incorrect_key() {
        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let key_1 = random_bytes32(&mut rng).into();
        let value_1 = random_bytes32(&mut rng);
        tree.insert(key_1, &value_1).unwrap();

        let key_2 = random_bytes32(&mut rng).into();
        let value_2 = random_bytes32(&mut rng);
        tree.insert(key_2, &value_2).unwrap();

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng).into();
            let value = random_bytes32(&mut rng);
            tree.insert(key, &value).unwrap();
        }

        let root = tree.root();

        // Given
        // - Generate a proof with key_1
        let proof = tree.generate_proof(&key_1).unwrap();

        // When
        // - Attempt to verify the proof with key_2
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(&root, &key_2, &value_2),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };

        // Then
        assert!(!inclusion);
    }

    #[test]
    fn exclusion_proof__verify__returns_true_for_correct_key() {
        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng);
            let value = random_bytes32(&mut rng);
            tree.insert(key.into(), &value).unwrap();
        }

        let root = tree.root();

        // Given
        let key: MerkleTreeKey = random_bytes32(&mut rng).into();
        let proof = tree.generate_proof(&key).unwrap();

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(&root, &key),
        };

        // Then
        assert!(exclusion);
    }

    #[test]
    fn exclusion_proof__verify__returns_true_for_any_key_in_empty_tree() {
        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let mut storage = StorageMap::<TestTable>::new();
        let tree = MerkleTree::new(&mut storage);
        let root = tree.root();

        // Given
        let key = random_bytes32(&mut rng).into();
        let proof = tree.generate_proof(&key).unwrap();

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(&root, &key),
        };

        // Then
        assert!(exclusion);
    }
}
