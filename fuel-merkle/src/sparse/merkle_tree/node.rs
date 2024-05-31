use crate::{
    common::{
        error::DeserializeError,
        node::{
            ChildError,
            ChildResult,
            Node as NodeTrait,
            ParentNode as ParentNodeTrait,
        },
        path::{
            Path,
            Side,
        },
<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
        Prefix,
    },
    sparse::generic::{
        hash::{
            sum,
            zero_sum,
        },
        primitive::Primitive,
========
        sum,
        Bytes32,
        Prefix,
    },
    sparse::{
        hash::{
            calculate_hash,
            calculate_leaf_hash,
            calculate_node_hash,
        },
        primitive::{
            Primitive,
            PrimitiveView,
        },
        zero_sum,
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
    },
    storage::{
        Mappable,
        StorageInspect,
    },
};

use crate::common::Bytes;
use core::{
    fmt,
    marker::PhantomData,
};
use digest::Digest;

#[derive(Clone, PartialEq, Eq)]
<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
pub(crate) enum Node<const N: usize> {
========
pub(super) enum Node {
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
    Node {
        hash: Bytes<N>,
        height: u32,
        prefix: Prefix,
        bytes_lo: Bytes<N>,
        bytes_hi: Bytes<N>,
    },
    Placeholder,
}

<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
impl<const N: usize> Node<N> {
    fn calculate_hash(
        prefix: &Prefix,
        bytes_lo: &Bytes<N>,
        bytes_hi: &Bytes<N>,
    ) -> Bytes<N> {
        let mut hash = sha2::Sha256::new();
        hash.update(prefix);
        hash.update(bytes_lo);
        hash.update(bytes_hi);
        let h = hash.finalize();
        let mut vec = h.to_vec();
        vec.truncate(N);
        vec.try_into().unwrap()
    }

    pub fn max_height() -> u32 {
        Node::<N>::key_size_in_bits()
========
impl Node {
    pub fn max_height() -> u32 {
        Node::key_size_bits()
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
    }

    pub fn new(
        height: u32,
        prefix: Prefix,
        bytes_lo: Bytes<N>,
        bytes_hi: Bytes<N>,
    ) -> Self {
        Self::Node {
            hash: calculate_hash(&prefix, &bytes_lo, &bytes_hi),
            height,
            prefix,
            bytes_lo,
            bytes_hi,
        }
    }

    pub fn create_leaf<D: AsRef<[u8]>>(key: &Bytes<N>, data: D) -> Self {
        let bytes_hi = sum(data);
        Self::Node {
            hash: calculate_leaf_hash(key, &bytes_hi),
            height: 0u32,
            prefix: Prefix::Leaf,
            bytes_lo: *key,
            bytes_hi,
        }
    }

    pub fn create_node(left_child: &Node<N>, right_child: &Node<N>, height: u32) -> Self {
        let bytes_lo = *left_child.hash();
        let bytes_hi = *right_child.hash();
        Self::Node {
            hash: calculate_node_hash(&bytes_lo, &bytes_hi),
            height,
            prefix: Prefix::Node,
            bytes_lo,
            bytes_hi,
        }
    }

    pub fn create_node_on_path(
        path: &dyn Path,
        path_node: &Node<N>,
        side_node: &Node<N>,
    ) -> Self {
        if path_node.is_leaf() && side_node.is_leaf() {
            // When joining two leaves, the joined node is found where the paths
            // of the two leaves diverge. The joined node may be a direct parent
            // of the leaves or an ancestor multiple generations above the
            // leaves.
            // N.B.: A leaf can be a placeholder.
<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
            let parent_depth = path_node.common_path_length(side_node);
            let parent_height = Node::<N>::max_height() - parent_depth;
========
            #[allow(clippy::cast_possible_truncation)] // Key is 32 bytes
            let parent_depth = path_node.common_path_length(side_node) as u32;
            #[allow(clippy::arithmetic_side_effects)] // parent_depth <= max_height
            let parent_height = Node::max_height() - parent_depth;
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
            match path.get_instruction(parent_depth).unwrap() {
                Side::Left => Node::create_node(path_node, side_node, parent_height),
                Side::Right => Node::create_node(side_node, path_node, parent_height),
            }
        } else {
            // When joining two nodes, or a node and a leaf, the joined node is
            // the direct parent of the node with the greater height and an
            // ancestor of the node with the lesser height.
            // N.B.: A leaf can be a placeholder.
<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
            let parent_height = cmp::max(path_node.height(), side_node.height()) + 1;
            let parent_depth = Node::<N>::max_height() - parent_height;
========
            #[allow(clippy::arithmetic_side_effects)] // Neither node cannot be root
            let parent_height = path_node.height().max(side_node.height()) + 1;
            #[allow(clippy::arithmetic_side_effects)] // parent_height <= max_height
            let parent_depth = Node::max_height() - parent_height;
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
            match path.get_instruction(parent_depth).unwrap() {
                Side::Left => Node::create_node(path_node, side_node, parent_height),
                Side::Right => Node::create_node(side_node, path_node, parent_height),
            }
        }
    }

    pub fn create_placeholder() -> Self {
        Self::Placeholder
    }

<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
    pub fn common_path_length(&self, other: &Node<N>) -> u32 {
========
    pub fn common_path_length(&self, other: &Node) -> u64 {
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
        debug_assert!(self.is_leaf());
        debug_assert!(other.is_leaf());

        // If either of the nodes is a placeholder, the common path length is
        // defined to be 0. This is needed to prevent a 0 bit in the
        // placeholder's key from producing an erroneous match with a 0 bit in
        // the leaf's key.
        if self.is_placeholder() || other.is_placeholder() {
            0
        } else {
            self.leaf_key().common_path_length(other.leaf_key())
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Node::Node { height, .. } => *height,
            Node::Placeholder => 0,
        }
    }

<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
    pub fn prefix(&self) -> Prefix {
        match self {
            Node::Node { prefix, .. } => *prefix,
            Node::Placeholder => Prefix::Leaf,
        }
    }

    pub fn bytes_lo(&self) -> &Bytes<N> {
        match self {
            Node::Node { bytes_lo, .. } => bytes_lo,
            Node::Placeholder => zero_sum(),
        }
    }

    pub fn bytes_hi(&self) -> &Bytes<N> {
        match self {
            Node::Node { bytes_hi, .. } => bytes_hi,
            Node::Placeholder => zero_sum(),
        }
    }

========
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
    pub fn is_leaf(&self) -> bool {
        self.prefix() == Prefix::Leaf || self.is_placeholder()
    }

    pub fn is_node(&self) -> bool {
        self.prefix() == Prefix::Node
    }

<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
    pub fn leaf_key(&self) -> &Bytes<N> {
        assert!(self.is_leaf());
        self.bytes_lo()
    }

    pub fn leaf_data(&self) -> &Bytes<N> {
        assert!(self.is_leaf());
        self.bytes_hi()
    }

    pub fn left_child_key(&self) -> &Bytes<N> {
        assert!(self.is_node());
        self.bytes_lo()
    }

    pub fn right_child_key(&self) -> &Bytes<N> {
        assert!(self.is_node());
        self.bytes_hi()
    }

========
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
    pub fn is_placeholder(&self) -> bool {
        &Self::Placeholder == self
    }

    pub fn hash(&self) -> &Bytes<N> {
        match self {
            Node::Node { hash, .. } => hash,
            Node::Placeholder => zero_sum(),
        }
    }

    fn prefix(&self) -> Prefix {
        match self {
            Node::Node { prefix, .. } => *prefix,
            Node::Placeholder => Prefix::Leaf,
        }
    }

    fn bytes_lo(&self) -> &Bytes32 {
        match self {
            Node::Node { bytes_lo, .. } => bytes_lo,
            Node::Placeholder => zero_sum(),
        }
    }

    fn bytes_hi(&self) -> &Bytes32 {
        match self {
            Node::Node { bytes_hi, .. } => bytes_hi,
            Node::Placeholder => zero_sum(),
        }
    }

    /// Get the leaf key of a leaf node.
    ///
    /// The leaf key is the lower 32 bytes stored in a leaf node.
    /// This method expects the node to be a leaf node, and this precondition
    /// must be guaranteed at the call site for correctness. This method should
    /// only be used within contexts where this precondition can be guaranteed,
    /// such as the [MerkleTree](super::MerkleTree).
    ///
    /// In `debug`, this method will panic if the node is not a leaf node to
    /// indicate to the developer that there is a potential problem in the
    /// tree's implementation.  
    pub(super) fn leaf_key(&self) -> &Bytes32 {
        debug_assert!(self.is_leaf());
        self.bytes_lo()
    }

    /// Get the leaf data of a leaf node.
    ///
    /// The leaf key is the upper 32 bytes stored in a leaf node.
    /// This method expects the node to be a leaf node, and this precondition
    /// must be guaranteed at the call site for correctness. This method should
    /// only be used within contexts where this precondition can be guaranteed,
    /// such as the [MerkleTree](super::MerkleTree).
    ///
    /// In `debug`, this method will panic if the node is not a leaf node to
    /// indicate to the developer that there is a potential problem in the
    /// tree's implementation.
    pub(super) fn leaf_data(&self) -> &Bytes32 {
        debug_assert!(self.is_leaf());
        self.bytes_hi()
    }

    /// Get the left child key of an internal node.
    ///
    /// The left child key is the lower 32 bytes stored in an internal node.
    /// This method expects the node to be an internal node, and this
    /// precondition must be guaranteed at the call site for correctness. This
    /// method should only be used within contexts where this precondition can
    /// be guaranteed, such as the [MerkleTree](super::MerkleTree).
    ///
    /// In `debug`, this method will panic if the node is not an internal node
    /// to indicate to the developer that there is a potential problem in the
    /// tree's implementation.
    pub(super) fn left_child_key(&self) -> &Bytes32 {
        debug_assert!(self.is_node());
        self.bytes_lo()
    }

    /// Get the right child key of an internal node.
    ///
    /// The right child key is the upper 32 bytes stored in an internal node.
    /// This method expects the node to be an internal node, and this
    /// precondition must be guaranteed at the call site for correctness. This
    /// method should only be used within contexts where this precondition can
    /// be guaranteed, such as the [MerkleTree](super::MerkleTree).
    ///
    /// In `debug`, this method will panic if the node is not an internal node
    /// to indicate to the developer that there is a potential problem in the
    /// tree's implementation.
    pub(super) fn right_child_key(&self) -> &Bytes32 {
        debug_assert!(self.is_node());
        self.bytes_hi()
    }
}

impl<const KEY_SIZE: usize> AsRef<Node<KEY_SIZE>> for Node<KEY_SIZE> {
    fn as_ref(&self) -> &Node<KEY_SIZE> {
        self
    }
}

impl<const KEY_SIZE: usize> NodeTrait for Node<KEY_SIZE> {
    type Key = Bytes<KEY_SIZE>;

    fn height(&self) -> u32 {
        Node::height(self)
    }

    #[allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)] // const
    fn key_size_bits() -> u32 {
        core::mem::size_of::<Self::Key>() as u32 * 8
    }

    fn leaf_key(&self) -> Self::Key {
        *Node::leaf_key(self)
    }

    fn is_leaf(&self) -> bool {
        Node::is_leaf(self)
    }

    fn is_node(&self) -> bool {
        Node::is_node(self)
    }
}

<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
impl<const KEY_SIZE: usize> fmt::Debug for Node<KEY_SIZE> {
========
impl From<&Node> for Primitive {
    fn from(node: &Node) -> Self {
        (
            node.height(),
            node.prefix() as u8,
            *node.bytes_lo(),
            *node.bytes_hi(),
        )
    }
}

impl TryFrom<Primitive> for Node {
    type Error = DeserializeError;

    fn try_from(primitive: Primitive) -> Result<Self, Self::Error> {
        let height = primitive.height();
        let prefix = primitive.prefix()?;
        let bytes_lo = *primitive.bytes_lo();
        let bytes_hi = *primitive.bytes_hi();
        let node = Self::new(height, prefix, bytes_lo, bytes_hi);
        Ok(node)
    }
}

impl fmt::Debug for Node {
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_node() {
            f.debug_struct("Node (Internal)")
                .field("Height", &self.height())
                .field("Hash", &hex::encode(self.hash()))
                .field("Left child key", &hex::encode(self.left_child_key()))
                .field("Right child key", &hex::encode(self.right_child_key()))
                .finish()
        } else {
            f.debug_struct("Node (Leaf)")
                .field("Height", &self.height())
                .field("Hash", &hex::encode(self.hash()))
                .field("Leaf key", &hex::encode(self.leaf_key()))
                .field("Leaf data", &hex::encode(self.leaf_data()))
                .finish()
        }
    }
}

<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
pub(crate) struct StorageNode<'storage, const KEY_SIZE: usize, TableType, StorageType> {
========
pub(super) struct StorageNode<'storage, TableType, StorageType> {
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
    storage: &'storage StorageType,
    node: Node<KEY_SIZE>,
    phantom_table: PhantomData<TableType>,
}

impl<const KEY_SIZE: usize, TableType, StorageType> Clone
    for StorageNode<'_, KEY_SIZE, TableType, StorageType>
{
    fn clone(&self) -> Self {
        Self {
            storage: self.storage,
            node: self.node.clone(),
            phantom_table: Default::default(),
        }
    }
}

impl<'s, const KEY_SIZE: usize, TableType, StorageType>
    StorageNode<'s, KEY_SIZE, TableType, StorageType>
{
    pub fn new(storage: &'s StorageType, node: Node<KEY_SIZE>) -> Self {
        Self {
            node,
            storage,
            phantom_table: Default::default(),
        }
    }
}

impl<const KEY_SIZE: usize, TableType, StorageType>
    StorageNode<'_, KEY_SIZE, TableType, StorageType>
{
    pub fn hash(&self) -> &Bytes<KEY_SIZE> {
        self.node.hash()
    }

    pub fn into_node(self) -> Node<KEY_SIZE> {
        self.node
    }
}

impl<const KEY_SIZE: usize, TableType, StorageType> NodeTrait
    for StorageNode<'_, KEY_SIZE, TableType, StorageType>
{
    type Key = Bytes<KEY_SIZE>;

    fn height(&self) -> u32 {
        self.node.height()
    }

    #[allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)] // const
    fn key_size_bits() -> u32 {
        core::mem::size_of::<Self::Key>() as u32 * 8
    }

    fn leaf_key(&self) -> Self::Key {
        *self.node.leaf_key()
    }

    fn is_leaf(&self) -> bool {
        self.node.is_leaf()
    }

    fn is_node(&self) -> bool {
        self.node.is_node()
    }
}

#[derive(Debug, Clone, derive_more::Display)]
pub enum StorageNodeError<StorageError> {
    #[display(fmt = "{}", _0)]
    StorageError(StorageError),
    #[display(fmt = "{}", _0)]
    DeserializeError(DeserializeError),
}

impl<const KEY_SIZE: usize, TableType, StorageType> ParentNodeTrait
    for StorageNode<'_, KEY_SIZE, TableType, StorageType>
where
    StorageType: StorageInspect<TableType>,
    TableType: Mappable<
        Key = Bytes<KEY_SIZE>,
        Value = Primitive<KEY_SIZE>,
        OwnedValue = Primitive<KEY_SIZE>,
    >,
{
    type Error = StorageNodeError<StorageType::Error>;

    fn left_child(&self) -> ChildResult<Self> {
        if self.is_leaf() {
            return Err(ChildError::NodeIsLeaf)
        }
        let key = self.node.left_child_key();
        if key == zero_sum() {
            return Ok(Self::new(self.storage, Node::create_placeholder()))
        }
        let primitive = self
            .storage
            .get(key)
            .map_err(StorageNodeError::StorageError)?
            .ok_or(ChildError::ChildNotFound(*key))?;
        Ok(primitive
            .into_owned()
            .try_into()
            .map(|node| Self::new(self.storage, node))
            .map_err(StorageNodeError::DeserializeError)?)
    }

    fn right_child(&self) -> ChildResult<Self> {
        if self.is_leaf() {
            return Err(ChildError::NodeIsLeaf)
        }
        let key = self.node.right_child_key();
        if key == zero_sum() {
            return Ok(Self::new(self.storage, Node::create_placeholder()))
        }
        let primitive = self
            .storage
            .get(key)
            .map_err(StorageNodeError::StorageError)?
            .ok_or(ChildError::ChildNotFound(*key))?;
        Ok(primitive
            .into_owned()
            .try_into()
            .map(|node| Self::new(self.storage, node))
            .map_err(StorageNodeError::DeserializeError)?)
    }
}

impl<const KEY_SIZE: usize, TableType, StorageType> fmt::Debug
    for StorageNode<'_, KEY_SIZE, TableType, StorageType>
where
    StorageType: StorageInspect<TableType>,
    TableType: Mappable<
        Key = Bytes<KEY_SIZE>,
        Value = Primitive<KEY_SIZE>,
        OwnedValue = Primitive<KEY_SIZE>,
    >,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_node() {
            f.debug_struct("StorageNode (Internal)")
                .field("Height", &self.height())
                .field("Hash", &hex::encode(self.hash()))
                .field("Left child key", &hex::encode(self.node.left_child_key()))
                .field("Right child key", &hex::encode(self.node.right_child_key()))
                .finish()
        } else {
            f.debug_struct("StorageNode (Leaf)")
                .field("Height", &self.height())
                .field("Hash", &hex::encode(self.hash()))
                .field("Leaf key", &hex::encode(self.node.leaf_key()))
                .field("Leaf data", &hex::encode(self.node.leaf_data()))
                .finish()
        }
    }
}

#[cfg(test)]
mod test_node {
    use super::Node;
    use crate::{
        common::{
            error::DeserializeError,
            sum,
            Bytes32,
            Prefix,
            PrefixError,
        },
        sparse::{
<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
            generic::{
                hash::{
                    sum,
                    zero_sum,
                },
                Node,
            },
========
            zero_sum,
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
            Primitive,
        },
    };

    fn leaf_hash(key: &Bytes32, data: &[u8]) -> Bytes32 {
        let mut buffer = [0; 65];
        buffer[0..1].clone_from_slice(Prefix::Leaf.as_ref());
        buffer[1..33].clone_from_slice(key);
        buffer[33..65].clone_from_slice(&sum::<_, 32>(data));
        sum(buffer)
    }

    #[test]
    fn test_create_leaf_returns_a_valid_leaf() {
        let leaf = Node::<32>::create_leaf(&sum(b"LEAF"), [1u8; 32]);
        assert_eq!(leaf.is_leaf(), true);
        assert_eq!(leaf.is_node(), false);
        assert_eq!(leaf.height(), 0);
        assert_eq!(leaf.prefix(), Prefix::Leaf);
        assert_eq!(*leaf.leaf_key(), sum(b"LEAF"));
        assert_eq!(*leaf.leaf_data(), sum([1u8; 32]));
    }

    #[test]
    fn test_create_node_returns_a_valid_node() {
        let left_child = Node::create_leaf(&sum(b"LEFT CHILD"), [1u8; 32]);
        let right_child = Node::create_leaf(&sum(b"RIGHT CHILD"), [1u8; 32]);
        let node = Node::create_node(&left_child, &right_child, 1);
        assert_eq!(node.is_leaf(), false);
        assert_eq!(node.is_node(), true);
        assert_eq!(node.height(), 1);
        assert_eq!(node.prefix(), Prefix::Node);
        assert_eq!(
            *node.left_child_key(),
            leaf_hash(&sum(b"LEFT CHILD"), &[1u8; 32])
        );
        assert_eq!(
            *node.right_child_key(),
            leaf_hash(&sum(b"RIGHT CHILD"), &[1u8; 32])
        );
    }

    #[test]
    fn test_create_placeholder_returns_a_placeholder_node() {
        let node = Node::<32>::create_placeholder();
        assert_eq!(node.is_placeholder(), true);
        assert_eq!(node.hash(), zero_sum());
    }

    #[test]
    fn test_create_leaf_from_primitive_returns_a_valid_leaf() {
        let primitive = (0, Prefix::Leaf as u8, [0xff; 32], [0xff; 32]);

        let node: Node<32> = primitive.try_into().unwrap();
        assert_eq!(node.is_leaf(), true);
        assert_eq!(node.is_node(), false);
        assert_eq!(node.height(), 0);
        assert_eq!(node.prefix(), Prefix::Leaf);
        assert_eq!(*node.leaf_key(), [0xff; 32]);
        assert_eq!(*node.leaf_data(), [0xff; 32]);
    }

    #[test]
    fn test_create_node_from_primitive_returns_a_valid_node() {
        let primitive = (255, Prefix::Node as u8, [0xff; 32], [0xff; 32]);

        let node: Node<32> = primitive.try_into().unwrap();
        assert_eq!(node.is_leaf(), false);
        assert_eq!(node.is_node(), true);
        assert_eq!(node.height(), 255);
        assert_eq!(node.prefix(), Prefix::Node);
        assert_eq!(*node.left_child_key(), [0xff; 32]);
        assert_eq!(*node.right_child_key(), [0xff; 32]);
    }

    #[test]
    fn test_create_from_primitive_returns_deserialize_error_if_invalid_prefix() {
        let primitive = (0xff, 0xff, [0xff; 32], [0xff; 32]);

        // Should return Error; prefix 0xff is does not represent a node or leaf
        let err = Node::try_from(primitive)
            .expect_err("Expected try_from() to be Error; got OK");
        assert!(matches!(
            err,
            DeserializeError::PrefixError(PrefixError::InvalidPrefix(0xff))
        ));
    }

    /// For leaf node `node` of leaf data `d` with key `k`:
    /// ```node = (0x00, k, h(serialize(d)))```
    #[test]
    fn test_leaf_primitive_returns_expected_primitive() {
        let expected_primitive =
            (0_u32, Prefix::Leaf as u8, sum(b"LEAF"), sum([1u8; 32]));

        let leaf = Node::create_leaf(&sum(b"LEAF"), [1u8; 32]);
        let primitive = Primitive::from(&leaf);

        assert_eq!(primitive, expected_primitive);
    }

    /// For internal node `node` with children `l` and `r`:
    /// ```node = (0x01, l.v, r.v)```
    #[test]
    fn test_node_primitive_returns_expected_primitive() {
        let expected_primitive = (
            1_u32,
            Prefix::Node as u8,
            leaf_hash(&sum(b"LEFT CHILD"), &[1u8; 32]),
            leaf_hash(&sum(b"RIGHT CHILD"), &[1u8; 32]),
        );

        let left_child = Node::create_leaf(&sum(b"LEFT CHILD"), [1u8; 32]);
        let right_child = Node::create_leaf(&sum(b"RIGHT CHILD"), [1u8; 32]);
        let node = Node::create_node(&left_child, &right_child, 1);
        let primitive = Primitive::from(&node);

        assert_eq!(primitive, expected_primitive);
    }

    /// For leaf node `node` of leaf data `d` with key `k`:
    /// ```node.v = h(0x00, k, h(serialize(d)))```
    #[test]
    fn test_leaf_hash_returns_expected_hash_value() {
        let mut expected_buffer = [0u8; 65];
        expected_buffer[0..1].clone_from_slice(Prefix::Leaf.as_ref());
        expected_buffer[1..33].clone_from_slice(&sum::<_, 32>(b"LEAF"));
        expected_buffer[33..65].clone_from_slice(&sum::<_, 32>([1u8; 32]));
        let expected_value = sum::<_, 32>(expected_buffer);

        let node = Node::create_leaf(&sum(b"LEAF"), [1u8; 32]);
        let value = *node.hash();

        assert_eq!(value, expected_value);
    }

    /// For internal node `node` with children `l` and `r`:
    /// ```node.v = h(0x01, l.v, r.v)```
    #[test]
    fn test_node_hash_returns_expected_hash_value() {
        let mut expected_buffer = [0u8; 65];
        expected_buffer[0..1].clone_from_slice(Prefix::Node.as_ref());
        expected_buffer[1..33]
            .clone_from_slice(&leaf_hash(&sum(b"LEFT CHILD"), &[1u8; 32]));
        expected_buffer[33..65]
            .clone_from_slice(&leaf_hash(&sum(b"RIGHT CHILD"), &[1u8; 32]));
        let expected_value = sum::<_, 32>(expected_buffer);

        let left_child = Node::create_leaf(&sum(b"LEFT CHILD"), [1u8; 32]);
        let right_child = Node::create_leaf(&sum(b"RIGHT CHILD"), [1u8; 32]);
        let node = Node::create_node(&left_child, &right_child, 1);
        let value = *node.hash();

        assert_eq!(value, expected_value);
    }
}

#[cfg(test)]
mod test_storage_node {
    use super::{
        Node,
        StorageNode,
        StorageNodeError,
    };
<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs

========
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
    use crate::{
        common::{
            error::DeserializeError,
            node::{
                ChildError,
                ParentNode,
            },
            sum,
            Bytes32,
            PrefixError,
            StorageMap,
        },
<<<<<<<< HEAD:fuel-merkle/src/sparse/generic/node.rs
        sparse::{
            generic::hash::sum,
            Primitive,
        },
========
        sparse::Primitive,
>>>>>>>> master:fuel-merkle/src/sparse/merkle_tree/node.rs
        storage::{
            Mappable,
            StorageMutate,
        },
    };

    pub struct TestTable;

    impl Mappable for TestTable {
        type Key = Self::OwnedKey;
        type OwnedKey = Bytes32;
        type OwnedValue = Primitive;
        type Value = Self::OwnedValue;
    }

    #[test]
    fn test_node_left_child_returns_the_left_child() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), [1u8; 32]);
        let _ = s.insert(leaf_0.hash(), &leaf_0.as_ref().into());

        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), [1u8; 32]);
        let _ = s.insert(leaf_1.hash(), &leaf_1.as_ref().into());

        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);
        let _ = s.insert(node_0.hash(), &node_0.as_ref().into());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.left_child().unwrap();

        assert_eq!(child.hash(), leaf_0.hash());
    }

    #[test]
    fn test_node_right_child_returns_the_right_child() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), [1u8; 32]);
        let _ = s.insert(leaf_0.hash(), &leaf_0.as_ref().into());

        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), [1u8; 32]);
        let _ = s.insert(leaf_1.hash(), &leaf_1.as_ref().into());

        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);
        let _ = s.insert(node_0.hash(), &node_0.as_ref().into());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.right_child().unwrap();

        assert_eq!(child.hash(), leaf_1.hash());
    }

    #[test]
    fn test_node_left_child_returns_placeholder_when_key_is_zero_sum() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf = Node::create_leaf(&sum(b"Goodbye World"), [1u8; 32]);
        let _ = s.insert(leaf.hash(), &leaf.as_ref().into());

        let node_0 = Node::create_node(&Node::create_placeholder(), &leaf, 1);
        let _ = s.insert(node_0.hash(), &node_0.as_ref().into());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.left_child().unwrap();

        assert!(child.node.is_placeholder());
    }

    #[test]
    fn test_node_right_child_returns_placeholder_when_key_is_zero_sum() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf = Node::create_leaf(&sum(b"Goodbye World"), [1u8; 32]);
        let _ = s.insert(leaf.hash(), &leaf.as_ref().into());

        let node_0 = Node::create_node(&leaf, &Node::create_placeholder(), 1);
        let _ = s.insert(node_0.hash(), &node_0.as_ref().into());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.right_child().unwrap();

        assert!(child.node.is_placeholder());
    }

    #[test]
    fn test_node_left_child_returns_error_when_node_is_leaf() {
        let s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), [1u8; 32]);
        let storage_node = StorageNode::new(&s, leaf_0);
        let err = storage_node
            .left_child()
            .expect_err("Expected left_child() to return Error; got OK");

        assert!(matches!(err, ChildError::NodeIsLeaf));
    }

    #[test]
    fn test_node_right_child_returns_error_when_node_is_leaf() {
        let s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), [1u8; 32]);
        let storage_node = StorageNode::new(&s, leaf_0);
        let err = storage_node
            .right_child()
            .expect_err("Expected right_child() to return Error; got OK");

        assert!(matches!(err, ChildError::NodeIsLeaf));
    }

    #[test]
    fn test_node_left_child_returns_error_when_key_is_not_found() {
        let s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), [0u8; 32]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), [1u8; 32]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .left_child()
            .expect_err("Expected left_child() to return Error; got Ok");

        let key = *storage_node.into_node().left_child_key();
        assert!(matches!(
            err,
            ChildError::ChildNotFound(k) if k == key
        ));
    }

    #[test]
    fn test_node_right_child_returns_error_when_key_is_not_found() {
        let s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), [1u8; 32]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), [1u8; 32]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .right_child()
            .expect_err("Expected right_child() to return Error; got Ok");

        let key = *storage_node.into_node().right_child_key();
        assert!(matches!(
            err,
            ChildError::ChildNotFound(k) if k == key
        ));
    }

    #[test]
    fn test_node_left_child_returns_deserialize_error_when_primitive_is_invalid() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), [1u8; 32]);
        let _ = s.insert(leaf_0.hash(), &(0xff, 0xff, [0xff; 32], [0xff; 32]));
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), [1u8; 32]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .left_child()
            .expect_err("Expected left_child() to be Error; got Ok");

        assert!(matches!(
            err,
            ChildError::Error(StorageNodeError::DeserializeError(
                DeserializeError::PrefixError(PrefixError::InvalidPrefix(0xff))
            ))
        ));
    }

    #[test]
    fn test_node_right_child_returns_deserialize_error_when_primitive_is_invalid() {
        let mut s = StorageMap::<TestTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), [1u8; 32]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), [1u8; 32]);
        let _ = s.insert(leaf_1.hash(), &(0xff, 0xff, [0xff; 32], [0xff; 32]));
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .right_child()
            .expect_err("Expected right_child() to be Error; got Ok");

        assert!(matches!(
            err,
            ChildError::Error(StorageNodeError::DeserializeError(
                DeserializeError::PrefixError(PrefixError::InvalidPrefix(0xff))
            ))
        ));
    }
}
