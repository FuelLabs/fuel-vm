use alloc::vec::Vec;
use derivative::Derivative;
use fuel_asm::InstructionResult;
use fuel_types::bytes::{self, padded_len_usize, SizedBytes, WORD_SIZE};
use fuel_types::{Address, AssetId, Bytes32, ContractId, MessageId, Word};

#[cfg(feature = "std")]
mod receipt_std;

mod receipt_repr;
mod script_result;

use receipt_repr::ReceiptRepr;

pub use script_result::ScriptExecutionResult;

use crate::Output;
use crate::Receipt::Panic;

#[derive(Debug, Clone, Derivative)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derivative(Eq, PartialEq, Hash)]
pub enum Receipt {
    Call {
        id: ContractId,
        to: ContractId,
        amount: Word,
        asset_id: AssetId,
        gas: Word,
        param1: Word,
        param2: Word,
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
        #[derivative(PartialEq = "ignore", Hash = "ignore")]
        contract_id: Option<ContractId>,
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

    MessageOut {
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Bytes32,
        len: Word,
        digest: Bytes32,
        data: Vec<u8>,
    },
}

impl Receipt {
    pub const fn call(
        id: ContractId,
        to: ContractId,
        amount: Word,
        asset_id: AssetId,
        gas: Word,
        param1: Word,
        param2: Word,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::Call {
            id,
            to,
            amount,
            asset_id,
            gas,
            param1,
            param2,
            pc,
            is,
        }
    }

    // return keyword is reserved
    pub const fn ret(id: ContractId, val: Word, pc: Word, is: Word) -> Self {
        Self::Return { id, val, pc, is }
    }

    pub fn return_data(
        id: ContractId,
        ptr: Word,
        digest: Bytes32,
        data: Vec<u8>,
        pc: Word,
        is: Word,
    ) -> Self {
        let len = bytes::padded_len(&data) as Word;

        Self::return_data_with_len(id, ptr, len, digest, data, pc, is)
    }

    pub const fn return_data_with_len(
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
        Panic {
            id,
            reason,
            pc,
            is,
            contract_id: None,
        }
    }

    pub fn with_panic_contract_id(mut self, _contract_id: Option<ContractId>) -> Self {
        if let Receipt::Panic {
            ref mut contract_id,
            ..
        } = self
        {
            *contract_id = _contract_id;
        }
        self
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

    pub fn log_data(
        id: ContractId,
        ra: Word,
        rb: Word,
        ptr: Word,
        digest: Bytes32,
        data: Vec<u8>,
        pc: Word,
        is: Word,
    ) -> Self {
        let len = bytes::padded_len(&data) as Word;

        Self::log_data_with_len(id, ra, rb, ptr, len, digest, data, pc, is)
    }

    pub const fn log_data_with_len(
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

    pub fn message_out_from_tx_output(
        txid: &Bytes32,
        idx: Word,
        sender: Address,
        recipient: Address,
        amount: Word,
        data: Vec<u8>,
    ) -> Self {
        let nonce = Output::message_nonce(txid, idx);
        let message_id = Output::message_id(&sender, &recipient, &nonce, amount, &data);
        let digest = Output::message_digest(&data);

        Self::message_out(message_id, sender, recipient, amount, nonce, digest, data)
    }

    pub fn message_out(
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Bytes32,
        digest: Bytes32,
        data: Vec<u8>,
    ) -> Self {
        let len = bytes::padded_len(&data) as Word;

        Self::message_out_with_len(
            message_id, sender, recipient, amount, nonce, len, digest, data,
        )
    }

    pub const fn message_out_with_len(
        message_id: MessageId,
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Bytes32,
        len: Word,
        digest: Bytes32,
        data: Vec<u8>,
    ) -> Self {
        Self::MessageOut {
            message_id,
            sender,
            recipient,
            amount,
            nonce,
            len,
            digest,
            data,
        }
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
            Self::MessageOut { .. } => None,
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
            Self::MessageOut { .. } => None,
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
            Self::MessageOut { .. } => None,
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
            Self::MessageOut { amount, .. } => Some(*amount),
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

    pub const fn param1(&self) -> Option<Word> {
        match self {
            Self::Call { param1, .. } => Some(*param1),
            _ => None,
        }
    }

    pub const fn param2(&self) -> Option<Word> {
        match self {
            Self::Call { param2, .. } => Some(*param2),
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
            Self::MessageOut { len, .. } => Some(*len),
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
            Self::MessageOut { digest, .. } => Some(digest),
            _ => None,
        }
    }

    pub fn data(&self) -> Option<&[u8]> {
        match self {
            Self::ReturnData { data, .. } => Some(data),
            Self::LogData { data, .. } => Some(data),
            Self::MessageOut { data, .. } => Some(data),
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

    pub const fn message_id(&self) -> Option<&MessageId> {
        match self {
            Self::MessageOut { message_id, .. } => Some(message_id),
            _ => None,
        }
    }

    pub const fn sender(&self) -> Option<&Address> {
        match self {
            Self::MessageOut { sender, .. } => Some(sender),
            _ => None,
        }
    }

    pub const fn recipient(&self) -> Option<&Address> {
        match self {
            Self::MessageOut { recipient, .. } => Some(recipient),
            _ => None,
        }
    }

    pub const fn nonce(&self) -> Option<&Bytes32> {
        match self {
            Self::MessageOut { nonce, .. } => Some(nonce),
            _ => None,
        }
    }

    pub const fn contract_id(&self) -> Option<&ContractId> {
        match self {
            Self::Panic { contract_id, .. } => contract_id.as_ref(),
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
                    + WORD_SIZE // param1
                    + WORD_SIZE // param2
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

            ReceiptRepr::MessageOut => {
                MessageId::LEN // message_id
                    + Address::LEN // sender
                    + Address::LEN // recipient
                    + WORD_SIZE // amount
                    + Bytes32::LEN // nonce
                    + WORD_SIZE // len
                    + Bytes32::LEN // digest
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
