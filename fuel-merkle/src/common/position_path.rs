use crate::common::{
    AsPathIterator,
    Position,
    node::Node,
    path_iterator::PathIter,
};

use super::path::Side;

/// # PositionPath
///
/// A PositionPath represents the path of positions created by traversing a
/// binary tree from the root position to the leaf position. The shape of the
/// tree determines path traversal, and can be described accurately by the
/// number of leaves comprising the tree. For example, traversing to the fifth
/// leaf of a balanced eight-leaf tree will generate a different path than
/// traversing to the fifth leaf of an imbalanced five-leaf tree.
///
/// A PositionPath exposes an `iter()` method for performing iteration on this
/// path. Each iteration returns a tuple containing the next position in the
/// path and the corresponding side node. Because the tree may be imbalanced, as
/// described by the path's `leaves_count` parameter, the side node may not
/// necessarily be the direct sibling of the path node; rather, it can be a
/// position at a lower spot in the tree altogether.
pub struct PositionPath {
    root: Position,
    leaf: Position,
    leaves_count: u64,
}

impl PositionPath {
    pub fn new(root: Position, leaf: Position, leaves_count: u64) -> Self {
        debug_assert!(leaves_count > 0);
        Self {
            root,
            leaf,
            leaves_count,
        }
    }

    pub fn iter(self) -> PositionPathIter {
        PositionPathIter::new(self.root, self.leaf, self.leaves_count)
    }
}

pub struct PositionPathIter {
    rightmost_position: Position,
    current_side_node: Option<Position>,
    path_iter: PathIter<Position>,
}

impl PositionPathIter {
    /// Panics if leaves_count is zero, as the tree is not valid
    pub fn new(root: Position, leaf: Position, leaves_count: u64) -> Self {
        Self {
            rightmost_position: Position::from_leaf_index(
                leaves_count
                    .checked_sub(1)
                    .expect("Path to a tree without leaves"),
            )
            .unwrap(),
            current_side_node: None,
            path_iter: root.as_path_iter(&leaf.leaf_key()),
        }
    }
}

impl Iterator for PositionPathIter {
    type Item = (Position, Position);

    fn next(&mut self) -> Option<Self::Item> {
        // Find the next set of path and side positions by iterating from the
        // given root position to the given leaf position and evaluating each
        // position against the tree described by the leaves count.
        let iter = self.path_iter.by_ref().map(|(path, side)| {
            // SAFETY: Path iteration over positions is infallible. Path
            // positions and side positions are both guaranteed to be valid in
            // this context.
            (path.unwrap(), side.unwrap())
        });
        for (path, side) in iter {
            let mut side = Position::from_in_order_index(side);
            // To determine if the position is in the tree, we observe that the
            // highest in-order index belongs to the tree's rightmost leaf
            // position (as defined by the `leaves_count` parameter) and that
            // all path nodes will have an in-order index less than or equal to
            // the in-order index of this rightmost leaf position. If a path
            // position has an in-order index greater than that of the rightmost
            // leaf position, it is invalid in the context of this tree and must
            // be discarded. However, the corresponding side node is valid (or
            // is the ancestor of a valid side node) and represents the side
            // node of a deeper path position that is also valid (i.e., has
            // in-order index less than or equal to that of the rightmost leaf).
            // We can save reference to it now so that we can generate the path
            // node and side node pair later, once both nodes are encountered.
            if path.in_order_index() <= self.rightmost_position.in_order_index() {
                // If we previously encountered a side node corresponding to an
                // invalid path node, we observe that the next valid path node
                // always pairs with this side node. Once the path and side node
                // have been paired, we continue to pair path and side nodes
                // normally.
                if let Some(node) = self.current_side_node.take() {
                    side = node;
                }

                // A side node with an in-order index greater than the index of
                // the rightmost leaf position is invalid and does not exist in
                // the context of this tree. The invalid side node will always
                // be the ancestor of the correct side node. Furthermore, the
                // correct side node will always be a leftward descendent of
                // this invalid side node.
                while side.in_order_index() > self.rightmost_position.in_order_index() {
                    side = side.child(Side::Left).expect("Verified above");
                }

                return Some((path, side))
            } else {
                // If the path node is invalid, save reference to the
                // corresponding side node.
                if self.current_side_node.is_none() {
                    self.current_side_node = Some(side);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    use crate::common::Position;
    use alloc::vec::Vec;

    #[test]
    fn test_path_set_returns_path_and_side_nodes_for_1_leaf() {
        let root = Position::from_in_order_index(0);
        let leaf = Position::from_leaf_index_unwrap(0);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 1).iter().unzip();
        let expected_path = [Position::from_in_order_index(0)];
        let expected_side = [Position::from_in_order_index(0)];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);
    }

    #[test]
    fn test_path_set_returns_path_and_side_nodes_for_4_leaves() {
        //       03
        //      /  \
        //     /    \
        //   01      05
        //  /  \    /  \
        // 00  02  04  06
        // 00  01  02  03

        let root = Position::from_in_order_index(3);

        let leaf = Position::from_leaf_index_unwrap(0);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 4).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(0),
        ];
        let expected_side = [
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(2),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(1);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 4).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(2),
        ];
        let expected_side = [
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(0),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(2);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 4).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(4),
        ];
        let expected_side = [
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(6),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(3);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 4).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(6),
        ];
        let expected_side = [
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(4),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);
    }

    #[test]
    fn test_path_set_returns_path_and_side_nodes_for_5_leaves() {
        //          07
        //         /  \
        //       03    \
        //      /  \    \
        //     /    \    \
        //   01      05   \
        //  /  \    /  \   \
        // 00  02  04  06  08
        // 00  01  02  03  04

        let root = Position::from_in_order_index(7);

        let leaf = Position::from_leaf_index_unwrap(0);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 5).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(0),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(8),
            Position::from_in_order_index(5),
            Position::from_in_order_index(2),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(1);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 5).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(2),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(8),
            Position::from_in_order_index(5),
            Position::from_in_order_index(0),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(2);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 5).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(4),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(8),
            Position::from_in_order_index(1),
            Position::from_in_order_index(6),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(3);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 5).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(6),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(8),
            Position::from_in_order_index(1),
            Position::from_in_order_index(4),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(4);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 5).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(8),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);
    }

    #[test]
    fn test_path_set_returns_path_and_side_nodes_for_6_leaves() {
        //            07
        //           /  \
        //          /    \
        //         /      \
        //       03        \
        //      /  \        \
        //     /    \        \
        //   01      05      09
        //  /  \    /  \    /  \
        // 00  02  04  06  08  10
        // 00  01  02  03  04  05

        let root = Position::from_in_order_index(7);

        let leaf = Position::from_leaf_index_unwrap(0);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 6).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(0),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(9),
            Position::from_in_order_index(5),
            Position::from_in_order_index(2),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(1);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 6).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(2),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(9),
            Position::from_in_order_index(5),
            Position::from_in_order_index(0),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(2);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 6).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(4),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(9),
            Position::from_in_order_index(1),
            Position::from_in_order_index(6),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(3);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 6).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(6),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(9),
            Position::from_in_order_index(1),
            Position::from_in_order_index(4),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(4);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 6).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(9),
            Position::from_in_order_index(8),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(10),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(5);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 6).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(9),
            Position::from_in_order_index(10),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(8),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);
    }

    #[test]
    fn test_path_set_returns_path_and_side_nodes_for_7_leaves() {
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
        //   01      05      09      \
        //  /  \    /  \    /  \      \
        // 00  02  04  06  08  10     12
        // 00  01  02  03  04  05     06

        let root = Position::from_in_order_index(7);

        let leaf = Position::from_leaf_index_unwrap(0);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 7).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(0),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(11),
            Position::from_in_order_index(5),
            Position::from_in_order_index(2),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(1);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 7).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(1),
            Position::from_in_order_index(2),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(11),
            Position::from_in_order_index(5),
            Position::from_in_order_index(0),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(2);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 7).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(4),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(11),
            Position::from_in_order_index(1),
            Position::from_in_order_index(6),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(3);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 7).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(5),
            Position::from_in_order_index(6),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(11),
            Position::from_in_order_index(1),
            Position::from_in_order_index(4),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(4);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 7).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(11),
            Position::from_in_order_index(9),
            Position::from_in_order_index(8),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(12),
            Position::from_in_order_index(10),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(5);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 7).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(11),
            Position::from_in_order_index(9),
            Position::from_in_order_index(10),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(12),
            Position::from_in_order_index(8),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);

        let leaf = Position::from_leaf_index_unwrap(6);
        let (path_positions, side_positions): (Vec<Position>, Vec<Position>) =
            root.path(&leaf, 7).iter().unzip();
        let expected_path = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(11),
            Position::from_in_order_index(12),
        ];
        let expected_side = [
            Position::from_in_order_index(7),
            Position::from_in_order_index(3),
            Position::from_in_order_index(9),
        ];
        assert_eq!(path_positions, expected_path);
        assert_eq!(side_positions, expected_side);
    }
}
