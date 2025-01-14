use core::mem::MaybeUninit;

use crate::common::Bytes32;

use jmt::storage::{
    LeafNode,
    Node as JmtNode,
    NodeKey,
};

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
    fn value(&self) -> &Bytes32;
}

impl ChildPrimitiveTrait for ChildPrimitive {
    fn version(&self) -> u64 {
        self.0
    }

    fn value(&self) -> &Bytes32 {
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
        self.child(index).map(|child| *child.value())
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
        self.6.get(0).map(|child| *child.value())
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
pub struct Wrapped<T>(pub T);

impl From<&JmtNode> for Wrapped<Primitive> {
    fn from(node: &JmtNode) -> Self {
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

                internal.children_sorted().for_each(|(_nibble, child)| {
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
                let value_hash = leaf.key_hash();
                let mut value_array: [MaybeUninit<ChildPrimitive>; 16] =
                    [MaybeUninit::uninit(); 16];
                let unused_children_bitmap = 0;
                let unused_leaves_bitmap = 0;

                // TODO: We use the terminology key_hash from JMT, but this should
                // actually be the storage slot
                let value = leaf.key_hash().0;

                unsafe {
                    value_array[0].as_mut_ptr().write((version, value));
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

impl From<&NodeKey> for Wrapped<PrimitiveKey> {
    fn from(node_key: &NodeKey) -> Self {
        let version = node_key.version();
        let jmt_nibble_path = node_key.nibble_path();
        let mut num_nibbles: u8 = 0;

        // Avoid initialization for performance purposes
        let mut nibble_path: [MaybeUninit<u8>; 32] = [MaybeUninit::uninit(); 32];
        let mut current_byte_index = 0;
        let mut should_shift = true;
        for nibble in jmt_nibble_path.nibbles() {
            let mut nibble_as_u8: u8 = nibble.into();

            // If should_shift == true then we are writing into an unitiliazied area of
            // the array
            if should_shift == true {
                // Safety: we access uninitialized memory for writing only.
                unsafe {
                    nibble_as_u8 = nibble_as_u8 << 4;
                    nibble_path[current_byte_index]
                        .as_mut_ptr()
                        .write(nibble_as_u8);
                }
                // Do not advence the byte index
                should_shift = false;
            // In this case, the byte at the current_byte_index has been initialized
            // already, hence we can fetch it from the array.
            } else {
                let previous_nibble =
                    // Safety: the memory has been initialized in the previous iteration of the loop
                    unsafe { nibble_path[current_byte_index].assume_init() };
                let combined_nibble = previous_nibble | nibble_as_u8;
                // Safety: we access uninitialized memory for writing only.
                unsafe {
                    nibble_path[current_byte_index]
                        .as_mut_ptr()
                        .write(combined_nibble);
                }
                current_byte_index += 1;
                should_shift = true;
            }
            num_nibbles += 1;
        }
        // If should_shift == true then current_byte_index has been updated at the last
        // iteration of the loop above and it points to uninitialized memory.
        // Otherwise current_byte_index has been initialised at the last iteration of the
        // loop and the first byte of uninitialised memory in the array is at
        // current_byte_index + 1.
        if should_shift == false {
            current_byte_index += 1
        }

        for i in current_byte_index..size_of::<Bytes32>() {
            // Safety: We access uninitialized memory for writing only.
            unsafe {
                nibble_path[i].as_mut_ptr().write(0);
            }
        }

        // Safety: all the bytes in the nibble_path array have been initialized, and the
        // memory layout of u8 is the same as MaybeUninit<u8>
        let nibble_path: Bytes32 = unsafe { std::mem::transmute(nibble_path) };

        Wrapped((version, num_nibbles, nibble_path))
    }
}
