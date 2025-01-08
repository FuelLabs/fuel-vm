use core::mem::MaybeUninit;

use crate::common::Bytes32;

use jmt::storage::Node as JmtNode;

/// ** Child representation in internal node: **
/// | Allocation | Data                       |
/// |------------|----------------------------|
/// | `00 - 08`  | Version (8 bytes, 0x00)    |
/// | `08 - 40`  | hash(Child) (32 bytes)     |
/// |------------|----------------------------|
/// 40 bytes total

pub type ChildPrimitive = (u64, Bytes32);

pub trait ChildPrimitiveTrait {
    fn version(&self) -> u64;
    fn key_hash(&self) -> &Bytes32;
}

impl ChildPrimitiveTrait for ChildPrimitive {
    fn version(&self) -> u64 {
        self.0
    }

    fn key_hash(&self) -> &Bytes32 {
        &self.1
    }
}

/// ** Internal node buffer:**
///
/// | Allocation | Data                                 |
/// |------------|--------------------------------------|
/// | `00 - 01`  | Prefix (1 byte, 0x01)                |
/// | `01 - 02`  | Num nibbles (1 byte)    - removed    |
/// | `02 - 10`  | Version (8 bytes)       - removed    |
/// | `10 - 42`  | hash(Key) (32 bytesP)   - removed    |
/// | `42 - 44`  | Children bitmap (2 bytes)            |
/// | `44 - 46`  | Leaves bitmap (2 bytes)              |
/// | `46 - 686` | Children (16 * 40 bytes)             |
/// |------------|--------------------------------------|
/// 685 bytes total

/// ** Leaf node buffer:**
///
/// | Allocation | Data                                 |
/// |------------|--------------------------------------|
/// | `00 - 01`  | Prefix (1 byte, 0x00)                |
/// | `01 - 02`  | Num nibbles (1 byte) - removed       |
/// | `02 - 10`  | Version (8 bytes)    - removed       |
/// | `10 - 42`  | hash(Key) (32 bytes) - removed       |
/// | `42 - 44`  | Zero (2 bytes)                       |
/// | `44 - 46`  | Zero (2 bytes)                       |
/// | `46 - 64 ` | Zero (8 bytes)                       |
/// | `64 - 96`  | hash(Data) (32 bytes)                |
/// | `96 - 686` | Zero(590 bytes)                      |
/// |------------|--------------------------------------|
/// 686 bytes total
/// The layout of a leaf node is the same as an internal node:
/// the bitmaps are ignored, and the value of the node is
/// stored at the same offset as the first child hash in an internal node.

pub type Primitive = (u8, u8, u64, Bytes32, u16, u16, [ChildPrimitive; 16]);

/// ** Key representation for all nodes: **
/// | Allocation | Data                       |
/// |------------|----------------------------|
/// | `00 - 08`  | Version (8 bytes, 0x00)    |
/// | `08 - 09`  | num_nibbles (1 byte)       |
/// | `08 - 40`  | nibble_path (32 bytes)     |
/// |------------|----------------------------|
/// 41 bytes total

pub type PrimitiveKey = (u64, u8, Bytes32);

pub trait PrimitiveKeyView {
    fn version(&self) -> u64;
    fn num_nibbles(&self) -> u8;
    fn nibble_path(&self) -> &Bytes32;
}

impl PrimitiveKeyView for PrimitiveKey {
    fn version(&self) -> u64 {
        self.0
    }

    fn num_nibbles(&self) -> u8 {
        self.1
    }

    fn nibble_path(&self) -> &Bytes32 {
        &self.2
    }
}

pub trait PrimitiveView {
    fn prefix(&self) -> u8;
    fn key_hash(&self) -> Bytes32;
    fn version(&self) -> u64;
    fn hash(&self) -> &Bytes32;
    fn value(&self) -> Option<Bytes32>;
    fn children(&self) -> Option<[ChildPrimitive; 16]>;
    fn child(&self, index: usize) -> Option<ChildPrimitive>;

    fn key_with_version(&self) -> (u64, Bytes32) {
        (self.version(), self.key_hash())
    }

    fn child_version(&self, index: usize) -> Option<u64> {
        self.child(index).map(|child| child.version())
    }

    fn child_hash(&self, index: usize) -> Option<Bytes32> {
        self.child(index).map(|child| *child.key_hash())
    }
}

impl PrimitiveView for Primitive {
    fn prefix(&self) -> u8 {
        self.0
    }

    fn key_hash(&self) -> Bytes32 {
        self.3
    }

    fn version(&self) -> u64 {
        self.2
    }

    fn hash(&self) -> &Bytes32 {
        &self.3
    }

    fn value(&self) -> Option<Bytes32> {
        self.6.get(0).map(|child| *child.key_hash())
    }

    fn children(&self) -> Option<[ChildPrimitive; 16]> {
        if self.prefix() == 1 {
            Some(self.6)
        } else {
            None
        }
    }

    fn child(&self, index: usize) -> Option<ChildPrimitive> {
        self.children()
            .and_then(|children| children.get(index).copied())
    }
}

// Primitive is a primitive type, hence we cannot implement foreign traits.
// We need to wrap it in a newtype to implement the From trait.
pub struct Wrapped<T>(T);

impl From<JmtNode> for Wrapped<Primitive> {
    fn from(node: JmtNode) -> Self {
        match node {
            JmtNode::Internal(internal) => {
                let prefix: u8 = 0x01;
                // TODO: Remove version, num_nibbles and key_hash from the internal node
                let version: u64 = 0;
                let num_nibbles = 0;
                let key_hash: Bytes32 = Bytes32::default();
                let (children_bitmap, leaves_bitmap) = internal.generate_bitmaps();
                let mut children: [MaybeUninit<ChildPrimitive>; 16] =
                    [MaybeUninit::uninit(); 16];
                let mut current_index: usize = 0;

                internal.children_sorted().for_each(|(nibble, child)| {
                    // Safety: the nibble is guaranteed to be less than 16
                    // https://github.com/penumbra-zone/jmt/blob/d6e9199de78939287c62fc61fb38ea38ff4bac67/src/types/nibble.rs#L26
                    let version = child.version;
                    let key_hash = child.hash;
                    unsafe {
                        children[current_index]
                            .as_mut_ptr()
                            .write((version, key_hash));
                    }
                    current_index += 1
                });

                for i in current_index..16 {
                    unsafe {
                        children[i].as_mut_ptr().write((0, Bytes32::default()));
                    }
                }

                // Safety: all children are initialized, and the
                // layout of ChildPrimitive is the same as MaybeUninit<ChildPrimitive>
                let children = unsafe {
                    let children: [ChildPrimitive; 16] = std::mem::transmute(children);
                    children
                };

                Wrapped((
                    prefix,
                    num_nibbles,
                    version,
                    key_hash,
                    children_bitmap,
                    leaves_bitmap,
                    children,
                ))
            }
            JmtNode::Leaf(leaf) => {
                let prefix: u8 = 0x01;
                // TODO: Remove version, num_nibbles and key_hash from the leaf node
                let num_nibbles = 0;
                let version = 0;
                let key_hash = Bytes32::default();
                let mut value_array: [MaybeUninit<ChildPrimitive>; 16] =
                    [MaybeUninit::uninit(); 16];
                let unused_children_bitmap = 0;
                let unused_leaves_bitmap = 0;

                // TODO: We use the terminology key_hash from JMT, but this should
                // actually be the storage slot
                let key_hash = leaf.key_hash().0;

                unsafe {
                    value_array[0].as_mut_ptr().write((version, key_hash));
                }

                for i in 1..16 {
                    unsafe {
                        value_array[i].as_mut_ptr().write((0, Bytes32::default()));
                    }
                }
                // Safety: all children are initialized, and the
                // layout of ChildPrimitive is the same as MaybeUninit<ChildPrimitive>
                let value_array = unsafe {
                    let value_array: [ChildPrimitive; 16] =
                        std::mem::transmute(value_array);
                    value_array
                };

                Wrapped((
                    prefix,
                    num_nibbles,
                    version,
                    key_hash,
                    unused_children_bitmap,
                    unused_leaves_bitmap,
                    value_array,
                ))
            }
            JmtNode::Null =>
            // TODO: Should we have a primitive format also for null node?
            {
                panic!("Cannot convert Null node to Primitive")
            }
        }
    }
}
