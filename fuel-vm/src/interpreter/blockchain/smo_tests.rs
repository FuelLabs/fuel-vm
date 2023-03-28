use crate::interpreter::contract::balance as contract_balance;
use crate::{interpreter::memory::Memory, storage::MemoryStorage};

use super::*;

use fuel_tx::Create;
use test_case::test_case;

struct Input {
    /// A
    recipient_mem_address: Word,
    /// B
    msg_data_ptr: Word,
    /// C
    msg_data_len: Word,
    /// D
    amount_coins_to_send: Word,
    internal: bool,
    max_message_data_length: Word,
    memory: Vec<(usize, Vec<u8>)>,
    /// Initial balance of the zeroed AssedId, same for both default contract and external context
    initial_balance: Word,
}

#[derive(Debug, PartialEq, Eq)]
struct Output {
    receipts: ReceiptsCtx,
    /// Resulting internal (contract) balance of zeroed AssetId with zeroed  ContractId
    internal_balance: Word,
    /// Resulting external balance of zeroed AssetId
    external_balance: Word,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            recipient_mem_address: Default::default(),
            msg_data_ptr: Default::default(),
            msg_data_len: Default::default(),
            amount_coins_to_send: Default::default(),
            internal: false,
            memory: vec![(400, Address::from([1u8; 32]).to_vec())],
            max_message_data_length: 100,
            initial_balance: 0,
        }
    }
}

#[test_case(
    Input {
        recipient_mem_address: 400,
        msg_data_ptr: 0,
        msg_data_len: 1,
        amount_coins_to_send: 0,
        ..Default::default()
    } => matches Ok(Output { .. })
    ; "sanity test (external)"
)]
#[test_case(
    Input {
        recipient_mem_address: 400,
        msg_data_ptr: 0,
        msg_data_len: 1,
        amount_coins_to_send: 0,
        internal: true,
        ..Default::default()
    } => matches Ok(Output { .. })
    ; "sanity test (internal)"
)]
#[test_case(
    Input {
        recipient_mem_address: 0,
        msg_data_ptr: 0,
        msg_data_len: 0,
        amount_coins_to_send: 0,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))
    ; "Call abi can't be zero"
)]
#[test_case(
    Input {
        recipient_mem_address: 0,
        msg_data_ptr: Word::MAX,
        msg_data_len: 1,
        amount_coins_to_send: 0,
        max_message_data_length: Word::MAX,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))
    ; "address + call abi length overflows"
)]
#[test_case(
    Input {
        recipient_mem_address: 0,
        msg_data_ptr: VM_MAX_RAM - 64,
        msg_data_len: 100,
        amount_coins_to_send: 0,
        max_message_data_length: Word::MAX,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))
    ; "address + call abi length overflows memory"
)]
#[test_case(
    Input {
        recipient_mem_address: 0,
        msg_data_ptr: 0,
        msg_data_len: 101,
        amount_coins_to_send: 0,
        max_message_data_length: 100,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::MessageDataTooLong))
    ; "call abi length > max message data length"
)]
#[test_case(
    Input {
        recipient_mem_address: 400,
        msg_data_ptr: 0,
        msg_data_len: 10,
        amount_coins_to_send: 30,
        initial_balance: 29,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance))
    ; "amount coins to send > balance from external context"
)]
#[test_case(
    Input {
        recipient_mem_address: 400,
        msg_data_ptr: 0,
        msg_data_len: 10,
        amount_coins_to_send: 30,
        initial_balance: 29,
        internal: true,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance))
    ; "amount coins to send > balance from internal context"
)]
#[test_case(
    Input {
        recipient_mem_address: 400,
        msg_data_ptr: 432,
        msg_data_len: 10,
        amount_coins_to_send: 20,
        initial_balance: 29,
        ..Default::default()
    } => matches Ok(Output { external_balance: 9, internal_balance: 29, .. })
    ; "coins sent succesfully from external context"
)]
#[test_case(
    Input {
        recipient_mem_address: 400,
        msg_data_ptr: 432,
        msg_data_len: 10,
        amount_coins_to_send: 20,
        initial_balance: 29,
        internal: true,
        ..Default::default()
    } => matches Ok(Output { external_balance: 29, internal_balance: 9, .. })
    ; "coins sent succesfully from internal context"
)]
#[test_case(
    Input {
        recipient_mem_address: 400,
        msg_data_ptr: 432,
        msg_data_len: 10,
        amount_coins_to_send: 20,
        initial_balance: 20,
        ..Default::default()
    } => matches Ok(Output { external_balance: 0, internal_balance: 20, .. })
    ; "spend all coins succesfully from external context"
)]
#[test_case(
    Input {
        recipient_mem_address: 400,
        msg_data_ptr: 432,
        msg_data_len: 10,
        amount_coins_to_send: 20,
        initial_balance: 20,
        internal: true,
        ..Default::default()
    } => matches Ok(Output { external_balance: 20, internal_balance: 0, .. })
    ; "spend all coins succesfully from internal context"
)]
fn test_smo(
    Input {
        recipient_mem_address,
        msg_data_len,
        msg_data_ptr,
        amount_coins_to_send,
        internal,
        memory: mem,
        max_message_data_length,
        initial_balance,
    }: Input,
) -> Result<Output, RuntimeError> {
    let asset = AssetId::zeroed();

    let mut memory: Memory<MEM_SIZE> = vec![0; MEM_SIZE].try_into().unwrap();
    for (offset, bytes) in mem {
        memory[offset..offset + bytes.len()].copy_from_slice(bytes.as_slice());
    }
    let mut receipts = Default::default();
    let mut tx = Create::default();
    let mut storage = MemoryStorage::new(Default::default(), Address::default());
    storage
        .merkle_contract_asset_id_balance_insert(&ContractId::default(), &asset, initial_balance)
        .unwrap();
    let mut balances =
        RuntimeBalances::try_from_iter([(asset, initial_balance)].into_iter()).expect("Should be valid balance");
    let fp = 0;
    let mut pc = 0;
    let input = MessageOutputCtx {
        max_message_data_length,
        memory: &mut memory,
        tx_offset: 0,
        receipts: &mut receipts,
        tx: &mut tx,
        balances: &mut balances,
        storage: &mut storage,
        current_contract: if internal { Some(ContractId::default()) } else { None },
        fp: Reg::new(&fp),
        pc: RegMut::new(&mut pc),
        recipient_mem_address,
        msg_data_len,
        msg_data_ptr,
        amount_coins_to_send,
    };

    input.message_output()?;

    Ok(Output {
        receipts,
        internal_balance: contract_balance(&storage, &ContractId::default(), &asset).unwrap(),
        external_balance: balances.balance(&asset).unwrap(),
    })
}
