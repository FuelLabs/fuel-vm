use crate::{
    common::{error::DeserializeError, Bytes1, Bytes32, Bytes4, Msb, Node as NodeTrait, Prefix},
    sparse::{hash::sum, merkle_tree::NodesTable, zero_sum},
};

// TODO: Return errors instead of `unwrap` during work with storage.
use fuel_storage::StorageInspect;

use core::{cmp, fmt, mem::size_of, ops::Range};

const LEFT: u8 = 0;

/// **Leaf buffer:**
///
/// | Allocation | Data                       |
/// |------------|----------------------------|
/// | `00 - 04`  | Height (4 bytes)           |
/// | `04 - 05`  | Prefix (1 byte, `0x00`)    |
/// | `05 - 37`  | hash(Key) (32 bytes)       |
/// | `37 - 69`  | hash(Data) (32 bytes)      |
///
/// **Node buffer:**
///
/// | Allocation | Data                       |
/// |------------|----------------------------|
/// | `00 - 04`  | Height (4 bytes)           |
/// | `04 - 05`  | Prefix (1 byte, `0x01`)    |
/// | `05 - 37`  | Left child key (32 bytes)  |
/// | `37 - 69`  | Right child key (32 bytes) |
///
const BUFFER_SIZE: usize =
    size_of::<Bytes4>() + size_of::<Bytes1>() + size_of::<Bytes32>() + size_of::<Bytes32>();
pub type Buffer = [u8; BUFFER_SIZE];

#[derive(Clone)]
pub(crate) struct Node {
    buffer: Buffer,
}

impl Node {
    pub fn max_height() -> usize {
        Node::key_size_in_bits()
    }

    pub fn create_leaf(key: &Bytes32, data: &[u8]) -> Self {
        let buffer = Self::default_buffer();
        let mut node = Self { buffer };
        node.set_height(0);
        node.set_prefix(Prefix::Leaf);
        node.set_bytes_lo(key);
        node.set_bytes_hi(&sum(data));
        node
    }

    pub fn create_node(left_child: &Node, right_child: &Node, height: u32) -> Self {
        let buffer = Self::default_buffer();
        let mut node = Self { buffer };
        node.set_height(height);
        node.set_prefix(Prefix::Node);
        node.set_bytes_lo(&left_child.hash());
        node.set_bytes_hi(&right_child.hash());
        node
    }

    pub fn create_node_on_path(path: &Bytes32, path_node: &Node, side_node: &Node) -> Self {
        if path_node.is_leaf() && side_node.is_leaf() {
            // When joining two leaves, the joined node is found where the paths
            // of the two leaves diverge. The joined node may be a direct parent
            // of the leaves or an ancestor multiple generations above the
            // leaves.
            // N.B.: A leaf can be a placeholder.
            let parent_depth = path_node.common_path_length(side_node);
            let parent_height = (Node::max_height() - parent_depth) as u32;
            if path.get_bit_at_index_from_msb(parent_depth).unwrap() == LEFT {
                Node::create_node(path_node, side_node, parent_height)
            } else {
                Node::create_node(side_node, path_node, parent_height)
            }
        } else {
            // When joining two nodes, or a node and a leaf, the joined node is
            // the direct parent of the node with the greater height and an
            // ancestor of the node with the lesser height.
            // N.B.: A leaf can be a placeholder.
            let parent_height = cmp::max(path_node.height(), side_node.height()) + 1;
            let parent_depth = Node::max_height() - parent_height as usize;
            if path.get_bit_at_index_from_msb(parent_depth).unwrap() == LEFT {
                Node::create_node(path_node, side_node, parent_height)
            } else {
                Node::create_node(side_node, path_node, parent_height)
            }
        }
    }

    pub fn create_placeholder() -> Self {
        let buffer = Self::default_buffer();
        Self { buffer }
    }

    pub fn common_path_length(&self, other: &Node) -> usize {
        debug_assert!(self.is_leaf());
        debug_assert!(other.is_leaf());

        // If either of the nodes is a placeholder, the common path length is
        // defined to be 0. This is needed to prevent a 0 bit in the
        // placeholder's key from producing an erroneous match with a 0 bit in
        // the leaf's key.
        if self.is_placeholder() || other.is_placeholder() {
            0
        } else {
            self.leaf_key().common_prefix_count(other.leaf_key())
        }
    }

    pub fn height(&self) -> u32 {
        let bytes = self.bytes_height();
        u32::from_be_bytes(bytes.try_into().unwrap())
    }

    pub fn prefix(&self) -> Prefix {
        // Safety: By the time a Node is created, it will always have a valid
        // prefix.
        self.bytes_prefix()[0].try_into().unwrap()
    }

    pub fn leaf_key(&self) -> &Bytes32 {
        assert!(self.is_leaf());
        self.bytes_lo().try_into().unwrap()
    }

    pub fn leaf_data(&self) -> &Bytes32 {
        assert!(self.is_leaf());
        self.bytes_hi().try_into().unwrap()
    }

    pub fn left_child_key(&self) -> &Bytes32 {
        assert!(self.is_node());
        self.bytes_lo().try_into().unwrap()
    }

    pub fn right_child_key(&self) -> &Bytes32 {
        assert!(self.is_node());
        self.bytes_hi().try_into().unwrap()
    }

    pub fn is_leaf(&self) -> bool {
        self.prefix() == Prefix::Leaf || self.is_placeholder()
    }

    pub fn is_node(&self) -> bool {
        self.prefix() == Prefix::Node
    }

    pub fn is_placeholder(&self) -> bool {
        self.bytes_lo() == zero_sum() && self.bytes_hi() == zero_sum()
    }

    pub fn as_buffer(&self) -> &Buffer {
        self.buffer().try_into().unwrap()
    }

    pub fn hash(&self) -> Bytes32 {
        if self.is_placeholder() {
            *zero_sum()
        } else {
            let range = Self::hash_range();
            sum(&self.buffer()[range])
        }
    }

    // PRIVATE

    const fn default_buffer() -> Buffer {
        [0; Self::buffer_size()]
    }

    // HEIGHT

    const fn height_offset() -> usize {
        0
    }

    const fn height_size() -> usize {
        size_of::<Bytes4>()
    }

    const fn height_range() -> Range<usize> {
        Self::height_offset()..(Self::height_offset() + Self::height_size())
    }

    // PREFIX

    const fn prefix_offset() -> usize {
        Self::height_offset() + Self::height_size()
    }

    const fn prefix_size() -> usize {
        size_of::<Bytes1>()
    }

    const fn prefix_range() -> Range<usize> {
        Self::prefix_offset()..(Self::prefix_offset() + Self::prefix_size())
    }

    // BYTES LO

    const fn bytes_lo_offset() -> usize {
        Self::prefix_offset() + Self::prefix_size()
    }

    const fn bytes_lo_size() -> usize {
        size_of::<Bytes32>()
    }

    const fn bytes_lo_range() -> Range<usize> {
        Self::bytes_lo_offset()..(Self::bytes_lo_offset() + Self::bytes_lo_size())
    }

    // BYTES HI

    const fn bytes_hi_offset() -> usize {
        Self::bytes_lo_offset() + Self::bytes_lo_size()
    }

    const fn bytes_hi_size() -> usize {
        size_of::<Bytes32>()
    }

    const fn bytes_hi_range() -> Range<usize> {
        Self::bytes_hi_offset()..(Self::bytes_hi_offset() + Self::bytes_hi_size())
    }

    // HASH

    const fn hash_range() -> Range<usize> {
        Self::prefix_offset()..Self::buffer_size()
    }

    // BUFFER

    const fn buffer_size() -> usize {
        BUFFER_SIZE
    }

    // PRIVATE

    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    // Height

    fn set_height(&mut self, height: u32) {
        let bytes = height.to_be_bytes();
        self.set_bytes_height(&bytes)
    }

    fn bytes_height_mut(&mut self) -> &mut [u8] {
        let range = Self::height_range();
        &mut self.buffer_mut()[range]
    }

    fn bytes_height(&self) -> &[u8] {
        let range = Self::height_range();
        &self.buffer()[range]
    }

    fn set_bytes_height(&mut self, bytes: &Bytes4) {
        self.bytes_height_mut().clone_from_slice(bytes)
    }

    // Prefix

    fn set_prefix(&mut self, prefix: Prefix) {
        self.set_bytes_prefix(prefix.as_ref());
    }

    fn bytes_prefix_mut(&mut self) -> &mut [u8] {
        let range = Self::prefix_range();
        &mut self.buffer_mut()[range]
    }

    fn bytes_prefix(&self) -> &[u8] {
        let range = Self::prefix_range();
        &self.buffer()[range]
    }

    fn set_bytes_prefix(&mut self, bytes: &Bytes1) {
        self.bytes_prefix_mut().clone_from_slice(bytes);
    }

    // Bytes lo

    fn bytes_lo_mut(&mut self) -> &mut [u8] {
        let range = Self::bytes_lo_range();
        &mut self.buffer_mut()[range]
    }

    fn bytes_lo(&self) -> &[u8] {
        let range = Self::bytes_lo_range();
        &self.buffer()[range]
    }

    fn set_bytes_lo(&mut self, bytes: &Bytes32) {
        self.bytes_lo_mut().clone_from_slice(bytes);
    }

    // Bytes hi

    fn bytes_hi_mut(&mut self) -> &mut [u8] {
        let range = Self::bytes_hi_range();
        &mut self.buffer_mut()[range]
    }

    fn bytes_hi(&self) -> &[u8] {
        let range = Self::bytes_hi_range();
        &self.buffer()[range]
    }

    fn set_bytes_hi(&mut self, bytes: &Bytes32) {
        self.bytes_hi_mut().clone_from_slice(bytes);
    }
}

impl TryFrom<Buffer> for Node {
    type Error = DeserializeError;

    fn try_from(value: Buffer) -> Result<Self, Self::Error> {
        let node = Self { buffer: value };

        // Validate the node created from the buffer
        Prefix::try_from(node.bytes_prefix()[0])?;

        Ok(node)
    }
}

impl crate::common::Node for Node {
    type Key = Bytes32;

    fn height(&self) -> u32 {
        Node::height(self)
    }

    fn leaf_key(&self) -> Self::Key {
        *Node::leaf_key(self)
    }

    fn is_leaf(&self) -> bool {
        Node::is_leaf(self)
    }
}

impl fmt::Debug for Node {
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

pub(crate) struct StorageNode<'storage, StorageType> {
    storage: &'storage StorageType,
    node: Node,
}

impl<StorageType> Clone for StorageNode<'_, StorageType> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage,
            node: self.node.clone(),
        }
    }
}

impl<'s, StorageType> StorageNode<'s, StorageType> {
    pub fn new(storage: &'s StorageType, node: Node) -> Self {
        Self { node, storage }
    }
}

impl<StorageType> StorageNode<'_, StorageType> {
    pub fn is_leaf(&self) -> bool {
        self.node.is_leaf()
    }

    pub fn is_node(&self) -> bool {
        self.node.is_node()
    }

    pub fn leaf_key(&self) -> &Bytes32 {
        self.node.leaf_key()
    }

    pub fn hash(&self) -> Bytes32 {
        self.node.hash()
    }

    pub fn height(&self) -> u32 {
        self.node.height()
    }

    pub fn into_node(self) -> Node {
        self.node
    }
}

impl<StorageType> StorageNode<'_, StorageType>
where
    StorageType: StorageInspect<NodesTable>,
    StorageType::Error: fmt::Debug,
{
    pub fn left_child(&self) -> Result<Option<Self>, DeserializeError> {
        assert!(self.is_node());
        let key = self.node.left_child_key();
        if key == zero_sum() {
            return Ok(Some(Self::new(self.storage, Node::create_placeholder())));
        }
        let buffer = self.storage.get(key).unwrap();
        Ok(buffer
            .map(|buffer| buffer.into_owned().try_into())
            .transpose()?
            .map(|node| Self::new(self.storage, node)))
    }

    pub fn right_child(&self) -> Result<Option<Self>, DeserializeError> {
        assert!(self.is_node());
        let key = self.node.right_child_key();
        if key == zero_sum() {
            return Ok(Some(Self::new(self.storage, Node::create_placeholder())));
        }
        let buffer = self.storage.get(key).unwrap();
        Ok(buffer
            .map(|buffer| buffer.into_owned().try_into())
            .transpose()?
            .map(|node| Self::new(self.storage, node)))
    }
}

impl<StorageType> crate::common::Node for StorageNode<'_, StorageType> {
    type Key = Bytes32;

    fn height(&self) -> u32 {
        StorageNode::height(self)
    }

    fn leaf_key(&self) -> Self::Key {
        *StorageNode::leaf_key(self)
    }

    fn is_leaf(&self) -> bool {
        StorageNode::is_leaf(self)
    }
}

impl<StorageType> crate::common::ParentNode for StorageNode<'_, StorageType>
where
    StorageType: StorageInspect<NodesTable>,
    StorageType::Error: fmt::Debug,
{
    fn left_child(&self) -> Self {
        StorageNode::left_child(self).unwrap().unwrap()
    }

    fn right_child(&self) -> Self {
        StorageNode::right_child(self).unwrap().unwrap()
    }
}

impl<StorageType> fmt::Debug for StorageNode<'_, StorageType>
where
    StorageType: StorageInspect<NodesTable>,
    StorageType::Error: fmt::Debug,
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
    use crate::{
        common::{error::DeserializeError, Bytes32, Prefix, PrefixError},
        sparse::{hash::sum, zero_sum, Node},
    };

    fn leaf_hash(key: &Bytes32, data: &[u8]) -> Bytes32 {
        let mut buffer = [0; 65];
        buffer[0..1].clone_from_slice(Prefix::Leaf.as_ref());
        buffer[1..33].clone_from_slice(key);
        buffer[33..65].clone_from_slice(&sum(data));
        sum(&buffer)
    }

    #[test]
    fn test_create_leaf_returns_a_valid_leaf() {
        let leaf = Node::create_leaf(&sum(b"LEAF"), &[1u8; 32]);
        assert_eq!(leaf.is_leaf(), true);
        assert_eq!(leaf.is_node(), false);
        assert_eq!(leaf.height(), 0);
        assert_eq!(leaf.prefix(), Prefix::Leaf);
        assert_eq!(leaf.leaf_key(), &sum(b"LEAF"));
        assert_eq!(leaf.leaf_data(), &sum(&[1u8; 32]));
    }

    #[test]
    fn test_create_node_returns_a_valid_node() {
        let left_child = Node::create_leaf(&sum(b"LEFT"), &[1u8; 32]);
        let right_child = Node::create_leaf(&sum(b"RIGHT"), &[1u8; 32]);
        let node = Node::create_node(&left_child, &right_child, 1);
        assert_eq!(node.is_leaf(), false);
        assert_eq!(node.is_node(), true);
        assert_eq!(node.height(), 1);
        assert_eq!(node.prefix(), Prefix::Node);
        assert_eq!(node.left_child_key(), &leaf_hash(&sum(b"LEFT"), &[1u8; 32]));
        assert_eq!(
            node.right_child_key(),
            &leaf_hash(&sum(b"RIGHT"), &[1u8; 32])
        );
    }

    #[test]
    fn test_create_placeholder_returns_a_placeholder_node() {
        let node = Node::create_placeholder();
        assert_eq!(node.is_placeholder(), true);
        assert_eq!(node.hash(), *zero_sum());
    }

    #[test]
    fn test_create_leaf_from_buffer_returns_a_valid_leaf() {
        let mut buffer = [0u8; 69];
        buffer[0..4].clone_from_slice(&0_u32.to_be_bytes());
        buffer[4..5].clone_from_slice(Prefix::Leaf.as_ref());
        buffer[5..37].clone_from_slice(&[1u8; 32]);
        buffer[37..69].clone_from_slice(&[1u8; 32]);

        let node: Node = buffer.try_into().unwrap();
        assert_eq!(node.is_leaf(), true);
        assert_eq!(node.is_node(), false);
        assert_eq!(node.height(), 0);
        assert_eq!(node.prefix(), Prefix::Leaf);
        assert_eq!(node.leaf_key(), &[1u8; 32]);
        assert_eq!(node.leaf_data(), &[1u8; 32]);
    }

    #[test]
    fn test_create_node_from_buffer_returns_a_valid_node() {
        let mut buffer = [0u8; 69];
        buffer[0..4].clone_from_slice(&256_u32.to_be_bytes());
        buffer[4..5].clone_from_slice(Prefix::Node.as_ref());
        buffer[5..37].clone_from_slice(&[1u8; 32]);
        buffer[37..69].clone_from_slice(&[1u8; 32]);

        let node: Node = buffer.try_into().unwrap();
        assert_eq!(node.is_leaf(), false);
        assert_eq!(node.is_node(), true);
        assert_eq!(node.height(), 256);
        assert_eq!(node.prefix(), Prefix::Node);
        assert_eq!(node.left_child_key(), &[1u8; 32]);
        assert_eq!(node.right_child_key(), &[1u8; 32]);
    }

    #[test]
    fn test_create_from_buffer_returns_deserialize_error_if_invalid_prefix() {
        let mut buffer = [0u8; 69];
        buffer[0..4].clone_from_slice(&0_u32.to_be_bytes());
        buffer[4..5].clone_from_slice(&[0x02]);
        buffer[5..37].clone_from_slice(&[1u8; 32]);
        buffer[37..69].clone_from_slice(&[1u8; 32]);

        // Should return Error; prefix 0x02 is does not represent a node or leaf
        let err = Node::try_from(buffer).expect_err("Expected try_from() to be Error; got OK");
        assert!(matches!(
            err,
            DeserializeError::PrefixError(PrefixError::InvalidPrefix(0x02))
        ));
    }

    /// For leaf node `node` of leaf data `d` with key `k`:
    /// ```node.buffer = (0x00, k, h(serialize(d)))```
    #[test]
    fn test_leaf_buffer_returns_expected_buffer() {
        let mut expected_buffer = [0u8; 69];
        expected_buffer[0..4].clone_from_slice(&0_u32.to_be_bytes());
        expected_buffer[4..5].clone_from_slice(Prefix::Leaf.as_ref());
        expected_buffer[5..37].clone_from_slice(&sum(b"LEAF"));
        expected_buffer[37..69].clone_from_slice(&sum(&[1u8; 32]));

        let leaf = Node::create_leaf(&sum(b"LEAF"), &[1u8; 32]);
        let buffer = leaf.buffer();

        assert_eq!(buffer, expected_buffer);
    }

    /// For internal node `node` with children `l` and `r`:
    /// ```node.buffer = (0x01, l.v, r.v)```
    #[test]
    fn test_node_buffer_returns_expected_buffer() {
        let mut expected_buffer = [0u8; 69];
        expected_buffer[0..4].clone_from_slice(&1_u32.to_be_bytes());
        expected_buffer[4..5].clone_from_slice(Prefix::Node.as_ref());
        expected_buffer[5..37].clone_from_slice(&leaf_hash(&sum(b"LEFT"), &[1u8; 32]));
        expected_buffer[37..69].clone_from_slice(&leaf_hash(&sum(b"RIGHT"), &[1u8; 32]));

        let left_child = Node::create_leaf(&sum(b"LEFT"), &[1u8; 32]);
        let right_child = Node::create_leaf(&sum(b"RIGHT"), &[1u8; 32]);
        let node = Node::create_node(&left_child, &right_child, 1);
        let buffer = node.buffer();

        assert_eq!(buffer, expected_buffer);
    }

    /// For leaf node `node` of leaf data `d` with key `k`:
    /// ```node.v = h(0x00, k, h(serialize(d)))```
    #[test]
    fn test_leaf_hash_returns_expected_hash_value() {
        let mut expected_buffer = [0u8; 65];
        expected_buffer[0..1].clone_from_slice(Prefix::Leaf.as_ref());
        expected_buffer[1..33].clone_from_slice(&sum(b"LEAF"));
        expected_buffer[33..65].clone_from_slice(&sum(&[1u8; 32]));
        let expected_value = sum(&expected_buffer);

        let node = Node::create_leaf(&sum(b"LEAF"), &[1u8; 32]);
        let value = node.hash();

        assert_eq!(value, expected_value);
    }

    /// For internal node `node` with children `l` and `r`:
    /// ```node.v = h(0x01, l.v, r.v)```
    #[test]
    fn test_node_hash_returns_expected_hash_value() {
        let mut expected_buffer = [0u8; 65];
        expected_buffer[0..1].clone_from_slice(Prefix::Node.as_ref());
        expected_buffer[1..33].clone_from_slice(&leaf_hash(&sum(b"LEFT"), &[1u8; 32]));
        expected_buffer[33..65].clone_from_slice(&leaf_hash(&sum(b"RIGHT"), &[1u8; 32]));
        let expected_value = sum(&expected_buffer);

        let left_child = Node::create_leaf(&sum(b"LEFT"), &[1u8; 32]);
        let right_child = Node::create_leaf(&sum(b"RIGHT"), &[1u8; 32]);
        let node = Node::create_node(&left_child, &right_child, 1);
        let value = node.hash();

        assert_eq!(value, expected_value);
    }
}

#[cfg(test)]
mod test_storage_node {
    use crate::{
        common::{error::DeserializeError, StorageMap},
        sparse::{hash::sum, merkle_tree::NodesTable, node::BUFFER_SIZE, Node, StorageNode},
    };
    use fuel_storage::StorageMutate;

    #[test]
    fn test_node_left_child_returns_the_left_child() {
        let mut s = StorageMap::<NodesTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let _ = s.insert(&leaf_0.hash(), leaf_0.as_buffer());

        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf_1.hash(), leaf_1.as_buffer());

        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);
        let _ = s.insert(&node_0.hash(), node_0.as_buffer());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.left_child().unwrap().unwrap();

        assert_eq!(child.hash(), leaf_0.hash());
    }

    #[test]
    fn test_node_right_child_returns_the_right_child() {
        let mut s = StorageMap::<NodesTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let _ = s.insert(&leaf_0.hash(), leaf_0.as_buffer());

        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf_1.hash(), leaf_1.as_buffer());

        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);
        let _ = s.insert(&node_0.hash(), node_0.as_buffer());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.right_child().unwrap().unwrap();

        assert_eq!(child.hash(), leaf_1.hash());
    }

    #[test]
    fn test_node_left_child_returns_placeholder_when_key_is_zero_sum() {
        let mut s = StorageMap::<NodesTable>::new();

        let leaf = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf.hash(), leaf.as_buffer());

        let node_0 = Node::create_node(&Node::create_placeholder(), &leaf, 1);
        let _ = s.insert(&node_0.hash(), node_0.as_buffer());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.left_child().unwrap().unwrap();

        assert!(child.node.is_placeholder());
    }

    #[test]
    fn test_node_right_child_returns_placeholder_when_key_is_zero_sum() {
        let mut s = StorageMap::<NodesTable>::new();

        let leaf = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf.hash(), leaf.as_buffer());

        let node_0 = Node::create_node(&leaf, &Node::create_placeholder(), 1);
        let _ = s.insert(&node_0.hash(), node_0.as_buffer());

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.right_child().unwrap().unwrap();

        assert!(child.node.is_placeholder());
    }

    #[test]
    fn test_node_left_child_returns_none_when_key_is_not_found() {
        let s = StorageMap::<NodesTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.left_child().unwrap();

        assert!(child.is_none());
    }

    #[test]
    fn test_node_right_child_returns_none_when_key_is_not_found() {
        let s = StorageMap::<NodesTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let child = storage_node.right_child().unwrap();

        assert!(child.is_none());
    }

    #[test]
    fn test_node_left_child_returns_deserialize_error_when_buffer_is_invalid() {
        let mut s = StorageMap::<NodesTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let _ = s.insert(&leaf_0.hash(), &[255; BUFFER_SIZE]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .left_child()
            .expect_err("Expected left_child() to be Error; got Ok");

        assert!(matches!(err, DeserializeError::PrefixError(_)));
    }

    #[test]
    fn test_node_right_child_returns_deserialize_error_when_buffer_is_invalid() {
        let mut s = StorageMap::<NodesTable>::new();

        let leaf_0 = Node::create_leaf(&sum(b"Hello World"), &[1u8; 32]);
        let leaf_1 = Node::create_leaf(&sum(b"Goodbye World"), &[1u8; 32]);
        let _ = s.insert(&leaf_1.hash(), &[255; BUFFER_SIZE]);
        let node_0 = Node::create_node(&leaf_0, &leaf_1, 1);

        let storage_node = StorageNode::new(&s, node_0);
        let err = storage_node
            .right_child()
            .expect_err("Expected right_child() to be Error; got Ok");

        assert!(matches!(err, DeserializeError::PrefixError(_)));
    }
}
