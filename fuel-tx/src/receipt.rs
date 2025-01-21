use crate::Output;
use alloc::vec::Vec;
use educe::Educe;
use fuel_asm::PanicInstruction;
use fuel_crypto::Hasher;
use fuel_types::{
    canonical::{
        Deserialize,
        Serialize,
    },
    fmt_option_truncated_hex,
    Address,
    AssetId,
    Bytes32,
    ContractId,
    MessageId,
    Nonce,
    Word,
};

mod receipt_repr;
mod script_result;

use crate::input::message::compute_message_id;
pub use script_result::ScriptExecutionResult;

#[derive(Clone, Educe, serde::Serialize, serde::Deserialize, Deserialize, Serialize)]
#[educe(Eq, PartialEq, Hash, Debug)]
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
        pc: Word,
        is: Word,
        #[educe(Debug(method("fmt_option_truncated_hex::<16>")))]
        #[educe(PartialEq(ignore))]
        #[educe(Hash(ignore))]
        #[canonical(skip)]
        data: Option<Vec<u8>>,
    },

    Panic {
        id: ContractId,
        reason: PanicInstruction,
        pc: Word,
        is: Word,
        #[educe(PartialEq(ignore))]
        #[educe(Hash(ignore))]
        #[canonical(skip)]
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
        pc: Word,
        is: Word,
        #[educe(Debug(method("fmt_option_truncated_hex::<16>")))]
        #[educe(PartialEq(ignore))]
        #[educe(Hash(ignore))]
        #[canonical(skip)]
        data: Option<Vec<u8>>,
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
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Nonce,
        len: Word,
        digest: Bytes32,
        #[educe(Debug(method("fmt_option_truncated_hex::<16>")))]
        #[educe(PartialEq(ignore))]
        #[educe(Hash(ignore))]
        #[canonical(skip)]
        data: Option<Vec<u8>>,
    },
    Mint {
        sub_id: Bytes32,
        contract_id: ContractId,
        val: Word,
        pc: Word,
        is: Word,
    },
    Burn {
        sub_id: Bytes32,
        contract_id: ContractId,
        val: Word,
        pc: Word,
        is: Word,
    },
}

impl core::fmt::Display for Receipt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Receipt::Call { id, .. } => write!(f, "Call({})", id),
            Receipt::Return { id, .. } => write!(f, "Return({})", id),
            Receipt::ReturnData { id, .. } => write!(f, "ReturnData({})", id),
            Receipt::Panic { id, .. } => write!(f, "Panic({})", id),
            Receipt::Revert { id, .. } => write!(f, "Revert({})", id),
            Receipt::Log { id, .. } => write!(f, "Log({})", id),
            Receipt::LogData { id, .. } => write!(f, "LogData({})", id),
            Receipt::Transfer { id, .. } => write!(f, "Transfer({})", id),
            Receipt::TransferOut { id, .. } => write!(f, "TransferOut({})", id),
            Receipt::ScriptResult { result, gas_used } => {
                write!(f, "ScriptResult({:?}, {})", result, gas_used)
            }
            Receipt::MessageOut {
                sender,
                recipient,
                amount,
                ..
            } => write!(f, "MessageOut({} -> {} : {})", sender, recipient, amount),
            Receipt::Mint {
                sub_id,
                contract_id,
                val,
                ..
            } => write!(
                f,
                "Mint(sub_id={}, contract_id={}, val={})",
                sub_id, contract_id, val
            ),
            Receipt::Burn {
                sub_id,
                contract_id,
                val,
                ..
            } => write!(
                f,
                "Burn(sub_id={}, contract_id={}, val={})",
                sub_id, contract_id, val
            ),
        }
    }
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
        pc: Word,
        is: Word,
        data: Vec<u8>,
    ) -> Self {
        let digest = Hasher::hash(&data);
        Self::return_data_with_len(
            id,
            ptr,
            data.len() as Word,
            digest,
            pc,
            is,
            Some(data),
        )
    }

    pub const fn return_data_with_len(
        id: ContractId,
        ptr: Word,
        len: Word,
        digest: Bytes32,
        pc: Word,
        is: Word,
        data: Option<Vec<u8>>,
    ) -> Self {
        Self::ReturnData {
            id,
            ptr,
            len,
            digest,
            pc,
            is,
            data,
        }
    }

    pub const fn panic(
        id: ContractId,
        reason: PanicInstruction,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::Panic {
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
        pc: Word,
        is: Word,
        data: Vec<u8>,
    ) -> Self {
        let digest = Hasher::hash(&data);
        Self::log_data_with_len(
            id,
            ra,
            rb,
            ptr,
            data.len() as Word,
            digest,
            pc,
            is,
            Some(data),
        )
    }

    pub const fn log_data_with_len(
        id: ContractId,
        ra: Word,
        rb: Word,
        ptr: Word,
        len: Word,
        digest: Bytes32,
        pc: Word,
        is: Word,
        data: Option<Vec<u8>>,
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
            data,
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

    pub fn message_out(
        txid: &Bytes32,
        idx: Word,
        sender: Address,
        recipient: Address,
        amount: Word,
        data: Vec<u8>,
    ) -> Self {
        let nonce = Output::message_nonce(txid, idx);
        let digest = Output::message_digest(&data);

        Self::message_out_with_len(
            sender,
            recipient,
            amount,
            nonce,
            data.len() as Word,
            digest,
            Some(data),
        )
    }

    pub const fn message_out_with_len(
        sender: Address,
        recipient: Address,
        amount: Word,
        nonce: Nonce,
        len: Word,
        digest: Bytes32,
        data: Option<Vec<u8>>,
    ) -> Self {
        Self::MessageOut {
            sender,
            recipient,
            amount,
            nonce,
            len,
            digest,
            data,
        }
    }

    pub fn mint(
        sub_id: Bytes32,
        contract_id: ContractId,
        val: Word,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::Mint {
            sub_id,
            contract_id,
            val,
            pc,
            is,
        }
    }

    pub fn burn(
        sub_id: Bytes32,
        contract_id: ContractId,
        val: Word,
        pc: Word,
        is: Word,
    ) -> Self {
        Self::Burn {
            sub_id,
            contract_id,
            val,
            pc,
            is,
        }
    }

    #[inline(always)]
    pub fn id(&self) -> Option<&ContractId> {
        trim_contract_id(match self {
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
            Self::Mint { contract_id, .. } => Some(contract_id),
            Self::Burn { contract_id, .. } => Some(contract_id),
        })
    }

    pub const fn sub_id(&self) -> Option<&Bytes32> {
        match self {
            Self::Mint { sub_id, .. } => Some(sub_id),
            Self::Burn { sub_id, .. } => Some(sub_id),
            _ => None,
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
            Self::Mint { pc, .. } => Some(*pc),
            Self::Burn { pc, .. } => Some(*pc),
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
            Self::Mint { is, .. } => Some(*is),
            Self::Burn { is, .. } => Some(*is),
        }
    }

    #[inline(always)]
    pub fn to(&self) -> Option<&ContractId> {
        trim_contract_id(match self {
            Self::Call { to, .. } => Some(to),
            Self::Transfer { to, .. } => Some(to),
            _ => None,
        })
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
            Self::Mint { val, .. } => Some(*val),
            Self::Burn { val, .. } => Some(*val),
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
            Self::ReturnData { data, .. } => data.as_ref().map(|data| data.as_slice()),
            Self::LogData { data, .. } => data.as_ref().map(|data| data.as_slice()),
            Self::MessageOut { data, .. } => data.as_ref().map(|data| data.as_slice()),
            _ => None,
        }
    }

    pub const fn reason(&self) -> Option<PanicInstruction> {
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

    pub fn message_id(&self) -> Option<MessageId> {
        match self {
            Self::MessageOut {
                sender,
                recipient,
                amount,
                nonce,
                data,
                ..
            } => data.as_ref().map(|data| {
                compute_message_id(sender, recipient, nonce, *amount, data.as_slice())
            }),
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

    pub const fn nonce(&self) -> Option<&Nonce> {
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
}

fn trim_contract_id(id: Option<&ContractId>) -> Option<&ContractId> {
    id.and_then(|id| {
        if id != &ContractId::zeroed() {
            Some(id)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::Receipt;
    use fuel_types::ContractId;

    // TODO: Rewrite the test cases when `Receipt` will have its struct for
    //  each variant. It will allow to use `Default` trait.
    #[rstest::rstest]
    #[case(
        Receipt::Call {
            id: ContractId::from([1; 32]),
            to: Default::default(),
            amount: 0,
            asset_id: Default::default(),
            gas: 0,
            param1: 0,
            param2: 0,
            pc: 0,
            is: 0,
        },
        Some(ContractId::from([1; 32]))
    )]
    #[case(
        Receipt::Call {
            id: ContractId::from([0; 32]),
            to: Default::default(),
            amount: 0,
            asset_id: Default::default(),
            gas: 0,
            param1: 0,
            param2: 0,
            pc: 0,
            is: 0,
        },
        None
    )]
    #[case(
        Receipt::Return {
            id: ContractId::from([2; 32]),
            val: 0,
            pc: 0,
            is: 0,
        },
        Some(ContractId::from([2; 32]))
    )]
    #[case(
        Receipt::Return {
            id: ContractId::from([0; 32]),
            val: 0,
            pc: 0,
            is: 0,
        },
        None
    )]
    fn receipt_id(#[case] receipt: Receipt, #[case] expected_id: Option<ContractId>) {
        assert_eq!(receipt.id(), expected_id.as_ref());
    }

    // TODO: Rewrite the test cases when `Receipt` will have its struct for
    //  each variant. It will allow to use `Default` trait.
    #[rstest::rstest]
    #[case(
        Receipt::Call {
            id: Default::default(),
            to: ContractId::from([1; 32]),
            amount: 0,
            asset_id: Default::default(),
            gas: 0,
            param1: 0,
            param2: 0,
            pc: 0,
            is: 0,
        },
        Some(ContractId::from([1; 32]))
    )]
    #[case(
        Receipt::Call {
            id: Default::default(),
            to: ContractId::from([0; 32]),
            amount: 0,
            asset_id: Default::default(),
            gas: 0,
            param1: 0,
            param2: 0,
            pc: 0,
            is: 0,
        },
        None
    )]
    #[case(
        Receipt::Return {
            id: Default::default(),
            val: 0,
            pc: 0,
            is: 0,
        },
        None
    )]
    fn receipt_to(#[case] receipt: Receipt, #[case] expected_to: Option<ContractId>) {
        assert_eq!(receipt.to(), expected_to.as_ref());
    }
}
