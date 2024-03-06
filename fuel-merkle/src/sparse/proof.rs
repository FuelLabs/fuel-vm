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
    pub fn root(&self) -> &Bytes32 {
        match self {
            Proof::Inclusion(proof) => &proof.root,
            Proof::Exclusion(proof) => &proof.root,
        }
    }

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
    pub root: Bytes32,
    pub proof_set: ProofSet,
}

impl InclusionProof {
    pub fn verify<K: Into<Bytes32>, V: AsRef<[u8]>>(&self, key: K, value: &V) -> bool {
        let Self { root, proof_set } = self;
        let key = key.into();
        let leaf = Node::create_leaf(&key, value);
        let mut current = *leaf.hash();
        for (i, side_hash) in proof_set.iter().enumerate() {
            let index = u32::try_from(proof_set.len() - 1 - i).expect("Index is valid");
            let prefix = Prefix::Node;
            current = match key.get_instruction(index).expect("Infallible") {
                Instruction::Left => Node::calculate_hash(&prefix, &current, side_hash),
                Instruction::Right => Node::calculate_hash(&prefix, side_hash, &current),
            };
        }
        current == *root
    }
}

impl Debug for InclusionProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let proof_set = self.proof_set.iter().map(hex::encode).collect::<Vec<_>>();
        f.debug_struct("InclusionProof")
            .field("Root", &hex::encode(self.root))
            .field("Proof set", &proof_set)
            .finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct ExclusionProof {
    pub root: Bytes32,
    pub proof_set: ProofSet,
    pub(crate) leaf: Node,
}

impl ExclusionProof {
    pub fn verify<K: Into<Bytes32>>(&self, key: K) -> bool {
        let Self {
            root,
            proof_set,
            leaf,
        } = self;
        let key = key.into();
        let mut current = *leaf.hash();
        for (i, side_hash) in proof_set.iter().enumerate() {
            let index = u32::try_from(proof_set.len() - 1 - i).expect("Index is valid");
            let prefix = Prefix::Node;
            current = match key.get_instruction(index).expect("Infallible") {
                Instruction::Left => Node::calculate_hash(&prefix, &current, side_hash),
                Instruction::Right => Node::calculate_hash(&prefix, side_hash, &current),
            };
        }
        current == *root
    }
}

impl Debug for ExclusionProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let proof_set = self.proof_set.iter().map(hex::encode).collect::<Vec<_>>();
        f.debug_struct("ExclusionProof")
            .field("Root", &hex::encode(self.root))
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
            proof::Proof,
            MerkleTree,
            Primitive,
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

        let k0 = [0u8; 32];
        let v0 = b"DATA_0";
        tree.update(k0.into(), v0)
            .expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let v1 = b"DATA_1";
        tree.update(k1.into(), v1)
            .expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let v2 = b"DATA_2";
        tree.update(k2.into(), v2)
            .expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let v3 = b"DATA_3";
        tree.update(k3.into(), v3)
            .expect("Expected successful update");

        {
            // Given
            let proof = tree.generate_proof(k0).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(k0, b"DATA_0"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(k1).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(k1, b"DATA_1"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(k2).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(k2, b"DATA_2"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(k3).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(k3, b"DATA_3"),
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

        let k0 = [0u8; 32];
        let v0 = b"DATA_0";
        tree.update(k0.into(), v0)
            .expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let v1 = b"DATA_1";
        tree.update(k1.into(), v1)
            .expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let v2 = b"DATA_2";
        tree.update(k2.into(), v2)
            .expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let v3 = b"DATA_3";
        tree.update(k3.into(), v3)
            .expect("Expected successful update");

        {
            // Given
            let proof = tree.generate_proof(k0).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(k0, b"DATA_100"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(!inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(k1).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(k1, b"DATA_100"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(!inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(k2).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(k2, b"DATA_100"),
                Proof::Exclusion(_) => panic!("Expected InclusionProof"),
            };

            // Then
            assert!(!inclusion);
        }

        {
            // Given
            let proof = tree.generate_proof(k3).unwrap();

            // When
            let inclusion = match proof {
                Proof::Inclusion(proof) => proof.verify(k3, b"DATA_100"),
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

        let k0 = [0u8; 32];
        let v0 = b"DATA_0";
        tree.update(k0.into(), v0)
            .expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let v1 = b"DATA_1";
        tree.update(k1.into(), v1)
            .expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let v2 = b"DATA_2";
        tree.update(k2.into(), v2)
            .expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let v3 = b"DATA_3";
        tree.update(k3.into(), v3)
            .expect("Expected successful update");

        // Given
        let proof = tree.generate_proof(k3).unwrap();

        // When
        let key = [1u8; 32];
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(key, b"DATA_3"),
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
        tree.update(k0.into(), v0)
            .expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let v1 = b"DATA_1";
        tree.update(k1.into(), v1)
            .expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let v2 = b"DATA_2";
        tree.update(k2.into(), v2)
            .expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let v3 = b"DATA_3";
        tree.update(k3.into(), v3)
            .expect("Expected successful update");

        // Given
        let key = [0xffu8; 32];
        let proof = tree.generate_proof(key).unwrap();

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(key),
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

        let k0 = [0u8; 32];
        let v0 = b"DATA_0";
        tree.update(k0.into(), v0)
            .expect("Expected successful update");

        let mut k1 = [0u8; 32];
        k1[0] = 0b01000000;
        let v1 = b"DATA_1";
        tree.update(k1.into(), v1)
            .expect("Expected successful update");

        let mut k2 = [0u8; 32];
        k2[0] = 0b01100000;
        let v2 = b"DATA_2";
        tree.update(k2.into(), v2)
            .expect("Expected successful update");

        let mut k3 = [0u8; 32];
        k3[0] = 0b01001000;
        let v3 = b"DATA_3";
        tree.update(k3.into(), v3)
            .expect("Expected successful update");

        // Given
        let key = [0xffu8; 32];
        let proof = tree.generate_proof(key).unwrap();

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(k1),
        };

        // Then
        assert!(!exclusion);
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
            proof::Proof,
            MerkleTree,
            MerkleTreeKey,
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
    fn inclusion_proof__verify__returns_true_for_correct_key_and_correct_value() {
        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let key = random_bytes32(&mut rng);
        let value = random_bytes32(&mut rng);
        tree.update(key.into(), &value).unwrap();

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng);
            let value = random_bytes32(&mut rng);
            tree.update(key.into(), &value).unwrap();
        }

        // Given
        let proof = tree.generate_proof(key).unwrap();

        // When
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(key, &value),
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

        let key = random_bytes32(&mut rng);
        let value = random_bytes32(&mut rng);
        tree.update(key.into(), &value).unwrap();

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng);
            let value = random_bytes32(&mut rng);
            tree.update(key.into(), &value).unwrap();
        }

        // Given
        let proof = tree.generate_proof(key).unwrap();

        // When
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(key, b"DATA"),
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
        tree.update(key_1, &value_1).unwrap();

        let key_2 = random_bytes32(&mut rng).into();
        let value_2 = random_bytes32(&mut rng);
        tree.update(key_2, &value_2).unwrap();

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng).into();
            let value = random_bytes32(&mut rng);
            tree.update(key, &value).unwrap();
        }

        // Given
        // - Generate a proof with key_1
        let proof = tree.generate_proof(key_1).unwrap();

        // When
        // - Attempt to verify the proof with key_2
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(key_2, &value_2),
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
            tree.update(key.into(), &value).unwrap();
        }

        // Given
        let key: MerkleTreeKey = random_bytes32(&mut rng).into();
        let proof = tree.generate_proof(key).unwrap();

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(key),
        };

        // Then
        assert!(exclusion);
    }

    #[test]
    fn exclusion_proof__verify__returns_true_for_any_key_in_empty_tree() {
        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let mut storage = StorageMap::<TestTable>::new();
        let tree = MerkleTree::new(&mut storage);

        // Given
        let key = random_bytes32(&mut rng);
        let proof = tree.generate_proof(key).unwrap();

        // When
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(key),
        };

        // Then
        assert!(exclusion);
    }
}
