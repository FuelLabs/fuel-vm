#![allow(non_snake_case)]

use crate::{
    consts::VM_MAX_RAM,
    prelude::*,
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    op,
    RegId,
};
use fuel_types::{
    canonical::Serialize,
    BlobId,
};
use policies::Policies;
use rand::{
    rngs::StdRng,
    Rng,
    RngCore,
    SeedableRng,
};
use test_case::test_case;

use super::test_helpers::{
    assert_success,
    RunResult,
};
use crate::tests::test_helpers::set_full_word;

#[test]
fn blob_cannot_be_reuploaded() {
    let mut rng = StdRng::seed_from_u64(2322u64);
    let mut client = MemoryClient::default();

    let input_amount = 1000;
    let spend_amount = 600;
    let asset_id = AssetId::BASE;

    let program: Witness = vec![op::ret(RegId::ONE)]
        .into_iter()
        .collect::<Vec<u8>>()
        .into();

    let policies = Policies::new().with_max_fee(0);

    let mut blob = Transaction::blob(
        BlobBody {
            id: BlobId::compute(program.as_ref()),
            witness_index: 0,
        },
        policies,
        vec![],
        vec![
            Output::change(rng.r#gen(), 0, asset_id),
            Output::coin(rng.r#gen(), spend_amount, asset_id),
        ],
        vec![program, Witness::default()],
    );
    blob.add_unsigned_coin_input(
        rng.r#gen(),
        &Default::default(),
        input_amount,
        asset_id,
        rng.r#gen(),
        Default::default(),
    );

    let consensus_params = ConsensusParameters::standard();

    let blob = blob
        .into_checked_basic(1.into(), &consensus_params)
        .expect("failed to generate checked tx");

    // upload the first time
    client
        .blob(blob.clone())
        .expect("First blob should be executed");
    let mut txtor: Transactor<_, _, _> = client.into();
    // reupload should fail
    let result = txtor.blob(blob).unwrap_err();
    assert_eq!(
        result,
        InterpreterError::Panic(PanicReason::BlobIdAlreadyUploaded)
    );
}

fn test_ctx_with_random_blob(size: usize) -> (TestBuilder, BlobId) {
    let mut test_context = TestBuilder::new(1234u64);

    let mut blob_data = vec![0; size];
    test_context.rng.fill_bytes(&mut blob_data);
    let blob_id = BlobId::compute(&blob_data);

    test_context.setup_blob(blob_data.clone());

    (test_context, blob_id)
}

#[rstest::rstest]
fn blob_size__works_with_different_sizes_in_external_context(
    #[values(0, 1, 2, 7, 8, 9, 1024, 1234, 9876)] size: usize,
) {
    // Given
    let (mut test_context, blob_id) = test_ctx_with_random_blob(size);

    // When
    let state = test_context
        .start_script(
            vec![
                op::gtf_args(0x11, RegId::ZERO, GTFArgs::ScriptData),
                op::bsiz(0x10, 0x11),
                op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
                op::ret(RegId::ONE),
            ],
            blob_id.to_bytes(),
        )
        .script_gas_limit(1_000_000)
        .fee_input()
        .execute();

    // Then
    let receipts = state.receipts();
    assert_success(receipts);
    let bsiz = receipts
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .next()
        .expect("Missing log receipt");
    assert_eq!(bsiz, size as u64);
}

#[rstest::rstest]
fn blob_size__works_with_different_sizes_in_internal_context(
    #[values(0, 1, 2, 7, 8, 9, 1024, 1234, 9876)] size: usize,
) {
    // Given
    let (mut test_context, blob_id) = test_ctx_with_random_blob(size);

    let contract_to_call = test_context
        .setup_contract(
            vec![
                op::gtf_args(0x11, RegId::ZERO, GTFArgs::ScriptData),
                op::bsiz(0x10, 0x11),
                op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
                op::ret(RegId::ONE),
            ],
            None,
            None,
        )
        .contract_id;

    // When
    let mut script_data = blob_id.to_bytes();
    script_data.extend(Call::new(contract_to_call, 0, 0).to_bytes());
    let state = test_context
        .start_script(
            vec![
                op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
                op::addi(0x10, 0x10, BlobId::LEN.try_into().unwrap()),
                op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
                op::ret(RegId::ONE),
            ],
            script_data,
        )
        .script_gas_limit(1_000_000)
        .contract_input(contract_to_call)
        .contract_output(&contract_to_call)
        .fee_input()
        .execute();

    // Then
    let receipts = state.receipts();
    assert_success(receipts);
    let bsiz = receipts
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .next()
        .expect("Missing log receipt");
    assert_eq!(bsiz, size as u64);
}

#[rstest::rstest]
fn load_blob__loading_whole_blob_works_with_different_sizes_in_external_context(
    #[values(0, 1, 2, 7, 8, 9, 1024, 1234, 9876)] size: usize,
) {
    // Given
    let (mut test_context, blob_id) = test_ctx_with_random_blob(size);

    // When
    let mut ops = set_full_word(0x12, size as u64);
    ops.extend([
        op::gtf_args(0x11, RegId::ZERO, GTFArgs::ScriptData),
        op::aloc(0x12),
        op::bldd(RegId::HP, 0x11, RegId::ZERO, 0x12),
        op::logd(RegId::ZERO, RegId::ZERO, RegId::HP, 0x12),
        op::ret(RegId::ONE),
    ]);

    let state = test_context
        .start_script(ops, blob_id.to_bytes())
        .script_gas_limit(1_000_000)
        .fee_input()
        .execute();

    // Then
    let receipts = state.receipts();
    assert_success(receipts);

    let bldd = receipts
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::LogData { data, .. } => Some(data.clone().unwrap()),
            _ => None,
        })
        .next()
        .expect("Missing logdata receipt");
    assert_eq!(BlobId::compute(&bldd), blob_id);
}

#[rstest::rstest]
fn load_blob__loading_whole_blob_works_with_different_sizes_in_internal_context(
    #[values(0, 1, 2, 7, 8, 9, 1024, 1234, 9876)] size: usize,
) {
    // Given
    let (mut test_context, blob_id) = test_ctx_with_random_blob(size);

    let mut ops = set_full_word(0x12, size as u64);
    ops.extend([
        op::gtf_args(0x11, RegId::ZERO, GTFArgs::ScriptData),
        op::aloc(0x12),
        op::bldd(RegId::HP, 0x11, RegId::ZERO, 0x12),
        op::logd(RegId::ZERO, RegId::ZERO, RegId::HP, 0x12),
        op::ret(RegId::ONE),
    ]);

    let contract_to_call = test_context.setup_contract(ops, None, None).contract_id;

    // When
    let mut script_data = blob_id.to_bytes();
    script_data.extend(Call::new(contract_to_call, 0, 0).to_bytes());
    let state = test_context
        .start_script(
            vec![
                op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
                op::addi(0x10, 0x10, BlobId::LEN.try_into().unwrap()),
                op::call(0x10, RegId::ZERO, RegId::ZERO, RegId::CGAS),
                op::ret(RegId::ONE),
            ],
            script_data,
        )
        .script_gas_limit(1_000_000)
        .contract_input(contract_to_call)
        .contract_output(&contract_to_call)
        .fee_input()
        .execute();

    let receipts = state.receipts();
    assert_success(receipts);

    let bldd = receipts
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::LogData { data, .. } => Some(data.clone().unwrap()),
            _ => None,
        })
        .next()
        .expect("Missing logdata receipt");
    assert_eq!(BlobId::compute(&bldd), blob_id);
}

#[test_case(0, 0 => RunResult::Panic(PanicReason::BlobNotFound) ; "0 blob not found")]
#[test_case(1, 0 => RunResult::Panic(PanicReason::BlobNotFound) ; "1 byte blob not found")]
#[test_case(8, 0 => RunResult::Panic(PanicReason::BlobNotFound) ; "8 byte blob not found")]
#[test_case(32, 0 => RunResult::Panic(PanicReason::BlobNotFound) ; "32 byte blob not found")]
#[test_case(1000, 0 => RunResult::Panic(PanicReason::BlobNotFound) ; "1000 byte blob not found")]
#[test_case(0, VM_MAX_RAM / 2 => RunResult::Panic(PanicReason::UninitalizedMemoryAccess) ; "0 blob, id in uninitialized memory")]
#[test_case(1, VM_MAX_RAM / 2 => RunResult::Panic(PanicReason::UninitalizedMemoryAccess) ; "1 byte blob, id in uninitialized memory")]
#[test_case(8, VM_MAX_RAM / 2 => RunResult::Panic(PanicReason::UninitalizedMemoryAccess) ; "8 byte blob, id in uninitialized memory")]
#[test_case(32, VM_MAX_RAM / 2 => RunResult::Panic(PanicReason::UninitalizedMemoryAccess) ; "32 byte blob, id in uninitialized memory")]
#[test_case(1000, VM_MAX_RAM / 2 => RunResult::Panic(PanicReason::UninitalizedMemoryAccess) ; "1000 byte blob, id in uninitialized memory")]
#[test_case(0, VM_MAX_RAM - 32 => RunResult::Panic(PanicReason::BlobNotFound) ; "0 blob, id in heap memory")]
#[test_case(1, VM_MAX_RAM - 32 => RunResult::Panic(PanicReason::BlobNotFound) ; "1 blob, id in heap memory")]
#[test_case(0, VM_MAX_RAM - 31 => RunResult::Panic(PanicReason::MemoryOverflow) ; "0 byte blob, id ends just past memory end")]
#[test_case(1, VM_MAX_RAM - 31 => RunResult::Panic(PanicReason::MemoryOverflow) ; "1 byte blob, id ends just past memory end")]
#[test_case(0, VM_MAX_RAM => RunResult::Panic(PanicReason::MemoryOverflow) ; "0 byte blob, id starts past memory end")]
#[test_case(1, VM_MAX_RAM => RunResult::Panic(PanicReason::MemoryOverflow) ; "1 byte blob, id starts past memory end")]
#[test_case(0, Word::MAX - 31 => RunResult::Panic(PanicReason::MemoryOverflow) ; "0 byte blob, id ends just past Word::MAX")]
#[test_case(1, Word::MAX - 31 => RunResult::Panic(PanicReason::MemoryOverflow) ; "1 byte blob, id ends just past Word::MAX")]
#[test_case(0, Word::MAX => RunResult::Panic(PanicReason::MemoryOverflow) ; "0 byte blob, id starts at Word::MAX")]
#[test_case(1, Word::MAX => RunResult::Panic(PanicReason::MemoryOverflow) ; "1 byte blob, id starts at Word::MAX")]
fn blob_size__bounds(size: usize, blob_id_ptr: Word) -> RunResult<()> {
    let mut test_context = TestBuilder::new(1234u64);

    let mut ops = set_full_word(0x10, size as u64);
    ops.extend(set_full_word(0x11, blob_id_ptr));
    ops.extend([
        op::movi(0x12, 32),
        op::aloc(0x12),
        op::bsiz(0x10, 0x11),
        op::ret(RegId::ONE),
    ]);

    let state = test_context
        .start_script(ops, vec![])
        .script_gas_limit(1_000_000)
        .fee_input()
        .execute();
    RunResult::extract_novalue(state.receipts())
}

#[test_case(0, vec![] => RunResult::Success(vec![]) ; "empty blob, all ok")]
#[test_case(10, vec![] => RunResult::Success(vec![1; 10]) ; "non-empty blob, all ok")]
#[test_case(1, vec![op::movi(0x11, 0)] => RunResult::Panic(PanicReason::BlobNotFound) ; "no such blob")]
#[test_case(1, set_full_word(0x11, VM_MAX_RAM - 30) => RunResult::Panic(PanicReason::MemoryOverflow) ; "blob id ends outside ram")]
#[test_case(1, set_full_word(0x11, VM_MAX_RAM) => RunResult::Panic(PanicReason::MemoryOverflow) ; "blob id starts outside ram")]
#[test_case(1, set_full_word(0x11, Word::MAX - 32) => RunResult::Panic(PanicReason::MemoryOverflow) ; "blob id ends Word::MAX")]
#[test_case(1, set_full_word(0x11, Word::MAX) => RunResult::Panic(PanicReason::MemoryOverflow) ; "blob id starts at Word::MAX")]
#[test_case(0, vec![op::movi(0x10, 0)] => RunResult::Panic(PanicReason::MemoryOwnership) ; "empty blob write_to write-only memory")]
#[test_case(1, vec![op::movi(0x10, 0)] => RunResult::Panic(PanicReason::MemoryOwnership) ; "blob write_to write-only memory")]
#[test_case(0, vec![op::subi(0x10, RegId::HP, 1)] => RunResult::Panic(PanicReason::UninitalizedMemoryAccess) ; "empty blob write_to uninitialized memory")]
#[test_case(1, vec![op::subi(0x10, RegId::HP, 1)] => RunResult::Panic(PanicReason::UninitalizedMemoryAccess) ; "blob write_to uninitialized memory")]
#[test_case(0, vec![op::addi(0x10, RegId::HP, 1)] => RunResult::Panic(PanicReason::MemoryOverflow) ; "empty blob write_to past memory end")]
#[test_case(1, vec![op::addi(0x10, RegId::HP, 1)] => RunResult::Panic(PanicReason::MemoryOverflow) ; "blob write_to past memory end")]
#[test_case(0, vec![op::movi(0x12, 0)] => RunResult::Success(vec![]); "empty blob write_to offset 0")]
#[test_case(1, vec![op::movi(0x12, 0)] => RunResult::Success(vec![1]); "blob write_to offset 0")]
#[test_case(1, vec![op::movi(0x12, 1)] => RunResult::Success(vec![0]); "blob write_to offset 1 zero fills")]
#[test_case(0, vec![op::not(0x12, RegId::ZERO)] => RunResult::Success(vec![]); "empty blob write_to offset Word::MAX")]
#[test_case(1, vec![op::not(0x12, RegId::ZERO)] => RunResult::Success(vec![0]); "blob write_to offset Word::MAX zero fills")]
#[test_case(0, vec![op::addi(0x13, 0x13, 0)] => RunResult::Success(vec![]); "empty blob read_len 0")]
#[test_case(0, vec![op::aloc(RegId::ONE), op::move_(0x10, RegId::HP), op::addi(0x13, 0x13, 1)] => RunResult::Success(vec![0]); "empty blob read_len 1 zero fills")]
#[test_case(1, vec![op::aloc(RegId::ONE), op::move_(0x10, RegId::HP), op::addi(0x13, 0x13, 1)] => RunResult::Success(vec![1, 0]); "blob read_len 1 zero fills")]
#[test_case(1, vec![op::movi(0x15, 2), op::aloc(0x15), op::move_(0x10, RegId::HP), op::addi(0x13, 0x13, 2)] => RunResult::Success(vec![1, 0, 0]); "blob read_len 2 zero fills")]
#[test_case(1, vec![op::movi(0x15, 3), op::aloc(0x15), op::move_(0x10, RegId::HP), op::addi(0x13, 0x13, 3)] => RunResult::Success(vec![1, 0, 0, 0]); "blob read_len 3 zero fills")]
#[test_case(0, vec![op::not(0x13, RegId::ZERO)] => RunResult::Panic(PanicReason::MemoryOverflow); "empty blob read_len Word::MAX")]
#[test_case(1, vec![op::not(0x13, RegId::ZERO)] => RunResult::Panic(PanicReason::MemoryOverflow); "blob read_len Word::MAX")]
#[test_case(1, vec![op::not(0x12, RegId::ZERO), op::not(0x13, RegId::ZERO)] => RunResult::Panic(PanicReason::MemoryOverflow); "both offset and len Word::MAX")]
#[test_case(1, vec![op::movi(0x10, 0), op::not(0x13, RegId::ZERO)] => RunResult::Panic(PanicReason::MemoryOverflow); "spans whole VM memory")]
fn blob_load_data__bounds(
    size: usize,
    modifications: Vec<Instruction>,
) -> RunResult<Vec<u8>> {
    let mut test_context = TestBuilder::new(1234u64);
    test_context.with_free_gas_costs();

    let blob_data = vec![1; size];
    let blob_id = BlobId::compute(&blob_data);

    test_context.setup_blob(blob_data.clone());

    let mut ops = set_full_word(0x13, size as u64);
    ops.extend([
        op::move_(0x14, RegId::HP), // VM_MAX_RAM
        op::gtf_args(0x11, RegId::ZERO, GTFArgs::ScriptData),
        op::aloc(0x13),
        op::move_(0x10, RegId::HP),
    ]);
    ops.extend(modifications);
    ops.extend([
        op::bldd(0x10, 0x11, 0x12, 0x13),
        op::sub(0x14, 0x14, RegId::HP), // Current heap size
        op::logd(RegId::ZERO, RegId::ZERO, RegId::HP, 0x14),
        op::ret(RegId::ONE),
    ]);

    let state = test_context
        .start_script(ops, blob_id.to_bytes())
        .script_gas_limit(1_000_000)
        .fee_input()
        .execute();
    RunResult::extract(state.receipts(), |receipts| {
        receipts
            .iter()
            .filter_map(|receipt| match receipt {
                Receipt::LogData { data, .. } => Some(data.clone().unwrap()),
                _ => None,
            })
            .next()
    })
}
