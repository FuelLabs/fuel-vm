use crate::{
    consts::VM_MAX_RAM,
    interpreter::BlobIdExt,
    prelude::*,
};
use alloc::vec;
use fuel_asm::{
    op,
    RegId,
};
use fuel_types::{
    canonical::Serialize,
    BlobId,
};
use rand::{
    Rng,
    RngCore,
};
use test_case::test_case;

use super::test_helpers::{
    assert_success,
    RunResult,
};
use crate::tests::test_helpers::set_full_word;

#[rstest::rstest]
fn blob_size_and_load_whole(
    #[values(0, 1, 2, 7, 8, 9, 1024, 1234, 9876)] size: usize,
    #[values(true, false)] external: bool,
) {
    let mut test_context = TestBuilder::new(1234u64);

    let mut blob_data = vec![0; size];
    test_context.rng.fill_bytes(&mut blob_data);
    let blob_id = BlobId::compute(&blob_data);

    test_context.setup_blob(blob_data.clone());

    let mut ops = set_full_word(0x12, size as u64);
    ops.extend([
        op::gtf_args(0x11, RegId::ZERO, GTFArgs::ScriptData),
        op::bsiz(0x10, 0x11),
        op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::aloc(0x10),
        op::bldd(RegId::HP, 0x11, RegId::ZERO, 0x10),
        op::logd(RegId::ZERO, RegId::ZERO, RegId::HP, 0x12),
        op::ret(RegId::ONE),
    ]);

    let mut script_data = blob_id.to_bytes();
    let state = if external {
        test_context
            .start_script(ops, script_data)
            .script_gas_limit(1_000_000)
            .fee_input()
            .execute()
    } else {
        let contract_to_call = test_context.setup_contract(ops, None, None).contract_id;

        script_data.extend(Call::new(contract_to_call, 0, 0).to_bytes());
        test_context
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
            .execute()
    };

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

    let bldd = receipts
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::LogData { data, .. } => Some(data.clone().unwrap()),
            _ => None,
        })
        .next()
        .expect("Missing logdata receipt");
    assert_eq!(bldd, blob_data);
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
fn blob_size_bounds(size: usize, blob_id_ptr: Word) -> RunResult<()> {
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
fn blob_load_data_bounds(
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

#[test_case(true; "script")]
#[test_case(false; "contract")]
fn blob_load_and_jump_happypath(external: bool) {
    let mut test_context = TestBuilder::new(1234u64);

    let canary = test_context.rng.gen();
    let asset_id = test_context.rng.gen();
    let initial_internal_balance = canary;

    let mut blob_code = if external {
        // In scripts, just log the canary and return
        set_full_word(0x10, canary)
    } else {
        // In contract contexts, log the balance (=canary) to make sure the blob shares
        // contract context
        vec![
            op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
            op::addi(0x10, 0x10, (BlobId::LEN + Call::LEN).try_into().unwrap()),
            op::bal(0x10, 0x10, RegId::FP),
        ]
    };
    blob_code.extend([
        op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);
    let blob_data: Vec<u8> = blob_code.into_iter().collect();

    let blob_id = BlobId::compute(&blob_data);
    test_context.setup_blob(blob_data.clone());

    let mut ops = set_full_word(0x12, blob_data.len() as u64);
    ops.extend([
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::move_(0x13, RegId::SSP), // Store jump target
        op::move_(0x14, RegId::SP),  // Store load target
        op::cfe(0x12),
        op::bldd(0x14, 0x10, RegId::ZERO, 0x12),
        op::sub(0x10, 0x13, RegId::IS), // Compute offset
        op::divi(0x10, 0x10, 4),        // Div for jmp instruction
        op::jmp(0x10),                  // Jump to loaded code
    ]);

    let mut script_data = blob_id.to_bytes();
    let state = if external {
        test_context
            .start_script(ops, script_data)
            .script_gas_limit(1_000_000)
            .fee_input()
            .execute()
    } else {
        let contract_to_call = test_context
            .setup_contract(ops, Some((asset_id, initial_internal_balance)), None)
            .contract_id;

        script_data.extend(Call::new(contract_to_call, 0, 0).to_bytes());
        script_data.extend(asset_id.to_bytes());
        test_context
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
            .execute()
    };

    assert_success(state.receipts());
    let extracted = state
        .receipts()
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .next()
        .expect("Missing log receipt");
    assert_eq!(extracted, canary);
}

/// Sequentially load multiple blobs.
#[test]
fn blob_load_multiple() {
    let mut test_context = TestBuilder::new(1234u64);

    let num_blobs = 10;

    let blob_ids: Vec<BlobId> = (0..num_blobs)
        .map(|i| {
            let mut blob_code = vec![
                op::movi(0x10, i),
                op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
            ];
            if i == 9 {
                blob_code.push(op::ret(RegId::ONE));
            }
            let blob_data: Vec<u8> = blob_code.into_iter().collect();
            let blob_id = BlobId::compute(&blob_data);
            test_context.setup_blob(blob_data.clone());
            blob_id
        })
        .collect();

    let ops = vec![
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::move_(0x13, RegId::SSP), // Store jump target
        op::movi(0x14, num_blobs),   // Loop counter
        // jnzb loop start
        op::bsiz(0x12, 0x10),                    // Get size
        op::move_(0x15, RegId::SP),              // Store $sp to target to
        op::cfe(0x12),                           // Extend stack
        op::bldd(0x15, 0x10, RegId::ZERO, 0x12), // Load full blob code
        op::addi(0x10, 0x10, 32),                // Next blob id pointer
        op::subi(0x14, 0x14, 1),                 // Decrement loop counter
        op::jnzb(0x14, RegId::ZERO, 5),          // Loop
        op::sub(0x10, 0x13, RegId::IS),          // Compute offset
        op::divi(0x10, 0x10, 4),                 // Div for jmp instruction
        op::jmp(0x10),                           // Jump to loaded code
    ];

    let state = test_context
        .start_script(
            ops,
            blob_ids.into_iter().flat_map(|b| b.to_bytes()).collect(),
        )
        .script_gas_limit(1_000_000)
        .fee_input()
        .execute();

    dbg!(state.receipts());

    assert_success(state.receipts());
    let extracted: Vec<Word> = state
        .receipts()
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .collect();
    assert_eq!(extracted, (0..num_blobs as Word).collect::<Vec<Word>>());
}
