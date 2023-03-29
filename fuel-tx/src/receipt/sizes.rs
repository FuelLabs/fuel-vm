use fuel_asm::Word;
use fuel_types::bytes::WORD_SIZE;
use fuel_types::mem_layout;
use fuel_types::Address;
use fuel_types::AssetId;
use fuel_types::Bytes32;
use fuel_types::ContractId;
use fuel_types::Nonce;

pub struct CallSizes;
mem_layout!(
    CallSizesLayout for CallSizes
    repr: u8 = WORD_SIZE,
    id: ContractId = {ContractId::LEN},
    to: ContractId = {ContractId::LEN},
    amount: Word = WORD_SIZE,
    asset_id: AssetId = {AssetId::LEN},
    gas: Word = WORD_SIZE,
    param1: Word = WORD_SIZE,
    param2: Word = WORD_SIZE,
    pc: Word = WORD_SIZE,
    is: Word = WORD_SIZE
);

pub struct ReturnSizes;
mem_layout!(
    ReturnSizesLayout for ReturnSizes
    repr: u8 = WORD_SIZE,
    id: ContractId = {ContractId::LEN},
    val: Word = WORD_SIZE,
    pc: Word = WORD_SIZE,
    is: Word = WORD_SIZE
);

pub struct ReturnDataSizes;
mem_layout!(
    ReturnDataSizesLayout for ReturnDataSizes
    repr: u8 = WORD_SIZE,
    id: ContractId = {ContractId::LEN},
    ptr: Word = WORD_SIZE,
    len: Word = WORD_SIZE,
    digest: Bytes32 = {Bytes32::LEN},
    pc: Word = WORD_SIZE,
    is: Word = WORD_SIZE
);

pub struct PanicSizes;
mem_layout!(
    PanicSizesLayout for PanicSizes
    repr: u8 = WORD_SIZE,
    id: ContractId = {ContractId::LEN},
    reason: Word = WORD_SIZE,
    pc: Word = WORD_SIZE,
    is: Word = WORD_SIZE
);

pub struct RevertSizes;
mem_layout!(
    RevertSizesLayout for RevertSizes
    repr: u8 = WORD_SIZE,
    id: ContractId = {ContractId::LEN},
    ra: Word = WORD_SIZE,
    pc: Word = WORD_SIZE,
    is: Word = WORD_SIZE
);

pub struct LogSizes;
mem_layout!(
    LogSizesLayout for LogSizes
    repr: u8 = WORD_SIZE,
    id: ContractId = {ContractId::LEN},
    ra: Word = WORD_SIZE,
    rb: Word = WORD_SIZE,
    rc: Word = WORD_SIZE,
    rd: Word = WORD_SIZE,
    pc: Word = WORD_SIZE,
    is: Word = WORD_SIZE
);

pub struct LogDataSizes;
mem_layout!(
    LogDataSizesLayout for LogDataSizes
    repr: u8 = WORD_SIZE,
    id: ContractId = {ContractId::LEN},
    ra: Word = WORD_SIZE,
    rb: Word = WORD_SIZE,
    ptr: Word = WORD_SIZE,
    len: Word = WORD_SIZE,
    digest: Bytes32 = {Bytes32::LEN},
    pc: Word = WORD_SIZE,
    is: Word = WORD_SIZE
);

pub struct TransferSizes;
mem_layout!(
    TransferSizesLayout for TransferSizes
    repr: u8 = WORD_SIZE,
    id: ContractId = {ContractId::LEN},
    to: ContractId = {ContractId::LEN},
    amount: Word = WORD_SIZE,
    asset_id: AssetId = {AssetId::LEN},
    pc: Word = WORD_SIZE,
    is: Word = WORD_SIZE
);

pub struct TransferOutSizes;
mem_layout!(
    TransferOutSizesLayout for TransferOutSizes
    repr: u8 = WORD_SIZE,
    id: ContractId = {ContractId::LEN},
    to: Address = {Address::LEN},
    amount: Word = WORD_SIZE,
    asset_id: AssetId = {AssetId::LEN},
    pc: Word = WORD_SIZE,
    is: Word = WORD_SIZE
);

pub struct ScriptResultSizes;
mem_layout!(
    ScriptResultSizesLayout for ScriptResultSizes
    repr: u8 = WORD_SIZE,
    result: Word = WORD_SIZE,
    gas_used: Word = WORD_SIZE
);

pub struct MessageOutSizes;
mem_layout!(
    MessageOutSizesLayout for MessageOutSizes
    repr: u8 = WORD_SIZE,
    sender: Address = {Address::LEN},
    recipient: Address = {Address::LEN},
    amount: Word = WORD_SIZE,
    nonce: Nonce = {Nonce::LEN},
    len: Word = WORD_SIZE,
    digest: Bytes32 = {Bytes32::LEN},
    data_len: Word = WORD_SIZE
);
