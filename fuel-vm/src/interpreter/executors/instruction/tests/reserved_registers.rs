use super::*;
use crate::checked_transaction::IntoChecked;
use fuel_asm::PanicReason::OutOfGas;
use fuel_tx::{ConsensusParameters, Transaction};
use quickcheck::TestResult;
use quickcheck_macros::quickcheck;

// Ensure none of the opcodes can write to reserved registers
#[quickcheck]
fn cant_write_to_reserved_registers(raw_random_instruction: u32) -> TestResult {
    let random_instruction = match Instruction::try_from(raw_random_instruction) {
        Ok(inst) => inst,
        Err(_) => return TestResult::discard(),
    };
    let opcode = random_instruction.opcode();

    // ignore if rA/rB isn't set to writeable register and the opcode should write to that register
    let [ra, rb, _, _] = random_instruction.reg_ids();
    match (ra, rb) {
        (Some(r), _) if writes_to_ra(opcode) && r >= RegId::WRITABLE => return TestResult::discard(),
        (_, Some(r)) if writes_to_rb(opcode) && r >= RegId::WRITABLE => return TestResult::discard(),
        _ => (),
    }

    let mut vm = Interpreter::with_memory_storage();

    let params = ConsensusParameters::default();
    let script = op::ret(0x10).to_bytes().to_vec();
    let block_height = Default::default();
    let tx = Transaction::script(
        0,
        params.max_gas_per_tx,
        Default::default(),
        script,
        vec![],
        vec![],
        vec![],
        vec![],
    );
    let tx = tx
        .into_checked(block_height, &params, vm.gas_costs())
        .expect("failed to check tx");

    vm.init_script(tx).expect("Failed to init VM");
    let res = vm.instruction(raw_random_instruction);

    if writes_to_ra(opcode) || writes_to_rb(opcode) {
        // if this opcode writes to $rA or $rB, expect an error since we're attempting to use a reserved register
        // This assumes that writeable register is validated before other properties of the instruction.
        match res.as_ref().map_err(|e| e.panic_reason()) {
            // expected failure
            Err(Some(ReservedRegisterNotWritable)) => {}
            // Some opcodes may run out of gas if they access too much data.
            // Simply discard these results as an alternative to structural fuzzing that avoids
            // out of gas errors.
            Err(Some(OutOfGas)) => return TestResult::discard(),
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

    // Ensure RegId::ZERO and RegId::ONE were not changed.
    // While not a perfect guarantee against the opcode writing a value
    // to an invalid register, this increases the likelihood of detecting
    // erroneous register access. This is not a comprehensive set of all possible
    // writeable violations but more can be added.
    if vm.registers[RegId::ZERO] != 0 {
        return TestResult::error("reserved register was modified!");
    }
    if vm.registers[RegId::ONE] != 1 {
        return TestResult::error("reserved register was modified!");
    }

    TestResult::passed()
}

// determines whether a given opcode stores a value into $rA
fn writes_to_ra(opcode: Opcode) -> bool {
    match opcode {
        Opcode::ADD => true,
        Opcode::ADDI => true,
        Opcode::AND => true,
        Opcode::ANDI => true,
        Opcode::DIV => true,
        Opcode::DIVI => true,
        Opcode::EQ => true,
        Opcode::EXP => true,
        Opcode::EXPI => true,
        Opcode::GT => true,
        Opcode::LT => true,
        Opcode::MLOG => true,
        Opcode::MROO => true,
        Opcode::MOD => true,
        Opcode::MODI => true,
        Opcode::MOVE => true,
        Opcode::MOVI => true,
        Opcode::MUL => true,
        Opcode::MULI => true,
        Opcode::NOT => true,
        Opcode::OR => true,
        Opcode::ORI => true,
        Opcode::SLL => true,
        Opcode::SLLI => true,
        Opcode::SRL => true,
        Opcode::SRLI => true,
        Opcode::SUB => true,
        Opcode::SUBI => true,
        Opcode::XOR => true,
        Opcode::XORI => true,
        Opcode::JI => false,
        Opcode::JNEI => false,
        Opcode::JNZI => false,
        Opcode::JMP => false,
        Opcode::JNE => false,
        Opcode::JMPF => false,
        Opcode::JMPB => false,
        Opcode::JNZF => false,
        Opcode::JNZB => false,
        Opcode::JNEF => false,
        Opcode::JNEB => false,
        Opcode::RET => false,
        Opcode::RETD => false,
        Opcode::CFEI => false,
        Opcode::CFSI => false,
        Opcode::LB => true,
        Opcode::LW => true,
        Opcode::ALOC => false,
        Opcode::MCL => false,
        Opcode::MCLI => false,
        Opcode::MCP => false,
        Opcode::MCPI => false,
        Opcode::MEQ => true,
        Opcode::SB => false,
        Opcode::SW => false,
        Opcode::BAL => true,
        Opcode::BHSH => false,
        Opcode::BHEI => true,
        Opcode::BURN => false,
        Opcode::CALL => false,
        Opcode::CCP => false,
        Opcode::CROO => false,
        Opcode::CSIZ => true,
        Opcode::CB => false,
        Opcode::LDC => false,
        Opcode::LOG => false,
        Opcode::LOGD => false,
        Opcode::MINT => false,
        Opcode::RVRT => false,
        Opcode::SMO => false,
        Opcode::SCWQ => false,
        Opcode::SRW => true,
        Opcode::SRWQ => false,
        Opcode::SWW => false,
        Opcode::SWWQ => false,
        Opcode::TR => false,
        Opcode::TRO => false,
        Opcode::ECR => false,
        Opcode::K256 => false,
        Opcode::S256 => false,
        Opcode::NOOP => false,
        Opcode::FLAG => false,
        Opcode::GM => true,
        Opcode::GTF => true,
        Opcode::TIME => true,
        Opcode::CFE => false,
        Opcode::CFS => false,
    }
}

// determines whether a given opcode stores a value into $rB
fn writes_to_rb(opcode: Opcode) -> bool {
    match opcode {
        Opcode::ADD => false,
        Opcode::ADDI => false,
        Opcode::AND => false,
        Opcode::ANDI => false,
        Opcode::DIV => false,
        Opcode::DIVI => false,
        Opcode::EQ => false,
        Opcode::EXP => false,
        Opcode::EXPI => false,
        Opcode::GT => false,
        Opcode::LT => false,
        Opcode::MLOG => false,
        Opcode::MROO => false,
        Opcode::MOD => false,
        Opcode::MODI => false,
        Opcode::MOVE => false,
        Opcode::MOVI => false,
        Opcode::MUL => false,
        Opcode::MULI => false,
        Opcode::NOT => false,
        Opcode::OR => false,
        Opcode::ORI => false,
        Opcode::SLL => false,
        Opcode::SLLI => false,
        Opcode::SRL => false,
        Opcode::SRLI => false,
        Opcode::SUB => false,
        Opcode::SUBI => false,
        Opcode::XOR => false,
        Opcode::XORI => false,
        Opcode::JI => false,
        Opcode::JNEI => false,
        Opcode::JNZI => false,
        Opcode::JMP => false,
        Opcode::JNE => false,
        Opcode::JMPF => false,
        Opcode::JMPB => false,
        Opcode::JNZF => false,
        Opcode::JNZB => false,
        Opcode::JNEF => false,
        Opcode::JNEB => false,
        Opcode::RET => false,
        Opcode::RETD => false,
        Opcode::CFEI => false,
        Opcode::CFSI => false,
        Opcode::LB => false,
        Opcode::LW => false,
        Opcode::ALOC => false,
        Opcode::MCL => false,
        Opcode::MCLI => false,
        Opcode::MCP => false,
        Opcode::MCPI => false,
        Opcode::MEQ => false,
        Opcode::SB => false,
        Opcode::SW => false,
        Opcode::BAL => false,
        Opcode::BHSH => false,
        Opcode::BHEI => false,
        Opcode::BURN => false,
        Opcode::CALL => false,
        Opcode::CCP => false,
        Opcode::CROO => false,
        Opcode::CSIZ => false,
        Opcode::CB => false,
        Opcode::LDC => false,
        Opcode::LOG => false,
        Opcode::LOGD => false,
        Opcode::MINT => false,
        Opcode::RVRT => false,
        Opcode::SMO => false,
        Opcode::SCWQ => true,
        Opcode::SRW => true,
        Opcode::SRWQ => true,
        Opcode::SWW => true,
        Opcode::SWWQ => true,
        Opcode::TR => false,
        Opcode::TRO => false,
        Opcode::ECR => false,
        Opcode::K256 => false,
        Opcode::S256 => false,
        Opcode::NOOP => false,
        Opcode::FLAG => false,
        Opcode::GM => false,
        Opcode::GTF => false,
        Opcode::TIME => false,
        Opcode::CFE => false,
        Opcode::CFS => false,
    }
}
