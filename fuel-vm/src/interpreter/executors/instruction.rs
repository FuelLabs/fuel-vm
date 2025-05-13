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
    verification::Verifier,
};

use fuel_asm::{
    Instruction,
    PanicInstruction,
    PanicReason,
    RawInstruction,
    RegId,
};

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    V: Verifier,
{
    /// Execute the current instruction located in `$m[$pc]`.
    pub fn execute<const PREDICATE: bool>(
        &mut self,
    ) -> Result<ExecuteState, InterpreterError<S::DataError>> {
        let raw_instruction = self.fetch_instruction()?;
        self.instruction_per_inner::<PREDICATE>(raw_instruction)
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
    pub fn instruction<R, const PREDICATE: bool>(
        &mut self,
        raw: R,
    ) -> Result<ExecuteState, InterpreterError<S::DataError>>
    where
        R: Into<RawInstruction> + Copy,
    {
        let raw = raw.into();
        let raw = raw.to_be_bytes();

        self.instruction_per_inner::<PREDICATE>(raw)
    }

    fn instruction_per_inner<const PREDICATE: bool>(
        &mut self,
        raw: [u8; 4],
    ) -> Result<ExecuteState, InterpreterError<S::DataError>> {
        if self.debugger.is_active() {
            let debug = self.eval_debugger_state();
            if !debug.should_continue() {
                return Ok(debug.into())
            }
        }

        self.instruction_inner::<PREDICATE>(raw).map_err(|e| {
            InterpreterError::from_runtime(e, RawInstruction::from_be_bytes(raw))
        })
    }

    fn instruction_inner<const PREDICATE: bool>(
        &mut self,
        raw: [u8; 4],
    ) -> IoResult<ExecuteState, S::DataError> {
        let instruction = Instruction::try_from(raw)
            .map_err(|_| RuntimeError::from(PanicReason::InvalidInstruction))?;

        if PREDICATE {
            // TODO additional branch that might be optimized after
            // https://github.com/FuelLabs/fuel-asm/issues/68
            if !instruction.opcode().is_predicate_allowed() {
                return Err(PanicReason::ContractInstructionNotAllowed.into())
            }
        }

        instruction.execute(self)
    }
}

pub trait Execute<M, S, Tx, Ecal, V>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    V: Verifier,
{
    fn execute(
        self,
        interpreter: &mut Interpreter<M, S, Tx, Ecal, V>,
    ) -> IoResult<ExecuteState, S::DataError>;
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for Instruction
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    V: Verifier,
{
    fn execute(
        self,
        interpreter: &mut Interpreter<M, S, Tx, Ecal, V>,
    ) -> IoResult<ExecuteState, S::DataError> {
        match self {
            Instruction::ADD(op) => op.execute(interpreter),
            Instruction::AND(op) => op.execute(interpreter),
            Instruction::DIV(op) => op.execute(interpreter),
            Instruction::EQ(op) => op.execute(interpreter),
            Instruction::EXP(op) => op.execute(interpreter),
            Instruction::GT(op) => op.execute(interpreter),
            Instruction::LT(op) => op.execute(interpreter),
            Instruction::MLOG(op) => op.execute(interpreter),
            Instruction::MROO(op) => op.execute(interpreter),
            Instruction::MOD(op) => op.execute(interpreter),
            Instruction::MOVE(op) => op.execute(interpreter),
            Instruction::MUL(op) => op.execute(interpreter),
            Instruction::NOT(op) => op.execute(interpreter),
            Instruction::OR(op) => op.execute(interpreter),
            Instruction::SLL(op) => op.execute(interpreter),
            Instruction::SRL(op) => op.execute(interpreter),
            Instruction::SUB(op) => op.execute(interpreter),
            Instruction::XOR(op) => op.execute(interpreter),
            Instruction::MLDV(op) => op.execute(interpreter),
            Instruction::RET(op) => op.execute(interpreter),
            Instruction::RETD(op) => op.execute(interpreter),
            Instruction::ALOC(op) => op.execute(interpreter),
            Instruction::MCL(op) => op.execute(interpreter),
            Instruction::MCP(op) => op.execute(interpreter),
            Instruction::MEQ(op) => op.execute(interpreter),
            Instruction::BHSH(op) => op.execute(interpreter),
            Instruction::BHEI(op) => op.execute(interpreter),
            Instruction::BURN(op) => op.execute(interpreter),
            Instruction::CALL(op) => op.execute(interpreter),
            Instruction::CCP(op) => op.execute(interpreter),
            Instruction::CROO(op) => op.execute(interpreter),
            Instruction::CSIZ(op) => op.execute(interpreter),
            Instruction::CB(op) => op.execute(interpreter),
            Instruction::LDC(op) => op.execute(interpreter),
            Instruction::LOG(op) => op.execute(interpreter),
            Instruction::LOGD(op) => op.execute(interpreter),
            Instruction::MINT(op) => op.execute(interpreter),
            Instruction::RVRT(op) => op.execute(interpreter),
            Instruction::SCWQ(op) => op.execute(interpreter),
            Instruction::SRW(op) => op.execute(interpreter),
            Instruction::SRWQ(op) => op.execute(interpreter),
            Instruction::SWW(op) => op.execute(interpreter),
            Instruction::SWWQ(op) => op.execute(interpreter),
            Instruction::TR(op) => op.execute(interpreter),
            Instruction::TRO(op) => op.execute(interpreter),
            Instruction::ECK1(op) => op.execute(interpreter),
            Instruction::ECR1(op) => op.execute(interpreter),
            Instruction::ED19(op) => op.execute(interpreter),
            Instruction::K256(op) => op.execute(interpreter),
            Instruction::S256(op) => op.execute(interpreter),
            Instruction::TIME(op) => op.execute(interpreter),
            Instruction::NOOP(op) => op.execute(interpreter),
            Instruction::FLAG(op) => op.execute(interpreter),
            Instruction::BAL(op) => op.execute(interpreter),
            Instruction::JMP(op) => op.execute(interpreter),
            Instruction::JNE(op) => op.execute(interpreter),
            Instruction::SMO(op) => op.execute(interpreter),
            Instruction::ADDI(op) => op.execute(interpreter),
            Instruction::ANDI(op) => op.execute(interpreter),
            Instruction::DIVI(op) => op.execute(interpreter),
            Instruction::EXPI(op) => op.execute(interpreter),
            Instruction::MODI(op) => op.execute(interpreter),
            Instruction::MULI(op) => op.execute(interpreter),
            Instruction::ORI(op) => op.execute(interpreter),
            Instruction::SLLI(op) => op.execute(interpreter),
            Instruction::SRLI(op) => op.execute(interpreter),
            Instruction::SUBI(op) => op.execute(interpreter),
            Instruction::XORI(op) => op.execute(interpreter),
            Instruction::JNEI(op) => op.execute(interpreter),
            Instruction::LB(op) => op.execute(interpreter),
            Instruction::LQW(op) => op.execute(interpreter),
            Instruction::LHW(op) => op.execute(interpreter),
            Instruction::LW(op) => op.execute(interpreter),
            Instruction::SB(op) => op.execute(interpreter),
            Instruction::SQW(op) => op.execute(interpreter),
            Instruction::SHW(op) => op.execute(interpreter),
            Instruction::SW(op) => op.execute(interpreter),
            Instruction::MCPI(op) => op.execute(interpreter),
            Instruction::GTF(op) => op.execute(interpreter),
            Instruction::MCLI(op) => op.execute(interpreter),
            Instruction::GM(op) => op.execute(interpreter),
            Instruction::MOVI(op) => op.execute(interpreter),
            Instruction::JNZI(op) => op.execute(interpreter),
            Instruction::JMPF(op) => op.execute(interpreter),
            Instruction::JMPB(op) => op.execute(interpreter),
            Instruction::JNZF(op) => op.execute(interpreter),
            Instruction::JNZB(op) => op.execute(interpreter),
            Instruction::JNEF(op) => op.execute(interpreter),
            Instruction::JNEB(op) => op.execute(interpreter),
            Instruction::JI(op) => op.execute(interpreter),
            Instruction::CFEI(op) => op.execute(interpreter),
            Instruction::CFSI(op) => op.execute(interpreter),
            Instruction::CFE(op) => op.execute(interpreter),
            Instruction::CFS(op) => op.execute(interpreter),
            Instruction::PSHL(op) => op.execute(interpreter),
            Instruction::PSHH(op) => op.execute(interpreter),
            Instruction::POPL(op) => op.execute(interpreter),
            Instruction::POPH(op) => op.execute(interpreter),
            Instruction::JAL(op) => op.execute(interpreter),
            Instruction::WDCM(op) => op.execute(interpreter),
            Instruction::WQCM(op) => op.execute(interpreter),
            Instruction::WDOP(op) => op.execute(interpreter),
            Instruction::WQOP(op) => op.execute(interpreter),
            Instruction::WDML(op) => op.execute(interpreter),
            Instruction::WQML(op) => op.execute(interpreter),
            Instruction::WDDV(op) => op.execute(interpreter),
            Instruction::WQDV(op) => op.execute(interpreter),
            Instruction::WDMD(op) => op.execute(interpreter),
            Instruction::WQMD(op) => op.execute(interpreter),
            Instruction::WDAM(op) => op.execute(interpreter),
            Instruction::WQAM(op) => op.execute(interpreter),
            Instruction::WDMM(op) => op.execute(interpreter),
            Instruction::WQMM(op) => op.execute(interpreter),
            Instruction::ECAL(op) => op.execute(interpreter),
            Instruction::BSIZ(op) => op.execute(interpreter),
            Instruction::BLDD(op) => op.execute(interpreter),
            Instruction::ECOP(op) => op.execute(interpreter),
            Instruction::EPAR(op) => op.execute(interpreter),
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
