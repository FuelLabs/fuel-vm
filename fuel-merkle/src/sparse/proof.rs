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
use core::fmt::Debug;

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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExclusionProof {
    pub root: Bytes32,
    pub proof_set: ProofSet,
}

impl ExclusionProof {
    pub fn verify<K: Into<Bytes32>>(&self, key: K) -> bool {
        let Self { root, proof_set } = self;
        let key = key.into();
        let leaf = Node::create_placeholder();
        let mut current = *leaf.hash();
        for (i, side_hash) in proof_set.iter().enumerate() {
            let index = u32::try_from(proof_set.len() - 1 - i).expect("Index is valid");
            let prefix = Prefix::Node;
            current = match key.get_instruction(index).expect("Infallible") {
                Instruction::Left => Node::calculate_hash(&prefix, &current, side_hash),
                Instruction::Right => Node::calculate_hash(&prefix, side_hash, &current),
            };
        }

        println!(
            "current: {}, root: {}",
            hex::encode(current),
            hex::encode(*root)
        );

        current == *root
    }
}

#[cfg(test)]
mod test {
    use crate::{
        common::{
            Bytes32,
            StorageMap,
        },
        sparse::{
            proof::{
                ExclusionProof,
                InclusionProof,
                Proof,
            },
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
    fn verify_inclusion_proof_returns_true_for_included_key_value() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

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

        let proof = tree.generate_proof(k0).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(k0, b"DATA_0"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(inclusion);

        let proof = tree.generate_proof(k1).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(k1, b"DATA_1"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(inclusion);

        let proof = tree.generate_proof(k2).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(k2, b"DATA_2"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(inclusion);

        let proof = tree.generate_proof(k3).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(k3, b"DATA_3"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(inclusion);
    }

    #[test]
    fn verify_inclusion_proof_returns_false_for_included_key_invalid_value() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

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

        let proof = tree.generate_proof(k0).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(k0, b"DATA_100"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(!inclusion);

        let proof = tree.generate_proof(k1).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(k1, b"DATA_100"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(!inclusion);

        let proof = tree.generate_proof(k2).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(k2, b"DATA_100"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(!inclusion);

        let proof = tree.generate_proof(k3).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(k3, b"DATA_100"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(!inclusion);
    }

    #[test]
    fn verify_inclusion_proof_for_existing_key_and_correct_value_returns_true() {
        let mut rng = StdRng::seed_from_u64(0xBAADF00D);
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

        let proof = tree.generate_proof(key).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(key, &value),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(inclusion);
    }

    #[test]
    fn verify_inclusion_proof_for_existing_key_and_incorrect_value_returns_false() {
        let mut rng = StdRng::seed_from_u64(0xBAADF00D);
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

        let proof = tree.generate_proof(key).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(key, b"DATA"),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(!inclusion);
    }

    #[test]
    fn verify_inclusion_proof_for_unrelated_key_value_returns_false() {
        let mut rng = StdRng::seed_from_u64(0xBAADF00D);
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

        let proof = tree.generate_proof(key_1).unwrap();
        let inclusion = match proof {
            Proof::Inclusion(proof) => proof.verify(key_2, &value_2),
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(!inclusion);
    }

    #[test]
    fn verify_exclusion_proof_for_nonexistent_key_returns_true() {
        let mut rng = StdRng::seed_from_u64(0xBAADF00D);
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng);
            let value = random_bytes32(&mut rng);
            tree.update(key.into(), &value).unwrap();
        }

        let key: MerkleTreeKey = random_bytes32(&mut rng).into();
        let proof = tree.generate_proof(key).unwrap();
        let exclusion = match proof {
            Proof::Inclusion(_) => panic!("Expected ExclusionProof"),
            Proof::Exclusion(proof) => proof.verify(key),
        };
        assert!(exclusion);
    }

    #[test]
    fn verify_exclusion_proof_empty_tree() {
        let mut storage = StorageMap::<TestTable>::new();
        let tree = MerkleTree::new(&mut storage);

        let mut k3 = [0u8; 32];
        k3[31] = 0b00000001;

        // Generate inclusion proof and convert to exclusion proof
        let proof = tree.generate_proof(k3).unwrap();
        let exclusion = match proof {
            Proof::Exclusion(proof) => {
                let ExclusionProof { root, proof_set } = proof;
                let proof = ExclusionProof { root, proof_set };
                proof.verify(k3)
            }
            Proof::Inclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(exclusion);
    }

    #[test]
    fn verify_exclusion_proof_single_leaf() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let key = [0u8; 32].into();
        let value = [0u8; 32];
        tree.update(key, &value).unwrap();

        let mut k3 = [0u8; 32];
        k3[31] = 0b00000001;

        let proof = tree.generate_proof(k3).unwrap();
        let exclusion = match proof {
            Proof::Exclusion(proof) => {
                let ExclusionProof { root, proof_set } = proof;
                let proof = ExclusionProof { root, proof_set };
                proof.verify(k3)
            }
            Proof::Inclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(exclusion);
    }

    #[test]
    fn verify_exclusion_proof_for_existent_key_returns_false() {
        let mut rng = StdRng::seed_from_u64(0xBAADF00D);
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let key = random_bytes32(&mut rng).into();
        let value = random_bytes32(&mut rng);
        tree.update(key, &value).unwrap();

        for _ in 0..1_000 {
            let key = random_bytes32(&mut rng).into();
            let value = random_bytes32(&mut rng);
            tree.update(key, &value).unwrap();
        }

        // Generate inclusion proof and convert to exclusion proof
        let proof = tree.generate_proof(key).unwrap();
        let exclusion = match proof {
            Proof::Inclusion(proof) => {
                let InclusionProof { root, proof_set } = proof;
                let proof = ExclusionProof { root, proof_set };
                proof.verify(key)
            }
            Proof::Exclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(!exclusion);
    }

    #[test]
    fn verify_exclusion_proof() {
        fn decode(value: &str) -> Bytes32 {
            hex::decode(value).unwrap().try_into().unwrap()
        }

        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let key =
            decode("88c232320c04cf0da7f906f68f869fa350c37e0e5f40c130ed5e181cc25bd25b");
        let value =
            decode("51d75dfbb2b8796b3bdd51148ccba626149563ef48deeb5173c7f194ba1f161f");
        tree.update(key.into(), &value).unwrap();
        println!("root after update: {:?}", tree.root_node());

        let key =
            decode("0589789b5488d2f496e0d809d1de085080f12244bfbe95e63effebe2f0e0401b");
        let value =
            decode("ed2a07f592ccfc00702998b926636214afc74a70f893769c02c648e3932ef5dd");
        tree.update(key.into(), &value).unwrap();
        println!("root after update: {:?}", tree.root_node());

        let key: MerkleTreeKey =
            decode("eb000b84dcbca506a0040b9857332c41e806c678152db9a15ff01a54f9758f9b")
                .into();

        let proof = tree.generate_proof(key).unwrap();
        dbg!(proof
            .proof_set()
            .iter()
            .map(|p| hex::encode(p))
            .collect::<Vec<_>>());
        let exclusion = match proof {
            Proof::Exclusion(proof) => {
                let ExclusionProof { root, proof_set } = proof;
                let proof = ExclusionProof { root, proof_set };
                proof.verify(key)
            }
            Proof::Inclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(exclusion);
    }

    #[test]
    fn verify_exclusion_proof_2() {
        fn decode(value: &str) -> Bytes32 {
            hex::decode(value).unwrap().try_into().unwrap()
        }

        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let key =
            decode("78b4ae4df1dfbac38b2ed4c7f05ce8bc48232ccc7414b17d174db534bb0c81eb");
        let value =
            decode("4b26debba952b1fca7ee850a20eff077c00c62288731b40456b8eb26c27c692c");
        tree.update(key.into(), &value).unwrap();
        println!("root after update: {:?}", tree.root_node());

        let key =
            decode("60ec30b506455d11d9a239109b6fc655a00d7fe6b8898509fbb7b47e23a441dd");
        let value =
            decode("912edbd292ddbc2907604db618ff43b8829086d7cab6506b6912089b4e07cc51");
        tree.update(key.into(), &value).unwrap();
        println!("root after update: {:?}", tree.root_node());

        let key: MerkleTreeKey =
            decode("c1d60be1664856f5a6e6be46b0c796f8780c6144cbd4cb50ef4365470c4999cf")
                .into();

        let proof = tree.generate_proof(key).unwrap();
        dbg!(proof
            .proof_set()
            .iter()
            .map(|p| hex::encode(p))
            .collect::<Vec<_>>());
        let exclusion = match proof {
            Proof::Exclusion(proof) => {
                let ExclusionProof { root, proof_set } = proof;
                let proof = ExclusionProof { root, proof_set };
                proof.verify(key)
            }
            Proof::Inclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(exclusion);
    }
}
