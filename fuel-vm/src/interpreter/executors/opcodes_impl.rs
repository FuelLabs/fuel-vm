use crate::{
    constraints::reg_key::ProgramRegistersSegment,
    error::IoResult,
    interpreter::{
        alu,
        executors::instruction::{
            checked_nth_root,
            Execute,
        },
        flow::{
            JumpArgs,
            JumpMode,
        },
        EcalHandler,
        ExecutableTransaction,
        Interpreter,
        Memory,
    },
    prelude::InterpreterStorage,
    state::ExecuteState,
};
use core::ops::Div;
use fuel_asm::{
    op::{
        ADD,
        ADDI,
    },
    wideint,
    PanicReason,
};
use fuel_types::Word;

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for ADD
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
        interpriter.gas_charge(interpriter.gas_costs().add())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_capture_overflow(
            a,
            u128::overflowing_add,
            interpriter.registers[b].into(),
            interpriter.registers[c].into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for ADDI
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
        interpriter.gas_charge(interpriter.gas_costs().addi())?;
        let (a, b, imm) = self.unpack();
        interpriter.alu_capture_overflow(
            a,
            u128::overflowing_add,
            interpriter.registers[b].into(),
            imm.into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::AND
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
        interpriter.gas_charge(interpriter.gas_costs().and())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_set(a, interpriter.registers[b] & interpriter.registers[c])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::ANDI
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
        interpriter.gas_charge(interpriter.gas_costs().andi())?;
        let (a, b, imm) = self.unpack();
        interpriter.alu_set(a, interpriter.registers[b] & Word::from(imm))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::DIV
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
        interpriter.gas_charge(interpriter.gas_costs().div())?;
        let (a, b, c) = self.unpack();
        let c = interpriter.registers[c];
        interpriter.alu_error(a, Word::div, interpriter.registers[b], c, c == 0)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::DIVI
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
        interpriter.gas_charge(interpriter.gas_costs().divi())?;
        let (a, b, imm) = self.unpack();
        let imm = Word::from(imm);
        interpriter.alu_error(a, Word::div, interpriter.registers[b], imm, imm == 0)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::EQ
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
        interpriter.gas_charge(interpriter.gas_costs().eq_())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_set(
            a,
            (interpriter.registers[b] == interpriter.registers[c]) as Word,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::EXP
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
        interpriter.gas_charge(interpriter.gas_costs().exp())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_boolean_overflow(
            a,
            alu::exp,
            interpriter.registers[b],
            interpriter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::EXPI
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
        interpriter.gas_charge(interpriter.gas_costs().expi())?;
        let (a, b, imm) = self.unpack();
        let expo = u32::from(imm);
        interpriter.alu_boolean_overflow(
            a,
            Word::overflowing_pow,
            interpriter.registers[b],
            expo,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::GT
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
        interpriter.gas_charge(interpriter.gas_costs().gt())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_set(
            a,
            (interpriter.registers[b] > interpriter.registers[c]) as Word,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::LT
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
        interpriter.gas_charge(interpriter.gas_costs().lt())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_set(
            a,
            (interpriter.registers[b] < interpriter.registers[c]) as Word,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WDCM
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
        interpriter.gas_charge(interpriter.gas_costs().wdcm())?;
        let (a, b, c, imm) = self.unpack();
        let args = wideint::CompareArgs::from_imm(imm)
            .ok_or(PanicReason::InvalidImmediateValue)?;
        interpriter.alu_wideint_cmp_u128(
            a,
            interpriter.registers[b],
            interpriter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WQCM
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
        interpriter.gas_charge(interpriter.gas_costs().wqcm())?;
        let (a, b, c, imm) = self.unpack();
        let args = wideint::CompareArgs::from_imm(imm)
            .ok_or(PanicReason::InvalidImmediateValue)?;
        interpriter.alu_wideint_cmp_u256(
            a,
            interpriter.registers[b],
            interpriter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WDOP
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
        interpriter.gas_charge(interpriter.gas_costs().wdop())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::MathArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpriter.alu_wideint_op_u128(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WQOP
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
        interpriter.gas_charge(interpriter.gas_costs().wqop())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::MathArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpriter.alu_wideint_op_u256(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WDML
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
        interpriter.gas_charge(interpriter.gas_costs().wdml())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::MulArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpriter.alu_wideint_mul_u128(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WQML
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
        interpriter.gas_charge(interpriter.gas_costs().wqml())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::MulArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpriter.alu_wideint_mul_u256(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WDDV
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
        interpriter.gas_charge(interpriter.gas_costs().wddv())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::DivArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpriter.alu_wideint_div_u128(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WQDV
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
        interpriter.gas_charge(interpriter.gas_costs().wqdv())?;
        let (a, b, c, imm) = self.unpack();
        let args =
            wideint::DivArgs::from_imm(imm).ok_or(PanicReason::InvalidImmediateValue)?;
        interpriter.alu_wideint_div_u256(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            args,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WDMD
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
        interpriter.gas_charge(interpriter.gas_costs().wdmd())?;
        let (a, b, c, d) = self.unpack();
        interpriter.alu_wideint_muldiv_u128(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WQMD
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
        interpriter.gas_charge(interpriter.gas_costs().wqmd())?;
        let (a, b, c, d) = self.unpack();
        interpriter.alu_wideint_muldiv_u256(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WDAM
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
        interpriter.gas_charge(interpriter.gas_costs().wdam())?;
        let (a, b, c, d) = self.unpack();
        interpriter.alu_wideint_addmod_u128(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}
impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WQAM
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
        interpriter.gas_charge(interpriter.gas_costs().wqam())?;
        let (a, b, c, d) = self.unpack();
        interpriter.alu_wideint_addmod_u256(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}
impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WDMM
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
        interpriter.gas_charge(interpriter.gas_costs().wdmm())?;
        let (a, b, c, d) = self.unpack();
        interpriter.alu_wideint_mulmod_u128(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}
impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::WQMM
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
        interpriter.gas_charge(interpriter.gas_costs().wqmm())?;
        let (a, b, c, d) = self.unpack();
        interpriter.alu_wideint_mulmod_u256(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MLOG
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
        interpriter.gas_charge(interpriter.gas_costs().mlog())?;
        let (a, b, c) = self.unpack();
        let (lhs, rhs) = (interpriter.registers[b], interpriter.registers[c]);
        interpriter.alu_error(
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

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MOD
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
        interpriter.gas_charge(interpriter.gas_costs().mod_op())?;
        let (a, b, c) = self.unpack();
        let rhs = interpriter.registers[c];
        interpriter.alu_error(
            a,
            Word::wrapping_rem,
            interpriter.registers[b],
            rhs,
            rhs == 0,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MODI
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
        interpriter.gas_charge(interpriter.gas_costs().modi())?;
        let (a, b, imm) = self.unpack();
        let rhs = Word::from(imm);
        interpriter.alu_error(
            a,
            Word::wrapping_rem,
            interpriter.registers[b],
            rhs,
            rhs == 0,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MOVE
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
        interpriter.gas_charge(interpriter.gas_costs().move_op())?;
        let (a, b) = self.unpack();
        interpriter.alu_set(a, interpriter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MOVI
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
        interpriter.gas_charge(interpriter.gas_costs().movi())?;
        let (a, imm) = self.unpack();
        interpriter.alu_set(a, Word::from(imm))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MROO
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
        interpriter.gas_charge(interpriter.gas_costs().mroo())?;
        let (a, b, c) = self.unpack();
        let (lhs, rhs) = (interpriter.registers[b], interpriter.registers[c]);
        interpriter.alu_error(
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

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MUL
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
        interpriter.gas_charge(interpriter.gas_costs().mul())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_capture_overflow(
            a,
            u128::overflowing_mul,
            interpriter.registers[b].into(),
            interpriter.registers[c].into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MULI
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
        interpriter.gas_charge(interpriter.gas_costs().muli())?;
        let (a, b, imm) = self.unpack();
        interpriter.alu_capture_overflow(
            a,
            u128::overflowing_mul,
            interpriter.registers[b].into(),
            imm.into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MLDV
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
        interpriter.gas_charge(interpriter.gas_costs().mldv())?;
        let (a, b, c, d) = self.unpack();
        interpriter.alu_muldiv(
            a,
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::NOOP
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
        interpriter.gas_charge(interpriter.gas_costs().noop())?;
        interpriter.alu_clear()?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::NOT
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
        interpriter.gas_charge(interpriter.gas_costs().not())?;
        let (a, b) = self.unpack();
        interpriter.alu_set(a, !interpriter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::OR
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
        interpriter.gas_charge(interpriter.gas_costs().or())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_set(a, interpriter.registers[b] | interpriter.registers[c])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::ORI
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
        interpriter.gas_charge(interpriter.gas_costs().ori())?;
        let (a, b, imm) = self.unpack();
        interpriter.alu_set(a, interpriter.registers[b] | Word::from(imm))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SLL
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
        interpriter.gas_charge(interpriter.gas_costs().sll())?;
        let (a, b, c) = self.unpack();

        interpriter.alu_set(
            a,
            if let Ok(c) = interpriter.registers[c].try_into() {
                Word::checked_shl(interpriter.registers[b], c).unwrap_or_default()
            } else {
                0
            },
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SLLI
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
        interpriter.gas_charge(interpriter.gas_costs().slli())?;
        let (a, b, imm) = self.unpack();
        let rhs = u32::from(imm);
        interpriter.alu_set(
            a,
            interpriter.registers[b]
                .checked_shl(rhs)
                .unwrap_or_default(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SRL
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
        interpriter.gas_charge(interpriter.gas_costs().srl())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_set(
            a,
            if let Ok(c) = interpriter.registers[c].try_into() {
                Word::checked_shr(interpriter.registers[b], c).unwrap_or_default()
            } else {
                0
            },
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SRLI
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
        interpriter.gas_charge(interpriter.gas_costs().srli())?;
        let (a, b, imm) = self.unpack();
        let rhs = u32::from(imm);
        interpriter.alu_set(
            a,
            interpriter.registers[b]
                .checked_shr(rhs)
                .unwrap_or_default(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SUB
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
        interpriter.gas_charge(interpriter.gas_costs().sub())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_capture_overflow(
            a,
            u128::overflowing_sub,
            interpriter.registers[b].into(),
            interpriter.registers[c].into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SUBI
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
        interpriter.gas_charge(interpriter.gas_costs().subi())?;
        let (a, b, imm) = self.unpack();
        interpriter.alu_capture_overflow(
            a,
            u128::overflowing_sub,
            interpriter.registers[b].into(),
            imm.into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::XOR
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
        interpriter.gas_charge(interpriter.gas_costs().xor())?;
        let (a, b, c) = self.unpack();
        interpriter.alu_set(a, interpriter.registers[b] ^ interpriter.registers[c])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::XORI
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
        interpriter.gas_charge(interpriter.gas_costs().xori())?;
        let (a, b, imm) = self.unpack();
        interpriter.alu_set(a, interpriter.registers[b] ^ Word::from(imm))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JI
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
        interpriter.gas_charge(interpriter.gas_costs().ji())?;
        let imm = self.unpack();
        interpriter.jump(JumpArgs::new(JumpMode::Absolute).to_address(imm.into()))?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JNEI
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
        interpriter.gas_charge(interpriter.gas_costs().jnei())?;
        let (a, b, imm) = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::Absolute)
                .with_condition(interpriter.registers[a] != interpriter.registers[b])
                .to_address(imm.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JNZI
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
        interpriter.gas_charge(interpriter.gas_costs().jnzi())?;
        let (a, imm) = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::Absolute)
                .with_condition(interpriter.registers[a] != 0)
                .to_address(imm.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JMP
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
        interpriter.gas_charge(interpriter.gas_costs().jmp())?;
        let a = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::Absolute).to_address(interpriter.registers[a]),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JNE
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
        interpriter.gas_charge(interpriter.gas_costs().jne())?;
        let (a, b, c) = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::Absolute)
                .with_condition(interpriter.registers[a] != interpriter.registers[b])
                .to_address(interpriter.registers[c]),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JMPF
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
        interpriter.gas_charge(interpriter.gas_costs().jmpf())?;
        let (a, offset) = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::RelativeForwards)
                .to_address(interpriter.registers[a])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JMPB
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
        interpriter.gas_charge(interpriter.gas_costs().jmpb())?;
        let (a, offset) = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::RelativeBackwards)
                .to_address(interpriter.registers[a])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JNZF
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
        interpriter.gas_charge(interpriter.gas_costs().jnzf())?;
        let (a, b, offset) = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::RelativeForwards)
                .with_condition(interpriter.registers[a] != 0)
                .to_address(interpriter.registers[b])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JNZB
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
        interpriter.gas_charge(interpriter.gas_costs().jnzb())?;
        let (a, b, offset) = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::RelativeBackwards)
                .with_condition(interpriter.registers[a] != 0)
                .to_address(interpriter.registers[b])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JNEF
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
        interpriter.gas_charge(interpriter.gas_costs().jnef())?;
        let (a, b, c, offset) = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::RelativeForwards)
                .with_condition(interpriter.registers[a] != interpriter.registers[b])
                .to_address(interpriter.registers[c])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::JNEB
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
        interpriter.gas_charge(interpriter.gas_costs().jneb())?;
        let (a, b, c, offset) = self.unpack();
        interpriter.jump(
            JumpArgs::new(JumpMode::RelativeBackwards)
                .with_condition(interpriter.registers[a] != interpriter.registers[b])
                .to_address(interpriter.registers[c])
                .plus_fixed(offset.into()),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::RET
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
        interpriter.gas_charge(interpriter.gas_costs().ret())?;
        let a = self.unpack();
        let ra = interpriter.registers[a];
        interpriter.ret(ra)?;
        Ok(ExecuteState::Return(ra))
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::RETD
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
        let (a, b) = self.unpack();
        let len = interpriter.registers[b];
        interpriter.dependent_gas_charge(interpriter.gas_costs().retd(), len)?;
        Ok(interpriter
            .ret_data(interpriter.registers[a], len)
            .map(ExecuteState::ReturnData)?)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::RVRT
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
        interpriter.gas_charge(interpriter.gas_costs().rvrt())?;
        let a = self.unpack();
        let ra = interpriter.registers[a];
        interpriter.revert(ra)?;
        Ok(ExecuteState::Revert(ra))
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SMO
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
        let (a, b, c, d) = self.unpack();
        interpriter.dependent_gas_charge(
            interpriter.gas_costs().smo(),
            interpriter.registers[c],
        )?;
        interpriter.message_output(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::ALOC
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
        let a = self.unpack();
        let number_of_bytes = interpriter.registers[a];
        interpriter
            .dependent_gas_charge(interpriter.gas_costs().aloc(), number_of_bytes)?;
        interpriter.malloc(number_of_bytes)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::CFEI
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
        let number_of_bytes = self.unpack().into();
        interpriter
            .dependent_gas_charge(interpriter.gas_costs().cfei(), number_of_bytes)?;
        interpriter.stack_pointer_overflow(Word::overflowing_add, number_of_bytes)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::CFE
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
        let a = self.unpack();
        let number_of_bytes = interpriter.registers[a];
        interpriter
            .dependent_gas_charge(interpriter.gas_costs().cfe(), number_of_bytes)?;
        interpriter.stack_pointer_overflow(Word::overflowing_add, number_of_bytes)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::CFSI
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
        interpriter.gas_charge(interpriter.gas_costs().cfsi())?;
        let imm = self.unpack();
        interpriter.stack_pointer_overflow(Word::overflowing_sub, imm.into())?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::CFS
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
        interpriter.gas_charge(interpriter.gas_costs().cfsi())?;
        let a = self.unpack();
        interpriter
            .stack_pointer_overflow(Word::overflowing_sub, interpriter.registers[a])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::PSHL
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
        interpriter.gas_charge(interpriter.gas_costs().pshl())?;
        let bitmask = self.unpack();
        interpriter.push_selected_registers(ProgramRegistersSegment::Low, bitmask)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::PSHH
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
        interpriter.gas_charge(interpriter.gas_costs().pshh())?;
        let bitmask = self.unpack();
        interpriter.push_selected_registers(ProgramRegistersSegment::High, bitmask)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::POPL
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
        interpriter.gas_charge(interpriter.gas_costs().popl())?;
        let bitmask = self.unpack();
        interpriter.pop_selected_registers(ProgramRegistersSegment::Low, bitmask)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::POPH
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
        interpriter.gas_charge(interpriter.gas_costs().poph())?;
        let bitmask = self.unpack();
        interpriter.pop_selected_registers(ProgramRegistersSegment::High, bitmask)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::LB
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
        interpriter.gas_charge(interpriter.gas_costs().lb())?;
        let (a, b, imm) = self.unpack();
        interpriter.load_byte(a, interpriter.registers[b], imm.into())?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::LW
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
        interpriter.gas_charge(interpriter.gas_costs().lw())?;
        let (a, b, imm) = self.unpack();
        interpriter.load_word(a, interpriter.registers[b], imm)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MCL
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
        let (a, b) = self.unpack();
        let len = interpriter.registers[b];
        interpriter.dependent_gas_charge(interpriter.gas_costs().mcl(), len)?;
        interpriter.memclear(interpriter.registers[a], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MCLI
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
        let (a, imm) = self.unpack();
        let len = Word::from(imm);
        interpriter.dependent_gas_charge(interpriter.gas_costs().mcli(), len)?;
        interpriter.memclear(interpriter.registers[a], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MCP
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
        let (a, b, c) = self.unpack();
        let len = interpriter.registers[c];
        interpriter.dependent_gas_charge(interpriter.gas_costs().mcp(), len)?;
        interpriter.memcopy(interpriter.registers[a], interpriter.registers[b], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MCPI
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
        let (a, b, imm) = self.unpack();
        let len = imm.into();
        interpriter.dependent_gas_charge(interpriter.gas_costs().mcpi(), len)?;
        interpriter.memcopy(interpriter.registers[a], interpriter.registers[b], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MEQ
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
        let (a, b, c, d) = self.unpack();
        let len = interpriter.registers[d];
        interpriter.dependent_gas_charge(interpriter.gas_costs().meq(), len)?;
        interpriter.memeq(a, interpriter.registers[b], interpriter.registers[c], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SB
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
        interpriter.gas_charge(interpriter.gas_costs().sb())?;
        let (a, b, imm) = self.unpack();
        interpriter.store_byte(
            interpriter.registers[a],
            interpriter.registers[b],
            imm.into(),
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SW
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
        interpriter.gas_charge(interpriter.gas_costs().sw())?;
        let (a, b, imm) = self.unpack();
        interpriter.store_word(
            interpriter.registers[a],
            interpriter.registers[b],
            imm,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::BAL
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
        interpriter.gas_charge(interpriter.gas_costs().bal())?;
        let (a, b, c) = self.unpack();
        interpriter.contract_balance(
            a,
            interpriter.registers[b],
            interpriter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::BHEI
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
        interpriter.gas_charge(interpriter.gas_costs().bhei())?;
        let a = self.unpack();
        interpriter.block_height(a)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::BHSH
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
        interpriter.gas_charge(interpriter.gas_costs().bhsh())?;
        let (a, b) = self.unpack();
        interpriter.block_hash(interpriter.registers[a], interpriter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::BURN
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
        interpriter.gas_charge(interpriter.gas_costs().burn())?;
        let (a, b) = self.unpack();
        interpriter.burn(interpriter.registers[a], interpriter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::CALL
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
        // We charge for the gas inside of the `prepare_call` function.
        let (a, b, c, d) = self.unpack();

        // Enter call context
        interpriter.prepare_call(a, b, c, d)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::CB
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
        interpriter.gas_charge(interpriter.gas_costs().cb())?;
        let a = self.unpack();
        interpriter.block_proposer(interpriter.registers[a])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::CCP
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
        let (a, b, c, d) = self.unpack();
        interpriter.code_copy(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::CROO
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
        let (a, b) = self.unpack();
        interpriter.code_root(interpriter.registers[a], interpriter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::CSIZ
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
        // We charge for the gas inside of the `code_size` function.
        let (a, b) = self.unpack();
        interpriter.code_size(a, interpriter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::LDC
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
        // We charge for the gas inside of the `load_contract_code` function.
        let (a, b, c, mode) = self.unpack();
        interpriter.load_contract_code(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            mode,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::LOG
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
        interpriter.gas_charge(interpriter.gas_costs().log())?;
        let (a, b, c, d) = self.unpack();
        interpriter.log(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::LOGD
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
        let (a, b, c, d) = self.unpack();
        interpriter.dependent_gas_charge(
            interpriter.gas_costs().logd(),
            interpriter.registers[d],
        )?;
        interpriter.log_data(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::MINT
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
        interpriter.gas_charge(interpriter.gas_costs().mint())?;
        let (a, b) = self.unpack();
        interpriter.mint(interpriter.registers[a], interpriter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SCWQ
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
        let (a, b, c) = self.unpack();
        interpriter.dependent_gas_charge(
            interpriter.gas_costs().scwq(),
            interpriter.registers[c],
        )?;
        interpriter.state_clear_qword(
            interpriter.registers[a],
            b,
            interpriter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SRW
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
        interpriter.gas_charge(interpriter.gas_costs().srw())?;
        let (a, b, c) = self.unpack();
        interpriter.state_read_word(a, b, interpriter.registers[c])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SRWQ
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
        let (a, b, c, d) = self.unpack();
        interpriter.dependent_gas_charge(
            interpriter.gas_costs().srwq(),
            interpriter.registers[d],
        )?;
        interpriter.state_read_qword(
            interpriter.registers[a],
            b,
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SWW
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
        interpriter.gas_charge(interpriter.gas_costs().sww())?;
        let (a, b, c) = self.unpack();
        interpriter.state_write_word(
            interpriter.registers[a],
            b,
            interpriter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::SWWQ
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
        let (a, b, c, d) = self.unpack();
        interpriter.dependent_gas_charge(
            interpriter.gas_costs().swwq(),
            interpriter.registers[d],
        )?;
        interpriter.state_write_qword(
            interpriter.registers[a],
            b,
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::TIME
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
        interpriter.gas_charge(interpriter.gas_costs().time())?;
        let (a, b) = self.unpack();
        interpriter.timestamp(a, interpriter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::ECK1
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
        interpriter.gas_charge(interpriter.gas_costs().eck1())?;
        let (a, b, c) = self.unpack();
        interpriter.secp256k1_recover(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::ECR1
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
        interpriter.gas_charge(interpriter.gas_costs().ecr1())?;
        let (a, b, c) = self.unpack();
        interpriter.secp256r1_recover(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::ED19
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
        let (a, b, c, len) = self.unpack();
        let mut len = interpriter.registers[len];

        // Backwards compatibility with old contracts
        if len == 0 {
            len = 32;
        }

        interpriter.dependent_gas_charge(interpriter.gas_costs().ed19(), len)?;
        interpriter.ed25519_verify(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            len,
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::K256
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
        let (a, b, c) = self.unpack();
        let len = interpriter.registers[c];
        interpriter.dependent_gas_charge(interpriter.gas_costs().k256(), len)?;
        interpriter.keccak256(interpriter.registers[a], interpriter.registers[b], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::S256
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
        let (a, b, c) = self.unpack();
        let len = interpriter.registers[c];
        interpriter.dependent_gas_charge(interpriter.gas_costs().s256(), len)?;
        interpriter.sha256(interpriter.registers[a], interpriter.registers[b], len)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::FLAG
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
        interpriter.gas_charge(interpriter.gas_costs().flag())?;
        let a = self.unpack();
        interpriter.set_flag(interpriter.registers[a])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::GM
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
        interpriter.gas_charge(interpriter.gas_costs().gm())?;
        let (a, imm) = self.unpack();
        interpriter.metadata(a, imm.into())?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::GTF
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
        interpriter.gas_charge(interpriter.gas_costs().gtf())?;
        let (a, b, imm) = self.unpack();
        interpriter.get_transaction_field(a, interpriter.registers[b], imm.into())?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::TR
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
        interpriter.gas_charge(interpriter.gas_costs().tr())?;
        let (a, b, c) = self.unpack();
        interpriter.transfer(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::TRO
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
        interpriter.gas_charge(interpriter.gas_costs().tro())?;
        let (a, b, c, d) = self.unpack();
        interpriter.transfer_output(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::ECAL
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
        let (a, b, c, d) = self.unpack();
        interpriter.external_call(a, b, c, d)?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::BSIZ
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
        // We charge for this inside the function.
        let (a, b) = self.unpack();
        interpriter.blob_size(a, interpriter.registers[b])?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::BLDD
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
        // We charge for this inside the function.
        let (a, b, c, d) = self.unpack();
        interpriter.blob_load_data(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::ECOP
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
        interpriter
            .gas_charge(interpriter.gas_costs().ecop().map_err(PanicReason::from)?)?;
        let (a, b, c, d) = self.unpack();
        interpriter.ec_operation(
            interpriter.registers[a],
            interpriter.registers[b],
            interpriter.registers[c],
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}

impl<M, S, Tx, Ecal> Execute<M, S, Tx, Ecal> for fuel_asm::op::EPAR
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
        let (a, b, c, d) = self.unpack();
        let len = interpriter.registers[c];
        interpriter.dependent_gas_charge(
            interpriter.gas_costs().epar().map_err(PanicReason::from)?,
            len,
        )?;
        interpriter.ec_pairing(
            a,
            interpriter.registers[b],
            len,
            interpriter.registers[d],
        )?;
        Ok(ExecuteState::Proceed)
    }
}
