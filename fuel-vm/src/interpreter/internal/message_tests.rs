#![allow(clippy::arithmetic_side_effects, clippy::cast_possible_truncation)]

use alloc::{
    vec,
    vec::Vec,
};

use crate::{
    consts::MEM_SIZE,
    error::PanicOrBug,
};

use super::*;
use fuel_tx::{
    Create,
    field::Policies as PoliciesField,
    policies::Policies,
};
use test_case::test_case;

#[test_case(0, 0, 0 => None)]
#[test_case(0, 0, 1 => Some(88))]
#[test_case(88, 0, 1 => Some(176))]
#[test_case(0, 1, 2 => Some(160))]
#[test_case(0, 2, 3 => Some(232))]
#[test_case(0, 1, 3 => Some(160))]
#[test_case(44, 2, 3 => Some(276))]
#[test_case(88, 1, 1 => None)]
// #[test_case(usize::MAX, 0, 1 => None ; "tx_offset and num_outputs should be constrained
// but they aren't")]
fn test_absolute_output_offset(
    tx_offset: usize,
    idx: usize,
    num_outputs: usize,
) -> Option<usize> {
    let mut tx = Create::default();
    *tx.policies_mut() = Policies::default();
    *tx.outputs_mut() = vec![Output::default(); num_outputs];

    absolute_output_offset(&tx, tx_offset, idx)
}

#[test_case(
    0 => with |r: Result<_, _>| check_memory(r.unwrap(), &[(88, Output::default().to_bytes())])
    ; "Output at start of memory"
)]
#[test_case(
    200 => with |r: Result<_, _>| check_memory(r.unwrap(), &[(200 + 88, Output::default().to_bytes())])
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
fn test_update_memory_output(tx_offset: usize) -> SimpleResult<MemoryInstance> {
    let mut tx = Create::default();
    *tx.policies_mut() = Policies::default();
    *tx.outputs_mut() = vec![Output::default()];
    let mut memory: MemoryInstance = vec![0; MEM_SIZE].try_into().unwrap();
    update_memory_output(&tx, &mut memory, tx_offset, 0).map(|_| memory)
}

fn check_memory(result: MemoryInstance, expected: &[(usize, Vec<u8>)]) {
    for (offset, bytes) in expected {
        assert_eq!(
            &result[*offset..*offset + bytes.len()],
            bytes.as_slice(),
            "memory mismatch at {offset}"
        );
    }
}
