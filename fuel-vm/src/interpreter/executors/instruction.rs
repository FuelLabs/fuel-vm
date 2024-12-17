use crate::{
    constraints::reg_key::ProgramRegistersSegment,
    error::{
        InterpreterError,
        IoResult,
        RuntimeError,
    },
    interpreter::{
        alu,
        flow::{
            JumpArgs,
            JumpMode,
        },
        EcalHandler,
        ExecutableTransaction,
        Interpreter,
        Memory,
    },
    state::ExecuteState,
    storage::InterpreterStorage,
};

use fuel_asm::{
    wideint,
    Instruction,
    PanicInstruction,
    PanicReason,
    RawInstruction,
    RegId,
};
use fuel_types::Word;

use core::ops::Div;

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
        self.instruction(raw_instruction)
    }

    /// Reads the current instruction located in `$m[$pc]`,
    /// performing memory boundary checks.
    fn fetch_instruction(
        &self,
    ) -> Result<RawInstruction, InterpreterError<S::DataError>> {
        let pc = self.registers[RegId::PC];
        let instruction = RawInstruction::from_be_bytes(
            self.memory().read_bytes(pc).map_err(|reason| {
                InterpreterError::PanicInstruction(PanicInstruction::error(
                    reason,
                    0, // The value is meaningless since fetch was out-of-bounds
                ))
            })?,
        );
        if pc < self.registers[RegId::IS] || pc >= self.registers[RegId::SSP] {
            return Err(InterpreterError::PanicInstruction(PanicInstruction::error(
                PanicReason::MemoryNotExecutable,
                instruction,
            )))
        }
        Ok(instruction)
    }

    /// Execute a provided instruction
    pub fn instruction<R: Into<RawInstruction> + Copy>(
        &mut self,
        raw: R,
    ) -> Result<ExecuteState, InterpreterError<S::DataError>> {
        if self.debugger.is_active() {
            let debug = self.eval_debugger_state();
            if !debug.should_continue() {
                return Ok(debug.into())
            }
        }

        self.instruction_inner(raw.into())
            .map_err(|e| InterpreterError::from_runtime(e, raw.into()))
    }

    fn instruction_inner(
        &mut self,
        raw: RawInstruction,
    ) -> IoResult<ExecuteState, S::DataError> {
        let instruction = Instruction::try_from(raw)
            .map_err(|_| RuntimeError::from(PanicReason::InvalidInstruction))?;

        // TODO additional branch that might be optimized after
        // https://github.com/FuelLabs/fuel-asm/issues/68
        if self.is_predicate() && !instruction.opcode().is_predicate_allowed() {
            return Err(PanicReason::ContractInstructionNotAllowed.into())
        }

        // Short-hand for retrieving the value from the register with the given ID.
        // We use a macro to "close over" `self.registers` without taking ownership of it.
        macro_rules! r {
            ($id:expr) => {
                self.registers[$id]
            };
        }

        match instruction {
            Instruction::ADD(add) => {
                self.gas_charge(self.gas_costs().add())?;
                let (a, b, c) = add.unpack();
                self.alu_capture_overflow(
                    a.into(),
                    u128::overflowing_add,
                    r!(b).into(),
                    r!(c).into(),
                )?;
            }

            Instruction::ADDI(addi) => {
                self.gas_charge(self.gas_costs().addi())?;
                let (a, b, imm) = addi.unpack();
                self.alu_capture_overflow(
                    a.into(),
                    u128::overflowing_add,
                    r!(b).into(),
                    imm.into(),
                )?;
            }

            Instruction::AND(and) => {
                self.gas_charge(self.gas_costs().and())?;
                let (a, b, c) = and.unpack();
                self.alu_set(a.into(), r!(b) & r!(c))?;
            }

            Instruction::ANDI(andi) => {
                self.gas_charge(self.gas_costs().andi())?;
                let (a, b, imm) = andi.unpack();
                self.alu_set(a.into(), r!(b) & Word::from(imm))?;
            }

            Instruction::DIV(div) => {
                self.gas_charge(self.gas_costs().div())?;
                let (a, b, c) = div.unpack();
                let c = r!(c);
                self.alu_error(a.into(), Word::div, r!(b), c, c == 0)?;
            }

            Instruction::DIVI(divi) => {
                self.gas_charge(self.gas_costs().divi())?;
                let (a, b, imm) = divi.unpack();
                let imm = Word::from(imm);
                self.alu_error(a.into(), Word::div, r!(b), imm, imm == 0)?;
            }

            Instruction::EQ(eq) => {
                self.gas_charge(self.gas_costs().eq_())?;
                let (a, b, c) = eq.unpack();
                self.alu_set(a.into(), (r!(b) == r!(c)) as Word)?;
            }

            Instruction::EXP(exp) => {
                self.gas_charge(self.gas_costs().exp())?;
                let (a, b, c) = exp.unpack();
                self.alu_boolean_overflow(a.into(), alu::exp, r!(b), r!(c))?;
            }

            Instruction::EXPI(expi) => {
                self.gas_charge(self.gas_costs().expi())?;
                let (a, b, imm) = expi.unpack();
                let expo = u32::from(imm);
                self.alu_boolean_overflow(a.into(), Word::overflowing_pow, r!(b), expo)?;
            }

            Instruction::GT(gt) => {
                self.gas_charge(self.gas_costs().gt())?;
                let (a, b, c) = gt.unpack();
                self.alu_set(a.into(), (r!(b) > r!(c)) as Word)?;
            }

            Instruction::LT(lt) => {
                self.gas_charge(self.gas_costs().lt())?;
                let (a, b, c) = lt.unpack();
                self.alu_set(a.into(), (r!(b) < r!(c)) as Word)?;
            }

            Instruction::WDCM(wdcm) => {
                self.gas_charge(self.gas_costs().wdcm())?;
                let (a, b, c, imm) = wdcm.unpack();
                let args = wideint::CompareArgs::from_imm(imm)
                    .ok_or(PanicReason::InvalidImmediateValue)?;
                self.alu_wideint_cmp_u128(a.into(), r!(b), r!(c), args)?;
            }

            Instruction::WQCM(wqcm) => {
                self.gas_charge(self.gas_costs().wqcm())?;
                let (a, b, c, imm) = wqcm.unpack();
                let args = wideint::CompareArgs::from_imm(imm)
                    .ok_or(PanicReason::InvalidImmediateValue)?;
                self.alu_wideint_cmp_u256(a.into(), r!(b), r!(c), args)?;
            }

            Instruction::WDOP(wdop) => {
                self.gas_charge(self.gas_costs().wdop())?;
                let (a, b, c, imm) = wdop.unpack();
                let args = wideint::MathArgs::from_imm(imm)
                    .ok_or(PanicReason::InvalidImmediateValue)?;
                self.alu_wideint_op_u128(r!(a), r!(b), r!(c), args)?;
            }

            Instruction::WQOP(wqop) => {
                self.gas_charge(self.gas_costs().wqop())?;
                let (a, b, c, imm) = wqop.unpack();
                let args = wideint::MathArgs::from_imm(imm)
                    .ok_or(PanicReason::InvalidImmediateValue)?;
                self.alu_wideint_op_u256(r!(a), r!(b), r!(c), args)?;
            }

            Instruction::WDML(wdml) => {
                self.gas_charge(self.gas_costs().wdml())?;
                let (a, b, c, imm) = wdml.unpack();
                let args = wideint::MulArgs::from_imm(imm)
                    .ok_or(PanicReason::InvalidImmediateValue)?;
                self.alu_wideint_mul_u128(r!(a), r!(b), r!(c), args)?;
            }

            Instruction::WQML(wqml) => {
                self.gas_charge(self.gas_costs().wqml())?;
                let (a, b, c, imm) = wqml.unpack();
                let args = wideint::MulArgs::from_imm(imm)
                    .ok_or(PanicReason::InvalidImmediateValue)?;
                self.alu_wideint_mul_u256(r!(a), r!(b), r!(c), args)?;
            }

            Instruction::WDDV(wddv) => {
                self.gas_charge(self.gas_costs().wddv())?;
                let (a, b, c, imm) = wddv.unpack();
                let args = wideint::DivArgs::from_imm(imm)
                    .ok_or(PanicReason::InvalidImmediateValue)?;
                self.alu_wideint_div_u128(r!(a), r!(b), r!(c), args)?;
            }

            Instruction::WQDV(wqdv) => {
                self.gas_charge(self.gas_costs().wqdv())?;
                let (a, b, c, imm) = wqdv.unpack();
                let args = wideint::DivArgs::from_imm(imm)
                    .ok_or(PanicReason::InvalidImmediateValue)?;
                self.alu_wideint_div_u256(r!(a), r!(b), r!(c), args)?;
            }

            Instruction::WDMD(wdmd) => {
                self.gas_charge(self.gas_costs().wdmd())?;
                let (a, b, c, d) = wdmd.unpack();
                self.alu_wideint_muldiv_u128(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::WQMD(wqmd) => {
                self.gas_charge(self.gas_costs().wqmd())?;
                let (a, b, c, d) = wqmd.unpack();
                self.alu_wideint_muldiv_u256(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::WDAM(wdam) => {
                self.gas_charge(self.gas_costs().wdam())?;
                let (a, b, c, d) = wdam.unpack();
                self.alu_wideint_addmod_u128(r!(a), r!(b), r!(c), r!(d))?;
            }
            Instruction::WQAM(wqam) => {
                self.gas_charge(self.gas_costs().wqam())?;
                let (a, b, c, d) = wqam.unpack();
                self.alu_wideint_addmod_u256(r!(a), r!(b), r!(c), r!(d))?;
            }
            Instruction::WDMM(wdmm) => {
                self.gas_charge(self.gas_costs().wdmm())?;
                let (a, b, c, d) = wdmm.unpack();
                self.alu_wideint_mulmod_u128(r!(a), r!(b), r!(c), r!(d))?;
            }
            Instruction::WQMM(wqmm) => {
                self.gas_charge(self.gas_costs().wqmm())?;
                let (a, b, c, d) = wqmm.unpack();
                self.alu_wideint_mulmod_u256(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::MLOG(mlog) => {
                self.gas_charge(self.gas_costs().mlog())?;
                let (a, b, c) = mlog.unpack();
                let (lhs, rhs) = (r!(b), r!(c));
                self.alu_error(
                    a.into(),
                    |l, r| {
                        l.checked_ilog(r)
                            .expect("checked_ilog returned None for valid values")
                            as Word
                    },
                    lhs,
                    rhs,
                    lhs == 0 || rhs <= 1,
                )?;
            }

            Instruction::MOD(mod_) => {
                self.gas_charge(self.gas_costs().mod_op())?;
                let (a, b, c) = mod_.unpack();
                let rhs = r!(c);
                self.alu_error(a.into(), Word::wrapping_rem, r!(b), rhs, rhs == 0)?;
            }

            Instruction::MODI(modi) => {
                self.gas_charge(self.gas_costs().modi())?;
                let (a, b, imm) = modi.unpack();
                let rhs = Word::from(imm);
                self.alu_error(a.into(), Word::wrapping_rem, r!(b), rhs, rhs == 0)?;
            }

            Instruction::MOVE(move_) => {
                self.gas_charge(self.gas_costs().move_op())?;
                let (a, b) = move_.unpack();
                self.alu_set(a.into(), r!(b))?;
            }

            Instruction::MOVI(movi) => {
                self.gas_charge(self.gas_costs().movi())?;
                let (a, imm) = movi.unpack();
                self.alu_set(a.into(), Word::from(imm))?;
            }

            Instruction::MROO(mroo) => {
                self.gas_charge(self.gas_costs().mroo())?;
                let (a, b, c) = mroo.unpack();
                let (lhs, rhs) = (r!(b), r!(c));
                self.alu_error(
                    a.into(),
                    |l, r| {
                        checked_nth_root(l, r)
                            .expect("checked_nth_root returned None for valid values")
                            as Word
                    },
                    lhs,
                    rhs,
                    rhs == 0,
                )?;
            }

            Instruction::MUL(mul) => {
                self.gas_charge(self.gas_costs().mul())?;
                let (a, b, c) = mul.unpack();
                self.alu_capture_overflow(
                    a.into(),
                    u128::overflowing_mul,
                    r!(b).into(),
                    r!(c).into(),
                )?;
            }

            Instruction::MULI(muli) => {
                self.gas_charge(self.gas_costs().muli())?;
                let (a, b, imm) = muli.unpack();
                self.alu_capture_overflow(
                    a.into(),
                    u128::overflowing_mul,
                    r!(b).into(),
                    imm.into(),
                )?;
            }

            Instruction::MLDV(mldv) => {
                self.gas_charge(self.gas_costs().mldv())?;
                let (a, b, c, d) = mldv.unpack();
                self.alu_muldiv(a.into(), r!(b), r!(c), r!(d))?;
            }

            Instruction::NOOP(_noop) => {
                self.gas_charge(self.gas_costs().noop())?;
                self.alu_clear()?;
            }

            Instruction::NOT(not) => {
                self.gas_charge(self.gas_costs().not())?;
                let (a, b) = not.unpack();
                self.alu_set(a.into(), !r!(b))?;
            }

            Instruction::OR(or) => {
                self.gas_charge(self.gas_costs().or())?;
                let (a, b, c) = or.unpack();
                self.alu_set(a.into(), r!(b) | r!(c))?;
            }

            Instruction::ORI(ori) => {
                self.gas_charge(self.gas_costs().ori())?;
                let (a, b, imm) = ori.unpack();
                self.alu_set(a.into(), r!(b) | Word::from(imm))?;
            }

            Instruction::SLL(sll) => {
                self.gas_charge(self.gas_costs().sll())?;
                let (a, b, c) = sll.unpack();

                self.alu_set(
                    a.into(),
                    if let Ok(c) = r!(c).try_into() {
                        Word::checked_shl(r!(b), c).unwrap_or_default()
                    } else {
                        0
                    },
                )?;
            }

            Instruction::SLLI(slli) => {
                self.gas_charge(self.gas_costs().slli())?;
                let (a, b, imm) = slli.unpack();
                let rhs = u32::from(imm);
                self.alu_set(a.into(), r!(b).checked_shl(rhs).unwrap_or_default())?;
            }

            Instruction::SRL(srl) => {
                self.gas_charge(self.gas_costs().srl())?;
                let (a, b, c) = srl.unpack();
                self.alu_set(
                    a.into(),
                    if let Ok(c) = r!(c).try_into() {
                        Word::checked_shr(r!(b), c).unwrap_or_default()
                    } else {
                        0
                    },
                )?;
            }

            Instruction::SRLI(srli) => {
                self.gas_charge(self.gas_costs().srli())?;
                let (a, b, imm) = srli.unpack();
                let rhs = u32::from(imm);
                self.alu_set(a.into(), r!(b).checked_shr(rhs).unwrap_or_default())?;
            }

            Instruction::SUB(sub) => {
                self.gas_charge(self.gas_costs().sub())?;
                let (a, b, c) = sub.unpack();
                self.alu_capture_overflow(
                    a.into(),
                    u128::overflowing_sub,
                    r!(b).into(),
                    r!(c).into(),
                )?;
            }

            Instruction::SUBI(subi) => {
                self.gas_charge(self.gas_costs().subi())?;
                let (a, b, imm) = subi.unpack();
                self.alu_capture_overflow(
                    a.into(),
                    u128::overflowing_sub,
                    r!(b).into(),
                    imm.into(),
                )?;
            }

            Instruction::XOR(xor) => {
                self.gas_charge(self.gas_costs().xor())?;
                let (a, b, c) = xor.unpack();
                self.alu_set(a.into(), r!(b) ^ r!(c))?;
            }

            Instruction::XORI(xori) => {
                self.gas_charge(self.gas_costs().xori())?;
                let (a, b, imm) = xori.unpack();
                self.alu_set(a.into(), r!(b) ^ Word::from(imm))?;
            }

            Instruction::JI(ji) => {
                self.gas_charge(self.gas_costs().ji())?;
                let imm = ji.unpack();
                self.jump(JumpArgs::new(JumpMode::Absolute).to_address(imm.into()))?;
            }

            Instruction::JNEI(jnei) => {
                self.gas_charge(self.gas_costs().jnei())?;
                let (a, b, imm) = jnei.unpack();
                self.jump(
                    JumpArgs::new(JumpMode::Absolute)
                        .with_condition(r!(a) != r!(b))
                        .to_address(imm.into()),
                )?;
            }

            Instruction::JNZI(jnzi) => {
                self.gas_charge(self.gas_costs().jnzi())?;
                let (a, imm) = jnzi.unpack();
                self.jump(
                    JumpArgs::new(JumpMode::Absolute)
                        .with_condition(r!(a) != 0)
                        .to_address(imm.into()),
                )?;
            }

            Instruction::JMP(jmp) => {
                self.gas_charge(self.gas_costs().jmp())?;
                let a = jmp.unpack();
                self.jump(JumpArgs::new(JumpMode::Absolute).to_address(r!(a)))?;
            }

            Instruction::JNE(jne) => {
                self.gas_charge(self.gas_costs().jne())?;
                let (a, b, c) = jne.unpack();
                self.jump(
                    JumpArgs::new(JumpMode::Absolute)
                        .with_condition(r!(a) != r!(b))
                        .to_address(r!(c)),
                )?;
            }

            Instruction::JMPF(jmpf) => {
                self.gas_charge(self.gas_costs().jmpf())?;
                let (a, offset) = jmpf.unpack();
                self.jump(
                    JumpArgs::new(JumpMode::RelativeForwards)
                        .to_address(r!(a))
                        .plus_fixed(offset.into()),
                )?;
            }

            Instruction::JMPB(jmpb) => {
                self.gas_charge(self.gas_costs().jmpb())?;
                let (a, offset) = jmpb.unpack();
                self.jump(
                    JumpArgs::new(JumpMode::RelativeBackwards)
                        .to_address(r!(a))
                        .plus_fixed(offset.into()),
                )?;
            }

            Instruction::JNZF(jnzf) => {
                self.gas_charge(self.gas_costs().jnzf())?;
                let (a, b, offset) = jnzf.unpack();
                self.jump(
                    JumpArgs::new(JumpMode::RelativeForwards)
                        .with_condition(r!(a) != 0)
                        .to_address(r!(b))
                        .plus_fixed(offset.into()),
                )?;
            }

            Instruction::JNZB(jnzb) => {
                self.gas_charge(self.gas_costs().jnzb())?;
                let (a, b, offset) = jnzb.unpack();
                self.jump(
                    JumpArgs::new(JumpMode::RelativeBackwards)
                        .with_condition(r!(a) != 0)
                        .to_address(r!(b))
                        .plus_fixed(offset.into()),
                )?;
            }

            Instruction::JNEF(jnef) => {
                self.gas_charge(self.gas_costs().jnef())?;
                let (a, b, c, offset) = jnef.unpack();
                self.jump(
                    JumpArgs::new(JumpMode::RelativeForwards)
                        .with_condition(r!(a) != r!(b))
                        .to_address(r!(c))
                        .plus_fixed(offset.into()),
                )?;
            }

            Instruction::JNEB(jneb) => {
                self.gas_charge(self.gas_costs().jneb())?;
                let (a, b, c, offset) = jneb.unpack();
                self.jump(
                    JumpArgs::new(JumpMode::RelativeBackwards)
                        .with_condition(r!(a) != r!(b))
                        .to_address(r!(c))
                        .plus_fixed(offset.into()),
                )?;
            }

            Instruction::RET(ret) => {
                self.gas_charge(self.gas_costs().ret())?;
                let a = ret.unpack();
                let ra = r!(a);
                self.ret(ra)?;
                return Ok(ExecuteState::Return(ra))
            }

            Instruction::RETD(retd) => {
                let (a, b) = retd.unpack();
                let len = r!(b);
                self.dependent_gas_charge(self.gas_costs().retd(), len)?;
                return Ok(self.ret_data(r!(a), len).map(ExecuteState::ReturnData)?)
            }

            Instruction::RVRT(rvrt) => {
                self.gas_charge(self.gas_costs().rvrt())?;
                let a = rvrt.unpack();
                let ra = r!(a);
                self.revert(ra)?;
                return Ok(ExecuteState::Revert(ra))
            }

            Instruction::SMO(smo) => {
                let (a, b, c, d) = smo.unpack();
                self.dependent_gas_charge(self.gas_costs().smo(), r!(c))?;
                self.message_output(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::ALOC(aloc) => {
                let a = aloc.unpack();
                let number_of_bytes = r!(a);
                self.dependent_gas_charge(self.gas_costs().aloc(), number_of_bytes)?;
                self.malloc(number_of_bytes)?;
            }

            Instruction::CFEI(cfei) => {
                let number_of_bytes = cfei.unpack().into();
                self.dependent_gas_charge(self.gas_costs().cfei(), number_of_bytes)?;
                self.stack_pointer_overflow(Word::overflowing_add, number_of_bytes)?;
            }

            Instruction::CFE(cfe) => {
                let a = cfe.unpack();
                let number_of_bytes = r!(a);
                self.dependent_gas_charge(self.gas_costs().cfe(), number_of_bytes)?;
                self.stack_pointer_overflow(Word::overflowing_add, number_of_bytes)?;
            }

            Instruction::CFSI(cfsi) => {
                self.gas_charge(self.gas_costs().cfsi())?;
                let imm = cfsi.unpack();
                self.stack_pointer_overflow(Word::overflowing_sub, imm.into())?;
            }

            Instruction::CFS(cfs) => {
                self.gas_charge(self.gas_costs().cfsi())?;
                let a = cfs.unpack();
                self.stack_pointer_overflow(Word::overflowing_sub, r!(a))?;
            }

            Instruction::PSHL(pshl) => {
                self.gas_charge(self.gas_costs().pshl())?;
                let bitmask = pshl.unpack();
                self.push_selected_registers(ProgramRegistersSegment::Low, bitmask)?;
            }

            Instruction::PSHH(pshh) => {
                self.gas_charge(self.gas_costs().pshh())?;
                let bitmask = pshh.unpack();
                self.push_selected_registers(ProgramRegistersSegment::High, bitmask)?;
            }

            Instruction::POPL(popl) => {
                self.gas_charge(self.gas_costs().popl())?;
                let bitmask = popl.unpack();
                self.pop_selected_registers(ProgramRegistersSegment::Low, bitmask)?;
            }

            Instruction::POPH(poph) => {
                self.gas_charge(self.gas_costs().poph())?;
                let bitmask = poph.unpack();
                self.pop_selected_registers(ProgramRegistersSegment::High, bitmask)?;
            }

            Instruction::LB(lb) => {
                self.gas_charge(self.gas_costs().lb())?;
                let (a, b, imm) = lb.unpack();
                self.load_byte(a.into(), r!(b), imm.into())?;
            }

            Instruction::LW(lw) => {
                self.gas_charge(self.gas_costs().lw())?;
                let (a, b, imm) = lw.unpack();
                self.load_word(a.into(), r!(b), imm)?;
            }

            Instruction::MCL(mcl) => {
                let (a, b) = mcl.unpack();
                let len = r!(b);
                self.dependent_gas_charge(self.gas_costs().mcl(), len)?;
                self.memclear(r!(a), len)?;
            }

            Instruction::MCLI(mcli) => {
                let (a, imm) = mcli.unpack();
                let len = Word::from(imm);
                self.dependent_gas_charge(self.gas_costs().mcli(), len)?;
                self.memclear(r!(a), len)?;
            }

            Instruction::MCP(mcp) => {
                let (a, b, c) = mcp.unpack();
                let len = r!(c);
                self.dependent_gas_charge(self.gas_costs().mcp(), len)?;
                self.memcopy(r!(a), r!(b), len)?;
            }

            Instruction::MCPI(mcpi) => {
                let (a, b, imm) = mcpi.unpack();
                let len = imm.into();
                self.dependent_gas_charge(self.gas_costs().mcpi(), len)?;
                self.memcopy(r!(a), r!(b), len)?;
            }

            Instruction::MEQ(meq) => {
                let (a, b, c, d) = meq.unpack();
                let len = r!(d);
                self.dependent_gas_charge(self.gas_costs().meq(), len)?;
                self.memeq(a.into(), r!(b), r!(c), len)?;
            }

            Instruction::SB(sb) => {
                self.gas_charge(self.gas_costs().sb())?;
                let (a, b, imm) = sb.unpack();
                self.store_byte(r!(a), r!(b), imm.into())?;
            }

            Instruction::SW(sw) => {
                self.gas_charge(self.gas_costs().sw())?;
                let (a, b, imm) = sw.unpack();
                self.store_word(r!(a), r!(b), imm)?;
            }

            Instruction::BAL(bal) => {
                self.gas_charge(self.gas_costs().bal())?;
                let (a, b, c) = bal.unpack();
                self.contract_balance(a.into(), r!(b), r!(c))?;
            }

            Instruction::BHEI(bhei) => {
                self.gas_charge(self.gas_costs().bhei())?;
                let a = bhei.unpack();
                self.block_height(a.into())?;
            }

            Instruction::BHSH(bhsh) => {
                self.gas_charge(self.gas_costs().bhsh())?;
                let (a, b) = bhsh.unpack();
                self.block_hash(r!(a), r!(b))?;
            }

            Instruction::BURN(burn) => {
                self.gas_charge(self.gas_costs().burn())?;
                let (a, b) = burn.unpack();
                self.burn(r!(a), r!(b))?;
            }

            Instruction::CALL(call) => {
                // We charge for the gas inside of the `prepare_call` function.
                let (a, b, c, d) = call.unpack();

                // Enter call context
                self.prepare_call(a, b, c, d)?;
            }

            Instruction::CB(cb) => {
                self.gas_charge(self.gas_costs().cb())?;
                let a = cb.unpack();
                self.block_proposer(r!(a))?;
            }

            Instruction::CCP(ccp) => {
                let (a, b, c, d) = ccp.unpack();
                self.code_copy(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::CROO(croo) => {
                let (a, b) = croo.unpack();
                self.code_root(r!(a), r!(b))?;
            }

            Instruction::CSIZ(csiz) => {
                // We charge for the gas inside of the `code_size` function.
                let (a, b) = csiz.unpack();
                self.code_size(a.into(), r!(b))?;
            }

            Instruction::LDC(ldc) => {
                // We charge for the gas inside of the `load_contract_code` function.
                let (a, b, c, mode) = ldc.unpack();
                self.load_contract_code(r!(a), r!(b), r!(c), mode)?;
            }

            Instruction::LOG(log) => {
                self.gas_charge(self.gas_costs().log())?;
                let (a, b, c, d) = log.unpack();
                self.log(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::LOGD(logd) => {
                let (a, b, c, d) = logd.unpack();
                self.dependent_gas_charge(self.gas_costs().logd(), r!(d))?;
                self.log_data(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::MINT(mint) => {
                self.gas_charge(self.gas_costs().mint())?;
                let (a, b) = mint.unpack();
                self.mint(r!(a), r!(b))?;
            }

            Instruction::SCWQ(scwq) => {
                let (a, b, c) = scwq.unpack();
                self.dependent_gas_charge(self.gas_costs().scwq(), r!(c))?;
                self.state_clear_qword(r!(a), b.into(), r!(c))?;
            }

            Instruction::SRW(srw) => {
                self.gas_charge(self.gas_costs().srw())?;
                let (a, b, c) = srw.unpack();
                self.state_read_word(a.into(), b.into(), r!(c))?;
            }

            Instruction::SRWQ(srwq) => {
                let (a, b, c, d) = srwq.unpack();
                self.dependent_gas_charge(self.gas_costs().srwq(), r!(d))?;
                self.state_read_qword(r!(a), b.into(), r!(c), r!(d))?;
            }

            Instruction::SWW(sww) => {
                self.gas_charge(self.gas_costs().sww())?;
                let (a, b, c) = sww.unpack();
                self.state_write_word(r!(a), b.into(), r!(c))?;
            }

            Instruction::SWWQ(swwq) => {
                let (a, b, c, d) = swwq.unpack();
                self.dependent_gas_charge(self.gas_costs().swwq(), r!(d))?;
                self.state_write_qword(r!(a), b.into(), r!(c), r!(d))?;
            }

            Instruction::TIME(time) => {
                self.gas_charge(self.gas_costs().time())?;
                let (a, b) = time.unpack();
                self.timestamp(a.into(), r!(b))?;
            }

            Instruction::ECK1(eck1) => {
                self.gas_charge(self.gas_costs().eck1())?;
                let (a, b, c) = eck1.unpack();
                self.secp256k1_recover(r!(a), r!(b), r!(c))?;
            }

            Instruction::ECR1(ecr1) => {
                self.gas_charge(self.gas_costs().ecr1())?;
                let (a, b, c) = ecr1.unpack();
                self.secp256r1_recover(r!(a), r!(b), r!(c))?;
            }

            Instruction::ED19(ed19) => {
                let (a, b, c, len) = ed19.unpack();
                let mut len = r!(len);

                // Backwards compatibility with old contracts
                if len == 0 {
                    len = 32;
                }

                self.dependent_gas_charge(self.gas_costs().ed19(), len)?;
                self.ed25519_verify(r!(a), r!(b), r!(c), len)?;
            }

            Instruction::K256(k256) => {
                let (a, b, c) = k256.unpack();
                let len = r!(c);
                self.dependent_gas_charge(self.gas_costs().k256(), len)?;
                self.keccak256(r!(a), r!(b), len)?;
            }

            Instruction::S256(s256) => {
                let (a, b, c) = s256.unpack();
                let len = r!(c);
                self.dependent_gas_charge(self.gas_costs().s256(), len)?;
                self.sha256(r!(a), r!(b), len)?;
            }

            Instruction::FLAG(flag) => {
                self.gas_charge(self.gas_costs().flag())?;
                let a = flag.unpack();
                self.set_flag(r!(a))?;
            }

            Instruction::GM(gm) => {
                self.gas_charge(self.gas_costs().gm())?;
                let (a, imm) = gm.unpack();
                self.metadata(a.into(), imm.into())?;
            }

            Instruction::GTF(gtf) => {
                self.gas_charge(self.gas_costs().gtf())?;
                let (a, b, imm) = gtf.unpack();
                self.get_transaction_field(a.into(), r!(b), imm.into())?;
            }

            Instruction::TR(tr) => {
                self.gas_charge(self.gas_costs().tr())?;
                let (a, b, c) = tr.unpack();
                self.transfer(r!(a), r!(b), r!(c))?;
            }

            Instruction::TRO(tro) => {
                self.gas_charge(self.gas_costs().tro())?;
                let (a, b, c, d) = tro.unpack();
                self.transfer_output(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::ECAL(ecal) => {
                let (a, b, c, d) = ecal.unpack();
                self.external_call(a, b, c, d)?;
            }

            Instruction::BSIZ(bsiz) => {
                // We charge for this inside the function.
                let (a, b) = bsiz.unpack();
                self.blob_size(a.into(), r!(b))?;
            }

            Instruction::BLDD(bldd) => {
                // We charge for this inside the function.
                let (a, b, c, d) = bldd.unpack();
                self.blob_load_data(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::ECOP(ecop) => {
                self.gas_charge(self.gas_costs().ecop().map_err(PanicReason::from)?)?;
                let (a, b, c, d) = ecop.unpack();
                self.ec_operation(r!(a), r!(b), r!(c), r!(d))?;
            }

            Instruction::EPAR(epar) => {
                let (a, b, c, d) = epar.unpack();
                let len = r!(c);
                self.dependent_gas_charge(
                    self.gas_costs().epar().map_err(PanicReason::from)?,
                    len,
                )?;
                self.ec_pairing(a.into(), r!(b), len, r!(d))?;
            }
        }

        Ok(ExecuteState::Proceed)
    }
}

/// Computes nth root of target, rounding down to nearest integer.
/// This function uses the floating point operation to get an approximate solution,
/// but corrects the result using exponentation to check for inaccuracy.
fn checked_nth_root(target: u64, nth_root: u64) -> Option<u64> {
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
