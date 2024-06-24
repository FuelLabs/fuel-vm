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

use super::test_helpers::assert_success;
use crate::tests::test_helpers::set_full_word;

#[derive(Debug, PartialEq)]
enum RunResult<T> {
    Success(T),
    UnableToExtractValue,
    Revert,
    Panic(PanicReason),
    GenericFailure(u64),
}

impl<T> RunResult<T> {
    fn is_ok(&self) -> bool {
        matches!(self, RunResult::Success(_))
    }

    fn map<F: FnOnce(T) -> R, R>(self, f: F) -> RunResult<R> {
        match self {
            RunResult::Success(v) => RunResult::Success(f(v)),
            RunResult::UnableToExtractValue => RunResult::UnableToExtractValue,
            RunResult::Revert => RunResult::Revert,
            RunResult::Panic(r) => RunResult::Panic(r),
            RunResult::GenericFailure(v) => RunResult::GenericFailure(v),
        }
    }

    fn extract(
        receipts: &[Receipt],
        value_extractor: fn(&[Receipt]) -> Option<T>,
    ) -> RunResult<T> {
        let Receipt::ScriptResult { result, .. } = receipts.last().unwrap() else {
            unreachable!("No script result");
        };

        match *result {
            ScriptExecutionResult::Success => match value_extractor(receipts) {
                Some(v) => RunResult::Success(v),
                None => RunResult::UnableToExtractValue,
            },
            ScriptExecutionResult::Revert => RunResult::Revert,
            ScriptExecutionResult::Panic => RunResult::Panic({
                let Receipt::Panic { reason, .. } = receipts[receipts.len() - 2] else {
                    unreachable!("No panic receipt");
                };
                *reason.reason()
            }),
            ScriptExecutionResult::GenericFailure(value) => {
                RunResult::GenericFailure(value)
            }
        }
    }
}

impl RunResult<()> {
    fn extract_novalue(receipts: &[Receipt]) -> RunResult<()> {
        Self::extract(receipts, |_| Some(()))
    }
}

#[rstest::rstest]
fn blob_size_and_load_whole(#[values(0, 1, 2, 7, 8, 9, 1024, 1234, 9876)] size: usize) {
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

    let state = test_context
        .start_script(ops, blob_id.to_bytes())
        .script_gas_limit(1_000_000)
        .fee_input()
        .execute();
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

#[test]
fn blob_load_code_and_jump_happypath() {
    let mut test_context = TestBuilder::new(1234u64);

    let canary = test_context.rng.gen();

    let mut blob_code = set_full_word(0x10, canary);
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
        op::bldc(0x10, 0x11, 0x12),
        op::sub(0x10, 0x13, RegId::IS), // Compute offset
        op::divi(0x10, 0x10, 4),        // Div for jmp instruction
        op::jmp(0x10),                  // Jump to loaded code
    ]);

    let state = test_context
        .start_script(ops, blob_id.to_bytes())
        .script_gas_limit(1_000_000)
        .fee_input()
        .execute();

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

#[test]
fn blob_load_code_preserves_stack_values() {
    let mut test_context = TestBuilder::new(1234u64);

    let canary1: u64 = test_context.rng.gen();
    let canary2: u64 = test_context.rng.gen();

    let blob_data: Vec<u8> = canary1.to_be_bytes().into_iter().collect();

    let blob_id = BlobId::compute(&blob_data);
    test_context.setup_blob(blob_data.clone());

    let mut ops = set_full_word(0x12, blob_data.len() as u64);
    ops.extend(set_full_word(0x13, canary2));
    ops.extend([
        // Store canary to stack
        op::cfei(8),
        op::sw(RegId::SSP, 0x13, 0),
        op::move_(0x14, RegId::SSP),
        // Load more code
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::bldc(0x10, 0x11, 0x12),
        // Log the canary
        op::movi(0x15, 8),
        op::logd(RegId::ZERO, RegId::ZERO, 0x14, 0x15),
        op::ret(RegId::ONE),
    ]);

    let state = test_context
        .start_script(ops, blob_id.to_bytes())
        .script_gas_limit(1_000_000)
        .fee_input()
        .execute();

    assert_success(state.receipts());
    let extracted = state
        .receipts()
        .iter()
        .filter_map(|receipt| match receipt {
            Receipt::LogData { data, .. } => Some(data.clone().unwrap()),
            _ => None,
        })
        .next()
        .expect("Missing logdata receipt");
    let extracted = Word::from_be_bytes(extracted.try_into().unwrap());
    assert_eq!(extracted, canary2);
}

#[test_case(0, vec![] => RunResult::Success(vec![]); "load empty blob")]
#[test_case(1, vec![] => RunResult::Success(vec![0]); "load blob of size 1")]
#[test_case(2, vec![] => RunResult::Success(vec![0, 1]); "load blob of size 2")]
#[test_case(2, vec![op::movi(0x12, 4)] => RunResult::Success(vec![0, 1, 0, 0]); "load past end zero fills")]
#[test_case(1, vec![op::movi(0x10, 0)] => RunResult::Panic(PanicReason::BlobNotFound) ; "no such blob")]
#[test_case(1, set_full_word(0x10, VM_MAX_RAM - 30) => RunResult::Panic(PanicReason::MemoryOverflow) ; "blob id ends outside ram")]
#[test_case(1, set_full_word(0x10, VM_MAX_RAM) => RunResult::Panic(PanicReason::MemoryOverflow) ; "blob id starts outside ram")]
#[test_case(1, set_full_word(0x10, Word::MAX - 32) => RunResult::Panic(PanicReason::MemoryOverflow) ; "blob id ends Word::MAX")]
#[test_case(1, set_full_word(0x10, Word::MAX) => RunResult::Panic(PanicReason::MemoryOverflow) ; "blob id starts at Word::MAX")]
#[test_case(4, vec![op::movi(0x11, 1)] => RunResult::Success(vec![1, 2, 3, 0]); "offset 1")]
#[test_case(4, vec![op::movi(0x11, 1), op::movi(0x12, 2)] => RunResult::Success(vec![1, 2]); "offset 1 len 2")]
#[test_case(2, set_full_word(0x11, Word::MAX) => RunResult::Success(vec![0, 0]); "offset Word::MAX")]
#[test_case(4, vec![op::sub(0x14, RegId::HP, RegId::SP), op::aloc(0x14)] => RunResult::Panic(PanicReason::MemoryGrowthOverlap) ; "would overlap heap")]
#[test_case(4, vec![op::sub(0x14, RegId::HP, RegId::SP),op::subi(0x14, 0x14, 4), op::aloc(0x14)] => RunResult::Success(vec![0,1,2,3]) ; "barely fits")]
fn blob_load_code_bounds(
    size: usize,
    modifications: Vec<Instruction>,
) -> RunResult<Vec<u8>> {
    let mut test_context = TestBuilder::new(1234u64);

    let blob_data: Vec<u8> = (0..size).map(|v| v as u8).collect();

    let blob_id = BlobId::compute(&blob_data);
    test_context.setup_blob(blob_data.clone());

    let mut ops = set_full_word(0x12, blob_data.len() as u64);
    ops.extend([
        op::gtf_args(0x10, RegId::ZERO, GTFArgs::ScriptData),
        op::move_(0x20, RegId::SP),
    ]);
    ops.extend(modifications);
    ops.extend([
        op::bldc(0x10, 0x11, 0x12),
        op::logd(RegId::ZERO, RegId::ZERO, 0x20, 0x12),
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
