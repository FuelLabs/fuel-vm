//! Inter-contract call supporting structures

use crate::consts::*;

use fuel_asm::PanicReason;
use fuel_types::bytes::{self, SizedBytes};
use fuel_types::{AssetId, ContractId, Word};

use crate::consts::WORD_SIZE;
use std::io::{self, Write};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Call structure representation, composed of a called contract `to` and two
/// word arguments.
///
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/vm/instruction_set.md#call-call-contract>
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
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/vm/index.md#call-frames>
pub struct CallFrame {
    to: ContractId,
    asset_id: AssetId,
    registers: [Word; VM_REGISTER_COUNT],
    code_size: Word,
    a: Word,
    b: Word,
}

impl CallFrame {
    /// Create a new call frame.
    pub const fn new(
        to: ContractId,
        asset_id: AssetId,
        registers: [Word; VM_REGISTER_COUNT],
        code_size: Word,
        a: Word,
        b: Word,
    ) -> Self {
        Self {
            to,
            asset_id,
            registers,
            code_size,
            a,
            b,
        }
    }

    /// Start of the contract id offset from the beginning of the call frame.
    pub const fn contract_id_offset() -> usize {
        0
    }

    /// Start of the asset id offset from the beginning of the call frame.
    pub const fn asset_id_offset() -> usize {
        ContractId::LEN
    }

    /// Start of the registers offset from the beginning of the call frame.
    pub const fn registers_offset() -> usize {
        Self::asset_id_offset() + AssetId::LEN
    }

    /// Start of the code size offset from the beginning of the call frame.
    pub const fn code_size_offset() -> usize {
        Self::registers_offset() + WORD_SIZE * (VM_REGISTER_COUNT)
    }

    /// Start of the `a` argument offset from the beginning of the call frame.
    pub const fn a_offset() -> usize {
        Self::code_size_offset() + WORD_SIZE
    }

    /// Start of the `b` argument offset from the beginning of the call frame.
    pub const fn b_offset() -> usize {
        Self::a_offset() + WORD_SIZE
    }

    /// Size of the call frame in bytes.
    pub const fn serialized_size() -> usize {
        Self::b_offset() + WORD_SIZE
    }

    /// Registers prior to the called execution.
    pub const fn registers(&self) -> &[Word] {
        &self.registers
    }

    /// Called contract id.
    pub const fn to(&self) -> &ContractId {
        &self.to
    }

    /// Contract code length in bytes.
    pub fn code_size(&self) -> Word {
        self.code_size
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

    /// Set the value of the context gas for this call frame.
    pub fn set_context_gas(&mut self) -> &mut Word {
        &mut self.registers[REG_CGAS]
    }

    /// Set the value of the global gas for this call frame.
    pub fn set_global_gas(&mut self) -> &mut Word {
        &mut self.registers[REG_GGAS]
    }
}

impl SizedBytes for CallFrame {
    fn serialized_size(&self) -> usize {
        Self::serialized_size()
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

        let buf = bytes::store_number_unchecked(buf, self.code_size);
        let buf = bytes::store_number_unchecked(buf, self.a);
        bytes::store_number_unchecked(buf, self.b);

        Ok(n)
    }
}

impl io::Write for CallFrame {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = Self::serialized_size();
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

        let (code_size, buf) = unsafe { bytes::restore_word_unchecked(buf) };
        let (a, buf) = unsafe { bytes::restore_word_unchecked(buf) };
        let (b, _) = unsafe { bytes::restore_word_unchecked(buf) };

        self.to = to.into();
        self.asset_id = asset_id.into();
        self.code_size = code_size;
        self.a = a;
        self.b = b;

        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
