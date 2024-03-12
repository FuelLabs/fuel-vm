use alloc::{
    vec,
    vec::Vec,
};

use crate::{
    error::PanicOrBug,
    interpreter::memory::Memory,
};

use super::*;
use fuel_tx::Create;
use fuel_vm::consts::*;
use test_case::test_case;

#[test_case(0, 0, 0 => None)]
#[test_case(0, 0, 1 => Some(96))]
#[test_case(88, 0, 1 => Some(184))]
#[test_case(0, 1, 2 => Some(168))]
#[test_case(0, 2, 3 => Some(240))]
#[test_case(0, 1, 3 => Some(168))]
#[test_case(44, 2, 3 => Some(284))]
#[test_case(88, 1, 1 => None)]
// #[test_case(usize::MAX, 0, 1 => None ; "tx_offset and num_outputs should be constrained
// but they aren't")]
fn test_absolute_output_offset(
    tx_offset: usize,
    idx: usize,
    num_outputs: usize,
) -> Option<usize> {
    let mut tx = Create::default();
    *tx.outputs_mut() = vec![Output::default(); num_outputs];

    absolute_output_offset(&tx, tx_offset, idx)
}

#[test_case(
    0 => with |r: Result<_, _>| check_memory(r.unwrap(), &[(96, Output::default().to_bytes())])
    ; "Output at start of memory"
)]
#[test_case(
    200 => with |r: Result<_, _>| check_memory(r.unwrap(), &[(200 + 96, Output::default().to_bytes())])
    ; "Output at 200 in memory"
)]
#[test_case(
    MEM_SIZE - 1 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow))
    ; "Output at MEM_SIZE - 1 should overflow"
)]
#[test_case(
    MEM_SIZE - 1 - 112 => Err(PanicOrBug::Panic(PanicReason::MemoryOverflow))
    ; "Output at MEM_SIZE - 1 - output_size should overflow"
)]
fn test_update_memory_output(tx_offset: usize) -> SimpleResult<Memory> {
    let mut tx = Create::default();
    *tx.outputs_mut() = vec![Output::default()];
    let mut memory: Memory = vec![0; MEM_SIZE].try_into().unwrap();
    update_memory_output(&mut tx, &mut memory, tx_offset, 0).map(|_| memory)
}

fn check_memory(result: Memory, expected: &[(usize, Vec<u8>)]) {
    for (offset, bytes) in expected {
        assert_eq!(
            &result[*offset..*offset + bytes.len()],
            bytes.as_slice(),
            "memory mismatch at {offset}"
        );
    }
}
