use crate::common::node::ParentNode;
use crate::common::Msb;

/// #Path Iterator
///
/// A naturally arising property of binary trees is that a leaf index encodes
/// the unique path needed to traverse from the root of the tree to that leaf.
/// The index's binary representation can be read left to right as a sequence of
/// traversal instructions: a 0 bit means "descend left" and a 1 bit means "
/// descend right". By following the `x` bits composing the index, starting at
/// the root, descending to the left child at each `0`, descending to the right
/// child at each `1`, we arrive at the leaf position, having touched every node
/// position along the path formed by this index. Note that this algorithm does
/// not prescribe how to descend from one node to the next; it describes merely
/// the direction in which to descend at each step.
///
/// Alternatively, this can be interpreted as reading the index's most
/// significant bit (MSB) at an offset `n`: read the `n`th bit to the right of
/// the MSB. Here, `n` is a given step in the tree traversal, starting at 0, and
/// incrementing by 1 at each depth until the leaf is reached. The
/// traversal path is then the list of nodes calculated by traversing the tree
/// using the instruction (`0` or `1`) indicated at `x`<sub>`n`</sub>, where `x`
/// is the index in binary representation, and `n` is the offset for each digit
/// in `x` from the MSB.
///
/// Reversing this path gives us the path from the leaf to the root.
///
/// Imagine a 3-bit integer type `u3` underpinning a tree's leaf indices. 3 bits
/// give our tree a maximum height of 3, and a maximum number of leaf nodes
/// 2<sup>3</sup> = 8. For demonstration, internal nodes are numbered using
/// in-order indices (note that this would require an integer type with 4 bits
/// or more in practice). In-order indexing provides a deterministic way to
/// descend from one node to the next (see [Position](crate::common::Position)).
///
/// ```text
///                             07
///                            /  \
///                           /    \
///                          /      \
///                         /        \
///                        /          \
///                       /            \
///                     03              11
///                    /  \            /  \
///                   /    \          /    \
///                 01      05      09      13
///                /  \    /  \    /  \    /  \
/// In-order idx: 00  02  04  06  08  10  12  14
///     Leaf idx:  0   1   2   3   4   5   6   7
/// ```
///
/// Let us now find the path to leaf with index `6`. In the above diagram, this
/// is the seventh leaf in the leaf layer. A priori, we can see that the path
/// from the root to this leaf is represented by the following list of in-order
/// indices: `07, 11, 13, 12` (note that the leaf index that corresponds to the
/// in-order index `12` is `6`).
///
/// ```text
/// 0d6: u3 = 0b110
///         = Right, Right, Left
/// ```
///
/// Starting at the tree's root at index `07`, we can follow the instructions
/// encoded by the binary representation of leaf `6` (`0b110`). In combination
/// with our in-order index rules for descending nodes, we evaluate the
/// following: 1. The first bit is `1`; move right from `07` to `11`.
/// 2. The next bit is `1`; move right from `11` to `13`.
/// 3. The next and final bit is `0`; move left from `13` to `12`.
///
/// We have arrived at the desired leaf position with in-order index `12` and
/// leaf index `6`. Indeed, following the instructions at each bit has produced
/// the same list of positional indices that we observed earlier: `07, 11, 13,
/// 12`.
pub struct PathIter<T> {
    leaf: T,
    current: Option<(T, T)>,
    current_offset: usize,
}

impl<T> PathIter<T>
where
    T: ParentNode + Clone,
{
    pub fn new(root: &T, leaf: &T) -> Self {
        let initial = (root.clone(), root.clone());

        // The initial offset from the MSB.
        //
        // The offset from the MSB indicates which bit to read when deducing the
        // path from the root to the leaf. As we descend down the tree,
        // increasing the traversal depth, we increment this offset and read the
        // corresponding bit to get the next traversal instruction.
        //
        // In the general case, we start by reading the first bit of the path at
        // offset 0. This happens when the path fills its allocated memory;
        // e.g., a path of 256 instructions is encoded within a 256 bit
        // allocation for the leaf key. This also means that the key size in
        // bits is equal to the maximum height of the tree.
        //
        // In the case that the length of the path is less than the number of
        // bits in the key, the initial offset from the MSB must be augmented to
        // accommodate the shortened path. This occurs when the key is allocated
        // with a larger address space to reduce collisions of node addresses.
        //
        // E.g,
        // With an 8-bit key and heights 1 through 7:
        //
        // Height Depth
        // 7      0                        127                    Offset = Bits - Height = 8 - 7 = 1
        //                                 / \
        //                                /   \
        // ...                          ...   ...
        //                              /       \
        //                             /         \
        // 3       4                  07         247              Offset = Bits - Height = 8 - 3 = 5
        //                           /  \        / \
        //                          /    \     ...  \
        //                         /      \          \
        //                        /        \          \
        //                       /          \          \
        //                      /            \          \
        // 2       5          03              11        251       Offset = Bits - Height = 8 - 2 = 6
        //                   /  \            /  \       / \
        //                  /    \          /    \    ...  \
        // 1       6      01      05      09      13       253    Offset = Bits - Height = 8 - 1 = 7
        //               /  \    /  \    /  \    /  \      / \
        // 0       7    00  02  04  06  08  10  12  14   252 254
        //              00  01  02  03  04  05  06  07   126 127
        //
        let initial_offset = T::key_size_in_bits() - root.height() as usize;
        Self {
            leaf: leaf.clone(),
            current: Some(initial),
            current_offset: initial_offset,
        }
    }
}

impl<T> Iterator for PathIter<T>
where
    T: ParentNode + Clone,
    T::Key: Msb,
{
    type Item = (T, T);

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.current.clone();

        if let Some(ref path_node_side_node) = self.current {
            let path_node = &path_node_side_node.0;
            if !path_node.is_leaf() {
                let path = self.leaf.leaf_key();
                let instruction = path
                    .get_bit_at_index_from_msb(self.current_offset)
                    .expect("Unable to perform path iteration due to invalid indexing!");
                if instruction == 0 {
                    let next = (path_node.left_child(), path_node.right_child());
                    self.current = Some(next);
                } else {
                    let next = (path_node.right_child(), path_node.left_child());
                    self.current = Some(next);
                }
                self.current_offset += 1;
            } else {
                self.current = None;
            }
        }

        value
    }
}

pub trait AsPathIterator<T> {
    fn as_path_iter(&self, leaf: &Self) -> PathIter<T>;
}

impl<T> AsPathIterator<T> for T
where
    T: ParentNode + Clone,
{
    fn as_path_iter(&self, leaf: &Self) -> PathIter<T> {
        PathIter::new(self, leaf)
    }
}

#[cfg(test)]
mod test {
    use crate::common::{AsPathIterator, Bytes8, Node, ParentNode};
    use alloc::vec::Vec;

    #[derive(Debug, Clone, PartialEq)]
    struct TestNode {
        value: u64,
    }

    impl TestNode {
        pub fn in_order_index(&self) -> u64 {
            self.value
        }

        pub fn leaf_index(&self) -> u64 {
            assert!(self.is_leaf());
            self.value / 2
        }

        pub fn from_in_order_index(index: u64) -> Self {
            Self { value: index }
        }
        pub fn from_leaf_index(index: u64) -> Self {
            Self { value: index * 2 }
        }

        pub fn height(&self) -> u32 {
            (!self.in_order_index()).trailing_zeros()
        }

        pub fn is_leaf(&self) -> bool {
            self.in_order_index() % 2 == 0
        }

        fn child(&self, direction: i64) -> Self {
            assert!(!self.is_leaf());
            let shift = 1 << (self.height() - 1);
            let index = self.in_order_index() as i64 + shift * direction;
            Self::from_in_order_index(index as u64)
        }
    }

    impl Node for TestNode {
        type Key = Bytes8;

        fn height(&self) -> u32 {
            TestNode::height(self)
        }

        fn leaf_key(&self) -> Self::Key {
            TestNode::leaf_index(self).to_be_bytes()
        }

        fn is_leaf(&self) -> bool {
            TestNode::is_leaf(self)
        }
    }

    impl ParentNode for TestNode {
        fn left_child(&self) -> Self {
            TestNode::child(self, -1)
        }

        fn right_child(&self) -> Self {
            TestNode::child(self, 1)
        }
    }

    #[test]
    fn test_path_iter_returns_path() {
        //
        //               07
        //              /  \
        //             /    \
        //            /      \
        //           /        \
        //          /          \
        //         /            \
        //       03              11
        //      /  \            /  \
        //     /    \          /    \
        //   01      05      09      13
        //  /  \    /  \    /  \    /  \
        // 00  02  04  06  08  10  12  14
        // 00  01  02  03  04  05  06  07
        //
        type Node = TestNode;
        let root = Node::from_in_order_index(7);

        {
            let leaf = Node::from_leaf_index(0);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(3),
                Node::from_in_order_index(1),
                Node::from_leaf_index(0),
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(1);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(3),
                Node::from_in_order_index(1),
                Node::from_leaf_index(1),
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(2);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(3),
                Node::from_in_order_index(5),
                Node::from_leaf_index(2),
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(3);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(3),
                Node::from_in_order_index(5),
                Node::from_leaf_index(3),
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(4);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(11),
                Node::from_in_order_index(9),
                Node::from_leaf_index(4),
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(5);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(11),
                Node::from_in_order_index(9),
                Node::from_leaf_index(5),
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(6);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(11),
                Node::from_in_order_index(13),
                Node::from_leaf_index(6),
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(7);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(11),
                Node::from_in_order_index(13),
                Node::from_leaf_index(7),
            ];
            assert_eq!(path, expected_path);
        }
    }

    #[test]
    fn test_path_iter_returns_side_nodes() {
        //
        //               07
        //              /  \
        //             /    \
        //            /      \
        //           /        \
        //          /          \
        //         /            \
        //       03              11
        //      /  \            /  \
        //     /    \          /    \
        //   01      05      09      13
        //  /  \    /  \    /  \    /  \
        // 00  02  04  06  08  10  12  14
        // 00  01  02  03  04  05  06  07
        //
        type Node = TestNode;
        let root = Node::from_in_order_index(7); // 2^3 - 1

        {
            let leaf = Node::from_leaf_index(0);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.1);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(11), // Sibling of node 3
                Node::from_in_order_index(5),  // Sibling of node 1
                Node::from_leaf_index(1),      // Sibling of leaf 0
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(1);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.1);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(11), // Sibling of node 3
                Node::from_in_order_index(5),  // Sibling of node 1
                Node::from_leaf_index(0),      // Sibling of leaf 1
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(2);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.1);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(11), // Sibling of node 3
                Node::from_in_order_index(1),  // Sibling of node 5
                Node::from_leaf_index(3),      // Sibling of leaf 2
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(3);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.1);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(11), // Sibling of node 3
                Node::from_in_order_index(1),  // Sibling of node 5
                Node::from_leaf_index(2),      // Sibling of leaf 3
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(4);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.1);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(3),  // Sibling of node 11
                Node::from_in_order_index(13), // Sibling of node 9
                Node::from_leaf_index(5),      // Sibling of leaf 4
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(5);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.1);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(3),  // Sibling of node 11
                Node::from_in_order_index(13), // Sibling of node 9
                Node::from_leaf_index(4),      // Sibling of leaf 5
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(6);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.1);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(3), // Sibling of node 11
                Node::from_in_order_index(9), // Sibling of node 13
                Node::from_leaf_index(7),     // Sibling of leaf 6
            ];
            assert_eq!(path, expected_path);
        }

        {
            let leaf = Node::from_leaf_index(7);
            let iter = root.as_path_iter(&leaf).map(|pair| pair.1);
            let path: Vec<Node> = iter.collect();
            let expected_path = vec![
                Node::from_in_order_index(7),
                Node::from_in_order_index(3), // Sibling of node 11
                Node::from_in_order_index(9), // Sibling of node 13
                Node::from_leaf_index(6),     // Sibling of leaf 7
            ];
            assert_eq!(path, expected_path);
        }
    }

    #[test]
    fn test_path_iter_height_4() {
        type Node = TestNode;
        let root = Node::from_in_order_index(15); // 2^4 - 1
        let leaf = Node::from_leaf_index(4); // 0b0100

        let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
        let path: Vec<Node> = iter.collect();

        let expected_path = vec![
            Node::from_in_order_index(15),
            Node::from_in_order_index(7),
            Node::from_in_order_index(11),
            Node::from_in_order_index(9),
            Node::from_in_order_index(8),
        ];
        assert_eq!(path, expected_path);
    }

    #[test]
    fn test_path_iter_height_8() {
        type Node = TestNode;
        let root = Node::from_in_order_index(255); // 2^8 - 1
        let leaf = Node::from_leaf_index(61); // 0b00111101

        let iter = root.as_path_iter(&leaf).map(|pair| pair.0);
        let path: Vec<Node> = iter.collect();

        let expected_path = vec![
            Node::from_in_order_index(255),
            Node::from_in_order_index(127),
            Node::from_in_order_index(63),
            Node::from_in_order_index(95),
            Node::from_in_order_index(111),
            Node::from_in_order_index(119),
            Node::from_in_order_index(123),
            Node::from_in_order_index(121),
            Node::from_leaf_index(61),
        ];
        assert_eq!(path, expected_path);
    }

    #[test]
    fn test_path_iter_returns_root_root_when_root_is_leaf() {
        type Node = TestNode;
        let root = Node::from_in_order_index(0);
        let leaf = Node::from_leaf_index(0);

        let iter = root.as_path_iter(&leaf);
        let path: Vec<(Node, Node)> = iter.collect();

        let expected_path = vec![(Node::from_in_order_index(0), Node::from_in_order_index(0))];
        assert_eq!(path, expected_path);
    }

    #[test]
    fn test_path_iter_into_path_nodes_and_side_nodes() {
        type Node = TestNode;
        let root = Node::from_in_order_index(7);
        let leaf = Node::from_leaf_index(0);
        let iter = root.as_path_iter(&leaf);
        let (path_nodes, side_nodes): (Vec<Node>, Vec<Node>) = iter.unzip();

        let expected_path_nodes = vec![
            Node::from_in_order_index(7),
            Node::from_in_order_index(3),
            Node::from_in_order_index(1),
            Node::from_leaf_index(0),
        ];
        assert_eq!(path_nodes, expected_path_nodes);

        let expected_side_nodes = vec![
            Node::from_in_order_index(7),
            Node::from_in_order_index(11), // Sibling of node 3
            Node::from_in_order_index(5),  // Sibling of node 1
            Node::from_leaf_index(1),      // Sibling of leaf 0
        ];
        assert_eq!(side_nodes, expected_side_nodes);
    }
}
