use crate::{
    checked_transaction::IntoChecked,
    constraints::reg_key::{
        HP,
        IS,
        ONE,
        SSP,
    },
    interpreter::{
        InterpreterParams,
        MemoryInstance,
        NotSupportedEcal,
    },
    memory_client::MemoryClient,
    prelude::{
        MemoryStorage,
        Transactor,
    },
    tests::test_helpers::{
        assert_success,
        run_script,
        set_full_word,
    },
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    Instruction,
    PanicReason,
    RegId,
    op,
};
use fuel_tx::{
    ConsensusParameters,
    Finalizable,
    GasCosts,
    Receipt,
    TransactionBuilder,
};
use test_case::test_case;

fn ldcv2_data_init() -> Vec<Instruction> {
    vec![
        // Write bytes 0..16 to end of the heap
        op::move_(0x11, RegId::HP),
        op::movi(0x10, 16),
        op::aloc(0x10),
        // loop:
        op::sub(0x10, 0x10, 1),
        op::sub(0x11, 0x11, 1),
        op::sb(0x11, 0x10, 0),
        op::jnzb(0x10, RegId::ZERO, 2), // jmp loop
    ]
}

#[test_case(0, 0 => Vec::<u8>::new())]
#[test_case(0, 1 => vec![0; 8])]
#[test_case(0, 2 => vec![0, 1, 0, 0, 0, 0, 0, 0])]
#[test_case(0, 16 => vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15])]
#[test_case(1, 15 => vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0])]
#[test_case(4, 12 => vec![4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 0, 0, 0])]
#[test_case(7, 9 => vec![7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 0, 0, 0, 0, 0, 0])]
#[test_case(8, 8 => vec![8, 9, 10, 11, 12, 13, 14, 15])]
#[test_case(9, 7 => vec![9, 10, 11, 12, 13, 14, 15, 0])]
fn ldcv2__has_correct_padding(offset: u32, len: u32) -> Vec<u8> {
    let mut script = ldcv2_data_init();
    let padded_len = len.next_multiple_of(8) as u64;
    script.extend(set_full_word(0x14, padded_len));
    script.extend([
        op::movi(0x11, offset),
        op::movi(0x12, len),
        op::move_(0x13, RegId::SSP),
        op::ldc(RegId::HP, 0x11, 0x12, 2),
        op::logd(RegId::SSP, RegId::ZERO, 0x13, 0x14),
        op::ret(0x1),
    ]);
    let receipts = run_script(script);
    assert_success(&receipts);
    let Some(Receipt::LogData { data, .. }) = receipts.first() else {
        panic!("Expected LogData receipt");
    };
    data.clone().unwrap()
}

fn ldcv2_reason_helper(script: Vec<Instruction>) -> Result<(), PanicReason> {
    let gas_price = 0;

    // make gas costs free
    let gas_costs = GasCosts::free();

    let mut consensus_params = ConsensusParameters::default();
    consensus_params.set_gas_costs(gas_costs);

    let interpreter_params = InterpreterParams::new(gas_price, &consensus_params);

    let mut client = MemoryClient::<_, NotSupportedEcal>::from_txtor(Transactor::new(
        MemoryInstance::new(),
        MemoryStorage::default(),
        interpreter_params,
    ));

    let gas_limit = 1_000_000;
    let maturity = Default::default();
    let height = Default::default();

    let script = TransactionBuilder::script(script.into_iter().collect(), vec![])
        .script_gas_limit(gas_limit)
        .maturity(maturity)
        .add_fee_input()
        .finalize()
        .into_checked(height, &consensus_params)
        .expect("failed to check tx");

    let receipts = client.transact(script);
    if let Receipt::Panic { id: _, reason, .. } = receipts.first().expect("No receipt") {
        Err(*reason.reason())
    } else {
        Ok(())
    }
}

#[test]
fn ldcv2__fails_with_nonempty_stack() {
    // Given
    let script = vec![
        op::cfei(0x1), // sp += 1
        op::ldc(IS, RegId::ZERO, RegId::ONE, 2),
    ];

    // When
    let result = ldcv2_reason_helper(script);

    // Then
    assert_eq!(result, Err(PanicReason::ExpectedUnallocatedStack));
}

#[test]
fn ldcv2__fails_when_mem_offset_is_above_reg_hp() {
    // Given
    let script = vec![op::ldc(IS, RegId::HP, ONE, 2)];

    // When
    let result = ldcv2_reason_helper(script);

    // Then
    assert_eq!(result, Err(PanicReason::MemoryOverflow));
}

#[test]
fn ldcv2__fails_when_size_overflows() {
    // Given
    let script = vec![
        op::not(0x20, RegId::ZERO),
        op::ldc(RegId::HP, RegId::ZERO, 0x20, 2),
    ];

    // When
    let result = ldcv2_reason_helper(script);

    // Then
    assert_eq!(result, Err(PanicReason::MemoryOverflow));
}

#[test]
fn ldcv2__fails_when_source_beyond_max_ram() {
    // Given
    let script = vec![op::ldc(HP, RegId::ZERO, ONE, 2)];

    // When
    let result = ldcv2_reason_helper(script);

    // Then
    assert_eq!(result, Err(PanicReason::MemoryOverflow));
}

#[test]
fn ldcv2__works_with_zero_length() {
    // Given
    let script = vec![op::ldc(SSP, RegId::ZERO, RegId::ZERO, 2), op::ret(0x1)];

    // When
    let result = ldcv2_reason_helper(script);

    // Then
    assert_eq!(result, Ok(()));
}

#[test]
fn ldcv2__fails_when_memory_overlaps() {
    // Given
    let script = vec![op::ldc(SSP, RegId::ZERO, RegId::ONE, 2)];

    // When
    let result = ldcv2_reason_helper(script);

    // Then
    assert_eq!(result, Err(PanicReason::MemoryWriteOverlap));
}
