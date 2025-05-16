use crate::{
    constraints::reg_key::ProgramRegistersSegment,
    error::IoResult,
    interpreter::{
        EcalHandler,
        ExecutableTransaction,
        Interpreter,
        Memory,
        alu,
        executors::instruction::{
            Execute,
            checked_nth_root,
        },
        flow::{
            JumpArgs,
            JumpMode,
        },
    },
    prelude::InterpreterStorage,
    state::ExecuteState,
    verification::Verifier,
};
use core::ops::Div;
use fuel_asm::{
    Instruction,
    PanicReason,
    RegId,
    narrowint,
    op::{
        ADD,
        ADDI,
    },
    wideint,
};
use fuel_types::Word;

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for ADD
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
        interpreter.gas_charge(interpreter.gas_costs().add())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_capture_overflow(
            a,
            u128::overflowing_add,
            interpreter.registers[b].into(),
            interpreter.registers[c].into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for ADDI
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
        interpreter.gas_charge(interpreter.gas_costs().addi())?;
        let (a, b, imm) = self.unpack();
        interpreter.alu_capture_overflow(
            a,
            u128::overflowing_add,
            interpreter.registers[b].into(),
            imm.into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::AND
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
        interpreter.gas_charge(interpreter.gas_costs().and())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_set(a, interpreter.registers[b] & interpreter.registers[c])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::ANDI
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
        interpreter.gas_charge(interpreter.gas_costs().andi())?;
        let (a, b, imm) = self.unpack();
        interpreter.alu_set(a, interpreter.registers[b] & Word::from(imm))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::DIV
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
        interpreter.gas_charge(interpreter.gas_costs().div())?;
        let (a, b, c) = self.unpack();
        let c = interpreter.registers[c];
        interpreter.alu_error(a, Word::div, interpreter.registers[b], c, c == 0)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::DIVI
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
        interpreter.gas_charge(interpreter.gas_costs().divi())?;
        let (a, b, imm) = self.unpack();
        let imm = Word::from(imm);
        interpreter.alu_error(a, Word::div, interpreter.registers[b], imm, imm == 0)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::EQ
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
        interpreter.gas_charge(interpreter.gas_costs().eq_())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_set(
            a,
            (interpreter.registers[b] == interpreter.registers[c]) as Word,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::EXP
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
        interpreter.gas_charge(interpreter.gas_costs().exp())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_boolean_overflow(
            a,
            alu::exp,
            interpreter.registers[b],
            interpreter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::EXPI
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
        interpreter.gas_charge(interpreter.gas_costs().expi())?;
        let (a, b, imm) = self.unpack();
        let expo = u32::from(imm);
        interpreter.alu_boolean_overflow(
            a,
            Word::overflowing_pow,
            interpreter.registers[b],
            expo,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::GT
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
        interpreter.gas_charge(interpreter.gas_costs().gt())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_set(
            a,
            (interpreter.registers[b] > interpreter.registers[c]) as Word,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::LT
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
        interpreter.gas_charge(interpreter.gas_costs().lt())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_set(
            a,
            (interpreter.registers[b] < interpreter.registers[c]) as Word,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WDCM
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
        interpreter.gas_charge(interpreter.gas_costs().wdcm())?;
        let (a, b, c, imm) = self.unpack();
        let args = wideint::CompareArgs::from_imm(imm)
            .ok_or(PanicReason::InvalidImmediateValue)?;
        interpreter.alu_wideint_cmp_u128(
            a,
            interpreter.registers[b],
            interpreter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WQCM
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
        interpreter.gas_charge(interpreter.gas_costs().wqcm())?;
        let (a, b, c, imm) = self.unpack();
        let args = wideint::CompareArgs::from_imm(imm)
            .ok_or(PanicReason::InvalidImmediateValue)?;
        interpreter.alu_wideint_cmp_u256(
            a,
            interpreter.registers[b],
            interpreter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WDOP
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
        interpreter.gas_charge(interpreter.gas_costs().wdop())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::MathArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpreter.alu_wideint_op_u128(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WQOP
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
        interpreter.gas_charge(interpreter.gas_costs().wqop())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::MathArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpreter.alu_wideint_op_u256(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WDML
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
        interpreter.gas_charge(interpreter.gas_costs().wdml())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::MulArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpreter.alu_wideint_mul_u128(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WQML
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
        interpreter.gas_charge(interpreter.gas_costs().wqml())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::MulArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpreter.alu_wideint_mul_u256(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WDDV
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
        interpreter.gas_charge(interpreter.gas_costs().wddv())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::DivArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpreter.alu_wideint_div_u128(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WQDV
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
        interpreter.gas_charge(interpreter.gas_costs().wqdv())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::DivArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpreter.alu_wideint_div_u256(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WDMD
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
        interpreter.gas_charge(interpreter.gas_costs().wdmd())?;
        let (a, b, c, d) = self.unpack();
        interpreter.alu_wideint_muldiv_u128(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WQMD
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
        interpreter.gas_charge(interpreter.gas_costs().wqmd())?;
        let (a, b, c, d) = self.unpack();
        interpreter.alu_wideint_muldiv_u256(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WDAM
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
        interpreter.gas_charge(interpreter.gas_costs().wdam())?;
        let (a, b, c, d) = self.unpack();
        interpreter.alu_wideint_addmod_u128(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}
impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WQAM
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
        interpreter.gas_charge(interpreter.gas_costs().wqam())?;
        let (a, b, c, d) = self.unpack();
        interpreter.alu_wideint_addmod_u256(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}
impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WDMM
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
        interpreter.gas_charge(interpreter.gas_costs().wdmm())?;
        let (a, b, c, d) = self.unpack();
        interpreter.alu_wideint_mulmod_u128(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}
impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::WQMM
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
        interpreter.gas_charge(interpreter.gas_costs().wqmm())?;
        let (a, b, c, d) = self.unpack();
        interpreter.alu_wideint_mulmod_u256(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MLOG
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
        interpreter.gas_charge(interpreter.gas_costs().mlog())?;
        let (a, b, c) = self.unpack();
        let (lhs, rhs) = (interpreter.registers[b], interpreter.registers[c]);
        interpreter.alu_error(
            a,
            |l, r| {
                l.checked_ilog(r)
                    .expect("checked_ilog returned None for valid values")
                    as Word
            },
            lhs,
            rhs,
            lhs == 0 || rhs <= 1,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MOD
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
        interpreter.gas_charge(interpreter.gas_costs().mod_op())?;
        let (a, b, c) = self.unpack();
        let rhs = interpreter.registers[c];
        interpreter.alu_error(
            a,
            Word::wrapping_rem,
            interpreter.registers[b],
            rhs,
            rhs == 0,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MODI
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
        interpreter.gas_charge(interpreter.gas_costs().modi())?;
        let (a, b, imm) = self.unpack();
        let rhs = Word::from(imm);
        interpreter.alu_error(
            a,
            Word::wrapping_rem,
            interpreter.registers[b],
            rhs,
            rhs == 0,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MOVE
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
        interpreter.gas_charge(interpreter.gas_costs().move_op())?;
        let (a, b) = self.unpack();
        interpreter.alu_set(a, interpreter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MOVI
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
        interpreter.gas_charge(interpreter.gas_costs().movi())?;
        let (a, imm) = self.unpack();
        interpreter.alu_set(a, Word::from(imm))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MROO
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
        interpreter.gas_charge(interpreter.gas_costs().mroo())?;
        let (a, b, c) = self.unpack();
        let (lhs, rhs) = (interpreter.registers[b], interpreter.registers[c]);
        interpreter.alu_error(
            a,
            |l, r| {
                checked_nth_root(l, r)
                    .expect("checked_nth_root returned None for valid values")
                    as Word
            },
            lhs,
            rhs,
            rhs == 0,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MUL
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
        interpreter.gas_charge(interpreter.gas_costs().mul())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_capture_overflow(
            a,
            u128::overflowing_mul,
            interpreter.registers[b].into(),
            interpreter.registers[c].into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MULI
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
        interpreter.gas_charge(interpreter.gas_costs().muli())?;
        let (a, b, imm) = self.unpack();
        interpreter.alu_capture_overflow(
            a,
            u128::overflowing_mul,
            interpreter.registers[b].into(),
            imm.into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MLDV
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
        interpreter.gas_charge(interpreter.gas_costs().mldv())?;
        let (a, b, c, d) = self.unpack();
        interpreter.alu_muldiv(
            a,
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::NIOP
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
        interpreter
            .gas_charge(interpreter.gas_costs().niop().map_err(PanicReason::from)?)?;
        let (a, b, c, imm) = self.unpack();
        let args = narrowint::MathArgs::from_imm(imm)
            .ok_or(PanicReason::InvalidImmediateValue)?;
        interpreter.alu_narrowint_op(
            a,
            interpreter.registers[b],
            interpreter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::NOOP
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
        interpreter.gas_charge(interpreter.gas_costs().noop())?;
        interpreter.alu_clear()?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::NOT
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
        interpreter.gas_charge(interpreter.gas_costs().not())?;
        let (a, b) = self.unpack();
        interpreter.alu_set(a, !interpreter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::OR
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
        interpreter.gas_charge(interpreter.gas_costs().or())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_set(a, interpreter.registers[b] | interpreter.registers[c])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::ORI
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
        interpreter.gas_charge(interpreter.gas_costs().ori())?;
        let (a, b, imm) = self.unpack();
        interpreter.alu_set(a, interpreter.registers[b] | Word::from(imm))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SLL
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
        interpreter.gas_charge(interpreter.gas_costs().sll())?;
        let (a, b, c) = self.unpack();

        interpreter.alu_set(
            a,
            if let Ok(c) = interpreter.registers[c].try_into() {
                Word::checked_shl(interpreter.registers[b], c).unwrap_or_default()
            } else {
                0
            },
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SLLI
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
        interpreter.gas_charge(interpreter.gas_costs().slli())?;
        let (a, b, imm) = self.unpack();
        let rhs = u32::from(imm);
        interpreter.alu_set(
            a,
            interpreter.registers[b]
                .checked_shl(rhs)
                .unwrap_or_default(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SRL
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
        interpreter.gas_charge(interpreter.gas_costs().srl())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_set(
            a,
            if let Ok(c) = interpreter.registers[c].try_into() {
                Word::checked_shr(interpreter.registers[b], c).unwrap_or_default()
            } else {
                0
            },
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SRLI
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
        interpreter.gas_charge(interpreter.gas_costs().srli())?;
        let (a, b, imm) = self.unpack();
        let rhs = u32::from(imm);
        interpreter.alu_set(
            a,
            interpreter.registers[b]
                .checked_shr(rhs)
                .unwrap_or_default(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SUB
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
        interpreter.gas_charge(interpreter.gas_costs().sub())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_capture_overflow(
            a,
            u128::overflowing_sub,
            interpreter.registers[b].into(),
            interpreter.registers[c].into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SUBI
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
        interpreter.gas_charge(interpreter.gas_costs().subi())?;
        let (a, b, imm) = self.unpack();
        interpreter.alu_capture_overflow(
            a,
            u128::overflowing_sub,
            interpreter.registers[b].into(),
            imm.into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::XOR
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
        interpreter.gas_charge(interpreter.gas_costs().xor())?;
        let (a, b, c) = self.unpack();
        interpreter.alu_set(a, interpreter.registers[b] ^ interpreter.registers[c])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::XORI
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
        interpreter.gas_charge(interpreter.gas_costs().xori())?;
        let (a, b, imm) = self.unpack();
        interpreter.alu_set(a, interpreter.registers[b] ^ Word::from(imm))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JI
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
        interpreter.gas_charge(interpreter.gas_costs().ji())?;
        let imm = self.unpack();
        interpreter.jump(JumpArgs::new(JumpMode::RelativeIS).to_address(imm.into()))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JNEI
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
        interpreter.gas_charge(interpreter.gas_costs().jnei())?;
        let (a, b, imm) = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeIS)
                .with_condition(interpreter.registers[a] != interpreter.registers[b])
                .to_address(imm.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JNZI
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
        interpreter.gas_charge(interpreter.gas_costs().jnzi())?;
        let (a, imm) = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeIS)
                .with_condition(interpreter.registers[a] != 0)
                .to_address(imm.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JMP
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
        interpreter.gas_charge(interpreter.gas_costs().jmp())?;
        let a = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeIS).to_address(interpreter.registers[a]),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JNE
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
        interpreter.gas_charge(interpreter.gas_costs().jne())?;
        let (a, b, c) = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeIS)
                .with_condition(interpreter.registers[a] != interpreter.registers[b])
                .to_address(interpreter.registers[c]),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JMPF
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
        interpreter.gas_charge(interpreter.gas_costs().jmpf())?;
        let (a, offset) = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeForwards)
                .to_address(interpreter.registers[a])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JMPB
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
        interpreter.gas_charge(interpreter.gas_costs().jmpb())?;
        let (a, offset) = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeBackwards)
                .to_address(interpreter.registers[a])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JNZF
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
        interpreter.gas_charge(interpreter.gas_costs().jnzf())?;
        let (a, b, offset) = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeForwards)
                .with_condition(interpreter.registers[a] != 0)
                .to_address(interpreter.registers[b])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JNZB
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
        interpreter.gas_charge(interpreter.gas_costs().jnzb())?;
        let (a, b, offset) = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeBackwards)
                .with_condition(interpreter.registers[a] != 0)
                .to_address(interpreter.registers[b])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JNEF
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
        interpreter.gas_charge(interpreter.gas_costs().jnef())?;
        let (a, b, c, offset) = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeForwards)
                .with_condition(interpreter.registers[a] != interpreter.registers[b])
                .to_address(interpreter.registers[c])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JNEB
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
        interpreter.gas_charge(interpreter.gas_costs().jneb())?;
        let (a, b, c, offset) = self.unpack();
        interpreter.jump(
            JumpArgs::new(JumpMode::RelativeBackwards)
                .with_condition(interpreter.registers[a] != interpreter.registers[b])
                .to_address(interpreter.registers[c])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::JAL
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
        interpreter.gas_charge(interpreter.gas_costs().jmp())?;
        let (reg_ret_addr, reg_target, offset) = self.unpack();

        // Storing return address to zero register discards it instead
        // While we use saturating_add here, the PC shoudln't ever have values above the
        // memory size anyway
        let ret_addr =
            interpreter.registers[RegId::PC].saturating_add(Instruction::SIZE as u64);
        interpreter.set_user_reg_or_discard(reg_ret_addr, ret_addr)?;

        interpreter.jump(
            JumpArgs::new(JumpMode::Assign)
                .to_address(interpreter.registers[reg_target])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::RET
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
        interpreter.gas_charge(interpreter.gas_costs().ret())?;
        let a = self.unpack();
        let ra = interpreter.registers[a];
        interpreter.ret(ra)?;
        Ok(ExecuteState::Return(ra))
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::RETD
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
        let (a, b) = self.unpack();
        let len = interpreter.registers[b];
        interpreter.dependent_gas_charge(interpreter.gas_costs().retd(), len)?;
        Ok(interpreter
            .ret_data(interpreter.registers[a], len)
            .map(ExecuteState::ReturnData)?)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::RVRT
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
        interpreter.gas_charge(interpreter.gas_costs().rvrt())?;
        let a = self.unpack();
        let ra = interpreter.registers[a];
        interpreter.revert(ra)?;
        Ok(ExecuteState::Revert(ra))
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SMO
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
        let (a, b, c, d) = self.unpack();
        interpreter.dependent_gas_charge(
            interpreter.gas_costs().smo(),
            interpreter.registers[c],
        )?;
        interpreter.message_output(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::ALOC
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
        let a = self.unpack();
        let number_of_bytes = interpreter.registers[a];
        interpreter
            .dependent_gas_charge(interpreter.gas_costs().aloc(), number_of_bytes)?;
        interpreter.malloc(number_of_bytes)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::CFEI
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
        let number_of_bytes = self.unpack().into();
        interpreter
            .dependent_gas_charge(interpreter.gas_costs().cfei(), number_of_bytes)?;
        interpreter.stack_pointer_overflow(Word::overflowing_add, number_of_bytes)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::CFE
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
        let a = self.unpack();
        let number_of_bytes = interpreter.registers[a];
        interpreter
            .dependent_gas_charge(interpreter.gas_costs().cfe(), number_of_bytes)?;
        interpreter.stack_pointer_overflow(Word::overflowing_add, number_of_bytes)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::CFSI
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
        interpreter.gas_charge(interpreter.gas_costs().cfsi())?;
        let imm = self.unpack();
        interpreter.stack_pointer_overflow(Word::overflowing_sub, imm.into())?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::CFS
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
        interpreter.gas_charge(interpreter.gas_costs().cfsi())?;
        let a = self.unpack();
        interpreter
            .stack_pointer_overflow(Word::overflowing_sub, interpreter.registers[a])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::PSHL
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
        interpreter.gas_charge(interpreter.gas_costs().pshl())?;
        let bitmask = self.unpack();
        interpreter.push_selected_registers(ProgramRegistersSegment::Low, bitmask)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::PSHH
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
        interpreter.gas_charge(interpreter.gas_costs().pshh())?;
        let bitmask = self.unpack();
        interpreter.push_selected_registers(ProgramRegistersSegment::High, bitmask)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::POPL
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
        interpreter.gas_charge(interpreter.gas_costs().popl())?;
        let bitmask = self.unpack();
        interpreter.pop_selected_registers(ProgramRegistersSegment::Low, bitmask)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::POPH
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
        interpreter.gas_charge(interpreter.gas_costs().poph())?;
        let bitmask = self.unpack();
        interpreter.pop_selected_registers(ProgramRegistersSegment::High, bitmask)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::LB
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
        interpreter.gas_charge(interpreter.gas_costs().lb())?;
        let (a, b, imm) = self.unpack();
        interpreter.load_u8(a, interpreter.registers[b], imm)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::LQW
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
        interpreter.gas_charge(interpreter.gas_costs().lw())?;
        let (a, b, imm) = self.unpack();
        interpreter.load_u16(a, interpreter.registers[b], imm)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::LHW
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
        interpreter.gas_charge(interpreter.gas_costs().lw())?;
        let (a, b, imm) = self.unpack();
        interpreter.load_u32(a, interpreter.registers[b], imm)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::LW
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
        interpreter.gas_charge(interpreter.gas_costs().lw())?;
        let (a, b, imm) = self.unpack();
        interpreter.load_u64(a, interpreter.registers[b], imm)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MCL
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
        let (a, b) = self.unpack();
        let len = interpreter.registers[b];
        interpreter.dependent_gas_charge(interpreter.gas_costs().mcl(), len)?;
        interpreter.memclear(interpreter.registers[a], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MCLI
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
        let (a, imm) = self.unpack();
        let len = Word::from(imm);
        interpreter.dependent_gas_charge(interpreter.gas_costs().mcli(), len)?;
        interpreter.memclear(interpreter.registers[a], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MCP
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
        let (a, b, c) = self.unpack();
        let len = interpreter.registers[c];
        interpreter.dependent_gas_charge(interpreter.gas_costs().mcp(), len)?;
        interpreter.memcopy(interpreter.registers[a], interpreter.registers[b], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MCPI
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
        let (a, b, imm) = self.unpack();
        let len = imm.into();
        interpreter.dependent_gas_charge(interpreter.gas_costs().mcpi(), len)?;
        interpreter.memcopy(interpreter.registers[a], interpreter.registers[b], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MEQ
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
        let (a, b, c, d) = self.unpack();
        let len = interpreter.registers[d];
        interpreter.dependent_gas_charge(interpreter.gas_costs().meq(), len)?;
        interpreter.memeq(a, interpreter.registers[b], interpreter.registers[c], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SB
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
        interpreter.gas_charge(interpreter.gas_costs().sb())?;
        let (a, b, imm) = self.unpack();
        interpreter.store_u8(interpreter.registers[a], interpreter.registers[b], imm)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SQW
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
        interpreter.gas_charge(interpreter.gas_costs().sw())?;
        let (a, b, imm) = self.unpack();
        interpreter.store_u16(interpreter.registers[a], interpreter.registers[b], imm)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SHW
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
        interpreter.gas_charge(interpreter.gas_costs().sw())?;
        let (a, b, imm) = self.unpack();
        interpreter.store_u32(interpreter.registers[a], interpreter.registers[b], imm)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SW
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
        interpreter.gas_charge(interpreter.gas_costs().sw())?;
        let (a, b, imm) = self.unpack();
        interpreter.store_u64(interpreter.registers[a], interpreter.registers[b], imm)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::BAL
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
        interpreter.gas_charge(interpreter.gas_costs().bal())?;
        let (a, b, c) = self.unpack();
        interpreter.contract_balance(
            a,
            interpreter.registers[b],
            interpreter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::BHEI
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
        interpreter.gas_charge(interpreter.gas_costs().bhei())?;
        let a = self.unpack();
        interpreter.block_height(a)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::BHSH
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
        interpreter.gas_charge(interpreter.gas_costs().bhsh())?;
        let (a, b) = self.unpack();
        interpreter.block_hash(interpreter.registers[a], interpreter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::BURN
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
        interpreter.gas_charge(interpreter.gas_costs().burn())?;
        let (a, b) = self.unpack();
        interpreter.burn(interpreter.registers[a], interpreter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::CALL
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
        // We charge for the gas inside of the `prepare_call` function.
        let (a, b, c, d) = self.unpack();

        // Enter call context
        interpreter.prepare_call(a, b, c, d)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::CB
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
        interpreter.gas_charge(interpreter.gas_costs().cb())?;
        let a = self.unpack();
        interpreter.block_proposer(interpreter.registers[a])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::CCP
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
        let (a, b, c, d) = self.unpack();
        interpreter.code_copy(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::CROO
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
        let (a, b) = self.unpack();
        interpreter.code_root(interpreter.registers[a], interpreter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::CSIZ
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
        // We charge for the gas inside of the `code_size` function.
        let (a, b) = self.unpack();
        interpreter.code_size(a, interpreter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::LDC
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
        // We charge for the gas inside of the `load_contract_code` function.
        let (a, b, c, mode) = self.unpack();
        interpreter.load_contract_code(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            mode,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::LOG
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
        interpreter.gas_charge(interpreter.gas_costs().log())?;
        let (a, b, c, d) = self.unpack();
        interpreter.log(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::LOGD
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
        let (a, b, c, d) = self.unpack();
        interpreter.dependent_gas_charge(
            interpreter.gas_costs().logd(),
            interpreter.registers[d],
        )?;
        interpreter.log_data(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::MINT
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
        interpreter.gas_charge(interpreter.gas_costs().mint())?;
        let (a, b) = self.unpack();
        interpreter.mint(interpreter.registers[a], interpreter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SCWQ
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
        let (a, b, c) = self.unpack();
        interpreter.dependent_gas_charge(
            interpreter.gas_costs().scwq(),
            interpreter.registers[c],
        )?;
        interpreter.state_clear_qword(
            interpreter.registers[a],
            b,
            interpreter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SRW
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
        interpreter.gas_charge(interpreter.gas_costs().srw())?;
        let (a, b, c) = self.unpack();
        interpreter.state_read_word(a, b, interpreter.registers[c])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SRWQ
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
        let (a, b, c, d) = self.unpack();
        interpreter.dependent_gas_charge(
            interpreter.gas_costs().srwq(),
            interpreter.registers[d],
        )?;
        interpreter.state_read_qword(
            interpreter.registers[a],
            b,
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SWW
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
        interpreter.gas_charge(interpreter.gas_costs().sww())?;
        let (a, b, c) = self.unpack();
        interpreter.state_write_word(
            interpreter.registers[a],
            b,
            interpreter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::SWWQ
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
        let (a, b, c, d) = self.unpack();
        interpreter.dependent_gas_charge(
            interpreter.gas_costs().swwq(),
            interpreter.registers[d],
        )?;
        interpreter.state_write_qword(
            interpreter.registers[a],
            b,
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::TIME
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
        interpreter.gas_charge(interpreter.gas_costs().time())?;
        let (a, b) = self.unpack();
        interpreter.timestamp(a, interpreter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::ECK1
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
        interpreter.gas_charge(interpreter.gas_costs().eck1())?;
        let (a, b, c) = self.unpack();
        interpreter.secp256k1_recover(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::ECR1
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
        interpreter.gas_charge(interpreter.gas_costs().ecr1())?;
        let (a, b, c) = self.unpack();
        interpreter.secp256r1_recover(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::ED19
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
        let (a, b, c, len) = self.unpack();
        let mut len = interpreter.registers[len];

        // Backwards compatibility with old contracts
        if len == 0 {
            len = 32;
        }

        interpreter.dependent_gas_charge(interpreter.gas_costs().ed19(), len)?;
        interpreter.ed25519_verify(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            len,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::K256
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
        let (a, b, c) = self.unpack();
        let len = interpreter.registers[c];
        interpreter.dependent_gas_charge(interpreter.gas_costs().k256(), len)?;
        interpreter.keccak256(interpreter.registers[a], interpreter.registers[b], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::S256
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
        let (a, b, c) = self.unpack();
        let len = interpreter.registers[c];
        interpreter.dependent_gas_charge(interpreter.gas_costs().s256(), len)?;
        interpreter.sha256(interpreter.registers[a], interpreter.registers[b], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::FLAG
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
        interpreter.gas_charge(interpreter.gas_costs().flag())?;
        let a = self.unpack();
        interpreter.set_flag(interpreter.registers[a])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::GM
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
        interpreter.gas_charge(interpreter.gas_costs().gm())?;
        let (a, imm) = self.unpack();
        interpreter.metadata(a, imm.into())?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::GTF
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
        interpreter.gas_charge(interpreter.gas_costs().gtf())?;
        let (a, b, imm) = self.unpack();
        interpreter.get_transaction_field(a, interpreter.registers[b], imm.into())?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::TR
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
        interpreter.gas_charge(interpreter.gas_costs().tr())?;
        let (a, b, c) = self.unpack();
        interpreter.transfer(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::TRO
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
        interpreter.gas_charge(interpreter.gas_costs().tro())?;
        let (a, b, c, d) = self.unpack();
        interpreter.transfer_output(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::ECAL
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
        let (a, b, c, d) = self.unpack();
        interpreter.external_call(a, b, c, d)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::BSIZ
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
        // We charge for this inside the function.
        let (a, b) = self.unpack();
        interpreter.blob_size(a, interpreter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::BLDD
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
        // We charge for this inside the function.
        let (a, b, c, d) = self.unpack();
        interpreter.blob_load_data(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::ECOP
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
        interpreter
            .gas_charge(interpreter.gas_costs().ecop().map_err(PanicReason::from)?)?;
        let (a, b, c, d) = self.unpack();
        interpreter.ec_operation(
            interpreter.registers[a],
            interpreter.registers[b],
            interpreter.registers[c],
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal, V> Execute<M, S, Tx, Ecal, V> for fuel_asm::op::EPAR
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
        let (a, b, c, d) = self.unpack();
        let len = interpreter.registers[c];
        interpreter.dependent_gas_charge(
            interpreter.gas_costs().epar().map_err(PanicReason::from)?,
            len,
        )?;
        interpreter.ec_pairing(
            a,
            interpreter.registers[b],
            len,
            interpreter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}
