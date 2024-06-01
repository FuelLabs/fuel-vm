#![allow(clippy::iter_cloned_collect)] // https://github.com/rust-lang/rust-clippy/issues/9119

use crate::*;
use fuel_asm as _;
use strum::IntoEnumIterator;

#[test]
#[cfg(test)]
fn opcode() {
    // values picked to test edge cases
    let r = RegId::new_checked(0x2d).unwrap();
    let imm12 = 0x0bfd;
    let imm18 = 0x02fffd;
    let imm24 = 0xbffffd;

    let mut instructions = Vec::new();

    for opcode_int in 0..64 {
        let Ok(op) = Opcode::try_from(opcode_int) else {
            continue
        };

        instructions.push(op.test_construct(r, r, r, r, imm12));
        instructions.push(op.test_construct(r, r, r, r, imm18));
        instructions.push(op.test_construct(r, r, r, r, imm24));
    }

    for gm_arg in GMArgs::iter() {
        instructions.push(op::gm_args(r, gm_arg));
    }

    for gtf_arg in GTFArgs::iter() {
        instructions.push(op::gtf_args(r, r, gtf_arg));
    }

    // Pad to even length
    if instructions.len() % 2 != 0 {
        instructions.push(op::noop());
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
