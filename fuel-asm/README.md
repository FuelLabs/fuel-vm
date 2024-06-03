# Fuel ASM

[![build](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-vm/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuel-asm?label=latest)](https://crates.io/crates/fuel-asm)
[![docs](https://docs.rs/fuel-asm/badge.svg)](https://docs.rs/fuel-asm/)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Instruction set for the [FuelVM](https://github.com/FuelLabs/fuel-specs).

## Compile features

- `std`: Unless set, the crate will link to the core-crate instead of the std-crate. More info [here](https://docs.rust-embedded.org/book/intro/no-std.html).
- `serde`: Add support for [serde](https://crates.io/crates/serde) for the types exposed by this crate.

## Example

```rust
use fuel_asm::*;

// A sample program to perform ecrecover
let program = vec![
    op::move_(0x10, 0x01),     // set r[0x10] := $one
    op::slli(0x20, 0x10, 5),   // set r[0x20] := `r[0x10] << 5 == 32`
    op::slli(0x21, 0x10, 6),   // set r[0x21] := `r[0x10] << 6 == 64`
    op::aloc(0x21),            // alloc `r[0x21] == 64` to the heap
    op::addi(0x10, 0x07, 1),   // set r[0x10] := `$hp + 1` (allocated heap)
    op::move_(0x11, 0x04),     // set r[0x11] := $ssp
    op::add(0x12, 0x04, 0x20), // set r[0x12] := `$ssp + r[0x20]`
    op::eck1(0x10, 0x11, 0x12),// recover public key in memory[r[0x10], 64]
    op::ret(0x01),             // return `1`
];

// Convert program to bytes representation
let bytes: Vec<u8> = program.iter().copied().collect();

// A program can be reconstructed from an iterator of bytes
let restored: Result<Vec<Instruction>, _> = fuel_asm::from_bytes(bytes).collect();
assert_eq!(program, restored.unwrap());

// Every instruction can be described as `u32` big-endian bytes
let halfwords: Vec<u32> = program.iter().copied().collect();
let bytes = halfwords.iter().copied().map(u32::to_be_bytes).flatten();
let restored: Result<Vec<Instruction>, _> = fuel_asm::from_bytes(bytes).collect();
assert_eq!(program, restored.unwrap());

// We can also reconstruct the instructions individually
let restored: Result<Vec<Instruction>, _> = fuel_asm::from_u32s(halfwords).collect();
assert_eq!(program, restored.unwrap());

// An instruction is composed by the opcode representation, register IDs and immediate value.
let instruction = program[1];
assert_eq!(instruction.opcode(), Opcode::SLLI);
let slli = match instruction {
    Instruction::SLLI(slli) => slli,
    _ => panic!("unexpected instruction"),
};
let (ra, rb, imm) = slli.unpack();
assert_eq!(u8::from(ra), 0x20);
assert_eq!(u8::from(rb), 0x10);
assert_eq!(u32::from(imm), 5);
```
