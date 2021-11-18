use fuel_storage::Storage;
use std::convert::TryInto;
use std::fmt;
use std::mem::size_of;
use std::ops::Range;

use crate::common::{Buffer, Bytes1, Bytes32, LEAF, NODE};
use crate::sparse::hash::sum;
use crate::sparse::zero_sum;

#[derive(Clone)]
pub(crate) struct Node {
    buffer: Buffer,
}

impl Node {
    pub fn create_leaf(key: &[u8], data: &[u8]) -> Self {
        let buffer = Self::default_buffer();
        let mut node = Self { buffer };
        node.set_bytes_prefix(&[LEAF]);
        node.set_bytes_lo(&sum(key));
        node.set_bytes_hi(&sum(data));
        node
    }

    pub fn create_node(left_child_key: &Bytes32, right_child_key: &Bytes32) -> Self {
        let buffer = Self::default_buffer();
        let mut node = Self { buffer };
        node.set_bytes_prefix(&[NODE]);
        node.set_bytes_lo(left_child_key);
        node.set_bytes_hi(right_child_key);
        node
    }

    pub fn create_placeholder() -> Self {
        let buffer = Self::default_buffer();
        Self { buffer }
    }

    pub fn from_buffer(buffer: Buffer) -> Self {
        let node = Self { buffer };
        assert!(node.is_leaf() || node.is_node());
        node
    }

    pub fn prefix(&self) -> u8 {
        self.bytes_prefix()[0]
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
        self.prefix() == LEAF || self.is_placeholder()
    }

    pub fn is_node(&self) -> bool {
        self.prefix() == NODE
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
            sum(self.buffer())
        }
    }

    // PRIVATE

    // PREFIX

    const fn default_buffer() -> Buffer {
        [0; Self::buffer_size()]
    }

    const fn prefix_offset() -> usize {
        0
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

    // BUFFER

    const fn buffer_size() -> usize {
        Self::prefix_size() + Self::bytes_lo_size() + Self::bytes_hi_size()
    }

    // PRIVATE

    fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    fn buffer(&self) -> &[u8] {
        &self.buffer
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

impl crate::common::Node for Node {
    type Key = Bytes32;

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
                .field("Hash", &hex::encode(self.hash()))
                .field("Left child key", &hex::encode(self.left_child_key()))
                .field("Right child key", &hex::encode(self.right_child_key()))
                .finish()
        } else {
            f.debug_struct("Node (Leaf)")
                .field("Hash", &hex::encode(self.hash()))
                .field("Leaf key", &hex::encode(self.leaf_key()))
                .field("Leaf data", &hex::encode(self.leaf_data()))
                .finish()
        }
    }
}

type NodeStorage<'storage, StorageError> =
    dyn 'storage + Storage<Bytes32, Buffer, Error = StorageError>;

#[derive(Clone)]
pub(crate) struct StorageNode<'storage, StorageError> {
    storage: &'storage NodeStorage<'storage, StorageError>,
    node: Node,
}

impl<'a, 'storage, StorageError> StorageNode<'storage, StorageError>
where
    StorageError: std::error::Error + Clone,
{
    pub fn new(storage: &'storage NodeStorage<'storage, StorageError>, node: Node) -> Self {
        Self { node, storage }
    }

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

    pub fn left_child(&self) -> Option<Self> {
        assert!(self.is_node());
        let key = self.node.left_child_key();
        let buffer = self.storage.get(key).unwrap();
        buffer.map(|b| {
            let node = Node::from_buffer(*b);
            Self::new(self.storage, node)
        })
    }

    pub fn right_child(&self) -> Option<Self> {
        assert!(self.node.is_node());
        let key = self.node.right_child_key();
        let buffer = self.storage.get(key).unwrap();
        buffer.map(|b| {
            let node = Node::from_buffer(*b);
            Self::new(self.storage, node)
        })
    }

    pub fn into_node(self) -> Node {
        self.node
    }
}

impl<'storage, StorageError> crate::common::Node for StorageNode<'storage, StorageError>
where
    StorageError: std::error::Error + Clone,
{
    type Key = Bytes32;

    fn leaf_key(&self) -> Self::Key {
        *StorageNode::leaf_key(self)
    }

    fn is_leaf(&self) -> bool {
        StorageNode::is_leaf(self)
    }
}

impl<'storage, StorageError> crate::common::ParentNode for StorageNode<'storage, StorageError>
where
    StorageError: std::error::Error + Clone,
{
    fn left_child(&self) -> Self {
        StorageNode::left_child(self).unwrap()
    }

    fn right_child(&self) -> Self {
        StorageNode::right_child(self).unwrap()
    }
}

impl<'storage, StorageError> fmt::Debug for StorageNode<'storage, StorageError> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.node.is_node() {
            f.debug_struct("StorageNode (Internal)")
                .field("Hash", &hex::encode(self.node.hash()))
                .field("Left child key", &hex::encode(self.node.left_child_key()))
                .field("Right child key", &hex::encode(self.node.right_child_key()))
                .finish()
        } else {
            f.debug_struct("StorageNode (Leaf)")
                .field("Hash", &hex::encode(self.node.hash()))
                .field("Leaf key", &hex::encode(self.node.leaf_key()))
                .field("Leaf data", &hex::encode(self.node.leaf_data()))
                .finish()
        }
    }
}

#[cfg(test)]
mod test_node {
    use crate::common::{LEAF, NODE};
    use crate::sparse::hash::sum;
    use crate::sparse::{zero_sum, Node};

    #[test]
    fn test_create_leaf_returns_a_valid_leaf() {
        let leaf = Node::create_leaf(&[1u8; 32], &[1u8; 32]);
        assert_eq!(leaf.is_leaf(), true);
        assert_eq!(leaf.is_node(), false);
        assert_eq!(leaf.prefix(), LEAF);
        assert_eq!(leaf.leaf_key(), &sum(&[1u8; 32]));
        assert_eq!(leaf.leaf_data(), &sum(&[1u8; 32]));
    }

    #[test]
    fn test_create_node_returns_a_valid_node() {
        let node = Node::create_node(&[1u8; 32], &[1u8; 32]);
        assert_eq!(node.is_leaf(), false);
        assert_eq!(node.is_node(), true);
        assert_eq!(node.prefix(), NODE);
        assert_eq!(node.left_child_key(), &[1u8; 32]);
        assert_eq!(node.right_child_key(), &[1u8; 32]);
    }

    #[test]
    fn test_create_placeholder_returns_a_placeholder_node() {
        let node = Node::create_placeholder();
        assert_eq!(node.is_placeholder(), true);
        assert_eq!(node.hash(), *zero_sum());
    }

    #[test]
    fn test_create_leaf_from_buffer_returns_a_valid_leaf() {
        let mut buffer = [0u8; 65];
        buffer[0..1].clone_from_slice(&[LEAF]);
        buffer[1..33].clone_from_slice(&[1u8; 32]);
        buffer[33..65].clone_from_slice(&[1u8; 32]);

        let node = Node::from_buffer(buffer);
        assert_eq!(node.is_leaf(), true);
        assert_eq!(node.is_node(), false);
        assert_eq!(node.prefix(), LEAF);
        assert_eq!(node.leaf_key(), &[1u8; 32]);
        assert_eq!(node.leaf_data(), &[1u8; 32]);
    }

    #[test]
    fn test_create_node_from_buffer_returns_a_valid_node() {
        let mut buffer = [0u8; 65];
        buffer[0..1].clone_from_slice(&[NODE]);
        buffer[1..33].clone_from_slice(&[1u8; 32]);
        buffer[33..65].clone_from_slice(&[1u8; 32]);

        let node = Node::from_buffer(buffer);
        assert_eq!(node.is_leaf(), false);
        assert_eq!(node.is_node(), true);
        assert_eq!(node.prefix(), NODE);
        assert_eq!(node.left_child_key(), &[1u8; 32]);
        assert_eq!(node.right_child_key(), &[1u8; 32]);
    }

    #[test]
    #[should_panic]
    fn test_create_from_buffer_panics_if_invalid_prefix() {
        let mut buffer = [0u8; 65];
        buffer[0..1].clone_from_slice(&[0x02]);
        buffer[1..33].clone_from_slice(&[1u8; 32]);
        buffer[33..65].clone_from_slice(&[1u8; 32]);

        // Should panic; prefix 0x02 is does not represent a node or leaf
        Node::from_buffer(buffer);
    }

    /// For leaf node `node` of leaf data `d` with key `k`:
    /// ```node.buffer = (0x00, k, h(serialize(d)))```
    #[test]
    fn test_leaf_buffer_returns_expected_buffer() {
        let mut expected_buffer = [0u8; 65];
        expected_buffer[0..1].clone_from_slice(&[LEAF]);
        expected_buffer[1..33].clone_from_slice(&sum(&[1u8; 32]));
        expected_buffer[33..65].clone_from_slice(&sum(&[1u8; 32]));

        let leaf = Node::create_leaf(&[1u8; 32], &[1u8; 32]);
        let buffer = leaf.buffer();

        assert_eq!(buffer, expected_buffer);
    }

    /// For internal node `node` with children `l` and `r`:
    /// ```node.buffer = (0x01, l.v, r.v)```
    #[test]
    fn test_node_buffer_returns_expected_buffer() {
        let mut expected_buffer = [0u8; 65];
        expected_buffer[0..1].clone_from_slice(&[NODE]);
        expected_buffer[1..33].clone_from_slice(&[1u8; 32]);
        expected_buffer[33..65].clone_from_slice(&[1u8; 32]);

        let node = Node::create_node(&[1u8; 32], &[1u8; 32]);
        let buffer = node.buffer();

        assert_eq!(buffer, expected_buffer);
    }

    /// For leaf node `node` of leaf data `d` with key `k`:
    /// ```node.v = h(0x00, k, h(serialize(d)))```
    #[test]
    fn test_leaf_hash_returns_expected_hash_value() {
        let mut expected_buffer = [0u8; 65];
        expected_buffer[0..1].clone_from_slice(&[LEAF]);
        expected_buffer[1..33].clone_from_slice(&sum(&[1u8; 32]));
        expected_buffer[33..65].clone_from_slice(&sum(&[1u8; 32]));
        let expected_value = sum(&expected_buffer);

        let node = Node::create_leaf(&[1u8; 32], &[1u8; 32]);
        let value = node.hash();

        assert_eq!(value, expected_value);
    }

    /// For internal node `node` with children `l` and `r`:
    /// ```node.v = h(0x01, l.v, r.v)```
    #[test]
    fn test_node_hash_returns_expected_hash_value() {
        let mut expected_buffer = [0u8; 65];
        expected_buffer[0..1].clone_from_slice(&[NODE]);
        expected_buffer[1..33].clone_from_slice(&[1u8; 32]);
        expected_buffer[33..65].clone_from_slice(&[1u8; 32]);
        let expected_value = sum(&expected_buffer);

        let node = Node::create_node(&[1u8; 32], &[1u8; 32]);
        let value = node.hash();

        assert_eq!(value, expected_value);
    }
}

#[cfg(test)]
mod test_storage_node {
    use crate::common::{Bytes32, StorageError, StorageMap};
    use crate::sparse::node::Buffer;
    use crate::sparse::{Node, StorageNode};
    use fuel_storage::Storage;

    #[test]
    fn test_node_left_child_returns_the_left_child() {
        let mut s = StorageMap::<Bytes32, Buffer>::new();

        let leaf_0 = Node::create_leaf("Hello World".as_bytes(), &[1u8; 32]);
        let _ = s.insert(&leaf_0.hash(), leaf_0.as_buffer());

        let leaf_1 = Node::create_leaf("Goodbye World".as_bytes(), &[1u8; 32]);
        let _ = s.insert(&leaf_1.hash(), leaf_1.as_buffer());

        let node_0 = Node::create_node(&leaf_0.hash(), &leaf_1.hash());
        let _ = s.insert(&node_0.hash(), node_0.as_buffer());

        let storage_node = StorageNode::<StorageError>::new(&mut s, node_0);
        let child = storage_node.left_child().unwrap();

        assert_eq!(child.hash(), leaf_0.hash());
    }

    #[test]
    fn test_node_right_child_returns_the_right_child() {
        let mut s = StorageMap::<Bytes32, Buffer>::new();

        let leaf_0 = Node::create_leaf("Hello World".as_bytes(), &[1u8; 32]);
        let _ = s.insert(&leaf_0.hash(), leaf_0.as_buffer());

        let leaf_1 = Node::create_leaf("Goodbye World".as_bytes(), &[1u8; 32]);
        let _ = s.insert(&leaf_1.hash(), leaf_1.as_buffer());

        let node_0 = Node::create_node(&leaf_0.hash(), &leaf_1.hash());
        let _ = s.insert(&node_0.hash(), node_0.as_buffer());

        let storage_node = StorageNode::<StorageError>::new(&mut s, node_0);
        let child = storage_node.right_child().unwrap();

        assert_eq!(child.hash(), leaf_1.hash());
    }
}
