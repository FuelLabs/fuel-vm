use fuel_types::bytes::{self, SizedBytes};
use fuel_types::{Address, Bytes32, Color, ContractId, Word};

use std::convert::TryFrom;
use std::io::{self, Write};
use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ReceiptRepr {
    Call = 0x00,
    Return = 0x01,
    ReturnData = 0x02,
    Panic = 0x03,
    Revert = 0x04,
    Log = 0x05,
    LogData = 0x06,
    Transfer = 0x07,
    TransferOut = 0x08,
}

impl ReceiptRepr {
    pub const fn len(&self) -> usize {
        ContractId::LEN // id
                + WORD_SIZE // pc
                + WORD_SIZE // is
        + match self {
            Self::Call => {
                ContractId::LEN // to
                + WORD_SIZE // amount
                + Color::LEN // color
                + WORD_SIZE // gas
                + WORD_SIZE // a
                + WORD_SIZE // b
            }

            Self::Return => WORD_SIZE, // val

            Self::ReturnData => {
                WORD_SIZE // ptr
                + WORD_SIZE // len
                + Bytes32::LEN // digest
            }

            Self::Panic => WORD_SIZE, // reason
            Self::Revert => WORD_SIZE, // ra

            Self::Log => {
                WORD_SIZE // ra
                + WORD_SIZE // rb
                + WORD_SIZE // rc
                + WORD_SIZE // rd
            }

            Self::LogData => {
                WORD_SIZE // ra
                + WORD_SIZE // rb
                + WORD_SIZE // ptr
                + WORD_SIZE // len
                + Bytes32::LEN // digest
            }

            Self::Transfer => {
                ContractId::LEN // to
                + WORD_SIZE // amount
                + Color::LEN // digest
            }

            Self::TransferOut => {
                Address::LEN // to
                + WORD_SIZE // amount
                + Color::LEN // digest
            }
        }
    }
}

impl From<&Receipt> for ReceiptRepr {
    fn from(receipt: &Receipt) -> Self {
        match receipt {
            Receipt::Call { .. } => Self::Call,
            Receipt::Return { .. } => Self::Return,
            Receipt::ReturnData { .. } => Self::ReturnData,
            Receipt::Panic { .. } => Self::Panic,
            Receipt::Revert { .. } => Self::Revert,
            Receipt::Log { .. } => Self::Log,
            Receipt::LogData { .. } => Self::LogData,
            Receipt::Transfer { .. } => Self::Transfer,
            Receipt::TransferOut { .. } => Self::TransferOut,
        }
    }
}

impl TryFrom<Word> for ReceiptRepr {
    type Error = io::Error;

    fn try_from(b: Word) -> Result<Self, Self::Error> {
        match b {
            0x00 => Ok(Self::Call),
            0x01 => Ok(Self::Return),
            0x02 => Ok(Self::ReturnData),
            0x03 => Ok(Self::Panic),
            0x04 => Ok(Self::Revert),
            0x05 => Ok(Self::Log),
            0x06 => Ok(Self::LogData),
            0x07 => Ok(Self::Transfer),
            0x08 => Ok(Self::TransferOut),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "The provided identifier is invalid!",
            )),
        }
    }
}

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

    pub const fn id(&self) -> &ContractId {
        match self {
            Self::Call { id, .. } => id,
            Self::Return { id, .. } => id,
            Self::ReturnData { id, .. } => id,
            Self::Panic { id, .. } => id,
            Self::Revert { id, .. } => id,
            Self::Log { id, .. } => id,
            Self::LogData { id, .. } => id,
            Self::Transfer { id, .. } => id,
            Self::TransferOut { id, .. } => id,
        }
    }

    pub const fn pc(&self) -> Word {
        match self {
            Self::Call { pc, .. } => *pc,
            Self::Return { pc, .. } => *pc,
            Self::ReturnData { pc, .. } => *pc,
            Self::Panic { pc, .. } => *pc,
            Self::Revert { pc, .. } => *pc,
            Self::Log { pc, .. } => *pc,
            Self::LogData { pc, .. } => *pc,
            Self::Transfer { pc, .. } => *pc,
            Self::TransferOut { pc, .. } => *pc,
        }
    }

    pub const fn is(&self) -> Word {
        match self {
            Self::Call { is, .. } => *is,
            Self::Return { is, .. } => *is,
            Self::ReturnData { is, .. } => *is,
            Self::Panic { is, .. } => *is,
            Self::Revert { is, .. } => *is,
            Self::Log { is, .. } => *is,
            Self::LogData { is, .. } => *is,
            Self::Transfer { is, .. } => *is,
            Self::TransferOut { is, .. } => *is,
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
