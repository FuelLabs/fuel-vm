use fuel_asm::InstructionResult;
use fuel_types::bytes::{self, SizedBytes};
use fuel_types::{Address, Bytes32, Color, ContractId, Word};

use std::convert::TryFrom;
use std::io::{self, Write};
use std::mem;

mod receipt_repr;

use receipt_repr::ReceiptRepr;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub enum Receipt {
    Call {
        id: ContractId,
        to: ContractId,
        amount: Word,
        color: Color,
        gas: Word,
        a: Word,
        b: Word,
        pc: Word,
        is: Word,
    },

    Return {
        id: ContractId,
        val: Word,
        pc: Word,
        is: Word,
    },

    ReturnData {
        id: ContractId,
        ptr: Word,
        len: Word,
        digest: Bytes32,
        pc: Word,
        is: Word,
    },

    Panic {
        id: ContractId,
        reason: Word,
        pc: Word,
        is: Word,
    },

    Revert {
        id: ContractId,
        ra: Word,
        pc: Word,
        is: Word,
    },

    Log {
        id: ContractId,
        ra: Word,
        rb: Word,
        rc: Word,
        rd: Word,
        pc: Word,
        is: Word,
    },

    LogData {
        id: ContractId,
        ra: Word,
        rb: Word,
        ptr: Word,
        len: Word,
        digest: Bytes32,
        pc: Word,
        is: Word,
    },

    Transfer {
        id: ContractId,
        to: ContractId,
        amount: Word,
        color: Color,
        pc: Word,
        is: Word,
    },

    TransferOut {
        id: ContractId,
        to: Address,
        amount: Word,
        color: Color,
        pc: Word,
        is: Word,
    },

    ScriptResult {
        result: InstructionResult,
        gas_used: Word,
    },
}

impl Receipt {
    pub const fn call(
        id: ContractId,
        to: ContractId,
        amount: Word,
        color: Color,
        gas: Word,
        a: Word,
        b: Word,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::Call {
            id,
            to,
            amount,
            color,
            gas,
            a,
            b,
            pc,
            is,
        }
    }

    // return keyword is reserved
    pub const fn ret(id: ContractId, val: Word, pc: Word, is: Word) -> Self {
        Self::Return { id, val, pc, is }
    }

    pub const fn return_data(
        id: ContractId,
        ptr: Word,
        len: Word,
        digest: Bytes32,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::ReturnData {
            id,
            ptr,
            len,
            digest,
            pc,
            is,
        }
    }

    pub const fn panic(id: ContractId, reason: Word, pc: Word, is: Word) -> Self {
        Self::Panic { id, reason, pc, is }
    }

    pub const fn revert(id: ContractId, ra: Word, pc: Word, is: Word) -> Self {
        Self::Revert { id, ra, pc, is }
    }

    pub const fn log(
        id: ContractId,
        ra: Word,
        rb: Word,
        rc: Word,
        rd: Word,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::Log {
            id,
            ra,
            rb,
            rc,
            rd,
            pc,
            is,
        }
    }

    pub const fn log_data(
        id: ContractId,
        ra: Word,
        rb: Word,
        ptr: Word,
        len: Word,
        digest: Bytes32,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::LogData {
            id,
            ra,
            rb,
            ptr,
            len,
            digest,
            pc,
            is,
        }
    }

    pub const fn transfer(
        id: ContractId,
        to: ContractId,
        amount: Word,
        color: Color,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::Transfer {
            id,
            to,
            amount,
            color,
            pc,
            is,
        }
    }

    pub const fn transfer_out(
        id: ContractId,
        to: Address,
        amount: Word,
        color: Color,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::TransferOut {
            id,
            to,
            amount,
            color,
            pc,
            is,
        }
    }

    pub const fn script_result(result: InstructionResult, gas_used: Word) -> Self {
        Self::ScriptResult { result, gas_used }
    }

    pub const fn id(&self) -> Option<&ContractId> {
        match self {
            Self::Call { id, .. } => Some(id),
            Self::Return { id, .. } => Some(id),
            Self::ReturnData { id, .. } => Some(id),
            Self::Panic { id, .. } => Some(id),
            Self::Revert { id, .. } => Some(id),
            Self::Log { id, .. } => Some(id),
            Self::LogData { id, .. } => Some(id),
            Self::Transfer { id, .. } => Some(id),
            Self::TransferOut { id, .. } => Some(id),
            Self::ScriptResult { .. } => None,
        }
    }

    pub const fn pc(&self) -> Option<Word> {
        match self {
            Self::Call { pc, .. } => Some(*pc),
            Self::Return { pc, .. } => Some(*pc),
            Self::ReturnData { pc, .. } => Some(*pc),
            Self::Panic { pc, .. } => Some(*pc),
            Self::Revert { pc, .. } => Some(*pc),
            Self::Log { pc, .. } => Some(*pc),
            Self::LogData { pc, .. } => Some(*pc),
            Self::Transfer { pc, .. } => Some(*pc),
            Self::TransferOut { pc, .. } => Some(*pc),
            Self::ScriptResult { .. } => None,
        }
    }

    pub const fn is(&self) -> Option<Word> {
        match self {
            Self::Call { is, .. } => Some(*is),
            Self::Return { is, .. } => Some(*is),
            Self::ReturnData { is, .. } => Some(*is),
            Self::Panic { is, .. } => Some(*is),
            Self::Revert { is, .. } => Some(*is),
            Self::Log { is, .. } => Some(*is),
            Self::LogData { is, .. } => Some(*is),
            Self::Transfer { is, .. } => Some(*is),
            Self::TransferOut { is, .. } => Some(*is),
            Self::ScriptResult { .. } => None,
        }
    }

    pub const fn to(&self) -> Option<&ContractId> {
        match self {
            Self::Call { to, .. } => Some(to),
            Self::Transfer { to, .. } => Some(to),
            _ => None,
        }
    }

    pub const fn to_address(&self) -> Option<&Address> {
        match self {
            Self::TransferOut { to, .. } => Some(to),
            _ => None,
        }
    }

    pub const fn amount(&self) -> Option<Word> {
        match self {
            Self::Call { amount, .. } => Some(*amount),
            Self::Transfer { amount, .. } => Some(*amount),
            Self::TransferOut { amount, .. } => Some(*amount),
            _ => None,
        }
    }

    pub const fn color(&self) -> Option<&Color> {
        match self {
            Self::Call { color, .. } => Some(color),
            Self::Transfer { color, .. } => Some(color),
            Self::TransferOut { color, .. } => Some(color),
            _ => None,
        }
    }

    pub const fn gas(&self) -> Option<Word> {
        match self {
            Self::Call { gas, .. } => Some(*gas),
            _ => None,
        }
    }

    pub const fn a(&self) -> Option<Word> {
        match self {
            Self::Call { a, .. } => Some(*a),
            _ => None,
        }
    }

    pub const fn b(&self) -> Option<Word> {
        match self {
            Self::Call { b, .. } => Some(*b),
            _ => None,
        }
    }

    pub const fn val(&self) -> Option<Word> {
        match self {
            Self::Return { val, .. } => Some(*val),
            _ => None,
        }
    }

    pub const fn ptr(&self) -> Option<Word> {
        match self {
            Self::ReturnData { ptr, .. } => Some(*ptr),
            Self::LogData { ptr, .. } => Some(*ptr),
            _ => None,
        }
    }

    pub const fn len(&self) -> Option<Word> {
        match self {
            Self::ReturnData { len, .. } => Some(*len),
            Self::LogData { len, .. } => Some(*len),
            _ => None,
        }
    }

    pub const fn digest(&self) -> Option<&Bytes32> {
        match self {
            Self::ReturnData { digest, .. } => Some(digest),
            Self::LogData { digest, .. } => Some(digest),
            _ => None,
        }
    }

    pub const fn reason(&self) -> Option<Word> {
        match self {
            Self::Panic { reason, .. } => Some(*reason),
            _ => None,
        }
    }

    pub const fn ra(&self) -> Option<Word> {
        match self {
            Self::Revert { ra, .. } => Some(*ra),
            Self::Log { ra, .. } => Some(*ra),
            Self::LogData { ra, .. } => Some(*ra),
            _ => None,
        }
    }

    pub const fn rb(&self) -> Option<Word> {
        match self {
            Self::Log { rb, .. } => Some(*rb),
            Self::LogData { rb, .. } => Some(*rb),
            _ => None,
        }
    }

    pub const fn rc(&self) -> Option<Word> {
        match self {
            Self::Log { rc, .. } => Some(*rc),
            _ => None,
        }
    }

    pub const fn rd(&self) -> Option<Word> {
        match self {
            Self::Log { rd, .. } => Some(*rd),
            _ => None,
        }
    }

    pub const fn result(&self) -> Option<&InstructionResult> {
        match self {
            Self::ScriptResult { result, .. } => Some(result),
            _ => None,
        }
    }

    pub const fn gas_used(&self) -> Option<Word> {
        match self {
            Self::ScriptResult { gas_used, .. } => Some(*gas_used),
            _ => None,
        }
    }
}

impl io::Read for Receipt {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.serialized_size();

        if buf.len() < len {
            return Err(bytes::eof());
        }

        match self {
            Self::Call {
                id,
                to,
                amount,
                color,
                gas,
                a,
                b,
                pc,
                is,
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Call as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_array_unchecked(buf, to);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, color);
                let buf = bytes::store_number_unchecked(buf, *gas);
                let buf = bytes::store_number_unchecked(buf, *a);
                let buf = bytes::store_number_unchecked(buf, *b);
                let buf = bytes::store_number_unchecked(buf, *pc);
                bytes::store_number_unchecked(buf, *is);
            }

            Self::Return { id, val, pc, is } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Return as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *val);
                let buf = bytes::store_number_unchecked(buf, *pc);
                bytes::store_number_unchecked(buf, *is);
            }

            Self::ReturnData {
                id,
                ptr,
                len,
                digest,
                pc,
                is,
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::ReturnData as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *ptr);
                let buf = bytes::store_number_unchecked(buf, *len);
                let buf = bytes::store_array_unchecked(buf, digest);
                let buf = bytes::store_number_unchecked(buf, *pc);
                bytes::store_number_unchecked(buf, *is);
            }

            Self::Panic { id, reason, pc, is } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Panic as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *reason);
                let buf = bytes::store_number_unchecked(buf, *pc);
                bytes::store_number_unchecked(buf, *is);
            }

            Self::Revert { id, ra, pc, is } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Revert as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *ra);
                let buf = bytes::store_number_unchecked(buf, *pc);
                bytes::store_number_unchecked(buf, *is);
            }

            Self::Log {
                id,
                ra,
                rb,
                rc,
                rd,
                pc,
                is,
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Log as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *ra);
                let buf = bytes::store_number_unchecked(buf, *rb);
                let buf = bytes::store_number_unchecked(buf, *rc);
                let buf = bytes::store_number_unchecked(buf, *rd);
                let buf = bytes::store_number_unchecked(buf, *pc);
                bytes::store_number_unchecked(buf, *is);
            }

            Self::LogData {
                id,
                ra,
                rb,
                ptr,
                len,
                digest,
                pc,
                is,
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::LogData as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_number_unchecked(buf, *ra);
                let buf = bytes::store_number_unchecked(buf, *rb);
                let buf = bytes::store_number_unchecked(buf, *ptr);
                let buf = bytes::store_number_unchecked(buf, *len);
                let buf = bytes::store_array_unchecked(buf, digest);
                let buf = bytes::store_number_unchecked(buf, *pc);
                bytes::store_number_unchecked(buf, *is);
            }

            Self::Transfer {
                id,
                to,
                amount,
                color,
                pc,
                is,
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::Transfer as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_array_unchecked(buf, to);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, color);
                let buf = bytes::store_number_unchecked(buf, *pc);
                bytes::store_number_unchecked(buf, *is);
            }

            Self::TransferOut {
                id,
                to,
                amount,
                color,
                pc,
                is,
            } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::TransferOut as Word);

                let buf = bytes::store_array_unchecked(buf, id);
                let buf = bytes::store_array_unchecked(buf, to);
                let buf = bytes::store_number_unchecked(buf, *amount);
                let buf = bytes::store_array_unchecked(buf, color);
                let buf = bytes::store_number_unchecked(buf, *pc);
                bytes::store_number_unchecked(buf, *is);
            }

            Self::ScriptResult { result, gas_used } => {
                let buf = bytes::store_number_unchecked(buf, ReceiptRepr::ScriptResult as Word);

                let result = Word::from(*result);
                let buf = bytes::store_number_unchecked(buf, result);

                bytes::store_number_unchecked(buf, *gas_used);
            }
        }

        Ok(len)
    }
}

impl io::Write for Receipt {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.len() < WORD_SIZE {
            return Err(bytes::eof());
        }

        // Safety: buffer size is checked
        let (identifier, buf) = unsafe { bytes::restore_word_unchecked(buf) };
        let identifier = ReceiptRepr::try_from(identifier)?;
        let len = identifier.len();

        if buf.len() < len {
            return Err(bytes::eof());
        }

        // Safety: buf len is checked
        match identifier {
            ReceiptRepr::Call => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (to, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (color, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (gas, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (a, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (b, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let to = to.into();
                let color = color.into();

                *self = Self::call(id, to, amount, color, gas, a, b, pc, is);
            }

            ReceiptRepr::Return => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (val, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();

                *self = Self::ret(id, val, pc, is);
            }

            ReceiptRepr::ReturnData => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (ptr, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (len, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (digest, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let digest = digest.into();

                *self = Self::return_data(id, ptr, len, digest, pc, is);
            }

            ReceiptRepr::Panic => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (reason, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();

                *self = Self::panic(id, reason, pc, is);
            }

            ReceiptRepr::Revert => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (ra, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();

                *self = Self::revert(id, ra, pc, is);
            }

            ReceiptRepr::Log => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (ra, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (rb, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (rc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (rd, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();

                *self = Self::log(id, ra, rb, rc, rd, pc, is);
            }

            ReceiptRepr::LogData => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (ra, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (rb, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (ptr, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (len, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (digest, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let digest = digest.into();

                *self = Self::log_data(id, ra, rb, ptr, len, digest, pc, is);
            }

            ReceiptRepr::Transfer => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (to, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (color, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let to = to.into();
                let color = color.into();

                *self = Self::transfer(id, to, amount, color, pc, is);
            }

            ReceiptRepr::TransferOut => {
                let (id, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (to, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (amount, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (color, buf) = unsafe { bytes::restore_array_unchecked(buf) };
                let (pc, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (is, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let id = id.into();
                let to = to.into();
                let color = color.into();

                *self = Self::transfer_out(id, to, amount, color, pc, is);
            }

            ReceiptRepr::ScriptResult => {
                let (result, buf) = unsafe { bytes::restore_word_unchecked(buf) };
                let (gas_used, _) = unsafe { bytes::restore_word_unchecked(buf) };

                let result = InstructionResult::from(result);

                *self = Self::script_result(result, gas_used);
            }
        }

        Ok(len + WORD_SIZE)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl SizedBytes for Receipt {
    fn serialized_size(&self) -> usize {
        ReceiptRepr::from(self).len() + WORD_SIZE
    }
}

impl bytes::Deserializable for Receipt {
    fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        let mut instance = Self::ret(Default::default(), 0, 0, 0);

        instance.write(bytes)?;

        Ok(instance)
    }
}
