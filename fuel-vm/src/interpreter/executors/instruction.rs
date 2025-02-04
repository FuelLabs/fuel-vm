use crate::{
    error::{
        InterpreterError,
        IoResult,
        RuntimeError,
    },
    interpreter::{
        EcalHandler,
        ExecutableTransaction,
        Interpreter,
        Memory,
    },
    state::ExecuteState,
    storage::InterpreterStorage,
};

use fuel_asm::{
    Instruction,
    PanicInstruction,
    PanicReason,
    RawInstruction,
    RegId,
};

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    /// Execute the current instruction located in `$m[$pc]`.
    pub fn execute(&mut self) -> Result<ExecuteState, InterpreterError<S::DataError>> {
        let raw_instruction = self.fetch_instruction()?;
        self.instruction_per_inner(raw_instruction)
    }

    /// Reads the current instruction located in `$m[$pc]`,
    /// performing memory boundary checks.
    fn fetch_instruction(&self) -> Result<[u8; 4], InterpreterError<S::DataError>> {
        let pc = self.registers[RegId::PC];

        let raw_instruction: [u8; 4] =
            self.memory().read_bytes(pc).map_err(|reason| {
                InterpreterError::PanicInstruction(PanicInstruction::error(
                    reason,
                    0, // The value is meaningless since fetch was out-of-bounds
                ))
            })?;
        if pc < self.registers[RegId::IS] || pc >= self.registers[RegId::SSP] {
            return Err(InterpreterError::PanicInstruction(PanicInstruction::error(
                PanicReason::MemoryNotExecutable,
                RawInstruction::from_be_bytes(raw_instruction),
            )))
        }
        Ok(raw_instruction)
    }

    /// Execute a provided instruction
    pub fn instruction<R: Into<RawInstruction> + Copy>(
        &mut self,
        raw: R,
    ) -> Result<ExecuteState, InterpreterError<S::DataError>> {
        let raw = raw.into();
        let raw = raw.to_be_bytes();

        self.instruction_per_inner(raw)
    }

    fn instruction_per_inner(
        &mut self,
        raw: [u8; 4],
    ) -> Result<ExecuteState, InterpreterError<S::DataError>> {
        if self.debugger.is_active() {
            let debug = self.eval_debugger_state();
            if !debug.should_continue() {
                return Ok(debug.into())
            }
        }

        self.instruction_inner(raw).map_err(|e| {
            InterpreterError::from_runtime(e, RawInstruction::from_be_bytes(raw))
        })
    }

    fn instruction_inner(
        &mut self,
        raw: [u8; 4],
    ) -> IoResult<ExecuteState, S::DataError> {
        #[cfg(feature = "measure-opcodes")]
        let start = self.clock.raw();
        let instruction = Instruction::try_from(raw)
            .map_err(|_| RuntimeError::from(PanicReason::InvalidInstruction))?;
        #[cfg(feature = "measure-opcodes")]
        let opcode = instruction.opcode() as usize;

        // // TODO additional branch that might be optimized after
        // // https://github.com/FuelLabs/fuel-asm/issues/68
        // if self.is_predicate() && !instruction.opcode().is_predicate_allowed() {
        //     return Err(PanicReason::ContractInstructionNotAllowed.into())
        // }

        let result = instruction.execute(self);
        #[cfg(feature = "measure-opcodes")]
        {
            let end = self.clock.raw();
            self.opcode_times[opcode].0 += self.clock.delta(start, end);
            self.opcode_times[opcode].1 += 1;
        }
        result
    }
}

pub trait Execute<M, S, Tx, Ecal>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    fn execute(
        self,
        interpriter: &mut Interpreter<M, S, Tx, Ecal>,
    ) -> IoResult<ExecuteState, S::DataError>;
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for Instruction
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
{
    fn execute(
        self,
        interpriter: &mut Interpreter<M, S, Tx, Ecal>,
    ) -> IoResult<ExecuteState, S::DataError> {
        match self {
            Instruction::ADD(op) => op.execute(interpriter),
            Instruction::AND(op) => op.execute(interpriter),
            Instruction::DIV(op) => op.execute(interpriter),
            Instruction::EQ(op) => op.execute(interpriter),
            Instruction::EXP(op) => op.execute(interpriter),
            Instruction::GT(op) => op.execute(interpriter),
            Instruction::LT(op) => op.execute(interpriter),
            Instruction::MLOG(op) => op.execute(interpriter),
            Instruction::MROO(op) => op.execute(interpriter),
            Instruction::MOD(op) => op.execute(interpriter),
            Instruction::MOVE(op) => op.execute(interpriter),
            Instruction::MUL(op) => op.execute(interpriter),
            Instruction::NOT(op) => op.execute(interpriter),
            Instruction::OR(op) => op.execute(interpriter),
            Instruction::SLL(op) => op.execute(interpriter),
            Instruction::SRL(op) => op.execute(interpriter),
            Instruction::SUB(op) => op.execute(interpriter),
            Instruction::XOR(op) => op.execute(interpriter),
            Instruction::MLDV(op) => op.execute(interpriter),
            Instruction::RET(op) => op.execute(interpriter),
            Instruction::RETD(op) => op.execute(interpriter),
            Instruction::ALOC(op) => op.execute(interpriter),
            Instruction::MCL(op) => op.execute(interpriter),
            Instruction::MCP(op) => op.execute(interpriter),
            Instruction::MEQ(op) => op.execute(interpriter),
            Instruction::BHSH(op) => op.execute(interpriter),
            Instruction::BHEI(op) => op.execute(interpriter),
            Instruction::BURN(op) => op.execute(interpriter),
            Instruction::CALL(op) => op.execute(interpriter),
            Instruction::CCP(op) => op.execute(interpriter),
            Instruction::CROO(op) => op.execute(interpriter),
            Instruction::CSIZ(op) => op.execute(interpriter),
            Instruction::CB(op) => op.execute(interpriter),
            Instruction::LDC(op) => op.execute(interpriter),
            Instruction::LOG(op) => op.execute(interpriter),
            Instruction::LOGD(op) => op.execute(interpriter),
            Instruction::MINT(op) => op.execute(interpriter),
            Instruction::RVRT(op) => op.execute(interpriter),
            Instruction::SCWQ(op) => op.execute(interpriter),
            Instruction::SRW(op) => op.execute(interpriter),
            Instruction::SRWQ(op) => op.execute(interpriter),
            Instruction::SWW(op) => op.execute(interpriter),
            Instruction::SWWQ(op) => op.execute(interpriter),
            Instruction::TR(op) => op.execute(interpriter),
            Instruction::TRO(op) => op.execute(interpriter),
            Instruction::ECK1(op) => op.execute(interpriter),
            Instruction::ECR1(op) => op.execute(interpriter),
            Instruction::ED19(op) => op.execute(interpriter),
            Instruction::K256(op) => op.execute(interpriter),
            Instruction::S256(op) => op.execute(interpriter),
            Instruction::TIME(op) => op.execute(interpriter),
            Instruction::NOOP(op) => op.execute(interpriter),
            Instruction::FLAG(op) => op.execute(interpriter),
            Instruction::BAL(op) => op.execute(interpriter),
            Instruction::JMP(op) => op.execute(interpriter),
            Instruction::JNE(op) => op.execute(interpriter),
            Instruction::SMO(op) => op.execute(interpriter),
            Instruction::ADDI(op) => op.execute(interpriter),
            Instruction::ANDI(op) => op.execute(interpriter),
            Instruction::DIVI(op) => op.execute(interpriter),
            Instruction::EXPI(op) => op.execute(interpriter),
            Instruction::MODI(op) => op.execute(interpriter),
            Instruction::MULI(op) => op.execute(interpriter),
            Instruction::ORI(op) => op.execute(interpriter),
            Instruction::SLLI(op) => op.execute(interpriter),
            Instruction::SRLI(op) => op.execute(interpriter),
            Instruction::SUBI(op) => op.execute(interpriter),
            Instruction::XORI(op) => op.execute(interpriter),
            Instruction::JNEI(op) => op.execute(interpriter),
            Instruction::LB(op) => op.execute(interpriter),
            Instruction::LW(op) => op.execute(interpriter),
            Instruction::SB(op) => op.execute(interpriter),
            Instruction::SW(op) => op.execute(interpriter),
            Instruction::MCPI(op) => op.execute(interpriter),
            Instruction::GTF(op) => op.execute(interpriter),
            Instruction::MCLI(op) => op.execute(interpriter),
            Instruction::GM(op) => op.execute(interpriter),
            Instruction::MOVI(op) => op.execute(interpriter),
            Instruction::JNZI(op) => op.execute(interpriter),
            Instruction::JMPF(op) => op.execute(interpriter),
            Instruction::JMPB(op) => op.execute(interpriter),
            Instruction::JNZF(op) => op.execute(interpriter),
            Instruction::JNZB(op) => op.execute(interpriter),
            Instruction::JNEF(op) => op.execute(interpriter),
            Instruction::JNEB(op) => op.execute(interpriter),
            Instruction::JI(op) => op.execute(interpriter),
            Instruction::CFEI(op) => op.execute(interpriter),
            Instruction::CFSI(op) => op.execute(interpriter),
            Instruction::CFE(op) => op.execute(interpriter),
            Instruction::CFS(op) => op.execute(interpriter),
            Instruction::PSHL(op) => op.execute(interpriter),
            Instruction::PSHH(op) => op.execute(interpriter),
            Instruction::POPL(op) => op.execute(interpriter),
            Instruction::POPH(op) => op.execute(interpriter),
            Instruction::WDCM(op) => op.execute(interpriter),
            Instruction::WQCM(op) => op.execute(interpriter),
            Instruction::WDOP(op) => op.execute(interpriter),
            Instruction::WQOP(op) => op.execute(interpriter),
            Instruction::WDML(op) => op.execute(interpriter),
            Instruction::WQML(op) => op.execute(interpriter),
            Instruction::WDDV(op) => op.execute(interpriter),
            Instruction::WQDV(op) => op.execute(interpriter),
            Instruction::WDMD(op) => op.execute(interpriter),
            Instruction::WQMD(op) => op.execute(interpriter),
            Instruction::WDAM(op) => op.execute(interpriter),
            Instruction::WQAM(op) => op.execute(interpriter),
            Instruction::WDMM(op) => op.execute(interpriter),
            Instruction::WQMM(op) => op.execute(interpriter),
            Instruction::ECAL(op) => op.execute(interpriter),
            Instruction::BSIZ(op) => op.execute(interpriter),
            Instruction::BLDD(op) => op.execute(interpriter),
            Instruction::ECOP(op) => op.execute(interpriter),
            Instruction::EPAR(op) => op.execute(interpriter),
        }
    }
}

/// Computes nth root of target, rounding down to nearest integer.
/// This function uses the floating point operation to get an approximate solution,
/// but corrects the result using exponentation to check for inaccuracy.
pub fn checked_nth_root(target: u64, nth_root: u64) -> Option<u64> {
    if nth_root == 0 {
        // Zeroth root is not defined
        return None
    }

    if nth_root == 1 || target <= 1 {
        // Corner cases
        return Some(target)
    }

    if nth_root >= target || nth_root > 64 {
        // For any root >= target, result always 1
        // For any n>1, n**64 can never fit into u64
        return Some(1)
    }

    let nth_root = u32::try_from(nth_root).expect("Never loses bits, checked above");

    // Use floating point operation to get an approximation for the starting point.
    // This is at most off by one in either direction.

    #[cfg(feature = "std")]
    let powf = f64::powf;
    #[cfg(not(feature = "std"))]
    let powf = libm::pow;

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let guess = powf(target as f64, (nth_root as f64).recip()) as u64;

    debug_assert!(guess != 0, "This should never occur for {{target, n}} > 1");

    // Check if a value raised to nth_power is below the target value, handling overflow
    // correctly
    let is_nth_power_below_target = |v: u64| match v.checked_pow(nth_root) {
        Some(pow) => target < pow,
        None => true, // v**nth_root >= 2**64 and target < 2**64
    };

    // Compute guess**n to check if the guess is too large.
    // Note that if guess == 1, then g1 == 1 as well, meaning that we will not return
    // here.
    if is_nth_power_below_target(guess) {
        return Some(guess.saturating_sub(1))
    }

    // Check if the initial guess was correct
    let guess_plus_one = guess.checked_add(1).expect(
        "Guess cannot be u64::MAX, as we have taken a root > 2 of a value to get it",
    );
    if is_nth_power_below_target(guess_plus_one) {
        return Some(guess)
    }

    // If not, then the value above must be the correct one.
    Some(guess_plus_one)
}

#[cfg(test)]
mod tests;
