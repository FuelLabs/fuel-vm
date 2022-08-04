use super::*;
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

// Ensure none of the opcodes can write to reserved registers
#[quickcheck]
fn cant_write_to_reserved_registers(opcode: u8, fuzz: u16) -> TestResult {
    // opcode || REG_ONE || random fuzz
    let attempt_reserved_reg_access = ((opcode as u32) << 24) | (1 << 18) | ((fuzz >> 4) as u32);
    let instruction = Instruction::new(attempt_reserved_reg_access);
    let opcode = Opcode::new(instruction);
    // skip undefined opcodes
    if matches!(opcode, Opcode::Undefined) {
        return TestResult::discard();
    }

    let mut vm = Interpreter::with_memory_storage();

    vm.init_script(CheckedTransaction::default())
        .expect("Failed to init VM");
    let res = vm.instruction(instruction);

    if writes_to_ra(opcode) {
        // if this opcode writes to $rA, expect an error since we're attempting to use a reserved register
        if !matches!(
            res,
            Err(InterpreterError::PanicInstruction(r)) if r.reason() == &ReservedRegisterNotWritable
        ) {
            return TestResult::error(format!(
                "expected ReservedRegisterNotWritable error {:?}",
                (opcode, &res)
            ));
        }
    } else {
        // Ensure REG_ONE wasn't changed.
        // While not a perfect guarantee against the opcode writing a value
        // to an invalid register, this increases the likelihood of detecting
        // erroneous register access.
        if vm.registers[REG_ONE] != 1 {
            return TestResult::error("reserved register was modified!");
        }

        // throw err if a ReservedRegisterNotWritable err was detected outside our writes_to_ra check
        // This would likely happen if the opcode wasn't properly marked as true in `writes_to_ra`
        if matches!(
            res,
            Err(InterpreterError::PanicInstruction(r)) if r.reason() == &ReservedRegisterNotWritable
        ) {
            return TestResult::error(format!(
                "unexpected ReservedRegisterNotWritable, test configuration may be faulty {:?}",
                (opcode, &res)
            ));
        }
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
        Opcode::SRW(_, _) => true,
        Opcode::SRWQ(_, _) => false,
        Opcode::SWW(_, _) => false,
        Opcode::SWWQ(_, _) => false,
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
    }
}
