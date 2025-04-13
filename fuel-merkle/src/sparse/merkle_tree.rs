mod branch;
mod node;

use branch::{
    merge_branches,
    Branch,
};
use node::{
    Node,
    StorageNode,
    StorageNodeError,
};

use crate::{
    common::{
        error::DeserializeError,
        node::ChildError,
        AsPathIterator,
        Bytes32,
    },
    sparse::{
        empty_sum,
        proof::{
            ExclusionLeaf,
            ExclusionLeafData,
            ExclusionProof,
            InclusionProof,
            Proof,
        },
        Primitive,
    },
    storage::{
        Mappable,
        StorageInspect,
        StorageMutate,
    },
};
use alloc::{
    format,
    vec::Vec,
};
use core::{
    fmt::{
        Debug,
        Formatter,
    },
    iter,
    marker::PhantomData,
    ops::Deref,
};

#[derive(Debug, Clone, derive_more::Display)]
pub enum MerkleTreeError<StorageError> {
    #[display(
        fmt = "cannot load node with key {}; the key is not found in storage",
        "hex::encode(_0)"
    )]
    LoadError(Bytes32),

    #[display(fmt = "{}", _0)]
    StorageError(StorageError),

    #[display(fmt = "{}", _0)]
    DeserializeError(DeserializeError),

    #[display(fmt = "{}", _0)]
    ChildError(ChildError<Bytes32, StorageNodeError<StorageError>>),
}

impl<StorageError> From<StorageError> for MerkleTreeError<StorageError> {
    fn from(err: StorageError) -> MerkleTreeError<StorageError> {
        MerkleTreeError::StorageError(err)
    }
}

/// The safe Merkle tree storage key prevents Merkle tree structure manipulations.
/// The type contains only one constructor that hashes the storage key.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub struct MerkleTreeKey(Bytes32);

impl MerkleTreeKey {
    /// The safe way to create a `Self`. It hashes the `storage_key`, making
    /// it entirely random and preventing SMT structure manipulation.
    pub fn new<B>(storage_key: B) -> Self
    where
        B: AsRef<[u8]>,
    {
        use digest::Digest;
        let mut hash = sha2::Sha256::new();
        hash.update(storage_key.as_ref());
        let hash = hash.finalize().into();

        Self(hash)
    }

    /// Unsafe analog to create a `Self` that doesn't hash the `storage_key` unlike
    /// `Self::new`.
    ///
    /// # Safety
    ///
    /// It is safe to use this method if you know that `storage_key`
    /// was randomly generated like `ContractId` or `AssetId`.
    pub unsafe fn convert<B>(storage_key: B) -> Self
    where
        B: Into<Bytes32>,
    {
        Self(storage_key.into())
    }

    #[cfg(any(test, feature = "test-helpers"))]
    pub fn new_without_hash<B>(storage_key: B) -> Self
    where
        B: Into<Bytes32>,
    {
        unsafe { Self::convert(storage_key) }
    }
}

impl Debug for MerkleTreeKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&format!("MerkleTreeKey({})", hex::encode(self.0)))
    }
}

impl From<MerkleTreeKey> for Bytes32 {
    fn from(value: MerkleTreeKey) -> Self {
        value.0
    }
}

impl AsRef<[u8]> for MerkleTreeKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl AsRef<Bytes32> for MerkleTreeKey {
    fn as_ref(&self) -> &Bytes32 {
        &self.0
    }
}

impl Deref for MerkleTreeKey {
    type Target = Bytes32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(any(test, feature = "test-helpers"))]
impl From<Bytes32> for MerkleTreeKey {
    fn from(value: Bytes32) -> Self {
        Self::new_without_hash(value)
    }
}

#[derive(Debug)]
pub struct MerkleTree<TableType, StorageType> {
    root_node: Node,
    storage: StorageType,
    phantom_table: PhantomData<TableType>,
}

impl<TableType, StorageType> MerkleTree<TableType, StorageType> {
    pub const fn empty_root() -> &'static Bytes32 {
        empty_sum()
    }

    pub fn root(&self) -> Bytes32 {
        *self.root_node().hash()
    }

    pub fn into_storage(self) -> StorageType {
        self.storage
    }

    pub fn storage(&self) -> &StorageType {
        &self.storage
    }

    fn root_node(&self) -> &Node {
        &self.root_node
    }

    fn set_root_node(&mut self, node: Node) {
        debug_assert!(node.is_leaf() || node.height() == Node::max_height());
        self.root_node = node;
    }
}

impl<TableType, StorageType, StorageError> MerkleTree<TableType, StorageType>
where
    TableType: Mappable<Key = Bytes32, Value = Primitive, OwnedValue = Primitive>,
    StorageType: StorageInspect<TableType, Error = StorageError>,
{
    pub fn new(storage: StorageType) -> Self {
        Self {
            root_node: Node::create_placeholder(),
            storage,
            phantom_table: Default::default(),
        }
    }

    pub fn load(
        storage: StorageType,
        root: &Bytes32,
    ) -> Result<Self, MerkleTreeError<StorageError>> {
        if root == Self::empty_root() {
            let tree = Self::new(storage);
            Ok(tree)
        } else {
            let primitive = storage
                .get(root)?
                .ok_or_else(|| MerkleTreeError::LoadError(*root))?
                .into_owned();
            let tree = Self {
                root_node: primitive
                    .try_into()
                    .map_err(MerkleTreeError::DeserializeError)?,
                storage,
                phantom_table: Default::default(),
            };
            Ok(tree)
        }
    }

    fn path_set(
        &self,
        leaf_key: &Bytes32,
    ) -> Result<(Vec<Node>, Vec<Bytes32>), MerkleTreeError<StorageError>> {
        let root_node = self.root_node().clone();
        let root_storage_node = StorageNode::new(&self.storage, root_node);
        let (mut path_nodes, mut side_nodes): (Vec<Node>, Vec<Bytes32>) =
            root_storage_node
                .as_path_iter(leaf_key)
                .map(|(path_node, side_node)| {
                    Ok((
                        path_node.map_err(MerkleTreeError::ChildError)?.into_node(),
                        side_node.map_err(MerkleTreeError::ChildError)?,
                    ))
                })
                .collect::<Result<Vec<_>, MerkleTreeError<StorageError>>>()?
                .into_iter()
                .unzip();
        path_nodes.reverse();
        side_nodes.reverse();
        side_nodes.pop(); // The last element in the side nodes list is the
                          // root; remove it.

        Ok((path_nodes, side_nodes))
    }
}

impl<TableType, StorageType, StorageError> MerkleTree<TableType, StorageType>
where
    TableType: Mappable<Key = Bytes32, Value = Primitive, OwnedValue = Primitive>,
    StorageType: StorageMutate<TableType, Error = StorageError>,
{
    /// Build a sparse Merkle tree from a set of key-value pairs. This is
    /// equivalent to creating an empty sparse Merkle tree and sequentially
    /// calling [update](Self::update) for each key-value pair. This constructor
    /// is more performant than calling individual sequential updates and is the
    /// preferred approach when the key-values are known upfront. Leaves can be
    /// appended to the returned tree using `update` to further accumulate leaf
    /// data.
    pub fn from_set<B, I, D>(
        mut storage: StorageType,
        set: I,
    ) -> Result<Self, StorageError>
    where
        I: Iterator<Item = (B, D)>,
        B: Into<Bytes32>,
        D: AsRef<[u8]>,
    {
        let sorted = set
            .into_iter()
            .map(|(k, v)| (k.into(), v))
            .collect::<alloc::collections::BTreeMap<Bytes32, D>>();
        let mut branches = sorted
            .iter()
            .map(|(key, data)| Node::create_leaf(key, data))
            .map(Into::<Branch>::into)
            .collect::<Vec<_>>();

        for branch in branches.iter() {
            let leaf = &branch.node;
            storage.insert(leaf.hash(), &leaf.as_ref().into())?;
        }

        if branches.is_empty() {
            let tree = Self::new(storage);
            return Ok(tree)
        }

        if branches.len() == 1 {
            let leaf = branches.pop().expect("Expected at least 1 leaf").node;
            let mut tree = Self::new(storage);
            tree.set_root_node(leaf);
            return Ok(tree)
        }

        let mut nodes = Vec::<Branch>::with_capacity(branches.len());
        let mut proximities = Vec::<u32>::with_capacity(branches.len());

        // Building the tree starts by merging all leaf nodes where possible.
        // Given a set of leaf nodes sorted left to right (i.e., keys are sorted
        // in lexical order), we scan the leaf set right to left, and analyze a
        // moving window of three leaves: a center (or "current") leaf, its left
        // neighbor, and its right neighbor.
        //
        // When merging leaf nodes, we analyze this three-node window to
        // determine if the condition for merging is met: When the current node
        // is closer to its right neighbor than it is to its left neighbor, we
        // merge the current node with its right neighbor. The merged node then
        // becomes the center of the window, and we must check the merge
        // condition again. We calculate proximity using the common path length
        // between two nodes, which is also the depth of their shared ancestor
        // in the tree.
        //
        // This three-node window is centered around a current node, and moves
        // leftward: At the next iteration, the current node is now the right
        // node, the left node is now the current node, and so on. When we have
        // checked all windows, we know that we have merged all leaf nodes where
        // possible.
        while let Some(left) = branches.pop() {
            if let Some(current) = nodes.last() {
                #[allow(clippy::cast_possible_truncation)] // Key is 32 bytes
                let left_proximity = current.node.common_path_length(&left.node) as u32;
                while {
                    // The current node's proximity to its right neighbor was
                    // stored previously. We now compare the distances between
                    // the current node's left and right neighbors. If, and only
                    // if, the current node is closer to its right neighbor, we
                    // merge these nodes to form an ancestor node. We then
                    // reform the window, using the ancestor node in the center,
                    // to check if we must merge again.
                    //
                    // If the current node is closer to its left, we do not have
                    // enough information to merge nodes, and we must continue
                    // scanning the leaf set leftwards to find a configuration
                    // that satisfies the merge condition.
                    if let Some(right_proximity) = proximities.last() {
                        *right_proximity > left_proximity
                    } else {
                        false
                    }
                } {
                    // The current node is closer to its right neighbor than its
                    // left neighbor. We now merge the current node with its
                    // right neighbor.
                    let current =
                        nodes.pop().expect("Expected current node to be present");
                    let right = nodes.pop().expect("Expected right node to be present");
                    let merged = merge_branches(&mut storage, current, right)?;
                    nodes.push(merged);

                    // Now that the current node and its right neighbour are
                    // merged, the distance between them has collapsed and their
                    // proximity is no longer needed.
                    proximities.pop();
                }
                proximities.push(left_proximity);
            }
            nodes.push(left);
        }

        // Where possible, all the leaves have been merged. The remaining leaves
        // and nodes are stacked in order of height descending. This means that
        // they are also ordered with the leftmost leaves at the top and the
        // rightmost nodes at the bottom. We can iterate through the stack and
        // merge them left to right.
        let top = {
            let mut node = nodes
                .pop()
                .expect("Nodes stack must have at least 1 element");
            while let Some(next) = nodes.pop() {
                node = merge_branches(&mut storage, node, next)?;
            }
            node
        };

        // Lastly, all leaves and nodes are merged into one. The resulting node
        // may still be an ancestor node below the root. To calculate the final
        // root, we merge placeholder nodes along the path until the resulting
        // node has the final height and forms the root node.
        let mut node = top.node;
        let path = top.bits;
        let height = node.height();
        #[allow(clippy::arithmetic_side_effects)] // height <= max_height
        let depth = Node::max_height() - height;
        let placeholders = iter::repeat(Node::create_placeholder()).take(depth as usize);
        for placeholder in placeholders {
            node = Node::create_node_on_path(&path, &node, &placeholder);
            storage.insert(node.hash(), &node.as_ref().into())?;
        }

        let tree = Self {
            root_node: node,
            storage,
            phantom_table: Default::default(),
        };
        Ok(tree)
    }

    pub fn update(
        &mut self,
        key: MerkleTreeKey,
        data: &[u8],
    ) -> Result<(), MerkleTreeError<StorageError>> {
        let leaf_node = Node::create_leaf(key.as_ref(), data);
        self.storage
            .insert(leaf_node.hash(), &leaf_node.as_ref().into())?;

        if self.root_node().is_placeholder() {
            self.set_root_node(leaf_node);
        } else {
            let (path_nodes, side_nodes) = self.path_set(key.as_ref())?;
            self.update_with_path_set(
                &leaf_node,
                path_nodes.as_slice(),
                side_nodes.as_slice(),
            )?;
        }

        Ok(())
    }

    pub fn delete(
        &mut self,
        key: MerkleTreeKey,
    ) -> Result<(), MerkleTreeError<StorageError>> {
        if self.root() == *Self::empty_root() {
            // The zero root signifies that all leaves are empty, including the
            // given key.
            return Ok(())
        }

        let (path_nodes, side_nodes): (Vec<Node>, Vec<_>) =
            self.path_set(key.as_ref())?;

        match path_nodes.first() {
            Some(node) if *node.leaf_key() == key.as_ref() => {
                self.delete_with_path_set(path_nodes.as_slice(), side_nodes.as_slice())?;
            }
            _ => {}
        };

        Ok(())
    }

    fn update_with_path_set(
        &mut self,
        requested_leaf_node: &Node,
        path_nodes: &[Node],
        side_nodes: &[Bytes32],
    ) -> Result<(), StorageError> {
        let path = requested_leaf_node.leaf_key();
        let actual_leaf_node = &path_nodes[0];

        if requested_leaf_node == actual_leaf_node {
            return Ok(())
        }

        // Build the tree upwards starting with the requested leaf node.
        let mut current_node = requested_leaf_node.clone();

        // If we are creating a new leaf node, the corresponding side node will
        // be the first node in the path set. The side node will be the leaf
        // node currently closest to the requested new leaf node. When creating
        // a new leaf node, we must merge the leaf node with its corresponding
        // side node to create a common ancestor. We then continue building the
        // tree upwards from this ancestor node. This may require creating new
        // placeholder side nodes, in addition to the existing side node set.
        //
        // If we are updating an existing leaf node, the leaf node we are
        // updating is the first node in the path set. The side node set will
        // already include all the side nodes needed to build up the tree from
        // the requested leaf node, since these side nodes were already built
        // during the creation of the leaf node.
        //
        // We can determine if we are updating an existing leaf node, or if we
        // are creating a new leaf node, by comparing the paths of the requested
        // leaf node and the leaf node at the start of the path set. When the
        // paths are equal, it means the leaf nodes occupy the same location,
        // and we are updating an existing leaf. Otherwise, it means we are
        // adding a new leaf node.
        if requested_leaf_node.leaf_key() != actual_leaf_node.leaf_key() {
            // Merge leaves
            if !actual_leaf_node.is_placeholder() {
                current_node =
                    Node::create_node_on_path(path, &current_node, actual_leaf_node);
                self.storage
                    .insert(current_node.hash(), &current_node.as_ref().into())?;
            }

            // Merge placeholders
            let ancestor_depth = requested_leaf_node.common_path_length(actual_leaf_node);
            #[allow(clippy::cast_possible_truncation)] // Key is 32 bytes
            let placeholders_count =
                (ancestor_depth as usize).saturating_sub(side_nodes.len());
            let placeholders =
                iter::repeat(Node::create_placeholder()).take(placeholders_count);
            for placeholder in placeholders {
                current_node =
                    Node::create_node_on_path(path, &current_node, &placeholder);
                self.storage
                    .insert(current_node.hash(), &current_node.as_ref().into())?;
            }
        } else {
            self.storage.remove(actual_leaf_node.hash())?;
        }

        // Merge side nodes
        for (side_node, old_parent) in
            side_nodes.iter().zip(path_nodes.iter().skip(1 /* leaf */))
        {
            let new_parent = if old_parent.bytes_lo() == side_node {
                Node::create_node_from_hashes(
                    *side_node,
                    *current_node.hash(),
                    old_parent.height(),
                )
            } else {
                Node::create_node_from_hashes(
                    *current_node.hash(),
                    *side_node,
                    old_parent.height(),
                )
            };

            current_node = new_parent;
            self.storage
                .insert(current_node.hash(), &current_node.as_ref().into())?;
            self.storage.remove(old_parent.hash())?;
        }

        self.set_root_node(current_node);

        Ok(())
    }

    fn delete_with_path_set(
        &mut self,
        path_nodes: &[Node],
        side_nodes: &[Bytes32],
    ) -> Result<(), MerkleTreeError<StorageError>> {
        for node in path_nodes {
            self.storage.remove(node.hash())?;
        }

        let mut side_nodes_iter = side_nodes.iter();
        let mut path_nodes_iter = path_nodes.iter();
        path_nodes_iter.next(); // Skip the requested leaf node

        // The deleted leaf is replaced by a placeholder. Build the tree upwards
        // starting with the placeholder.
        let mut current_node = Node::create_placeholder();

        // If the first side node is a leaf, it means the ancestor node is now
        // parent to a placeholder (the deleted leaf node) and a leaf node (the
        // first side node). We can immediately discard the ancestor node from
        // further calculation and attach the orphaned leaf node to its next
        // ancestor. Any subsequent ancestor nodes composed of this leaf node
        // and a placeholder must be similarly discarded from further
        // calculation. We then create a valid ancestor node for the orphaned
        // leaf node by joining it with the earliest non-placeholder side node.
        if let Some(first_side_node) = side_nodes.first() {
            let first_side_node: Node = self
                .storage
                .get(first_side_node)?
                .ok_or(MerkleTreeError::LoadError(*first_side_node))?
                .into_owned()
                .try_into()
                .map_err(MerkleTreeError::DeserializeError)?;

            if first_side_node.is_leaf() {
                side_nodes_iter.next();
                current_node = first_side_node.clone();

                // Advance the side node iterator to the next non-placeholder
                // node. This may be either another leaf node or an internal
                // node. If only placeholder nodes exist beyond the first leaf
                // node, then that leaf node is, in fact, the new root node.
                //
                // Using `find(..)` advances the iterator beyond the next
                // non-placeholder side node and returns it. Therefore, we must
                // consume the side node at this point. If another non-
                // placeholder node was found in the side node collection, merge
                // it with the first side node. This guarantees that the current
                // node will be an internal node, and not a leaf, by the time we
                // start merging the remaining side nodes.
                // See https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.find.
                if let Some(side_node) = side_nodes_iter
                    .find(|side_node| *side_node != Node::Placeholder.hash())
                {
                    // Skip parents until the parent of the first side node is found
                    if let Some(old_parent) = path_nodes_iter.find(|parent| {
                        parent.bytes_lo() == side_node || parent.bytes_hi() == side_node
                    }) {
                        let new_parent = if old_parent.bytes_lo() == side_node {
                            Node::create_node_from_hashes(
                                *side_node,
                                *current_node.hash(),
                                old_parent.height(),
                            )
                        } else {
                            Node::create_node_from_hashes(
                                *current_node.hash(),
                                *side_node,
                                old_parent.height(),
                            )
                        };
                        current_node = new_parent;
                        self.storage
                            .insert(current_node.hash(), &current_node.as_ref().into())?;
                    }
                }
            }
        }

        // Merge side nodes
        for (side_node, old_parent) in side_nodes_iter.zip(path_nodes_iter) {
            let new_parent = if old_parent.bytes_lo() == side_node {
                Node::create_node_from_hashes(
                    *side_node,
                    *current_node.hash(),
                    old_parent.height(),
                )
            } else {
                Node::create_node_from_hashes(
                    *current_node.hash(),
                    *side_node,
                    old_parent.height(),
                )
            };

            current_node = new_parent;
            self.storage
                .insert(current_node.hash(), &current_node.as_ref().into())?;
        }

        self.set_root_node(current_node);

        Ok(())
    }
}

impl<TableType, StorageType, StorageError> MerkleTree<TableType, StorageType>
where
    TableType: Mappable<Key = Bytes32, Value = Primitive, OwnedValue = Primitive>,
    StorageType: StorageInspect<TableType, Error = StorageError>,
{
    pub fn generate_proof(
        &self,
        key: &MerkleTreeKey,
    ) -> Result<Proof, MerkleTreeError<StorageError>> {
        let path = key.as_ref();
        let (path_nodes, side_nodes) = self.path_set(path)?;
        // Identify the closest leaf that is included in the tree to the
        // requested leaf. The closest leaf, as returned by the path set
        // corresponding to the requested leaf, will be the requested leaf
        // itself, a different leaf than requested, or a placeholder.
        //
        // If the closest leaf is the requested leaf, then the requested leaf is
        // included in the tree, and we are requesting an inclusion proof.
        // Otherwise (i.e, the closest leaf is either another leaf or a
        // placeholder), the requested leaf is not in the tree, and we are
        // requesting an exclusion proof.
        //
        let actual_leaf = &path_nodes[0];
        let proof_set = side_nodes;
        let proof = if !actual_leaf.is_placeholder() && actual_leaf.leaf_key() == path {
            // If the requested key is part of the tree, build an inclusion
            // proof.
            let inclusion_proof = InclusionProof { proof_set };
            Proof::Inclusion(inclusion_proof)
        } else {
            // If the requested key is not part of the tree, we are verifying
            // that the given key is a placeholder, and we must build an
            // exclusion proof. When building an exclusion proof, the requested
            // leaf is unset and is currently a placeholder. The path to this
            // placeholder is designated by the requested leaf's key.
            //
            // If the closest leaf is a real leaf, and not a placeholder, we can
            // build the root upwards using this leaf's key and value. If the
            // closest leaf is a placeholder, it has a leaf key and a
            // placeholder value (the zero sum). The leaf key of this
            // placeholder leaf is unknown (since placeholders do not store
            // their leaf key), and by extension, the path from the root to the
            // placeholder is also unknown.
            //
            // However, in both cases, the path defined by the requested
            // placeholder is sufficiently close: All branches stemming from the
            // point where the paths of the requested placeholder and closest
            // leaf diverge are saturated with the closest leaf's hash. In the
            // case where the closest leaf is a placeholder, this hash is simply
            // the zero sum. The hash of any placeholder under this point of
            // divergence equates to this hash.
            //
            let leaf = if actual_leaf.is_placeholder() {
                ExclusionLeaf::Placeholder
            } else {
                ExclusionLeaf::Leaf(ExclusionLeafData {
                    leaf_key: *actual_leaf.leaf_key(),
                    leaf_value: *actual_leaf.leaf_data(),
                })
            };

            let exclusion_proof = ExclusionProof { proof_set, leaf };
            Proof::Exclusion(exclusion_proof)
        };
        Ok(proof)
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod test {
    use super::Node;
    use crate::{
        common::{
            sum,
            Bytes32,
            StorageMap,
        },
        sparse::{
            empty_sum,
            MerkleTree,
            MerkleTreeError,
            MerkleTreeKey,
            Primitive,
        },
    };
    use fuel_storage::Mappable;
    use hex;

    fn random_bytes32<R>(rng: &mut R) -> Bytes32
    where
        R: rand::Rng + ?Sized,
    {
        let mut bytes = [0u8; 32];
        rng.fill(bytes.as_mut());
        bytes
    }

    #[derive(Debug)]
    struct TestTable;

    impl Mappable for TestTable {
        type Key = Self::OwnedKey;
        type OwnedKey = Bytes32;
        type OwnedValue = Primitive;
        type Value = Self::OwnedValue;
    }

    fn key<B: AsRef<[u8]>>(data: B) -> MerkleTreeKey {
        MerkleTreeKey::new(data.as_ref())
    }

    #[test]
    fn test_empty_root() {
        let mut storage = StorageMap::<TestTable>::new();
        let tree = MerkleTree::new(&mut storage);
        let root = tree.root();
        let expected_root =
            "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_1() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root =
            "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x01"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root =
            "8d0ae412ca9ca0afcb3217af8bcd5a673e798bd6fd1dfacad17711e883f494cb";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_3() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x02"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root =
            "52295e42d8de2505fdc0cc825ff9fead419cbcf540d8b30c7c4b9c9b94c268b7";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_5() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x02"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x03"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x04"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root =
            "108f731f2414e33ae57e584dc26bd276db07874436b2264ca6e520c658185c6b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_10() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..10 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        let root = tree.root();
        let expected_root =
            "21ca4917e99da99a61de93deaf88c400d4c082991cb95779e444d43dd13e8849";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_100() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..100 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        let root = tree.root();
        let expected_root =
            "82bf747d455a55e2f7044a03536fc43f1f55d43b855e72c0110c986707a23e4d";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_with_repeated_inputs() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root =
            "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_overwrite_key() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x00"), b"CHANGE").unwrap();

        let root = tree.root();
        let expected_root =
            "dd97174c80e5e5aa3a31c61b05e279c1495c8a07b2a08bca5dbc9fb9774f9457";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_overwrite_key_2() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..10 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        let root_hash_before = tree.root();

        for i in 3_u32..7 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA_2").unwrap();
        }

        for i in 3_u32..7 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        let root_hash_after = tree.root();

        assert_eq!(root_hash_before, root_hash_after);
    }

    #[test]
    fn test_update_union() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..5 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        for i in 10_u32..15 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        for i in 20_u32..25 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        let root = tree.root();
        let expected_root =
            "7e6643325042cfe0fc76626c043b97062af51c7e9fc56665f12b479034bce326";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_sparse_union() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x02"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x04"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x06"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x08"), b"DATA").unwrap();

        let root = tree.root();
        let expected_root =
            "e912e97abc67707b2e6027338292943b53d01a7fbd7b244674128c7e468dd696";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_insert_empty_data_changes_root() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"").unwrap();

        let root = tree.root();
        let expected_root =
            "3529664b414de6285270f7ebda7a43e20ae0ff6191c07d876b86282eb8ce93ce";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_with_empty_data_changes_root() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x00"), b"").unwrap();

        let root = tree.root();
        let expected_root =
            "3529664b414de6285270f7ebda7a43e20ae0ff6191c07d876b86282eb8ce93ce";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_1_delete_1() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.delete(key(b"\x00\x00\x00\x00")).unwrap();

        let root = tree.root();
        let expected_root =
            "0000000000000000000000000000000000000000000000000000000000000000";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_2_delete_1() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.delete(key(b"\x00\x00\x00\x01")).unwrap();

        let root = tree.root();
        let expected_root =
            "39f36a7cb4dfb1b46f03d044265df6a491dffc1034121bc1071a34ddce9bb14b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_10_delete_5() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..10 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        for i in 5_u32..10 {
            let key = key(i.to_be_bytes());
            tree.delete(key).unwrap();
        }

        let root = tree.root();
        let expected_root =
            "108f731f2414e33ae57e584dc26bd276db07874436b2264ca6e520c658185c6b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_delete_non_existent_key() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x02"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x03"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x04"), b"DATA").unwrap();
        tree.delete(key(b"\x00\x00\x04\x00")).unwrap();

        let root = tree.root();
        let expected_root =
            "108f731f2414e33ae57e584dc26bd276db07874436b2264ca6e520c658185c6b";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_interleaved_update_delete() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..10 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        for i in 5_u32..15 {
            let key = key(i.to_be_bytes());
            tree.delete(key).unwrap();
        }

        for i in 10_u32..20 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        for i in 15_u32..25 {
            let key = key(i.to_be_bytes());
            tree.delete(key).unwrap();
        }

        for i in 20_u32..30 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        for i in 25_u32..35 {
            let key = key(i.to_be_bytes());
            tree.delete(key).unwrap();
        }

        let root = tree.root();
        let expected_root =
            "7e6643325042cfe0fc76626c043b97062af51c7e9fc56665f12b479034bce326";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_update_removes_old_entries() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);
        let tenth_index = 9u32;

        for i in 0_u32..tenth_index {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }
        let size_before_tenth = tree.storage().len();
        let tenth_key = key(tenth_index.to_be_bytes());

        // Given
        tree.update(tenth_key, b"DATA").unwrap();
        let size_after_tenth = tree.storage().len();
        assert_ne!(size_after_tenth, size_before_tenth);

        // When
        tree.update(tenth_key, b"ANOTHER_DATA").unwrap();

        // Then
        assert_eq!(tree.storage().len(), size_after_tenth);
    }

    #[test]
    fn test_update_with_the_same_value_does_not_remove_old_entries() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);
        let tenth_index = 9u32;

        for i in 0_u32..tenth_index {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }
        let size_before_tenth = tree.storage().len();
        let tenth_key = key(tenth_index.to_be_bytes());

        // Given
        tree.update(tenth_key, b"DATA").unwrap();
        let size_after_tenth = tree.storage().len();
        assert_ne!(size_after_tenth, size_before_tenth);

        // When
        tree.update(tenth_key, b"DATA").unwrap();

        // Then
        assert_eq!(tree.storage().len(), size_after_tenth);
    }

    #[test]
    fn test_delete_removes_path_entries() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);
        let tenth_index = 9u32;

        for i in 0_u32..tenth_index {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }
        let size_before_tenth = tree.storage().len();
        let tenth_key = key(tenth_index.to_be_bytes());

        // Given
        tree.update(tenth_key, b"DATA").unwrap();
        let size_after_tenth = tree.storage().len();
        assert_ne!(size_after_tenth, size_before_tenth);

        // When
        tree.delete(tenth_key).unwrap();

        // Then
        assert_eq!(tree.storage().len(), size_before_tenth);
    }

    #[test]
    fn test_delete_sparse_union() {
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        for i in 0_u32..10 {
            let key = key(i.to_be_bytes());
            tree.update(key, b"DATA").unwrap();
        }

        for i in 0_u32..5 {
            let key = key((i * 2 + 1).to_be_bytes());
            tree.delete(key).unwrap();
        }

        let root = tree.root();
        let expected_root =
            "e912e97abc67707b2e6027338292943b53d01a7fbd7b244674128c7e468dd696";
        assert_eq!(hex::encode(root), expected_root);
    }

    #[test]
    fn test_override_hash_key() {
        use fuel_storage::StorageInspect;
        let mut storage = StorageMap::<TestTable>::new();
        let mut tree = MerkleTree::new(&mut storage);

        let leaf_1_key = key(b"\x00\x00\x00\x00");
        let leaf_1_data = b"DATA_1";
        let leaf_1 = Node::create_leaf(&leaf_1_key.0, leaf_1_data);

        let leaf_2_key = MerkleTreeKey::new_without_hash(*leaf_1.hash());
        let leaf_2_data = b"DATA_2";
        let leaf_2 = Node::create_leaf(&leaf_2_key.0, leaf_2_data);

        tree.update(leaf_2_key, leaf_2_data).unwrap();
        tree.update(leaf_1_key, leaf_1_data).unwrap();
        assert_eq!(
            tree.storage
                .get(leaf_2.hash())
                .unwrap()
                .unwrap()
                .into_owned(),
            leaf_2.as_ref().into()
        );
        assert_eq!(
            tree.storage
                .get(leaf_1.hash())
                .unwrap()
                .unwrap()
                .into_owned(),
            leaf_1.as_ref().into()
        );
    }

    #[test]
    fn test_load_returns_a_valid_tree() {
        // Instantiate a new key-value storage backing and populate it using a sparse
        // Merkle tree. The root of the Merkle tree is the key that maps to the buffer
        // of the root node in the storage. When loading a Merkle tree from storage, we
        // need a reference to the storage object, as well as the root that allows us to
        // look up the buffer of the root node. We will later use this storage backing
        // and root to load a Merkle tree.
        let (mut storage_to_load, root_to_load) = {
            let mut storage = StorageMap::<TestTable>::new();
            let mut tree = MerkleTree::new(&mut storage);
            tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x01"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x02"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x03"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x04"), b"DATA").unwrap();
            let root = tree.root();
            (storage, root)
        };

        // Generate an expected root for this test by using both the set of `update`
        // data used when generating the loadable storage above and an additional set of
        // `update` data.
        let expected_root = {
            let mut storage = StorageMap::<TestTable>::new();
            let mut tree = MerkleTree::new(&mut storage);
            tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x01"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x02"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x03"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x04"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x05"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x06"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x07"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x08"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x09"), b"DATA").unwrap();
            tree.root()
        };

        let root = {
            // Create a Merkle tree by loading the generated storage and root.
            let mut tree = MerkleTree::load(&mut storage_to_load, &root_to_load).unwrap();
            // Build up the loaded tree using the additional set of `update` data so its
            // root matches the expected root. This verifies that the loaded tree has
            // successfully wrapped the given storage backing and assumed the correct
            // state so that future updates can be made seamlessly.
            tree.update(key(b"\x00\x00\x00\x05"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x06"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x07"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x08"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x09"), b"DATA").unwrap();
            tree.root()
        };

        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_load_returns_an_empty_tree_for_empty_sum_root() {
        let mut storage = StorageMap::<TestTable>::new();
        let tree = MerkleTree::load(&mut storage, empty_sum()).unwrap();
        let root = tree.root();

        assert_eq!(root, *empty_sum());
    }

    #[test]
    fn test_load_returns_a_load_error_if_the_storage_is_not_valid_for_the_root() {
        let mut storage = StorageMap::<TestTable>::new();

        {
            let mut tree = MerkleTree::new(&mut storage);
            tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x01"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x02"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x03"), b"DATA").unwrap();
            tree.update(key(b"\x00\x00\x00\x04"), b"DATA").unwrap();
        }

        let root = &sum(b"\xff\xff\xff\xff");
        let err = MerkleTree::load(&mut storage, root)
            .expect_err("Expected load() to return Error; got Ok");
        assert!(matches!(err, MerkleTreeError::LoadError(_)));
    }

    #[test]
    fn test_load_returns_a_deserialize_error_if_the_storage_is_corrupted() {
        use fuel_storage::StorageMutate;

        let mut storage = StorageMap::<TestTable>::new();

        let mut tree = MerkleTree::new(&mut storage);
        tree.update(key(b"\x00\x00\x00\x00"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x01"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x02"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x03"), b"DATA").unwrap();
        tree.update(key(b"\x00\x00\x00\x04"), b"DATA").unwrap();
        let root = tree.root();

        // Overwrite the root key-value with an invalid primitive to create a
        // DeserializeError.
        let primitive = (0xff, 0xff, [0xff; 32], [0xff; 32]);
        storage.insert(&root, &primitive).unwrap();

        let err = MerkleTree::load(&mut storage, &root)
            .expect_err("Expected load() to return Error; got Ok");
        assert!(matches!(err, MerkleTreeError::DeserializeError(_)));
    }

    #[test]
    fn test_from_set_yields_expected_root() {
        let rng = &mut rand::thread_rng();
        let gen = || {
            Some((
                MerkleTreeKey::new_without_hash(random_bytes32(rng)),
                random_bytes32(rng),
            ))
        };
        let data = std::iter::from_fn(gen).take(1_000).collect::<Vec<_>>();

        let expected_root = {
            let mut storage = StorageMap::<TestTable>::new();
            let mut tree = MerkleTree::new(&mut storage);
            let input = data.clone();
            for (key, value) in input.into_iter() {
                tree.update(key, &value).unwrap();
            }
            tree.root()
        };

        let root = {
            let mut storage = StorageMap::<TestTable>::new();
            let tree = MerkleTree::from_set(&mut storage, data.into_iter()).unwrap();
            tree.root()
        };

        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_from_empty_set_yields_expected_root() {
        let rng = &mut rand::thread_rng();
        let gen = || {
            Some((
                MerkleTreeKey::new_without_hash(random_bytes32(rng)),
                random_bytes32(rng),
            ))
        };
        let data = std::iter::from_fn(gen).take(0).collect::<Vec<_>>();

        let expected_root = {
            let mut storage = StorageMap::<TestTable>::new();
            let mut tree = MerkleTree::new(&mut storage);
            let input = data.clone();
            for (key, value) in input.into_iter() {
                tree.update(key, &value).unwrap();
            }
            tree.root()
        };

        let root = {
            let mut storage = StorageMap::<TestTable>::new();
            let tree = MerkleTree::from_set(&mut storage, data.into_iter()).unwrap();
            tree.root()
        };

        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_from_unit_set_yields_expected_root() {
        let rng = &mut rand::thread_rng();
        let gen = || {
            Some((
                MerkleTreeKey::new_without_hash(random_bytes32(rng)),
                random_bytes32(rng),
            ))
        };
        let data = std::iter::from_fn(gen).take(1).collect::<Vec<_>>();

        let expected_root = {
            let mut storage = StorageMap::<TestTable>::new();
            let mut tree = MerkleTree::new(&mut storage);
            let input = data.clone();
            for (key, value) in input.into_iter() {
                tree.update(key, &value).unwrap();
            }
            tree.root()
        };

        let root = {
            let mut storage = StorageMap::<TestTable>::new();
            let tree = MerkleTree::from_set(&mut storage, data.into_iter()).unwrap();
            tree.root()
        };

        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_from_set_with_duplicate_keys_yields_expected_root() {
        let rng = &mut rand::thread_rng();
        let keys = [
            key(b"\x00\x00\x00\x00"),
            key(b"\x00\x00\x00\x01"),
            key(b"\x00\x00\x00\x02"),
        ];
        let data = [
            (keys[0], random_bytes32(rng)),
            (keys[1], random_bytes32(rng)),
            (keys[2], random_bytes32(rng)),
            (keys[0], random_bytes32(rng)),
            (keys[1], random_bytes32(rng)),
            (keys[2], random_bytes32(rng)),
        ];

        let expected_root = {
            let mut storage = StorageMap::<TestTable>::new();
            let mut tree = MerkleTree::new(&mut storage);
            let input = data;
            for (key, value) in input.into_iter() {
                tree.update(key, &value).unwrap();
            }
            tree.root()
        };

        let root = {
            let mut storage = StorageMap::<TestTable>::new();
            let tree = MerkleTree::from_set(&mut storage, data.into_iter()).unwrap();
            tree.root()
        };

        assert_eq!(root, expected_root);
    }

    #[test]
    fn test_from_set_with_empty_data_yields_expected_root() {
        let rng = &mut rand::thread_rng();
        let keys = [
            key(b"\x00\x00\x00\x00"),
            key(b"\x00\x00\x00\x01"),
            key(b"\x00\x00\x00\x02"),
        ];
        let data = [
            (keys[0], random_bytes32(rng).to_vec()),
            (keys[1], random_bytes32(rng).to_vec()),
            (keys[2], random_bytes32(rng).to_vec()),
            (keys[0], b"".to_vec()),
            (keys[1], b"".to_vec()),
            (keys[2], b"".to_vec()),
        ];

        let expected_root = {
            let mut storage = StorageMap::<TestTable>::new();
            let mut tree = MerkleTree::new(&mut storage);
            let input = data.clone();
            for (key, value) in input.into_iter() {
                tree.update(key, &value).unwrap();
            }
            tree.root()
        };

        let root = {
            let mut storage = StorageMap::<TestTable>::new();
            let tree = MerkleTree::from_set(&mut storage, data.into_iter()).unwrap();
            tree.root()
        };

        assert_eq!(root, expected_root);
    }

    #[test]
    fn merkle_tree__generate_proof__returns_proof_with_proof_set_for_given_key() {
        // Given
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

        let l0 = Node::create_leaf(&k0, v0);
        let l1 = Node::create_leaf(&k1, v1);
        let l2 = Node::create_leaf(&k2, v2);
        let l3 = Node::create_leaf(&k3, v3);
        let n0 = Node::create_node(&l1, &l3, 252);
        let n1 = Node::create_node(&n0, &Node::create_placeholder(), 253);
        let n2 = Node::create_node(&n1, &l2, 254);
        let n3 = Node::create_node(&l0, &n2, 255);

        {
            // When
            let proof = tree.generate_proof(&k0.into()).expect("Expected proof");
            let expected_proof_set = [*n2.hash(), *Node::create_placeholder().hash()];

            // Then
            assert_eq!(*proof.proof_set(), expected_proof_set);
        }

        {
            // When
            let proof = tree.generate_proof(&k1.into()).expect("Expected proof");
            let expected_proof_set = [
                *l3.hash(),
                *Node::create_placeholder().hash(),
                *l2.hash(),
                *l0.hash(),
                *Node::create_placeholder().hash(),
            ];

            // Then
            assert_eq!(*proof.proof_set(), expected_proof_set);
        }

        {
            // When
            let proof = tree.generate_proof(&k2.into()).expect("Expected proof");
            let expected_proof_set =
                [*n1.hash(), *l0.hash(), *Node::create_placeholder().hash()];

            // Then
            assert_eq!(*proof.proof_set(), expected_proof_set);
        }

        {
            // When
            let proof = tree.generate_proof(&k3.into()).expect("Expected proof");
            let expected_proof_set = [
                *l1.hash(),
                *Node::create_placeholder().hash(),
                *l2.hash(),
                *l0.hash(),
                *Node::create_placeholder().hash(),
            ];

            // Then
            assert_eq!(*proof.proof_set(), expected_proof_set);
        }

        {
            // Test that supplying an arbitrary leaf "outside" the range of
            // leaves produces a valid proof set

            // When
            let key = [255u8; 32];
            let proof = tree.generate_proof(&key.into()).expect("Expected proof");
            let expected_proof_set = [*n3.hash()];

            // Then
            assert_eq!(*proof.proof_set(), expected_proof_set);
        }
    }

    #[test]
    fn merkle_tree__generate_proof__returns_inclusion_proof_for_included_key() {
        // Given
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

        // When
        let proof = tree.generate_proof(&k1.into()).expect("Expected proof");

        // Then
        assert!(proof.is_inclusion());
    }

    #[test]
    fn merkle_tree__generate_proof__returns_exclusion_proof_for_excluded_key() {
        // Given
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

        // When
        let key = [255u8; 32];
        let proof = tree.generate_proof(&key.into()).expect("Expected proof");

        // Then
        assert!(proof.is_exclusion());
    }
}
