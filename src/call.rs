use crate::consts::*;
use crate::contract::Contract;

use fuel_data::bytes::{self, SizedBytes};
use fuel_data::{Color, ContractId, Word};

use std::convert::TryFrom;
use std::io::{self, Write};
use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde-types", derive(serde::Serialize, serde::Deserialize))]
pub struct Call {
    to: ContractId,
    a: Word,
    b: Word,
}

impl Call {
    pub const fn new(to: ContractId, a: Word, b: Word) -> Self {
        Self { to, a, b }
    }

    pub const fn to(&self) -> &ContractId {
        &self.to
    }

    pub const fn a(&self) -> Word {
        self.a
    }

    pub const fn b(&self) -> Word {
        self.b
    }

    pub const fn into_inner(self) -> (ContractId, Word, Word) {
        (self.to, self.a, self.b)
    }
}

impl SizedBytes for Call {
    fn serialized_size(&self) -> usize {
        ContractId::LEN + 2 * WORD_SIZE
    }
}

impl io::Read for Call {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        let buf = bytes::store_array_unchecked(buf, &self.to);
        let buf = bytes::store_number_unchecked(buf, self.a);
        bytes::store_number_unchecked(buf, self.b);

        Ok(n)
    }
}

impl io::Write for Call {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        // Safety: checked buffer lenght
        let (to, buf) = unsafe { bytes::restore_array_unchecked(buf) };
        let (a, buf) = unsafe { bytes::restore_word_unchecked(buf) };
        let (b, _) = unsafe { bytes::restore_word_unchecked(buf) };

        self.to = to.into();
        self.a = a;
        self.b = b;

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl TryFrom<&[u8]> for Call {
    type Error = io::Error;

    fn try_from(bytes: &[u8]) -> io::Result<Self> {
        let mut call = Self::default();

        call.write(bytes)?;

        Ok(call)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallFrame {
    to: ContractId,
    color: Color,
    registers: [Word; VM_REGISTER_COUNT],
    a: Word,
    b: Word,
    code: Contract,
}

impl CallFrame {
    pub const fn new(
        to: ContractId,
        color: Color,
        registers: [Word; VM_REGISTER_COUNT],
        a: Word,
        b: Word,
        code: Contract,
    ) -> Self {
        Self {
            to,
            color,
            registers,
            a,
            b,
            code,
        }
    }

    pub fn code(&self) -> &[u8] {
        self.code.as_ref()
    }

    pub const fn code_offset() -> usize {
        ContractId::LEN + Color::LEN + WORD_SIZE * (3 + VM_REGISTER_COUNT)
    }

    pub const fn a_offset() -> usize {
        ContractId::LEN + Color::LEN + WORD_SIZE * (1 + VM_REGISTER_COUNT)
    }

    pub const fn b_offset() -> usize {
        ContractId::LEN + Color::LEN + WORD_SIZE * (2 + VM_REGISTER_COUNT)
    }

    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    pub const fn to(&self) -> &ContractId {
        &self.to
    }

    pub const fn a(&self) -> Word {
        self.a
    }

    pub const fn b(&self) -> Word {
        self.b
    }

    pub const fn context_gas(&self) -> Word {
        self.registers[REG_CGAS]
    }

    pub const fn color(&self) -> &Color {
        &self.color
    }
}

impl SizedBytes for CallFrame {
    fn serialized_size(&self) -> usize {
        Self::code_offset() + bytes::padded_len(self.code.as_ref())
    }
}

impl io::Read for CallFrame {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.serialized_size();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        let buf = bytes::store_array_unchecked(buf, &self.to);
        let buf = bytes::store_array_unchecked(buf, &self.color);
        let buf = self
            .registers
            .iter()
            .fold(buf, |buf, reg| bytes::store_number_unchecked(buf, *reg));

        let buf = bytes::store_number_unchecked(buf, self.code.as_ref().len() as Word);
        let buf = bytes::store_number_unchecked(buf, self.a);
        let buf = bytes::store_number_unchecked(buf, self.b);

        bytes::store_raw_bytes(buf, self.code.as_ref())?;

        Ok(n)
    }
}

impl io::Write for CallFrame {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut n = Self::code_offset();
        if buf.len() < n {
            return Err(bytes::eof());
        }

        // Safety: checked buffer length
        let (to, buf) = unsafe { bytes::restore_array_unchecked(buf) };
        let (color, buf) = unsafe { bytes::restore_array_unchecked(buf) };

        let buf = self.registers.iter_mut().fold(buf, |buf, reg| {
            let (r, buf) = unsafe { bytes::restore_word_unchecked(buf) };
            *reg = r;
            buf
        });

        let (code_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
        let (a, buf) = unsafe { bytes::restore_word_unchecked(buf) };
        let (b, buf) = unsafe { bytes::restore_word_unchecked(buf) };

        let (bytes, code, _) = bytes::restore_raw_bytes(buf, code_len)?;
        n += bytes;

        self.to = to.into();
        self.color = color.into();
        self.a = a;
        self.b = b;
        self.code = code.into();

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
