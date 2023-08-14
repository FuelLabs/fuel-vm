//! Inter-contract call supporting structures

use fuel_asm::{
    PanicReason,
    RegId,
};
use fuel_types::{
    bytes::{
        self,
        SizedBytes,
    },
    canonical::Deserialize,
    mem_layout,
    AssetId,
    ContractId,
    MemLayout,
    MemLocType,
    Word,
};

use crate::consts::{
    WORD_SIZE,
    *,
};

#[cfg(test)]
use fuel_types::canonical::Serialize;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(fuel_types::canonical::Deserialize, fuel_types::canonical::Serialize)]
/// Call structure representation, composed of a called contract `to` and two
/// word arguments.
///
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/fuel-vm/instruction-set.md#call-call-contract>
pub struct Call {
    to: ContractId,
    a: Word,
    b: Word,
}

mem_layout!(
    CallLayout for Call
    to: ContractId = {ContractId::LEN},
    a: Word = WORD_SIZE,
    b: Word = WORD_SIZE
);

impl Call {
    /// The size of the call structures in memory representation.
    pub const LEN: usize = <Call as MemLayout>::LEN;

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

impl From<Call> for [u8; Call::LEN] {
    fn from(val: Call) -> [u8; Call::LEN] {
        let mut buf = [0u8; Call::LEN];
        bytes::store_at(&mut buf, Call::layout(Call::LAYOUT.to), &val.to);
        bytes::store_number_at(&mut buf, Call::layout(Call::LAYOUT.a), val.a);
        bytes::store_number_at(&mut buf, Call::layout(Call::LAYOUT.b), val.b);
        buf
    }
}

impl From<[u8; Self::LEN]> for Call {
    fn from(buf: [u8; Self::LEN]) -> Self {
        let to = bytes::restore_at(&buf, Self::layout(Self::LAYOUT.to));
        let a = bytes::restore_number_at(&buf, Self::layout(Self::LAYOUT.a));
        let b = bytes::restore_number_at(&buf, Self::layout(Self::LAYOUT.b));

        Self {
            to: to.into(),
            a,
            b,
        }
    }
}

impl SizedBytes for Call {
    fn serialized_size(&self) -> usize {
        Self::LEN
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    Hash,
    fuel_types::canonical::Deserialize,
    fuel_types::canonical::Serialize,
)]
/// Call frame representation in the VM stack.
///
/// <https://github.com/FuelLabs/fuel-specs/blob/master/src/fuel-vm/index.md#call-frames>
pub struct CallFrame {
    to: ContractId,
    asset_id: AssetId,
    registers: [Word; VM_REGISTER_COUNT],
    code_size: Word,
    a: Word,
    b: Word,
}

mem_layout!(
    CallFrameLayout for CallFrame
    to: ContractId = {ContractId::LEN},
    asset_id: AssetId = {AssetId::LEN},
    registers: [u8; WORD_SIZE * VM_REGISTER_COUNT] = {WORD_SIZE * VM_REGISTER_COUNT},
    code_size: Word = WORD_SIZE,
    a: Word = WORD_SIZE,
    b: Word = WORD_SIZE
);

#[cfg(test)]
impl Default for CallFrame {
    fn default() -> Self {
        Self {
            to: ContractId::default(),
            asset_id: AssetId::default(),
            registers: [0; VM_REGISTER_COUNT],
            code_size: 0,
            a: 0,
            b: 0,
        }
    }
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
        Self::contract_id_offset() + ContractId::LEN
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

    /// Padding to the next word boundary.
    pub fn code_size_padding(&self) -> Word {
        const WORD_SIZE: Word = crate::consts::WORD_SIZE as Word;
        (WORD_SIZE - self.code_size() % WORD_SIZE) % WORD_SIZE
    }

    /// Total code size including padding.
    pub fn total_code_size(&self) -> Word {
        self.code_size() + self.code_size_padding()
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
    pub fn context_gas(&self) -> Word {
        self.registers[RegId::CGAS]
    }

    /// Asset ID of forwarded coins.
    pub const fn asset_id(&self) -> &AssetId {
        &self.asset_id
    }

    /// Returns the mutable value of the context gas for this call frame.
    pub fn context_gas_mut(&mut self) -> &mut Word {
        &mut self.registers[RegId::CGAS]
    }

    /// Returns the mutable value of the global gas for this call frame.
    pub fn global_gas_mut(&mut self) -> &mut Word {
        &mut self.registers[RegId::GGAS]
    }
}

impl SizedBytes for CallFrame {
    fn serialized_size(&self) -> usize {
        Self::serialized_size()
    }
}

impl TryFrom<&[u8]> for Call {
    type Error = PanicReason;

    fn try_from(mut value: &[u8]) -> Result<Self, Self::Error> {
        Self::decode(&mut value).map_err(|_| PanicReason::MalformedCallStructure)
    }
}

#[cfg(test)]
impl From<Call> for Vec<u8> {
    fn from(call: Call) -> Self {
        call.to_bytes()
    }
}

#[cfg(test)]
impl From<CallFrame> for Vec<u8> {
    fn from(call: CallFrame) -> Self {
        call.to_bytes()
    }
}
