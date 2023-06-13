use fuel_asm::Word;
use fuel_types::{
    bytes::WORD_SIZE,
    mem_layout,
    Address,
    AssetId,
    Bytes32,
    ContractId,
};

pub struct CoinSizes;
mem_layout!(
    CoinSizesLayout for CoinSizes
    repr: u8 = WORD_SIZE,
    to: Address = {Address::LEN},
    amount: Word = WORD_SIZE,
    asset_id: AssetId = {AssetId::LEN}
);

pub struct MessageSizes;
mem_layout!(
    MessageSizesLayout for MessageSizes
    repr: u8 = WORD_SIZE,
    recipient: Address = {Address::LEN},
    amount: Word = WORD_SIZE
);

pub struct ContractSizes;
mem_layout!(
    ContractSizesLayout for ContractSizes
    repr: u8 = WORD_SIZE,
    input_index: u8 = WORD_SIZE,
    balance_root: Bytes32 = {Bytes32::LEN},
    state_root: Bytes32 = {Bytes32::LEN}
);

pub struct ContractCreatedSizes;
mem_layout!(
    ContractCreatedSizesLayout for ContractCreatedSizes
    repr: u8 = WORD_SIZE,
    contract_id: ContractId = {ContractId::LEN},
    state_root: Bytes32 = {Bytes32::LEN}
);
