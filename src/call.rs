//! Inter-contract call supporting structures

use crate::consts::*;

use fuel_asm::PanicReason;
use fuel_tx::Contract;
use fuel_types::bytes::{self, SizedBytes};
use fuel_types::{AssetId, ContractId, Word};

use crate::{arith::checked_add_usize, consts::WORD_SIZE};
use std::io::{self, Write};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Call structure representation, composed of a called contract `to` and two
/// word arguments.
///
/// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/vm/opcodes.md#call-call-contract>
pub struct Call {
    to: ContractId,
    a: Word,
    b: Word,
}

impl Call {
    /// Create a new call structure representation.
    pub const fn new(to: ContractId, a: Word, b: Word) -> Self {
        Self { to, a, b }
    }

    /// Called contract.
    pub const fn to(&self) -> &ContractId {
        &self.to
    }

    /// `a` argument.
    pub const fn a(&self) -> Word {
        self.a
    }

    /// `b` argument.
    pub const fn b(&self) -> Word {
        self.b
    }

    /// Expose the internal attributes of the call description.
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
    type Error = PanicReason;

    fn try_from(bytes: &[u8]) -> Result<Self, PanicReason> {
        let mut call = Self::default();

        call.write(bytes).map_err(|_| PanicReason::MalformedCallStructure)?;

        Ok(call)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Call frame representation in the VM stack.
///
/// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/vm/main.md#call-frames>
pub struct CallFrame {
    to: ContractId,
    asset_id: AssetId,
    registers: [Word; VM_REGISTER_COUNT],
    a: Word,
    b: Word,
    code: Contract,
}

impl CallFrame {
    /// Create a new call frame.
    pub const fn new(
        to: ContractId,
        asset_id: AssetId,
        registers: [Word; VM_REGISTER_COUNT],
        a: Word,
        b: Word,
        code: Contract,
    ) -> Self {
        Self {
            to,
            asset_id,
            registers,
            a,
            b,
            code,
        }
    }

    /// Contract code of the called (`to`) id.
    pub fn code(&self) -> &[u8] {
        self.code.as_ref()
    }

    /// Contract code memory offset.
    pub const fn code_offset() -> usize {
        Self::code_size_offset() + WORD_SIZE
    }

    /// Contract code size memory offset.
    pub const fn code_size_offset() -> usize {
        ContractId::LEN + AssetId::LEN + WORD_SIZE * (2 + VM_REGISTER_COUNT)
    }

    /// `a` argument memory offset.
    pub const fn a_offset() -> usize {
        ContractId::LEN + AssetId::LEN + WORD_SIZE * (1 + VM_REGISTER_COUNT)
    }

    /// `b` argument memory offset.
    pub const fn b_offset() -> usize {
        ContractId::LEN + AssetId::LEN + WORD_SIZE * (2 + VM_REGISTER_COUNT)
    }

    /// Registers prior to the called execution.
    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    /// Called contract id.
    pub const fn to(&self) -> &ContractId {
        &self.to
    }

    /// `a` argument.
    pub const fn a(&self) -> Word {
        self.a
    }

    /// `b` argument.
    pub const fn b(&self) -> Word {
        self.b
    }

    /// Gas context prior to the called execution.
    pub const fn context_gas(&self) -> Word {
        self.registers[REG_CGAS]
    }

    /// Asset ID of forwarded coins.
    pub const fn asset_id(&self) -> &AssetId {
        &self.asset_id
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
        let buf = bytes::store_array_unchecked(buf, &self.asset_id);
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
        let (asset_id, buf) = unsafe { bytes::restore_array_unchecked(buf) };

        let buf = self.registers.iter_mut().fold(buf, |buf, reg| {
            let (r, buf) = unsafe { bytes::restore_word_unchecked(buf) };
            *reg = r;
            buf
        });

        let (code_len, buf) = unsafe { bytes::restore_usize_unchecked(buf) };
        let (a, buf) = unsafe { bytes::restore_word_unchecked(buf) };
        let (b, buf) = unsafe { bytes::restore_word_unchecked(buf) };

        let (bytes, code, _) = bytes::restore_raw_bytes(buf, code_len)?;

        n = checked_add_usize(n, bytes)?;

        self.to = to.into();
        self.asset_id = asset_id.into();
        self.a = a;
        self.b = b;
        self.code = code.into();

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
