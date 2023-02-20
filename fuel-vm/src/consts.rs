//! VM parameters

use fuel_types::{Bytes32, Word};

use std::mem;

/// Register count for checking constraints
pub const VM_REGISTER_COUNT: usize = 64;

/// The number of readable registers.
pub const VM_REGISTER_READ_COUNT: usize = 16;

/// The number of writable registers.
pub const VM_REGISTER_WRITE_COUNT: usize = VM_REGISTER_COUNT - VM_REGISTER_READ_COUNT;

/* MEMORY TYPES */

/// Length of a word, in bytes
pub const WORD_SIZE: usize = mem::size_of::<Word>();

/// Maximum memory in MiB
pub const FUEL_MAX_MEMORY_SIZE: u64 = 64;

/// Maximum VM RAM, in bytes.
pub const VM_MAX_RAM: u64 = 1024 * 1024 * FUEL_MAX_MEMORY_SIZE;

/// Size of the VM memory, in bytes.
pub const VM_MEMORY_SIZE: usize = VM_MAX_RAM as usize;

/// Maximum memory access size, in bytes.
pub const MEM_MAX_ACCESS_SIZE: u64 = VM_MAX_RAM;

/// Tighter of the two bounds for VM_MAX_RAM and usize::MAX
pub const MIN_VM_MAX_RAM_USIZE_MAX: u64 = if VM_MAX_RAM < usize::MAX as u64 {
    VM_MAX_RAM
} else {
    usize::MAX as u64
};

// no limits to heap for now.

/// Offset for the assets balances in VM memory
pub const VM_MEMORY_BALANCES_OFFSET: usize = Bytes32::LEN;

/// Encoded len of a register id in an instruction (unused)
pub const VM_REGISTER_WIDTH: u8 = 6;

/// Empty merkle root for receipts tree
pub const EMPTY_RECEIPTS_MERKLE_ROOT: [u8; 32] = [
    0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9, 0x24, 0x27, 0xae, 0x41,
    0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52, 0xb8, 0x55,
];
