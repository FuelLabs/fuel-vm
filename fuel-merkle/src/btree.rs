use crate::{
    btree::{
        internal_node::{
            Edge,
            InternalInsertionResult,
            InternalNode,
            LookupResult,
            StorageInternalNode,
        },
        leaf_node::{
            LeafInsertionResult,
            LeafNode,
            LeafSubNode,
            StorageLeafNode,
        },
    },
    common::Bytes32,
};

pub mod internal_node;
pub mod leaf_node;

pub const fn empty_child() -> &'static Bytes32 {
    const EMPTY_CHILD_HASH: [u8; 32] = [0; 32];
    &EMPTY_CHILD_HASH
}

pub type Key = Bytes32;
pub type ChildKey = Bytes32;
pub type HashedValue = Bytes32;
pub type ChildHash = Bytes32;

#[derive(Debug, Clone, derive_more::Display, PartialEq, Eq)]
pub enum MerkleTreeError<StorageError> {
    #[display(fmt = "cannot load node with key {_0:?}; the key is not found in storage")]
    LoadError(Bytes32),

    #[display(fmt = "the storage node is incompatible with the expected node type")]
    IncompatibleStorageNode,

    #[display(fmt = "{}", _0)]
    StorageError(StorageError),
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StorageNode {
    Leaf(StorageLeafNode),
    Internal(StorageInternalNode),
}

impl Default for StorageNode {
    fn default() -> Self {
        Self::Leaf(Default::default())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Node<const N: u8> {
    Leaf(LeafNode<N>),
    Internal(InternalNode<N>),
}

impl<const N: u8> Node<N> {
    pub fn hash(&mut self) -> Bytes32 {
        match self {
            Node::Leaf(leaf) => leaf.hash(),
            Node::Internal(internal) => internal.hash(),
        }
    }

    pub fn node_key(&self) -> &Key {
        match self {
            Node::Leaf(leaf) => leaf.node_key(),
            Node::Internal(internal) => internal.node_key(),
        }
    }
}

pub trait BTreeStorage<Table> {
    type StorageError;

    fn take(
        &mut self,
        prefix: &Bytes32,
        key: &Bytes32,
    ) -> Result<Option<StorageNode>, MerkleTreeError<Self::StorageError>>;

    fn set(
        &mut self,
        prefix: &Bytes32,
        key: &Bytes32,
        value: StorageNode,
    ) -> Result<(), MerkleTreeError<Self::StorageError>>;
}

impl<Table, S> BTreeStorage<Table> for &mut S
where
    S: BTreeStorage<Table>,
{
    type StorageError = S::StorageError;

    fn take(
        &mut self,
        prefix: &Bytes32,
        key: &Bytes32,
    ) -> Result<Option<StorageNode>, MerkleTreeError<Self::StorageError>> {
        <S as BTreeStorage<Table>>::take(self, prefix, key)
    }

    fn set(
        &mut self,
        prefix: &Bytes32,
        key: &Bytes32,
        value: StorageNode,
    ) -> Result<(), MerkleTreeError<Self::StorageError>> {
        <S as BTreeStorage<Table>>::set(self, prefix, key, value)
    }
}

#[derive(Default, Debug)]
pub struct BTreeMerkleTree<const N: u8, Storage, Table> {
    prefix: Bytes32,
    root: Option<Node<N>>,
    storage: Storage,
    _marker: core::marker::PhantomData<Table>,
}

impl<const N: u8, Storage, Table> BTreeMerkleTree<N, Storage, Table> {
    pub fn load(prefix: Bytes32, root: Option<Node<N>>, storage: Storage) -> Self {
        Self {
            prefix,
            root,
            storage,
            _marker: core::marker::PhantomData,
        }
    }

    pub fn root(&mut self) -> Bytes32 {
        self.root
            .as_mut()
            .map(|r| r.hash())
            .unwrap_or(*empty_child())
    }

    pub fn into_storage(self) -> Storage {
        self.storage
    }

    pub fn into_root(self) -> Option<Node<N>> {
        self.root
    }

    pub fn into_inner(self) -> (Option<Node<N>>, Storage) {
        (self.root, self.storage)
    }
}

impl<const N: u8, Storage, Table> BTreeMerkleTree<N, Storage, Table>
where
    Storage: BTreeStorage<Table>,
{
    pub fn take_child(
        &mut self,
        key: &Key,
    ) -> Result<Node<N>, MerkleTreeError<Storage::StorageError>> {
        let storage_node = self
            .storage
            .take(&self.prefix, key)?
            .ok_or(MerkleTreeError::LoadError(*key))?;

        match storage_node {
            StorageNode::Leaf(leaf) => LeafNode::from_storage(*key, leaf.sub_nodes)
                .ok_or(MerkleTreeError::IncompatibleStorageNode)
                .map(Node::Leaf),
            StorageNode::Internal(internal) => InternalNode::from_storage(*key, internal)
                .ok_or(MerkleTreeError::IncompatibleStorageNode)
                .map(Node::Internal),
        }
    }

    pub fn set_child(
        &mut self,
        key: &Key,
        node: Node<N>,
    ) -> Result<(), MerkleTreeError<Storage::StorageError>> {
        let storage_node = match node {
            Node::Leaf(leaf) => StorageNode::Leaf(leaf.into_storage_node()),
            Node::Internal(internal) => {
                StorageNode::Internal(internal.into_storage_node())
            }
        };

        self.storage.set(&self.prefix, key, storage_node)
    }
}

impl<const N: u8, Storage, Table> BTreeMerkleTree<N, Storage, Table>
where
    Storage: BTreeStorage<Table>,
{
    pub fn insert<V>(
        &mut self,
        key: Key,
        value: V,
    ) -> Result<(), MerkleTreeError<Storage::StorageError>>
    where
        V: AsRef<[u8]>,
    {
        use digest::Digest;
        let mut hash = sha2::Sha256::new();
        hash.update(value);
        self.update_inner(key, hash.finalize().into())
    }

    #[cfg(feature = "test-helpers")]
    pub fn insert_test(
        &mut self,
        key: Key,
        value: HashedValue,
    ) -> Result<(), MerkleTreeError<Storage::StorageError>> {
        self.update_inner(key, value)
    }

    pub fn remove(
        &mut self,
        key: Key,
    ) -> Result<(), MerkleTreeError<Storage::StorageError>> {
        self.update_inner(key, [0; 32])
    }

    fn update_inner(
        &mut self,
        key: Key,
        value: HashedValue,
    ) -> Result<(), MerkleTreeError<Storage::StorageError>> {
        let leaf = LeafSubNode { key, value };
        let new_root = if let Some(root) = self.root.take() {
            self.new_root(root, leaf)?
        } else {
            Node::Leaf(LeafNode::<N>::new_root(leaf))
        };

        self.root = Some(new_root);

        Ok(())
    }

    fn new_root(
        &mut self,
        root: Node<N>,
        leaf: LeafSubNode,
    ) -> Result<Node<N>, MerkleTreeError<Storage::StorageError>> {
        let insertion_result = self.update_leaf_recursion(root, leaf)?;

        let result = match insertion_result {
            InsertionResult::Leaf(leaf_result) => match leaf_result {
                LeafInsertionResult::Added(new_root)
                | LeafInsertionResult::Updated(new_root) => Node::Leaf(new_root),
                LeafInsertionResult::Overflowed {
                    new_left_leaf,
                    orphan_sub_node,
                    new_right_leaf,
                } => {
                    let left_edge = Edge {
                        key: *new_left_leaf.node_key(),
                        hash: new_left_leaf.hash(),
                    };
                    let right_edge = Edge {
                        key: *new_right_leaf.node_key(),
                        hash: new_right_leaf.hash(),
                    };

                    self.set_child(&left_edge.key, Node::Leaf(new_left_leaf))?;
                    self.set_child(&right_edge.key, Node::Leaf(new_right_leaf))?;
                    let new_root =
                        InternalNode::new_root(left_edge, orphan_sub_node, right_edge);
                    Node::Internal(new_root)
                }
            },
            InsertionResult::Internal(internal_result) => match internal_result {
                InternalInsertionResult::Added { updated_node } => {
                    let new_root = updated_node;
                    Node::Internal(new_root)
                }
                InternalInsertionResult::Updated(new_root) => Node::Internal(new_root),
                InternalInsertionResult::Overflowed {
                    mut new_left_node,
                    orphan_sub_node,
                    mut new_right_node,
                } => {
                    let left_edge = Edge {
                        key: *new_left_node.node_key(),
                        hash: new_left_node.hash(),
                    };
                    let right_edge = Edge {
                        key: *new_right_node.node_key(),
                        hash: new_right_node.hash(),
                    };

                    self.set_child(&left_edge.key, Node::Internal(new_left_node))?;
                    self.set_child(&right_edge.key, Node::Internal(new_right_node))?;
                    let new_root =
                        InternalNode::new_root(left_edge, orphan_sub_node, right_edge);
                    Node::Internal(new_root)
                }
            },
        };

        Ok(result)
    }

    fn update_leaf_recursion(
        &mut self,
        current: Node<N>,
        new_leaf: LeafSubNode,
    ) -> Result<InsertionResult<N>, MerkleTreeError<Storage::StorageError>> {
        let result = match current {
            Node::Leaf(leaf_node) => {
                InsertionResult::Leaf(leaf_node.insert_leaf(new_leaf))
            }
            Node::Internal(mut internal_node) => {
                let lookup = internal_node.lookup(&new_leaf.key);

                match lookup {
                    LookupResult::Found(leaf) => {
                        leaf.value = new_leaf.value;
                        InsertionResult::Internal(InternalInsertionResult::Updated(
                            internal_node,
                        ))
                    }
                    LookupResult::NoFound(edge) => {
                        let current = self.take_child(&edge.key)?;
                        let result = self.update_leaf_recursion(current, new_leaf)?;

                        match result {
                            InsertionResult::Leaf(result) => match result {
                                LeafInsertionResult::Added(updated_leaf)
                                | LeafInsertionResult::Updated(updated_leaf) => {
                                    edge.hash = updated_leaf.hash();

                                    debug_assert_eq!(edge.key, *updated_leaf.node_key());
                                    self.set_child(&edge.key, Node::Leaf(updated_leaf))?;

                                    InsertionResult::Internal(
                                        InternalInsertionResult::Updated(internal_node),
                                    )
                                }
                                LeafInsertionResult::Overflowed {
                                    new_left_leaf,
                                    orphan_sub_node,
                                    new_right_leaf,
                                } => {
                                    let left_edge = Edge {
                                        key: *new_left_leaf.node_key(),
                                        hash: new_left_leaf.hash(),
                                    };
                                    let right_edge = Edge {
                                        key: *new_right_leaf.node_key(),
                                        hash: new_right_leaf.hash(),
                                    };

                                    self.set_child(
                                        &left_edge.key,
                                        Node::Leaf(new_left_leaf),
                                    )?;
                                    self.set_child(
                                        &right_edge.key,
                                        Node::Leaf(new_right_leaf),
                                    )?;

                                    InsertionResult::Internal(internal_node.insert_edge(
                                        left_edge,
                                        orphan_sub_node,
                                        right_edge,
                                    ))
                                }
                            },
                            InsertionResult::Internal(child_internal_node) => {
                                match child_internal_node {
                                    InternalInsertionResult::Added {
                                        mut updated_node,
                                    } => {
                                        edge.hash = updated_node.hash();

                                        debug_assert_eq!(
                                            edge.key,
                                            *updated_node.node_key()
                                        );
                                        self.set_child(
                                            &edge.key,
                                            Node::Internal(updated_node),
                                        )?;

                                        InsertionResult::Internal(
                                            InternalInsertionResult::Updated(
                                                internal_node,
                                            ),
                                        )
                                    }
                                    InternalInsertionResult::Updated(
                                        mut updated_node,
                                    ) => {
                                        edge.hash = updated_node.hash();

                                        debug_assert_eq!(
                                            edge.key,
                                            *updated_node.node_key()
                                        );
                                        self.set_child(
                                            &edge.key,
                                            Node::Internal(updated_node),
                                        )?;

                                        InsertionResult::Internal(
                                            InternalInsertionResult::Updated(
                                                internal_node,
                                            ),
                                        )
                                    }
                                    InternalInsertionResult::Overflowed {
                                        mut new_left_node,
                                        orphan_sub_node,
                                        mut new_right_node,
                                    } => {
                                        let left_edge = Edge {
                                            key: *new_left_node.node_key(),
                                            hash: new_left_node.hash(),
                                        };
                                        let right_edge = Edge {
                                            key: *new_right_node.node_key(),
                                            hash: new_right_node.hash(),
                                        };

                                        self.set_child(
                                            &left_edge.key,
                                            Node::Internal(new_left_node),
                                        )?;
                                        self.set_child(
                                            &right_edge.key,
                                            Node::Internal(new_right_node),
                                        )?;

                                        InsertionResult::Internal(
                                            internal_node.insert_edge(
                                                left_edge,
                                                orphan_sub_node,
                                                right_edge,
                                            ),
                                        )
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        Ok(result)
    }
}

enum InsertionResult<const N: u8> {
    Leaf(LeafInsertionResult<N>),
    Internal(InternalInsertionResult<N>),
}

pub(crate) fn find_index_of_insertion(
    sub_nodes: &[LeafSubNode],
    key_to_insert: &Key,
) -> Result<usize, usize> {
    if sub_nodes.is_empty() {
        return Err(0);
    }

    if let Some(first) = sub_nodes.first() {
        if key_to_insert < &first.key {
            return Err(0);
        }
    }

    if let Some(last) = sub_nodes.last() {
        if key_to_insert > &last.key {
            return Err(sub_nodes.len());
        }
    }

    sub_nodes.binary_search_by(|sub_node| sub_node.key.cmp(key_to_insert))
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

    impl BTreeStorage<()> for Storage {
        type StorageError = ();

        fn take(
            &mut self,
            prefix: &Bytes32,
            key: &Bytes32,
        ) -> Result<Option<StorageNode>, MerkleTreeError<Self::StorageError>> {
            let old = self.nodes.remove(&(*prefix, *key));
            assert!(old.is_some());
            Ok(old)
        }

        fn set(
            &mut self,
            prefix: &Bytes32,
            key: &Bytes32,
            value: StorageNode,
        ) -> Result<(), MerkleTreeError<Self::StorageError>> {
            let old = self.nodes.insert((*prefix, *key), value);
            assert!(old.is_none());
            Ok(())
        }
    }

    #[test]
    fn insert_random_values_works() {
        let mut rng = StdRng::seed_from_u64(1234);
        let mut tree = BTreeMerkleTree::<4, Storage, ()>::default();

        for _ in 0..10000 {
            tree.insert::<Bytes32>(rng.gen(), rng.gen()).unwrap();
        }

        assert_ne!(tree.root(), *empty_child());
    }

    #[test]
    fn insert_random_values_works_each_insert_updates_root() {
        let mut rng = StdRng::seed_from_u64(1234);
        let mut tree = BTreeMerkleTree::<4, Storage, ()>::default();

        let mut old_root = tree.root();
        for _ in 0..10000 {
            // When
            tree.insert::<Bytes32>(rng.gen(), rng.gen()).unwrap();

            // Then
            let new_root = tree.root();
            assert_ne!(new_root, old_root);

            // Given
            old_root = new_root;
        }
    }

    #[test]
    fn updating_with_same_value_doesnt_affect_the_root() {
        let mut rng = StdRng::seed_from_u64(1234);
        let mut tree = BTreeMerkleTree::<4, Storage, ()>::default();

        for _ in 0..10000 {
            tree.insert::<Bytes32>(rng.gen(), rng.gen()).unwrap();
        }
        // Given
        let mut rng = StdRng::seed_from_u64(1234);
        let old_root = tree.root();

        for _ in 0..10000 {
            // When
            tree.insert::<Bytes32>(rng.gen(), rng.gen()).unwrap();

            // Then
            let new_root = tree.root();
            assert_eq!(new_root, old_root);
        }
    }
}
