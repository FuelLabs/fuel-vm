use crate::{
    common::Bytes32,
    sparse::zero_sum,
};
use core::{
    cmp::Ordering,
    mem,
};

use alloc::vec::Vec;

#[derive(Debug, Clone, derive_more::Display, PartialEq, Eq)]
pub enum MerkleTreeError<StorageError> {
    #[display(fmt = "cannot load node with key {_0:?}; the key is not found in storage")]
    LoadError(Bytes32),

    #[display(fmt = "{}", _0)]
    StorageError(StorageError),
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Child {
    pub height: u8,
    pub key: Bytes32,
    pub hash: Bytes32,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StorageNode {
    pub value: Bytes32,
    pub left_child: Option<Child>,
    pub right_child: Option<Child>,
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Node {
    pub key: Bytes32,
    pub value: Bytes32,
    pub left_child: Option<Child>,
    pub right_child: Option<Child>,
}

impl Node {
    pub fn from_storage(key: Bytes32, storage_node: StorageNode) -> Self {
        Self {
            key,
            value: storage_node.value,
            left_child: storage_node.left_child,
            right_child: storage_node.right_child,
        }
    }

    pub fn into_storage(self) -> (Bytes32, StorageNode) {
        (
            self.key,
            StorageNode {
                value: self.value,
                left_child: self.left_child,
                right_child: self.right_child,
            },
        )
    }
}

impl Node {
    pub fn set_left_child(&mut self, child: Node) {
        self.left_child = Some(Child {
            height: child.height(),
            key: child.key,
            hash: child.hash(),
        });
    }

    pub fn set_right_child(&mut self, child: Node) {
        self.right_child = Some(Child {
            height: child.height(),
            key: child.key,
            hash: child.hash(),
        });
    }

    pub fn left_height(&self) -> u8 {
        self.left_child
            .as_ref()
            .map(|child| child.height)
            .unwrap_or(0)
    }

    pub fn right_height(&self) -> u8 {
        self.right_child
            .as_ref()
            .map(|child| child.height)
            .unwrap_or(0)
    }

    pub fn height(&self) -> u8 {
        match (self.left_child.as_ref(), self.right_child.as_ref()) {
            (Some(left), Some(right)) => left.height.max(right.height).saturating_add(1),
            (Some(left), None) => left.height.saturating_add(1),
            (None, Some(right)) => right.height.saturating_add(1),
            (None, None) => 0,
        }
    }

    pub fn left_hash(&self) -> &Bytes32 {
        self.left_child
            .as_ref()
            .map(|child| &child.hash)
            .unwrap_or_else(|| zero_sum())
    }

    pub fn right_hash(&self) -> &Bytes32 {
        self.right_child
            .as_ref()
            .map(|child| &child.hash)
            .unwrap_or_else(|| zero_sum())
    }

    /// Calculate the hash of the node.
    /// The hash should be calculated once.
    pub fn hash(self) -> Bytes32 {
        use digest::Digest;
        let mut hash = sha2::Sha256::new();
        hash.update(self.key);
        hash.update(self.value);
        hash.update(self.left_hash());
        hash.update(self.right_hash());
        hash.finalize().into()
    }
}

pub trait AVLStorage<Table> {
    type StorageError;

    fn get(
        &self,
        prefix: &Bytes32,
        key: &Bytes32,
    ) -> Result<Option<StorageNode>, MerkleTreeError<Self::StorageError>>;

    fn set(
        &mut self,
        prefix: &Bytes32,
        key: &Bytes32,
        value: &StorageNode,
    ) -> Result<(), MerkleTreeError<Self::StorageError>>;
}

impl<Table, S> AVLStorage<Table> for &mut S
where
    S: AVLStorage<Table>,
{
    type StorageError = S::StorageError;

    fn get(
        &self,
        prefix: &Bytes32,
        key: &Bytes32,
    ) -> Result<Option<StorageNode>, MerkleTreeError<Self::StorageError>> {
        <S as AVLStorage<Table>>::get(self, prefix, key)
    }

    fn set(
        &mut self,
        prefix: &Bytes32,
        key: &Bytes32,
        value: &StorageNode,
    ) -> Result<(), MerkleTreeError<Self::StorageError>> {
        <S as AVLStorage<Table>>::set(self, prefix, key, value)
    }
}

#[derive(Default, Debug)]
pub struct AVLMerkleTree<Storage, Table> {
    prefix: Bytes32,
    root: Option<Node>,
    storage: Storage,
    _marker: core::marker::PhantomData<Table>,
}

enum Path {
    Left(Node),
    Right(Node),
    Current(Node),
}

impl<Storage, Table> AVLMerkleTree<Storage, Table> {
    pub fn load(prefix: Bytes32, root: Option<Node>, storage: Storage) -> Self {
        Self {
            prefix,
            root,
            storage,
            _marker: Default::default(),
        }
    }

    pub fn root(&self) -> Bytes32 {
        self.root.map(|node| node.hash()).unwrap_or(*zero_sum())
    }

    pub fn root_node(&self) -> &Option<Node> {
        &self.root
    }

    pub fn prefix(&self) -> Bytes32 {
        self.prefix
    }

    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    pub fn into_storage(self) -> Storage {
        self.storage
    }
}

impl<Storage, Table> AVLMerkleTree<Storage, Table>
where
    Storage: AVLStorage<Table>,
{
    pub fn delete(
        &mut self,
        key: Bytes32,
    ) -> Result<(), MerkleTreeError<Storage::StorageError>> {
        self.insert(key, [0; 32])
    }

    pub fn insert<V>(
        &mut self,
        key: Bytes32,
        value: V,
    ) -> Result<(), MerkleTreeError<Storage::StorageError>>
    where
        V: AsRef<[u8]>,
    {
        let value = to_bytes_32(value);
        let new_node = Node {
            key,
            value,
            left_child: None,
            right_child: None,
        };

        let old_node = self.get_node(&new_node.key);

        if let Ok(old_node) = old_node {
            if old_node.value == new_node.value {
                return Ok(());
            }
        }

        let new_root = if let Some(root) = self.root.take() {
            let new_root = self.insert_node(root, new_node);
            new_root?
        } else {
            new_node
        };

        self.store_node(new_root)?;
        self.root = Some(new_root);
        Ok(())
    }

    fn get_node(
        &self,
        key: &Bytes32,
    ) -> Result<Node, MerkleTreeError<Storage::StorageError>> {
        let storage_node = self
            .storage
            .get(&self.prefix, key)?
            .ok_or(MerkleTreeError::LoadError(*key))?;
        Ok(Node::from_storage(*key, storage_node))
    }

    fn store_node(
        &mut self,
        node: Node,
    ) -> Result<(), MerkleTreeError<Storage::StorageError>> {
        let (key, storage_node) = node.into_storage();
        self.storage.set(&self.prefix, &key, &storage_node)
    }

    fn insert_node(
        &mut self,
        root: Node,
        new_node: Node,
    ) -> Result<Node, MerkleTreeError<Storage::StorageError>> {
        let mut parents = self.iterate_down_and_keep_parents(root, &new_node.key)?;

        // The case when we update the value for the existing node.
        if let Some(Path::Current(node)) = parents.last_mut() {
            node.value = new_node.value;
            return self.update_parents_with_current_node(parents);
        }

        let mut current = new_node;
        self.store_node(current)?;
        while let Some(parent) = parents.pop() {
            match parent {
                Path::Left(parent) => {
                    let mut left_child = mem::replace(&mut current, parent);
                    let r = current.right_height();
                    let l = left_child.height();
                    if l <= r.saturating_add(1) {
                        // We don't need balancing
                        current.set_left_child(left_child);
                        self.store_node(current)?;
                    } else {
                        let left_height = left_child.left_height();
                        let right_height = left_child.right_height();

                        if right_height <= left_height {
                            // Small right rotation
                            current.left_child = left_child.right_child;
                            self.store_node(current)?;
                            left_child.set_right_child(current);
                            current = left_child;
                            self.store_node(current)?;
                        } else {
                            // Big right rotation
                            let mut left_right_child = left_child
                                .right_child
                                .map(|child| self.get_node(&child.key))
                                .expect(
                                    "Right subtree of the left has non zero height",
                                )?;

                            current.left_child = left_right_child.right_child;
                            self.store_node(current)?;
                            left_right_child.set_right_child(current);

                            left_child.right_child = left_right_child.left_child;
                            self.store_node(left_child)?;
                            left_right_child.set_left_child(left_child);

                            current = left_right_child;
                            self.store_node(current)?;
                        }
                    }
                }
                Path::Right(parent) => {
                    let mut right_child = mem::replace(&mut current, parent);
                    let l = current.left_height();
                    let r = right_child.height();
                    if r <= l.saturating_add(1) {
                        // We don't need balancing
                        current.set_right_child(right_child);
                        self.store_node(current)?;
                    } else {
                        let left_height = right_child.left_height();
                        let right_height = right_child.right_height();

                        if left_height <= right_height {
                            // Small left rotation
                            current.right_child = right_child.left_child;
                            self.store_node(current)?;
                            right_child.set_left_child(current);
                            current = right_child;
                            self.store_node(current)?;
                        } else {
                            // Big left rotation
                            let mut right_left_child = right_child
                                .left_child
                                .map(|child| self.get_node(&child.key))
                                .expect(
                                    "Left subtree of the right has non zero height",
                                )?;

                            current.right_child = right_left_child.left_child;
                            self.store_node(current)?;
                            right_left_child.set_left_child(current);

                            right_child.left_child = right_left_child.right_child;
                            self.store_node(right_child)?;
                            right_left_child.set_right_child(right_child);

                            current = right_left_child;
                            self.store_node(current)?;
                        }
                    }
                }
                Path::Current(_) => {
                    panic!("Expected a left or right node in the remaining path")
                }
            }
        }

        Ok(current)
    }

    fn iterate_down_and_keep_parents(
        &self,
        root: Node,
        key: &Bytes32,
    ) -> Result<Vec<Path>, MerkleTreeError<Storage::StorageError>> {
        let mut parents = Vec::with_capacity(32);
        let mut current = Some(root);
        while let Some(node) = current {
            match key.cmp(&node.key) {
                Ordering::Less => {
                    current = node
                        .left_child
                        .as_ref()
                        .map(|child| self.get_node(&child.key))
                        .transpose()?;
                    debug_assert_eq!(
                        current.map(|n| n.hash()),
                        node.left_child.map(|c| c.hash)
                    );
                    debug_assert_eq!(
                        current.map(|n| n.height()),
                        node.left_child.map(|c| c.height)
                    );
                    parents.push(Path::Left(node));
                }
                Ordering::Equal => {
                    parents.push(Path::Current(node));
                    break;
                }
                Ordering::Greater => {
                    current = node
                        .right_child
                        .as_ref()
                        .map(|child| self.get_node(&child.key))
                        .transpose()?;
                    debug_assert_eq!(
                        current.map(|n| n.hash()),
                        node.right_child.map(|c| c.hash)
                    );
                    debug_assert_eq!(
                        current.map(|n| n.height()),
                        node.right_child.map(|c| c.height)
                    );
                    parents.push(Path::Right(node));
                }
            }
        }
        Ok(parents)
    }

    fn update_parents_with_current_node(
        &mut self,
        mut parents: Vec<Path>,
    ) -> Result<Node, MerkleTreeError<Storage::StorageError>> {
        let Some(Path::Current(mut new_root_node)) = parents.pop() else {
            panic!("Expected a current node in the path")
        };

        while let Some(parent) = parents.pop() {
            self.store_node(new_root_node)?;
            match parent {
                Path::Left(parent) => {
                    let left_child = mem::replace(&mut new_root_node, parent);
                    new_root_node
                        .left_child
                        .as_mut()
                        .expect("During backward iteration, all parents and child exists")
                        .hash = left_child.hash();
                }
                Path::Right(parent) => {
                    let right_child = mem::replace(&mut new_root_node, parent);
                    new_root_node
                        .right_child
                        .as_mut()
                        .expect("During backward iteration, all parents and child exists")
                        .hash = right_child.hash();
                }
                Path::Current(_) => {
                    panic!("Expected a left or right node in the remaining path")
                }
            }
        }

        Ok(new_root_node)
    }
}

fn to_bytes_32<V: AsRef<[u8]>>(value: V) -> Bytes32 {
    if value.as_ref().len() == 32 {
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(value.as_ref());
        bytes
    } else {
        use digest::Digest;
        let mut hash = sha2::Sha256::new();
        hash.update(value);
        hash.finalize().into()
    }
}

#[allow(clippy::assign_op_pattern)]
#[allow(clippy::arithmetic_side_effects)]
#[cfg(any(test, feature = "test-helpers"))]
pub mod tests {
    use super::*;
    use hashbrown::HashMap;
    #[cfg(test)]
    use rand::{
        prelude::StdRng,
        Rng,
        SeedableRng,
    };

    #[derive(Default, Debug)]
    pub struct Storage {
        nodes: HashMap<(Bytes32, Bytes32), StorageNode>,
    }

    impl AVLStorage<()> for Storage {
        type StorageError = ();

        fn get(
            &self,
            prefix: &Bytes32,
            key: &Bytes32,
        ) -> Result<Option<StorageNode>, MerkleTreeError<Self::StorageError>> {
            Ok(self.nodes.get(&(*prefix, *key)).cloned())
        }

        fn set(
            &mut self,
            prefix: &Bytes32,
            key: &Bytes32,
            value: &StorageNode,
        ) -> Result<(), MerkleTreeError<Self::StorageError>> {
            self.nodes.insert((*prefix, *key), *value);
            Ok(())
        }
    }

    #[test]
    fn insert_random_values_works() {
        let mut rng = StdRng::seed_from_u64(1234);
        let mut tree = AVLMerkleTree::<Storage, ()>::default();

        for _ in 0..10000 {
            tree.insert::<Bytes32>(rng.gen(), rng.gen()).unwrap();
        }

        assert_ne!(tree.root(), *zero_sum());
    }

    #[test]
    fn insert_same_values_keep_the_root() {
        let mut rng = StdRng::seed_from_u64(1234);
        let mut tree = AVLMerkleTree::<Storage, ()>::default();

        let data = (0..10000)
            .map(|_| (rng.gen(), rng.gen()))
            .collect::<Vec<(Bytes32, Bytes32)>>();

        for (key, value) in data.clone() {
            tree.insert(key, value).unwrap();
        }

        // Given
        let expected_root = tree.root();

        // When
        for (key, value) in data {
            tree.insert(key, value).unwrap();
        }

        // Then
        let actual_root = tree.root();
        assert_eq!(actual_root, expected_root);
    }

    #[cfg(test)]
    mod proptest {
        use super::*;
        use ::proptest::{
            collection::vec,
            prelude::{
                any,
                Strategy,
            },
            prop_assert,
            prop_compose,
            proptest,
        };
        use core::fmt::{
            Debug,
            Formatter,
        };
        use hashbrown::HashSet;

        fn validate_tree(
            tree: &AVLMerkleTree<Storage, ()>,
            expected_number_of_node: usize,
        ) -> bool {
            let Some(root) = tree.root_node() else {
                return true
            };

            let prefix = tree.prefix();
            let storage = tree.storage();

            fn validate_tree_recursion(
                prefix: &Bytes32,
                storage: &Storage,
                node: &Node,
                actual_number_of_node: &mut usize,
            ) -> bool {
                let left = if let Some(left_child) = node.left_child {
                    let left_storage_node = storage
                        .get(prefix, &left_child.key)
                        .unwrap()
                        .expect("Left child not found");
                    let left_node = Node::from_storage(left_child.key, left_storage_node);

                    left_node.key < node.key
                        && node.left_hash() == &left_node.hash()
                        && node.left_height() == left_node.height()
                        && validate_tree_recursion(
                            prefix,
                            storage,
                            &left_node,
                            actual_number_of_node,
                        )
                } else {
                    node.left_hash() == zero_sum() && node.left_height() == 0
                };
                let right = if let Some(right_child) = node.right_child {
                    let right_storage_node = storage
                        .get(prefix, &right_child.key)
                        .unwrap()
                        .expect("Right child not found");
                    let right_node =
                        Node::from_storage(right_child.key, right_storage_node);

                    right_node.key > node.key
                        && node.right_hash() == &right_node.hash()
                        && node.right_height() == right_node.height()
                        && validate_tree_recursion(
                            prefix,
                            storage,
                            &right_node,
                            actual_number_of_node,
                        )
                } else {
                    node.right_hash() == zero_sum() && node.right_height() == 0
                };
                *actual_number_of_node = *actual_number_of_node + 1;
                left && right
                    && ((node.left_height() as i32) - (node.right_height() as i32)).abs()
                        <= 1
            }

            let mut actual_number_of_node = 0;
            let result = validate_tree_recursion(
                &prefix,
                storage,
                root,
                &mut actual_number_of_node,
            );
            assert_eq!(actual_number_of_node, expected_number_of_node);

            result
        }

        #[derive(Copy, Clone, Eq, PartialEq, proptest_derive::Arbitrary)]
        struct Value(Bytes32);

        impl Debug for Value {
            fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                f.write_str(&format!("Value({})", hex::encode(self.0)))
            }
        }

        impl AsRef<[u8]> for Value {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl From<Value> for Bytes32 {
            fn from(value: Value) -> Self {
                value.0
            }
        }

        fn _values(n: usize) -> impl Strategy<Value = Vec<(Value, Value)>> {
            vec(any::<(Value, Value)>(), n)
        }

        prop_compose! {
            fn values(min: usize, max: usize)(n in min..max)(v in _values(n)) -> Vec<(Value, Value)> {
                v.into_iter().collect::<Vec<_>>()
            }
        }

        prop_compose! {
            fn random_tree(min: usize, max: usize)(values in values(min, max)) -> (Vec<(Value, Value)>, AVLMerkleTree<Storage, ()>) {
                let mut tree = AVLMerkleTree::<Storage, ()>::default();
                for (key, value) in values.clone() {
                    tree.insert(key.0, value.0).unwrap();
                }
                (values, tree)
            }
        }

        proptest! {
            #[test]
            fn generated_tree_is_valid((values, tree) in random_tree(1, 1_000)){
                let number_of_unique_keys = values
                    .into_iter()
                    .map(|(key, _)| key.0)
                    .collect::<HashSet<_>>()
                    .len();

                // Given
                let tree = tree;

                // When
                let result = validate_tree(&tree, number_of_unique_keys);

                // Then
                prop_assert!(result)
            }
        }
    }
}
