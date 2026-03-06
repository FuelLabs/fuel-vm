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
    Opcode,
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
        let opcode = Opcode::try_from(raw[0])
            .map_err(|_| RuntimeError::from(PanicReason::InvalidInstruction))?;

        if PREDICATE && !opcode.is_predicate_allowed() {
            return Err(PanicReason::ContractInstructionNotAllowed.into())
        }

        execute_instruction(self, opcode, [raw[1], raw[2], raw[3]])
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

fn execute_instruction<M, S, Tx, Ecal, V>(
    interpreter: &mut Interpreter<M, S, Tx, Ecal, V>,
    opcode: Opcode,
    raw_args: [u8; 3],
) -> IoResult<ExecuteState, S::DataError>
where
    M: Memory,
    S: InterpreterStorage,
    Tx: ExecutableTransaction,
    Ecal: EcalHandler,
    V: Verifier,
{
    // Parses op args into the appropriate instruction struct.
    macro_rules! execute_op {
        ($op:ident) => {
            fuel_asm::op::$op::from_raw_args(raw_args)
                .map_err(|_| RuntimeError::from(PanicReason::InvalidInstruction))?
                .execute(interpreter)
        };
    }

    match opcode {
        Opcode::ADD => execute_op!(ADD),
        Opcode::AND => execute_op!(AND),
        Opcode::DIV => execute_op!(DIV),
        Opcode::EQ => execute_op!(EQ),
        Opcode::EXP => execute_op!(EXP),
        Opcode::GT => execute_op!(GT),
        Opcode::LT => execute_op!(LT),
        Opcode::MLOG => execute_op!(MLOG),
        Opcode::MROO => execute_op!(MROO),
        Opcode::MOD => execute_op!(MOD),
        Opcode::MOVE => execute_op!(MOVE),
        Opcode::MUL => execute_op!(MUL),
        Opcode::NOT => execute_op!(NOT),
        Opcode::OR => execute_op!(OR),
        Opcode::SLL => execute_op!(SLL),
        Opcode::SRL => execute_op!(SRL),
        Opcode::SUB => execute_op!(SUB),
        Opcode::XOR => execute_op!(XOR),
        Opcode::MLDV => execute_op!(MLDV),
        Opcode::NIOP => execute_op!(NIOP),
        Opcode::RET => execute_op!(RET),
        Opcode::RETD => execute_op!(RETD),
        Opcode::ALOC => execute_op!(ALOC),
        Opcode::MCL => execute_op!(MCL),
        Opcode::MCP => execute_op!(MCP),
        Opcode::MEQ => execute_op!(MEQ),
        Opcode::BHSH => execute_op!(BHSH),
        Opcode::BHEI => execute_op!(BHEI),
        Opcode::BURN => execute_op!(BURN),
        Opcode::CALL => execute_op!(CALL),
        Opcode::CCP => execute_op!(CCP),
        Opcode::CROO => execute_op!(CROO),
        Opcode::CSIZ => execute_op!(CSIZ),
        Opcode::CB => execute_op!(CB),
        Opcode::LDC => execute_op!(LDC),
        Opcode::LOG => execute_op!(LOG),
        Opcode::LOGD => execute_op!(LOGD),
        Opcode::MINT => execute_op!(MINT),
        Opcode::RVRT => execute_op!(RVRT),
        Opcode::SCWQ => execute_op!(SCWQ),
        Opcode::SRW => execute_op!(SRW),
        Opcode::SRWQ => execute_op!(SRWQ),
        Opcode::SWW => execute_op!(SWW),
        Opcode::SWWQ => execute_op!(SWWQ),
        Opcode::TR => execute_op!(TR),
        Opcode::TRO => execute_op!(TRO),
        Opcode::ECK1 => execute_op!(ECK1),
        Opcode::ECR1 => execute_op!(ECR1),
        Opcode::ED19 => execute_op!(ED19),
        Opcode::K256 => execute_op!(K256),
        Opcode::S256 => execute_op!(S256),
        Opcode::TIME => execute_op!(TIME),
        Opcode::NOOP => execute_op!(NOOP),
        Opcode::FLAG => execute_op!(FLAG),
        Opcode::BAL => execute_op!(BAL),
        Opcode::JMP => execute_op!(JMP),
        Opcode::JNE => execute_op!(JNE),
        Opcode::SMO => execute_op!(SMO),
        Opcode::ADDI => execute_op!(ADDI),
        Opcode::ANDI => execute_op!(ANDI),
        Opcode::DIVI => execute_op!(DIVI),
        Opcode::EXPI => execute_op!(EXPI),
        Opcode::MODI => execute_op!(MODI),
        Opcode::MULI => execute_op!(MULI),
        Opcode::ORI => execute_op!(ORI),
        Opcode::SLLI => execute_op!(SLLI),
        Opcode::SRLI => execute_op!(SRLI),
        Opcode::SUBI => execute_op!(SUBI),
        Opcode::XORI => execute_op!(XORI),
        Opcode::JNEI => execute_op!(JNEI),
        Opcode::LB => execute_op!(LB),
        Opcode::LQW => execute_op!(LQW),
        Opcode::LHW => execute_op!(LHW),
        Opcode::LW => execute_op!(LW),
        Opcode::SB => execute_op!(SB),
        Opcode::SQW => execute_op!(SQW),
        Opcode::SHW => execute_op!(SHW),
        Opcode::SW => execute_op!(SW),
        Opcode::MCPI => execute_op!(MCPI),
        Opcode::GTF => execute_op!(GTF),
        Opcode::MCLI => execute_op!(MCLI),
        Opcode::GM => execute_op!(GM),
        Opcode::MOVI => execute_op!(MOVI),
        Opcode::JNZI => execute_op!(JNZI),
        Opcode::JMPF => execute_op!(JMPF),
        Opcode::JMPB => execute_op!(JMPB),
        Opcode::JNZF => execute_op!(JNZF),
        Opcode::JNZB => execute_op!(JNZB),
        Opcode::JNEF => execute_op!(JNEF),
        Opcode::JNEB => execute_op!(JNEB),
        Opcode::JI => execute_op!(JI),
        Opcode::CFEI => execute_op!(CFEI),
        Opcode::CFSI => execute_op!(CFSI),
        Opcode::CFE => execute_op!(CFE),
        Opcode::CFS => execute_op!(CFS),
        Opcode::PSHL => execute_op!(PSHL),
        Opcode::PSHH => execute_op!(PSHH),
        Opcode::POPL => execute_op!(POPL),
        Opcode::POPH => execute_op!(POPH),
        Opcode::JAL => execute_op!(JAL),
        Opcode::WDCM => execute_op!(WDCM),
        Opcode::WQCM => execute_op!(WQCM),
        Opcode::WDOP => execute_op!(WDOP),
        Opcode::WQOP => execute_op!(WQOP),
        Opcode::WDML => execute_op!(WDML),
        Opcode::WQML => execute_op!(WQML),
        Opcode::WDDV => execute_op!(WDDV),
        Opcode::WQDV => execute_op!(WQDV),
        Opcode::WDMD => execute_op!(WDMD),
        Opcode::WQMD => execute_op!(WQMD),
        Opcode::WDAM => execute_op!(WDAM),
        Opcode::WQAM => execute_op!(WQAM),
        Opcode::WDMM => execute_op!(WDMM),
        Opcode::WQMM => execute_op!(WQMM),
        Opcode::ECAL => execute_op!(ECAL),
        Opcode::BSIZ => execute_op!(BSIZ),
        Opcode::BLDD => execute_op!(BLDD),
        Opcode::ECOP => execute_op!(ECOP),
        Opcode::EPAR => execute_op!(EPAR),
        Opcode::SCLR => execute_op!(SCLR),
        Opcode::SRDD => execute_op!(SRDD),
        Opcode::SRDI => execute_op!(SRDI),
        Opcode::SWRD => execute_op!(SWRD),
        Opcode::SWRI => execute_op!(SWRI),
        Opcode::SUPD => execute_op!(SUPD),
        Opcode::SUPI => execute_op!(SUPI),
        Opcode::SPLD => execute_op!(SPLD),
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
