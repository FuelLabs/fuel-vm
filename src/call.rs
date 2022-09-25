//! Inter-contract call supporting structures

use crate::consts::*;

use fuel_asm::PanicReason;
use fuel_tx::io::Deserialize;
use fuel_tx::Contract;
use fuel_types::bytes::{self, SizedBytes};
use fuel_types::{AssetId, ContractId, Word};

use std::mem;

const WORD_SIZE: usize = mem::size_of::<Word>();

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_tx::io::Deserialize, fuel_tx::io::Serialize)]
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

impl TryFrom<&[u8]> for Call {
    type Error = PanicReason;

    fn try_from(mut bytes: &[u8]) -> Result<Self, PanicReason> {
        Ok(Self::decode(&mut bytes).map_err(|_| PanicReason::MalformedCallStructure)?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, fuel_tx::io::Deserialize, fuel_tx::io::Serialize)]
/// Call frame representation in the VM stack.
///
/// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/vm/main.md#call-frames>
pub struct CallFrame {
    to: ContractId,
    asset_id: AssetId,
    registers: [Word; VM_REGISTER_COUNT],
    code: Contract,
    a: Word,
    b: Word,
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
