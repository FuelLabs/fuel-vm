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
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    op,
    Instruction,
    PanicReason,
    RegId,
};
use fuel_tx::{
    ConsensusParameters,
    Finalizable,
    GasCosts,
    Receipt,
    TransactionBuilder,
};

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
