# Fuel ASM

[![build](https://github.com/FuelLabs/fuel-asm/actions/workflows/ci.yml/badge.svg)](https://github.com/FuelLabs/fuel-asm/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/fuel-asm?label=latest)](https://crates.io/crates/fuel-asm)
[![docs](https://docs.rs/fuel-asm/badge.svg)](https://docs.rs/fuel-asm/)
[![discord](https://img.shields.io/badge/chat%20on-discord-orange?&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/xfpK4Pe)

Instruction set for the [FuelVM](https://github.com/FuelLabs/fuel-specs).

## Compile features

- `std`: Unless set, the crate will link to the core-crate instead of the std-crate. More info [here](https://docs.rust-embedded.org/book/intro/no-std.html).
- `serde-types`: Add support for [serde](https://crates.io/crates/serde) for the types exposed by this crate.
- `serde-types-minimal`: Add support for `no-std` [serde](https://crates.io/crates/serde) for the types exposed by this crate.

## Example

```rust
use fuel_asm::*;
use Opcode::*;

// A sample program to perform ecrecover
let program = vec![
    MOVE(0x10, 0x01),      // set r[0x10] := $one
    SLLI(0x20, 0x10, 5),   // set r[0x20] := `r[0x10] << 5 == 32`
    SLLI(0x21, 0x10, 6),   // set r[0x21] := `r[0x10] << 6 == 64`
    ALOC(0x21),            // alloc `r[0x21] == 64` to the heap
    ADDI(0x10, 0x07, 1),   // set r[0x10] := `$hp + 1` (allocated heap)
    MOVE(0x11, 0x04),      // set r[0x11] := $ssp
    ADD(0x12, 0x04, 0x20), // set r[0x12] := `$ssp + r[0x20]`
    ECR(0x10, 0x11, 0x12), // recover public key in memory[r[0x10], 64]
    RET(0x01),             // return `1`
];

// Convert program to bytes representation
let bytes: Vec<u8> = program.iter().copied().collect();

// A program can be reconstructed from an iterator of bytes
let restored = Opcode::from_bytes_iter(bytes.iter().copied());

assert_eq!(program, restored);

// Every instruction can be described as `u32` big-endian bytes
let halfwords: Vec<u32> = program.iter().copied().map(u32::from).collect();
let bytes = halfwords.iter().copied().map(u32::to_be_bytes).flatten();
let restored = Opcode::from_bytes_iter(bytes);

assert_eq!(program, restored);

// We can also reconstruct the instructions individually
let restored: Vec<Opcode> = halfwords.iter().copied().map(Opcode::from).collect();

assert_eq!(program, restored);

// We have an unchecked variant for optimal performance
let restored: Vec<Opcode> = halfwords
    .iter()
    .copied()
    .map(|w| unsafe { Opcode::from_bytes_unchecked(&w.to_be_bytes()) })
    .collect();

assert_eq!(program, restored);

// Finally, we have [`Instruction`] to allow optimal runtime parsing of the components of the
// opcode
//
// `Opcode` itself is only but an abstraction/helper to facilitate visualization, but the VM is
// expected to use raw instructions
let instrs: Vec<Instruction> = program.iter().copied().map(Instruction::from).collect();
let restored: Vec<Opcode> = instrs.iter().copied().map(Opcode::from).collect();

assert_eq!(program, restored);

// An instruction is composed by the opcode representation registers Id and immediate values
assert_eq!(instrs[1].op(), OpcodeRepr::SLLI as u8);
assert_eq!(instrs[1].ra(), 0x20);
assert_eq!(instrs[1].rb(), 0x10);
assert_eq!(instrs[1].imm12(), 5);
```
