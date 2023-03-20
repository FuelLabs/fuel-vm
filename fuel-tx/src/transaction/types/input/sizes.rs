use fuel_asm::Word;
use fuel_types::bytes::WORD_SIZE;
use fuel_types::Address;
use fuel_types::AssetId;
use fuel_types::Bytes32;
use fuel_types::ContractId;
use fuel_types::{mem_layout, Nonce};

use crate::TxPointer;
use crate::UtxoId;

pub struct CoinSizes;
mem_layout!(
    CoinSizesLayout for CoinSizes
    utxo_id: UtxoId = {UtxoId::LEN},
    owner: Address = {Address::LEN},
    amount: Word = WORD_SIZE,
    asset_id: AssetId = {AssetId::LEN},
    tx_pointer: TxPointer = {TxPointer::LEN},
    witness_index: u8 = WORD_SIZE,
    maturity: Word = WORD_SIZE,
    predicate_len: Word = WORD_SIZE,
    predicate_data_len: Word = WORD_SIZE
);

pub struct ContractSizes;
mem_layout!(
    ContractSizesLayout for ContractSizes
    tx_id: Bytes32 = {Bytes32::LEN},
    output_index: Word = WORD_SIZE,
    balance_root: Bytes32 = {Bytes32::LEN},
    state_root: Bytes32 = {Bytes32::LEN},
    tx_pointer: TxPointer = {TxPointer::LEN},
    contract_id: ContractId = {ContractId::LEN}
);

pub struct MessageSizes;
mem_layout!(
    MessageSizesLayout for MessageSizes
    sender: Address = {Address::LEN},
    recipient: Address = {Address::LEN},
    amount: Word = WORD_SIZE,
    nonce: Nonce = {Nonce::LEN},
    witness_index: u8 = WORD_SIZE,
    data_len: Word = WORD_SIZE,
    predicate_len: Word = WORD_SIZE,
    predicate_data_len: Word = WORD_SIZE
);

#[test]
fn test_consts() {
    let l = MessageSizesLayout::new();
    assert_eq!(l.sender.addr(), super::consts::INPUT_MESSAGE_SENDER_OFFSET - WORD_SIZE);
    assert_eq!(
        l.recipient.addr(),
        super::consts::INPUT_MESSAGE_RECIPIENT_OFFSET - WORD_SIZE
    );
    assert_eq!(
        MessageSizesLayout::LEN,
        super::consts::INPUT_MESSAGE_FIXED_SIZE - WORD_SIZE
    );
}
