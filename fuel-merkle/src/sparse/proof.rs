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

impl Debug for InclusionProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("InclusionProof");
        debug.field("Root", &hex::encode(self.root));
        let proof_set = self
            .proof_set
            .iter()
            .map(|p| hex::encode(p))
            .collect::<Vec<_>>();
        debug.field("Proof set", &proof_set);
        debug.finish()
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct ExclusionProof {
    pub root: Bytes32,
    pub proof_set: ProofSet,
    pub leaf: Option<(Bytes32, Bytes32)>,
}

impl ExclusionProof {
    pub fn verify<K: Into<Bytes32>>(&self, key: K) -> bool {
        let Self {
            root,
            proof_set,
            leaf,
        } = self;
        let key = key.into();
        let leaf = if let Some((leaf_key, leaf_data)) = leaf {
            Node::new(0, Prefix::Leaf, *leaf_key, *leaf_data)
        } else {
            Node::create_placeholder()
        };
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

    pub fn leaf_key(&self) -> Option<&Bytes32> {
        self.leaf.as_ref().map(|leaf| &leaf.0)
    }

    pub fn leaf_data(&self) -> Option<&Bytes32> {
        self.leaf.as_ref().map(|leaf| &leaf.1)
    }
}

impl Debug for ExclusionProof {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("ExclusionProof");
        debug.field("Root", &hex::encode(self.root));
        let proof_set = self
            .proof_set
            .iter()
            .map(|p| hex::encode(p))
            .collect::<Vec<_>>();
        debug.field("Proof set", &proof_set);
        if let Some(leaf_key) = self.leaf_key() {
            debug.field("Leaf key", &hex::encode(leaf_key));
        }
        if let Some(leaf_data) = self.leaf_data() {
            debug.field("Leaf data", &hex::encode(leaf_data));
        }
        debug.finish()
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
            Proof::Exclusion(proof) => proof.verify(k3),
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
            Proof::Exclusion(proof) => proof.verify(k3),
            Proof::Inclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(exclusion);
    }

    // #[test]
    // fn verify_exclusion_proof_for_existent_key_returns_false() {
    //     let mut rng = StdRng::seed_from_u64(0xBAADF00D);
    //     let mut storage = StorageMap::<TestTable>::new();
    //     let mut tree = MerkleTree::new(&mut storage);
    //
    //     let key = random_bytes32(&mut rng).into();
    //     let value = random_bytes32(&mut rng);
    //     tree.update(key, &value).unwrap();
    //
    //     for _ in 0..1_000 {
    //         let key = random_bytes32(&mut rng).into();
    //         let value = random_bytes32(&mut rng);
    //         tree.update(key, &value).unwrap();
    //     }
    //
    //     // Generate inclusion proof and convert to exclusion proof
    //     let proof = tree.generate_proof(key).unwrap();
    //     let exclusion = match proof {
    //         Proof::Inclusion(proof) => {
    //             let InclusionProof { root, proof_set } = proof;
    //             let proof = ExclusionProof { root, proof_set };
    //             proof.verify(key)
    //         }
    //         Proof::Exclusion(_) => panic!("Expected InclusionProof"),
    //     };
    //     assert!(!exclusion);
    // }

    // #[test]
    // fn verify_exclusion_proof() {
    //     fn decode(value: &str) -> Bytes32 {
    //         hex::decode(value).unwrap().try_into().unwrap()
    //     }
    //
    //     let mut storage = StorageMap::<TestTable>::new();
    //     let mut tree = MerkleTree::new(&mut storage);
    //
    //     let key =
    //         decode("88c232320c04cf0da7f906f68f869fa350c37e0e5f40c130ed5e181cc25bd25b");
    //     let value =
    //         decode("51d75dfbb2b8796b3bdd51148ccba626149563ef48deeb5173c7f194ba1f161f");
    //     tree.update(key.into(), &value).unwrap();
    //     println!("root after update: {:?}", tree.root_node());
    //
    //     let key =
    //         decode("0589789b5488d2f496e0d809d1de085080f12244bfbe95e63effebe2f0e0401b");
    //     let value =
    //         decode("ed2a07f592ccfc00702998b926636214afc74a70f893769c02c648e3932ef5dd");
    //     tree.update(key.into(), &value).unwrap();
    //     println!("root after update: {:?}", tree.root_node());
    //
    //     let key: MerkleTreeKey =
    //         decode("eb000b84dcbca506a0040b9857332c41e806c678152db9a15ff01a54f9758f9b")
    //             .into();
    //
    //     let proof = tree.generate_proof(key).unwrap();
    //     dbg!(proof
    //         .proof_set()
    //         .iter()
    //         .map(|p| hex::encode(p))
    //         .collect::<Vec<_>>());
    //     let exclusion = match proof {
    //         Proof::Exclusion(proof) => {
    //             let ExclusionProof { root, proof_set } = proof;
    //             let proof = ExclusionProof { root, proof_set };
    //             proof.verify(key)
    //         }
    //         Proof::Inclusion(_) => panic!("Expected InclusionProof"),
    //     };
    //     assert!(exclusion);
    // }

    #[test]
    fn verify_exclusion_proof() {
        fn decode(value: &str) -> Bytes32 {
            hex::decode(value).unwrap().try_into().unwrap()
        }

        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        {
            let key = decode(
                "7d00962717006101371800002a000100270c5200300b00063d0007012a4e1e19",
            );
            let value = decode(
                "0000000000000000000000000000000000000000000000000000000000000000",
            );
            tree.update(MerkleTreeKey::new(key), &value).unwrap();
            println!("root after update: {:?}", hex::encode(tree.root()));
        }

        {
            let key = decode(
                "7b533ca6399a24492042020f983b6e0a2530140e7ec020684f0b42852c407721",
            );
            let value = decode(
                "0000000000000000000000000001ce6398719b2999539d3a1cf8af6c40e3f0da",
            );
            tree.update(MerkleTreeKey::new(key), &value).unwrap();
            println!("root after update: {:?}", hex::encode(tree.root()));
        }

        {
            let key = decode(
                "5e002800037901346a0006d35f0f050000000f160062467000287a3f1f010903",
            );
            let value = decode(
                "83bf317a03d652151e8781666ffb541fcd63888824e6f93312d512751bd9313b",
            );
            tree.update(MerkleTreeKey::new(key), &value).unwrap();
            println!("root after update: {:?}", hex::encode(tree.root()));
        }

        let key: MerkleTreeKey =
            decode("552ce6cf8360dc64bc7b3d7e39f3251360b3146545abc1c96a73c3dae53cfa1f")
                .into();

        let proof = tree.generate_proof(key).unwrap();
        dbg!(&proof);
        let exclusion = match proof {
            Proof::Exclusion(proof) => proof.verify(key),
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
            Proof::Exclusion(proof) => proof.verify(key),
            Proof::Inclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(exclusion);
    }

    #[test]
    fn verify_exclusion_proof_3() {
        fn decode(value: &str) -> Bytes32 {
            hex::decode(value).unwrap().try_into().unwrap()
        }

        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let key =
            decode("6603ffdf22b977cd650aa16838417ff339b628ae459a74e5b3b7ccb343b6fbc9");
        let value =
            decode("ff3e2001c9dd742c18670e5392cde1ef5d593b2d7c7c7a93f0f6eb51c9c10187");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("10430dc4ba7036f41a18577e988e915722c29b8d54912b91aac2ac9d601fd405");
        let value =
            decode("aa3641fe890a9b56ee95f977fc14a2c5b589846ed70d21c749fbe4bd4177b1ea");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("e4c31a34ce63bc3e7d2dd35e11e5625b2b989b23415fe147f778738913b2ae11");
        let value =
            decode("bf17ef4e8cc11af18e0da1121c60040ca5e39291085656ecace45009b2aa8984");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("064a6caab4228341b28968781e25a3cf4cc26f051427517b6fc9fcc887c58037");
        let value =
            decode("d3dccbde050f4f4dfee6872441cfa7fba4cbfc971cddf6ced23f264ddb0dc225");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("df558542db6c4e288a5ae2d27cdee1b5f77b6a2fd75d7bdbd111449d37a872cc");
        let value =
            decode("34b72f0db3dd5d6665189492e8bcec945da83fb5097d28a4802d959431a45b01");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("45854545df868a8f0f362dca6c9cf47df1c6876806245ddadf63f1106d1acb31");
        let value =
            decode("2b01f9b8ea40f595f2fb512a4e9761a597a82943f1c5b1bc9d7e782612802bed");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("c6e07d829f4cf89ba59e5b59f1df66247359d9fdf93b501f6922d985d9297e5e");
        let value =
            decode("865f3498278e570e21af613379fffdf0e91b0e9cd6996ec0d14bececcdb05b44");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("9f94bce62ddf444ca02f8723ed6ba05d2087a4b442f546aa9d982134ea13b91d");
        let value =
            decode("3c76b62db0193a1bbfed2eef076b142655f03ad141cd4d5e6f0a6c65e6cf9f52");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("fcea8f34f900981fdd0cd034da015981a5f9b0a7c4e0e2f86531f6d669fbcb8e");
        let value =
            decode("f7f38bcfeb365a23e174515eab4dcf0bbc17946dd80d3c6c1f310a6e203d13bc");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("bfaf55f9fe3fa4c664361482f0b02dcfc96f23b6fecc381cbd00f95527a81321");
        let value =
            decode("a41d3de6edb6a2502537863feb2040a22c0770de1eeebe368e5cc0a5af2f31d5");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("14aa8541c9c58b402cb5fa4ab484f97a311ec7ee5e8a8c854596c76579801a19");
        let value =
            decode("d1fcb2786b61b9d48732a8c33826f4a99be3f562d53fa31867edd08f7f16d69c");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("63f3447ae1c7e709f2ebdf635121b1bfab252f172cb86f408e9224a4c5d87f52");
        let value =
            decode("ed5620d0b0dcaa3d1842f137cd0044774afb56339f23361959600381e04c49bc");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("751a9f07cf411b194d1b903d83cd8ba7ad10e4f303d86cf1e50ff4630212b752");
        let value =
            decode("20e1c45433d19cdd82506cb25d0a8b9e874bd0ff16bf155cb7322df267518854");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("f5d152ac2b89fa9f6aa5f5990f111f054e88c9a92a7aa8a1b23254e8d99cc007");
        let value =
            decode("9d062871a9f6e0ed1dd0b45aaefba4ca3ae8577e3b02fdc6c4021220df59374a");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("e21aa74c3640da62ab60748cd641100318d67d597913069373dad0db260823fa");
        let value =
            decode("b7fb238e1c189279017ad61ea1281b9538818aa653a8039e9ff953c174db3e81");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("c95d30db00f7f25159a4c16b5713e7f61e9bde1584f5faa248adf9995715873c");
        let value =
            decode("634169ebbc9d6c09c7763bc025be9184e3fda02a1e8e5f79902b5d9b351c107c");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("13dbf8a70310af5b581b4922cb3bb31986aaf8301fd477daa06801ad8fbf8bc4");
        let value =
            decode("4956fd3d76089f550f82e163e1db34cdc5327fe60a4f4df3e95b8bd2a7c4258b");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("a35e417bd00099324ded7c5a077271bfd0726379f3fabbba77b3a87656a0c272");
        let value =
            decode("d23ee247bc7ef3dedc6e63eca4363e5a2047c85c2f1104ba6703cc8c4dec36a0");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("3082d3ea0bfc4184ccb2d7f36a174e210fc388c15469172f4de0ee415f14eefb");
        let value =
            decode("1997e1fe595f3ac732e166089b95253403cf6ff9416a0e69d1e2aba30511fe2b");
        tree.update(key.into(), &value).unwrap();

        let key =
            decode("85c1ba9c4c9ffdb4e6627f5a63f6714cd8f5b40141929ba9895bf79ae33b6fcc");
        let value =
            decode("771f14eb90ce6187e4486a56808f56e28f56d934646a2fe3c4785525eed8ea85");
        tree.update(key.into(), &value).unwrap();

        dbg!(tree.root_node());

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
            Proof::Exclusion(proof) => proof.verify(key),
            Proof::Inclusion(_) => panic!("Expected InclusionProof"),
        };
        assert!(exclusion);
    }
}
