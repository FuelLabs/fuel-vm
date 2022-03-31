use fuel_asm::InstructionResult;
use fuel_types::bytes::{padded_len_usize, SizedBytes, WORD_SIZE};
use fuel_types::{Address, AssetId, Bytes32, ContractId, Word};

use alloc::vec::Vec;

#[cfg(feature = "std")]
mod receipt_std;

mod receipt_repr;
mod script_result;

use receipt_repr::ReceiptRepr;

pub use script_result::ScriptExecutionResult;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde-types-minimal",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum Receipt {
    Call {
        id: ContractId,
        to: ContractId,
        amount: Word,
        asset_id: AssetId,
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
        data: Vec<u8>,
        pc: Word,
        is: Word,
    },

    Panic {
        id: ContractId,
        reason: InstructionResult,
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
        data: Vec<u8>,
        pc: Word,
        is: Word,
    },

    Transfer {
        id: ContractId,
        to: ContractId,
        amount: Word,
        asset_id: AssetId,
        pc: Word,
        is: Word,
    },

    TransferOut {
        id: ContractId,
        to: Address,
        amount: Word,
        asset_id: AssetId,
        pc: Word,
        is: Word,
    },

    ScriptResult {
        result: ScriptExecutionResult,
        gas_used: Word,
    },
}

impl Receipt {
    pub const fn call(
        id: ContractId,
        to: ContractId,
        amount: Word,
        asset_id: AssetId,
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
            asset_id,
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
        data: Vec<u8>,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::ReturnData {
            id,
            ptr,
            len,
            digest,
            data,
            pc,
            is,
        }
    }

    pub const fn panic(id: ContractId, reason: InstructionResult, pc: Word, is: Word) -> Self {
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
        data: Vec<u8>,
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
            data,
            pc,
            is,
        }
    }

    pub const fn transfer(
        id: ContractId,
        to: ContractId,
        amount: Word,
        asset_id: AssetId,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::Transfer {
            id,
            to,
            amount,
            asset_id,
            pc,
            is,
        }
    }

    pub const fn transfer_out(
        id: ContractId,
        to: Address,
        amount: Word,
        asset_id: AssetId,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::TransferOut {
            id,
            to,
            amount,
            asset_id,
            pc,
            is,
        }
    }

    pub const fn script_result(result: ScriptExecutionResult, gas_used: Word) -> Self {
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

    pub const fn asset_id(&self) -> Option<&AssetId> {
        match self {
            Self::Call { asset_id, .. } => Some(asset_id),
            Self::Transfer { asset_id, .. } => Some(asset_id),
            Self::TransferOut { asset_id, .. } => Some(asset_id),
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

    pub const fn is_empty(&self) -> Option<bool> {
        match self.len() {
            Some(0) => Some(true),
            Some(_) => Some(false),
            None => None,
        }
    }

    pub const fn digest(&self) -> Option<&Bytes32> {
        match self {
            Self::ReturnData { digest, .. } => Some(digest),
            Self::LogData { digest, .. } => Some(digest),
            _ => None,
        }
    }

    pub fn data(&self) -> Option<&[u8]> {
        match self {
            Self::ReturnData { data, .. } => Some(data),
            Self::LogData { data, .. } => Some(data),
            _ => None,
        }
    }

    pub const fn reason(&self) -> Option<InstructionResult> {
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

    pub const fn result(&self) -> Option<&ScriptExecutionResult> {
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

    fn variant_len_without_data(variant: ReceiptRepr) -> usize {
        ContractId::LEN // id
                + WORD_SIZE // pc
                + WORD_SIZE // is
        + match variant {
            ReceiptRepr::Call => {
                ContractId::LEN // to
                + WORD_SIZE // amount
                + AssetId::LEN // asset_id
                + WORD_SIZE // gas
                + WORD_SIZE // a
                + WORD_SIZE // b
            }

            ReceiptRepr::Return => WORD_SIZE, // val

            ReceiptRepr::ReturnData => {
                WORD_SIZE // ptr
                + WORD_SIZE // len
                + Bytes32::LEN // digest
            }

            ReceiptRepr::Panic => WORD_SIZE, // reason
            ReceiptRepr::Revert => WORD_SIZE, // ra

            ReceiptRepr::Log => {
                WORD_SIZE // ra
                + WORD_SIZE // rb
                + WORD_SIZE // rc
                + WORD_SIZE // rd
            }

            ReceiptRepr::LogData => {
                WORD_SIZE // ra
                + WORD_SIZE // rb
                + WORD_SIZE // ptr
                + WORD_SIZE // len
                + Bytes32::LEN // digest
            }

            ReceiptRepr::Transfer => {
                ContractId::LEN // to
                + WORD_SIZE // amount
                + AssetId::LEN // digest
            }

            ReceiptRepr::TransferOut => {
                Address::LEN // to
                + WORD_SIZE // amount
                + AssetId::LEN // digest
            }

            ReceiptRepr::ScriptResult => {
                WORD_SIZE // status
                + WORD_SIZE // gas_used
            }
        }
    }
}

impl SizedBytes for Receipt {
    fn serialized_size(&self) -> usize {
        let data_len = self
            .data()
            .map(|data| WORD_SIZE + padded_len_usize(data.len()))
            .unwrap_or(0);

        Self::variant_len_without_data(ReceiptRepr::from(self)) + WORD_SIZE + data_len
    }
}
