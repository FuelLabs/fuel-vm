use super::*;
use fuel_asm::PanicReason::OutOfGas;
use fuel_tx::{ConsensusParameters, IntoChecked, Transaction};
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

// Ensure none of the opcodes can write to reserved registers
#[quickcheck]
fn cant_write_to_reserved_registers(raw_random_instruction: u32) -> TestResult {
    let random_instruction = Instruction::new(raw_random_instruction);
    let opcode = Opcode::new(random_instruction);
    // skip undefined opcodes
    if matches!(opcode, Opcode::Undefined) {
        return TestResult::discard();
    }
    // ignore if rA/rB isn't set to writeable register and the opcode should write to that register
    if (writes_to_ra(opcode) && random_instruction.ra() >= REG_WRITABLE)
        || (writes_to_rb(opcode) && random_instruction.rb() >= REG_WRITABLE)
    {
        return TestResult::discard();
    }

    let mut vm = Interpreter::with_memory_storage();

    let params = ConsensusParameters::default();
    let script = Opcode::RET(0x10).to_bytes().to_vec();
    let block_height = 0;
    let tx = Transaction::script(0, params.max_gas_per_tx, 0, script, vec![], vec![], vec![], vec![]);
    let tx = tx.into_checked(block_height, &params).expect("failed to check tx");

    vm.init_script(tx).expect("Failed to init VM");
    let res = vm.instruction(random_instruction);

    if writes_to_ra(opcode) || writes_to_rb(opcode) {
        // if this opcode writes to $rA or $rB, expect an error since we're attempting to use a reserved register
        // This assumes that writeable register is validated before other properties of the instruction.
        match res {
            Err(InterpreterError::PanicInstruction(r)) if r.reason() == &ReservedRegisterNotWritable => {
                // expected failure
            }
            Err(InterpreterError::PanicInstruction(r)) if r.reason() == &OutOfGas => {
                // Some opcodes may run out of gas if they access too much data.
                // Simply discard these results as an alternative to structural fuzzing that avoids
                // out of gas errors.
                return TestResult::discard();
            }
            _ => {
                return TestResult::error(format!(
                    "expected ReservedRegisterNotWritable error {:?}",
                    (opcode, &res)
                ));
            }
        }
    } else if matches!(
        res,
        Err(InterpreterError::PanicInstruction(r)) if r.reason() == &ReservedRegisterNotWritable
    ) {
        // throw err if a ReservedRegisterNotWritable err was detected outside our writes_to_ra/b check
        // This would likely happen if the opcode wasn't properly marked as true in `writes_to_ra/b`
        return TestResult::error(format!(
            "unexpected ReservedRegisterNotWritable, test configuration may be faulty {:?}",
            (opcode, &res)
        ));
    }

    // Ensure REG_ZERO and REG_ONE were not changed.
    // While not a perfect guarantee against the opcode writing a value
    // to an invalid register, this increases the likelihood of detecting
    // erroneous register access. This is not a comprehensive set of all possible
    // writeable violations but more can be added.
    if vm.registers[REG_ZERO] != 0 {
        return TestResult::error("reserved register was modified!");
    }
    if vm.registers[REG_ONE] != 1 {
        return TestResult::error("reserved register was modified!");
    }

    TestResult::passed()
}

// determines whether a given opcode stores a value into $rA
fn writes_to_ra(opcode: Opcode) -> bool {
    match opcode {
        Opcode::ADD(_, _, _) => true,
        Opcode::ADDI(_, _, _) => true,
        Opcode::AND(_, _, _) => true,
        Opcode::ANDI(_, _, _) => true,
        Opcode::DIV(_, _, _) => true,
        Opcode::DIVI(_, _, _) => true,
        Opcode::EQ(_, _, _) => true,
        Opcode::EXP(_, _, _) => true,
        Opcode::EXPI(_, _, _) => true,
        Opcode::GT(_, _, _) => true,
        Opcode::LT(_, _, _) => true,
        Opcode::MLOG(_, _, _) => true,
        Opcode::MROO(_, _, _) => true,
        Opcode::MOD(_, _, _) => true,
        Opcode::MODI(_, _, _) => true,
        Opcode::MOVE(_, _) => true,
        Opcode::MOVI(_, _) => true,
        Opcode::MUL(_, _, _) => true,
        Opcode::MULI(_, _, _) => true,
        Opcode::NOT(_, _) => true,
        Opcode::OR(_, _, _) => true,
        Opcode::ORI(_, _, _) => true,
        Opcode::SLL(_, _, _) => true,
        Opcode::SLLI(_, _, _) => true,
        Opcode::SRL(_, _, _) => true,
        Opcode::SRLI(_, _, _) => true,
        Opcode::SUB(_, _, _) => true,
        Opcode::SUBI(_, _, _) => true,
        Opcode::XOR(_, _, _) => true,
        Opcode::XORI(_, _, _) => true,
        Opcode::JI(_) => false,
        Opcode::JNEI(_, _, _) => false,
        Opcode::JNZI(_, _) => false,
        Opcode::JMP(_) => false,
        Opcode::JNE(_, _, _) => false,
        Opcode::RET(_) => false,
        Opcode::RETD(_, _) => false,
        Opcode::CFEI(_) => false,
        Opcode::CFSI(_) => false,
        Opcode::LB(_, _, _) => true,
        Opcode::LW(_, _, _) => true,
        Opcode::ALOC(_) => false,
        Opcode::MCL(_, _) => false,
        Opcode::MCLI(_, _) => false,
        Opcode::MCP(_, _, _) => false,
        Opcode::MCPI(_, _, _) => false,
        Opcode::MEQ(_, _, _, _) => true,
        Opcode::SB(_, _, _) => false,
        Opcode::SW(_, _, _) => false,
        Opcode::BAL(_, _, _) => true,
        Opcode::BHSH(_, _) => false,
        Opcode::BHEI(_) => true,
        Opcode::BURN(_) => false,
        Opcode::CALL(_, _, _, _) => false,
        Opcode::CCP(_, _, _, _) => false,
        Opcode::CROO(_, _) => false,
        Opcode::CSIZ(_, _) => true,
        Opcode::CB(_) => false,
        Opcode::LDC(_, _, _) => false,
        Opcode::LOG(_, _, _, _) => false,
        Opcode::LOGD(_, _, _, _) => false,
        Opcode::MINT(_) => false,
        Opcode::RVRT(_) => false,
        Opcode::SMO(_, _, _, _) => false,
        Opcode::SCWQ(_, _, _) => false,
        Opcode::SRW(_, _, _) => true,
        Opcode::SRWQ(_, _, _, _) => false,
        Opcode::SWW(_, _, _) => false,
        Opcode::SWWQ(_, _, _, _) => false,
        Opcode::TR(_, _, _) => false,
        Opcode::TRO(_, _, _, _) => false,
        Opcode::ECR(_, _, _) => false,
        Opcode::K256(_, _, _) => false,
        Opcode::S256(_, _, _) => false,
        Opcode::NOOP => false,
        Opcode::FLAG(_) => false,
        Opcode::GM(_, _) => true,
        Opcode::GTF(_, _, _) => true,
        Opcode::Undefined => false,
        Opcode::TIME(_, _) => true,
    }
}

// determines whether a given opcode stores a value into $rB
fn writes_to_rb(opcode: Opcode) -> bool {
    match opcode {
        Opcode::ADD(_, _, _) => false,
        Opcode::ADDI(_, _, _) => false,
        Opcode::AND(_, _, _) => false,
        Opcode::ANDI(_, _, _) => false,
        Opcode::DIV(_, _, _) => false,
        Opcode::DIVI(_, _, _) => false,
        Opcode::EQ(_, _, _) => false,
        Opcode::EXP(_, _, _) => false,
        Opcode::EXPI(_, _, _) => false,
        Opcode::GT(_, _, _) => false,
        Opcode::LT(_, _, _) => false,
        Opcode::MLOG(_, _, _) => false,
        Opcode::MROO(_, _, _) => false,
        Opcode::MOD(_, _, _) => false,
        Opcode::MODI(_, _, _) => false,
        Opcode::MOVE(_, _) => false,
        Opcode::MOVI(_, _) => false,
        Opcode::MUL(_, _, _) => false,
        Opcode::MULI(_, _, _) => false,
        Opcode::NOT(_, _) => false,
        Opcode::OR(_, _, _) => false,
        Opcode::ORI(_, _, _) => false,
        Opcode::SLL(_, _, _) => false,
        Opcode::SLLI(_, _, _) => false,
        Opcode::SRL(_, _, _) => false,
        Opcode::SRLI(_, _, _) => false,
        Opcode::SUB(_, _, _) => false,
        Opcode::SUBI(_, _, _) => false,
        Opcode::XOR(_, _, _) => false,
        Opcode::XORI(_, _, _) => false,
        Opcode::JI(_) => false,
        Opcode::JNEI(_, _, _) => false,
        Opcode::JNZI(_, _) => false,
        Opcode::JMP(_) => false,
        Opcode::JNE(_, _, _) => false,
        Opcode::RET(_) => false,
        Opcode::RETD(_, _) => false,
        Opcode::CFEI(_) => false,
        Opcode::CFSI(_) => false,
        Opcode::LB(_, _, _) => false,
        Opcode::LW(_, _, _) => false,
        Opcode::ALOC(_) => false,
        Opcode::MCL(_, _) => false,
        Opcode::MCLI(_, _) => false,
        Opcode::MCP(_, _, _) => false,
        Opcode::MCPI(_, _, _) => false,
        Opcode::MEQ(_, _, _, _) => false,
        Opcode::SB(_, _, _) => false,
        Opcode::SW(_, _, _) => false,
        Opcode::BAL(_, _, _) => false,
        Opcode::BHSH(_, _) => false,
        Opcode::BHEI(_) => false,
        Opcode::BURN(_) => false,
        Opcode::CALL(_, _, _, _) => false,
        Opcode::CCP(_, _, _, _) => false,
        Opcode::CROO(_, _) => false,
        Opcode::CSIZ(_, _) => false,
        Opcode::CB(_) => false,
        Opcode::LDC(_, _, _) => false,
        Opcode::LOG(_, _, _, _) => false,
        Opcode::LOGD(_, _, _, _) => false,
        Opcode::MINT(_) => false,
        Opcode::RVRT(_) => false,
        Opcode::SMO(_, _, _, _) => false,
        Opcode::SCWQ(_, _, _) => true,
        Opcode::SRW(_, _, _) => true,
        Opcode::SRWQ(_, _, _, _) => true,
        Opcode::SWW(_, _, _) => true,
        Opcode::SWWQ(_, _, _, _) => true,
        Opcode::TR(_, _, _) => false,
        Opcode::TRO(_, _, _, _) => false,
        Opcode::ECR(_, _, _) => false,
        Opcode::K256(_, _, _) => false,
        Opcode::S256(_, _, _) => false,
        Opcode::NOOP => false,
        Opcode::FLAG(_) => false,
        Opcode::GM(_, _) => false,
        Opcode::GTF(_, _, _) => false,
        Opcode::Undefined => false,
        Opcode::TIME(_, _) => false,
    }
}
