use crate::common::{
    node::{
        ChildResult,
        Node,
        ParentNode,
    },
    Bytes8,
    PositionPath,
};
use core::convert::Infallible;

/// # Position
///
/// A `Position` represents a node's position in a binary tree by encapsulating
/// the node's index data. Indices are calculated through in-order traversal of
/// the nodes, starting with the first leaf node. Indexing starts at 0.
///
/// Merkle Trees
///
/// In the context of Merkle trees, trees are constructed "upwards" from leaf
/// nodes. Therefore, indexing is done from the bottom up, starting with the
/// leaves, rather than top down, starting with the root, and we can guarantee a
/// deterministic construction of index data.
///
/// ```text
///               07
///              /  \
///             /    \
///            /      \
///           /        \
///          /          \
///         /            \
///       03              11
///      /  \            /  \
///     /    \          /    \
///   01      05      09      13
///  /  \    /  \    /  \    /  \
/// 00  02  04  06  08  10  12  14
/// ```
///
/// In-order indices can be considered internal to the `Position` struct and are
/// used to facilitate the calculation of positional attributes and the
/// construction of other nodes. Leaf nodes have both an in-order index as part
/// of the tree, and a leaf index determined by its position in the bottom row.
/// Because of the in-order traversal used to calculate the in-order indices,
/// leaf nodes have the property that their in-order index is always equal to
/// their leaf index multiplied by 2.
///
/// ```text
///                    /  \    /  \    /  \    /  \
///     Leaf indices: 00  01  02  03  04  05  06  07
/// In-order indices: 00  02  04  06  08  10  12  14
/// ```
///
/// This allows us to construct a `Position` (and its in-order index) by
/// providing either an in-order index directly or, in the case of a leaf, a
/// leaf index. This functionality is captured by `from_in_order_index()` and
/// `from_leaf_index()` respectively.
///
/// Traversal of a Merkle Tree can be performed by the methods on a given
/// `Position` to retrieve its sibling, parent, or uncle `Position`.
///
/// Merkle Mountain Ranges
///
/// Because the `Position` indices are calculated from in-order traversal
/// starting with the leaves, the deterministic quality of the indices holds
/// true for imbalanced binary trees, including Merkle Mountain Ranges. Consider
/// the following binary tree construction composed of seven leaves (with leaf
/// indices 0 through 6):
///
/// ```text
///       03
///      /  \
///     /    \
///   01      05      09
///  /  \    /  \    /  \
/// 00  02  04  06  08  10  12
/// ```
///
/// Note the absence of internal nodes that would be present in a fully balanced
/// tree: inner nodes with indices 7 and 11 are absent. This is owing to the
/// fact that node indices are calculated deterministically through in-order
/// traversal, not calculated as a sequence.
///
/// Traversal of a Merkle Mountain Range is still done in the same manner as a
/// balanced Merkle tree, using methods to retrieve a `Position's` sibling,
/// parent, or uncle `Position`. However, in such cases, the corresponding
/// sibling or uncle nodes are not guaranteed to exist in the tree.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Position(u64);

const LEFT_CHILD_DIRECTION: i64 = -1;
const RIGHT_CHILD_DIRECTION: i64 = 1;

impl Position {
    pub fn in_order_index(self) -> u64 {
        self.0
    }

    pub fn leaf_index(self) -> u64 {
        assert!(self.is_leaf());
        self.in_order_index() / 2
    }

    /// Construct a position from an in-order index.
    pub fn from_in_order_index(index: u64) -> Self {
        Position(index)
    }

    /// Construct a position from a leaf index. The in-order index corresponding
    /// to the leaf index will always equal the leaf index multiplied by 2.
    pub fn from_leaf_index(index: u64) -> Self {
        Position(index * 2)
    }

    /// The sibling position.
    /// A position shares the same parent and height as its sibling.
    pub fn sibling(self) -> Self {
        let shift = 1 << (self.height() + 1);
        let index = self.in_order_index() as i64 + shift * self.direction();
        Self::from_in_order_index(index as u64)
    }

    /// The parent position.
    /// The parent position has a height less 1 relative to this position.
    pub fn parent(self) -> Self {
        let shift = 1 << self.height();
        let index = self.in_order_index() as i64 + shift * self.direction();
        Self::from_in_order_index(index as u64)
    }

    /// The uncle position.
    /// The uncle position is the sibling of the parent and has a height less 1
    /// relative to this position.
    pub fn uncle(self) -> Self {
        self.parent().sibling()
    }

    /// The left child position.
    /// See [child](Self::child).
    pub fn left_child(self) -> Self {
        self.child(LEFT_CHILD_DIRECTION)
    }

    /// The right child position.
    /// See [child](Self::child).
    pub fn right_child(self) -> Self {
        self.child(RIGHT_CHILD_DIRECTION)
    }

    /// The height of the index in a binary tree.
    /// Leaf nodes represent height 0. A leaf's parent represents height 1.
    /// Height values monotonically increase as you ascend the tree.
    ///
    /// Height is deterministically calculated as the number of trailing zeros
    /// of the complement of the position's index. The following table
    /// demonstrates the relationship between a position's height and the
    /// trailing zeros.
    ///
    /// | Index (Dec) | Index (Bin) | !Index (Bin) | Trailing 0s | Height |
    /// |-------------|-------------|--------------|-------------|--------|
    /// |           0 |        0000 |         1111 |           0 |      0 |
    /// |           2 |        0010 |         1101 |           0 |      0 |
    /// |           4 |        0100 |         1011 |           0 |      0 |
    /// |           1 |        0001 |         1110 |           1 |      1 |
    /// |           5 |        0101 |         1010 |           1 |      1 |
    /// |           9 |        1001 |         0110 |           1 |      1 |
    /// |           3 |        0011 |         1100 |           2 |      2 |
    /// |          11 |        1011 |         0100 |           2 |      2 |
    pub fn height(self) -> u32 {
        (!self.in_order_index()).trailing_zeros()
    }

    /// Whether or not this position represents a leaf node.
    /// Returns `true` if the position is a leaf node.
    /// Returns `false` if the position is an internal node.
    ///
    /// A position is a leaf node if and only if its in-order index is even. A
    /// position is an internal node if and only if its in-order index is
    /// odd.
    pub fn is_leaf(self) -> bool {
        self.in_order_index() % 2 == 0
    }

    /// Whether or not this position represents an internal node.
    /// Returns `false` if the position is a leaf node.
    /// Returns `true` if the position is an internal node.
    ///
    /// When a position is an internal node, the position will have both a left
    /// and right child.
    pub fn is_node(self) -> bool {
        !self.is_leaf()
    }

    /// Given a leaf position and the total count of leaves in a tree, get the
    /// path from this position to the given leaf position. The shape of the
    /// tree is defined by the `leaves_count` parameter and constrains the
    /// path. See [PositionPath](crate::common::PositionPath).
    pub fn path(self, leaf: &Self, leaves_count: u64) -> PositionPath {
        PositionPath::new(self, *leaf, leaves_count)
    }

    // PRIVATE

    /// The child position of the current position given by the direction.
    /// A direction of `-1` denotes the left child. A direction of `+1` denotes
    /// the right child. A child position has a height less 1 than the
    /// current position.
    ///
    /// A child position is calculated as a function of the current position's
    /// index and height, and the supplied direction. The left child
    /// position has the in-order index arriving before the current index;
    /// the right child position has the in-order index arriving after the
    /// current index.
    fn child(self, direction: i64) -> Self {
        assert!(self.is_node());
        let shift = 1 << (self.height() - 1);
        let index = self.in_order_index() as i64 + shift * direction;
        Self::from_in_order_index(index as u64)
    }

    /// Orientation of the position index relative to its parent.
    /// Returns 0 if the index is left of its parent.
    /// Returns 1 if the index is right of its parent.
    ///
    /// The orientation is determined by the reading the `n`th rightmost digit
    /// of the index's binary value, where `n` = the height of the position
    /// `+ 1`. The following table demonstrates the relationships between a
    /// position's index, height, and orientation.
    ///
    /// | Index (Dec) | Index (Bin) | Height | Orientation |
    /// |-------------|-------------|--------|-------------|
    /// |           0 |        0000 |      0 |           0 |
    /// |           2 |        0010 |      0 |           1 |
    /// |           4 |        0100 |      0 |           0 |
    /// |           6 |        0110 |      0 |           1 |
    /// |           1 |        0001 |      1 |           0 |
    /// |           5 |        0101 |      1 |           1 |
    /// |           9 |        1001 |      1 |           0 |
    /// |          13 |        1101 |      1 |           1 |
    fn orientation(self) -> u8 {
        let shift = 1 << (self.height() + 1);
        (self.in_order_index() & shift != 0) as u8
    }

    /// The "direction" to travel to reach the parent node.
    /// Returns +1 if the index is left of its parent.
    /// Returns -1 if the index is right of its parent.
    fn direction(self) -> i64 {
        let scale = self.orientation() as i64 * 2 - 1; // Scale [0, 1] to [-1, 1];
        -scale
    }
}

impl Node for Position {
    type Key = Bytes8;

    fn height(&self) -> u32 {
        Position::height(*self)
    }

    fn leaf_key(&self) -> Self::Key {
        Position::leaf_index(*self).to_be_bytes()
    }

    fn is_leaf(&self) -> bool {
        Position::is_leaf(*self)
    }

    fn is_node(&self) -> bool {
        Position::is_node(*self)
    }
}

impl ParentNode for Position {
    type Error = Infallible;

    fn left_child(&self) -> ChildResult<Self> {
        Ok(Position::left_child(*self))
    }

    fn right_child(&self) -> ChildResult<Self> {
        Ok(Position::right_child(*self))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_in_order_index() {
        assert_eq!(Position::from_in_order_index(0).in_order_index(), 0);
        assert_eq!(Position::from_in_order_index(1).in_order_index(), 1);
        assert_eq!(Position::from_in_order_index(!0u64).in_order_index(), !0u64);
    }

    #[test]
    fn test_from_leaf_index() {
        assert_eq!(Position::from_leaf_index(0).in_order_index(), 0);
        assert_eq!(Position::from_leaf_index(1).in_order_index(), 2);
        assert_eq!(
            Position::from_leaf_index((!0u64) >> 1).in_order_index(),
            !0u64 - 1
        );
    }

    #[test]
    fn test_equality_returns_true_for_two_equal_positions() {
        assert_eq!(Position(0), Position(0));
        assert_eq!(Position::from_in_order_index(0), Position(0));
        assert_eq!(Position::from_leaf_index(1), Position(2));
    }

    #[test]
    fn test_equality_returns_false_for_two_unequal_positions() {
        assert_ne!(Position(0), Position(1));
        assert_ne!(Position::from_in_order_index(0), Position(1));
        assert_ne!(Position::from_leaf_index(0), Position(2));
    }

    #[test]
    fn test_height() {
        assert_eq!(Position(0).height(), 0);
        assert_eq!(Position(2).height(), 0);
        assert_eq!(Position(4).height(), 0);

        assert_eq!(Position(1).height(), 1);
        assert_eq!(Position(5).height(), 1);
        assert_eq!(Position(9).height(), 1);

        assert_eq!(Position(3).height(), 2);
        assert_eq!(Position(11).height(), 2);
        assert_eq!(Position(19).height(), 2);
    }

    #[test]
    fn test_sibling() {
        assert_eq!(Position(0).sibling(), Position(2));
        assert_eq!(Position(2).sibling(), Position(0));

        assert_eq!(Position(1).sibling(), Position(5));
        assert_eq!(Position(5).sibling(), Position(1));

        assert_eq!(Position(3).sibling(), Position(11));
        assert_eq!(Position(11).sibling(), Position(3));
    }

    #[test]
    fn test_parent() {
        assert_eq!(Position(0).parent(), Position(1));
        assert_eq!(Position(2).parent(), Position(1));

        assert_eq!(Position(1).parent(), Position(3));
        assert_eq!(Position(5).parent(), Position(3));

        assert_eq!(Position(3).parent(), Position(7));
        assert_eq!(Position(11).parent(), Position(7));
    }

    #[test]
    fn test_uncle() {
        assert_eq!(Position(0).uncle(), Position(5));
        assert_eq!(Position(2).uncle(), Position(5));
        assert_eq!(Position(4).uncle(), Position(1));
        assert_eq!(Position(6).uncle(), Position(1));

        assert_eq!(Position(1).uncle(), Position(11));
        assert_eq!(Position(5).uncle(), Position(11));
        assert_eq!(Position(9).uncle(), Position(3));
        assert_eq!(Position(13).uncle(), Position(3));
    }

    #[test]
    fn test_left_child() {
        assert_eq!(Position(7).left_child(), Position(3));
        assert_eq!(Position(3).left_child(), Position(1));
        assert_eq!(Position(1).left_child(), Position(0));
        assert_eq!(Position(11).left_child(), Position(9));
        assert_eq!(Position(9).left_child(), Position(8));
    }

    #[test]
    fn test_right_child() {
        assert_eq!(Position(7).right_child(), Position(11));
        assert_eq!(Position(3).right_child(), Position(5));
        assert_eq!(Position(1).right_child(), Position(2));
        assert_eq!(Position(11).right_child(), Position(13));
        assert_eq!(Position(9).right_child(), Position(10));
    }

    #[test]
    fn test_is_leaf() {
        assert_eq!(Position(0).is_leaf(), true);
        assert_eq!(Position(2).is_leaf(), true);
        assert_eq!(Position(4).is_leaf(), true);
        assert_eq!(Position(6).is_leaf(), true);

        assert_eq!(Position(1).is_leaf(), false);
        assert_eq!(Position(5).is_leaf(), false);
        assert_eq!(Position(9).is_leaf(), false);
        assert_eq!(Position(13).is_leaf(), false);
    }

    #[test]
    fn test_is_node() {
        assert_eq!(Position(0).is_node(), false);
        assert_eq!(Position(2).is_node(), false);
        assert_eq!(Position(4).is_node(), false);
        assert_eq!(Position(6).is_node(), false);

        assert_eq!(Position(1).is_node(), true);
        assert_eq!(Position(5).is_node(), true);
        assert_eq!(Position(9).is_node(), true);
        assert_eq!(Position(13).is_node(), true);
    }
}
