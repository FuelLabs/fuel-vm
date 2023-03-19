use crate::interpreter::memory::Memory;

use super::*;

use fuel_tx::Create;
use test_case::test_case;

struct Input {
    /// A
    recipient_mem_address: Word,
    /// B
    call_abi_len: Word,
    /// C
    message_output_idx: Word,
    /// D
    amount_coins_to_send: Word,
    max_message_data_length: Word,
    memory: Vec<(usize, Vec<u8>)>,
    balance: Vec<(AssetId, Word)>,
}

#[derive(Debug, PartialEq, Eq)]
struct Output {
    receipts: ReceiptsCtx,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            recipient_mem_address: Default::default(),
            call_abi_len: Default::default(),
            message_output_idx: Default::default(),
            amount_coins_to_send: Default::default(),
            memory: vec![(400, Address::from([1u8; 32]).to_vec())],
            max_message_data_length: 100,
            balance: Default::default(),
        }
    }
}

#[test_case(
    Input {
        recipient_mem_address: 400,
        call_abi_len: 1,
        message_output_idx: 0,
        amount_coins_to_send: 0,
        ..Default::default()
    } => matches Ok(Output { .. })
    ; "sanity test"
)]
#[test_case(
    Input {
        recipient_mem_address: 0,
        call_abi_len: 0,
        message_output_idx: 0,
        amount_coins_to_send: 0,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))
    ; "Call abi can't be zero"
)]
#[test_case(
    Input {
        recipient_mem_address: Word::MAX,
        call_abi_len: 1,
        message_output_idx: 0,
        amount_coins_to_send: 0,
        max_message_data_length: Word::MAX,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))
    ; "address + call abi length overflows"
)]
#[test_case(
    Input {
        recipient_mem_address: VM_MAX_RAM - 64,
        call_abi_len: 100,
        message_output_idx: 0,
        amount_coins_to_send: 0,
        max_message_data_length: Word::MAX,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::ArithmeticOverflow))
    ; "address + call abi length overflows memory"
)]
#[test_case(
    Input {
        recipient_mem_address: 400,
        call_abi_len: 101,
        message_output_idx: 0,
        amount_coins_to_send: 0,
        max_message_data_length: 100,
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::MemoryOverflow))
    ; "call abi length > max message data length"
)]
#[test_case(
    Input {
        recipient_mem_address: 400,
        call_abi_len: 10,
        message_output_idx: 0,
        amount_coins_to_send: 30,
        balance: [(AssetId::zeroed(), 29)].into_iter().collect(),
        ..Default::default()
    } => Err(RuntimeError::Recoverable(PanicReason::NotEnoughBalance))
    ; "amount coins to send > balance"
)]
// TODO: Test the above on an internal context
fn test_smo(
    Input {
        recipient_mem_address,
        call_abi_len,
        message_output_idx,
        amount_coins_to_send,
        memory: mem,
        max_message_data_length,
        balance,
    }: Input,
) -> Result<Output, RuntimeError> {
    let mut memory: Memory<MEM_SIZE> = vec![0; MEM_SIZE].try_into().unwrap();
    for (offset, bytes) in mem {
        memory[offset..offset + bytes.len()].copy_from_slice(bytes.as_slice());
    }
    let mut receipts = Default::default();
    let mut tx = Create::default();
    let mut balances = RuntimeBalances::try_from_iter(balance).expect("Should be valid balance");
    let fp = 0;
    let mut pc = 0;
    let input = MessageOutputCtx {
        max_message_data_length,
        memory: &mut memory,
        tx_offset: 0,
        receipts: &mut receipts,
        tx: &mut tx,
        balances: &mut balances,
        fp: Reg::new(&fp),
        pc: RegMut::new(&mut pc),
        recipient_mem_address,
        call_abi_len,
        message_output_idx,
        amount_coins_to_send,
    };
    input.message_output().map(|_| Output { receipts })
}
