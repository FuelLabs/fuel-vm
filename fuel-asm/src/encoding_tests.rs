#![allow(clippy::iter_cloned_collect)] // https://github.com/rust-lang/rust-clippy/issues/9119

use crate::*;
use fuel_asm as _;
use proptest::prelude::*;
use strum::IntoEnumIterator;

proptest! {
    #[test]
    fn test_instruction_encoding(raw_instruction in 0..=u32::MAX) {
        let ins = Instruction::try_from(raw_instruction);
        prop_assume!(ins.is_ok()); // Only valid instructions are considered
        let ins = ins.unwrap();

        assert_eq!(ins, Instruction::try_from(raw_instruction.to_be_bytes()).unwrap());
        assert_eq!(ins.to_bytes(), raw_instruction.to_be_bytes());
        assert_eq!(ins.opcode() as u8, (raw_instruction >> 24) as u8);
    }
}

/// Go through all possible opcodes and argument position combinations.
/// Verify that those that can be converted to instructions can be converted back and they
/// stay identical.
#[test]
fn validate_all_opcodes() {
    let arg_offsets = [0, 6, 12, 18];

    let mut instructions = Vec::new();
    for mask_pattern in [
        0,
        u32::MAX,
        0b1010_1010_1010_1010_1010_1010_1010_1010,
        0b0101_0101_0101_0101_0101_0101_0101_0101,
    ] {
        for opcode_int in 0..=u8::MAX {
            // Valid opcodes only
            let Ok(op) = Opcode::try_from(opcode_int) else {
                continue
            };
            for regs_in_use in 0..=3 {
                for has_imm in [false, true] {
                    // Construct the instruction
                    let mut raw: RawInstruction = (op as u32) << 24u32;
                    for offset in arg_offsets.iter().take(regs_in_use) {
                        raw |= (mask_pattern & 0b11_1111) << offset;
                    }

                    if has_imm {
                        let imm_bits = 6 * (3 - regs_in_use);
                        raw |= mask_pattern & ((1 << imm_bits) - 1);
                    }

                    let Ok(ins) = Instruction::try_from(raw) else {
                        continue
                    };
                    instructions.push(ins);
                }
            }
        }
    }

    for r in [0, 1, 0b11_1111] {
        for gm_arg in GMArgs::iter() {
            instructions.push(op::gm_args(r, gm_arg));
        }

        for gtf_arg in GTFArgs::iter() {
            instructions.push(op::gtf_args(r, r, gtf_arg));
        }
    }

    let bytes: Vec<u8> = instructions.iter().copied().collect();

    let instructions_from_bytes: Result<Vec<Instruction>, _> =
        from_bytes(bytes.iter().copied()).collect();

    assert_eq!(instructions, instructions_from_bytes.unwrap());

    #[cfg(feature = "serde")]
    for ins in &instructions {
        let ins_ser = bincode::serialize(ins).expect("Failed to serialize opcode");
        let ins_de: Instruction =
            bincode::deserialize(&ins_ser).expect("Failed to serialize opcode");
        assert_eq!(ins, &ins_de);
    }
}

#[test]
fn instruction_try_from_fails_with_invalid_opcode() {
    let unused: u8 = 0xff; // Some unused opcode
    Opcode::try_from(unused).expect_err("The opcode should be unused");
    Instruction::try_from([unused, 0, 0, 0]).expect_err("Invalid opcode should fail");
}

#[test]
fn instruction_try_from_fails_with_reserved_bits_set() {
    let op_with_reserved_part = Opcode::NOOP as u8; // This has reserved bits
    Instruction::try_from((op_with_reserved_part as u32) << 24)
        .expect("Reserved bits zero should succeed");
    for mask in 1..(1 << 24) {
        let raw = (op_with_reserved_part as u32) << 24 | mask;
        Instruction::try_from(raw).expect_err("Reserved bits set should fail");
    }
}

#[test]
fn panic_reason_description() {
    let imm24 = 0xbfffff;

    for r in PanicReason::iter() {
        let b = r as u8;
        let r_p = PanicReason::from(b);
        let w = Word::from(r as u8);
        let r_q = PanicReason::from(u8::try_from(w).unwrap());
        assert_eq!(r, r_p);
        assert_eq!(r, r_q);

        let op = op::ji(imm24);
        let pd = PanicInstruction::error(r, op.into());
        let w = Word::from(pd);
        let pd_p = PanicInstruction::from(w);
        assert_eq!(pd, pd_p);

        #[cfg(feature = "serde")]
        {
            let pd_s = bincode::serialize(&pd).expect("Failed to serialize instruction");
            let pd_s: PanicInstruction =
                bincode::deserialize(&pd_s).expect("Failed to deserialize instruction");

            assert_eq!(pd_s, pd);
        }
    }
}
