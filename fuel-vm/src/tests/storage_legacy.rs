use crate::{
    consts::VM_MAX_RAM,
    prelude::*,
    tests::test_helpers::{
        assert_panics,
        assert_success,
        set_full_word,
    },
};
use alloc::{
    vec,
    vec::Vec,
};
use fuel_asm::{
    RegId,
    op,
};

use super::storage::call_contract_once;

// Register aliases for readability
const STATUS: RegId = RegId::new(0x20);
const KEY: RegId = RegId::new(0x21);
const BUF: RegId = RegId::new(0x22);

/// Produces assembly that loads an out-of-bounds pointer into register 0x10.
/// 0x11 is clobbered as a scratch register.
enum MemOobCase {
    /// Key pointer is exactly at the boundary: ptr + 32 = VM_MAX_RAM + 1
    OffByOne,
    /// Key pointer is one step further past the boundary
    Outside,
    /// Key pointer is u64::MAX — adding 32 overflows
    Overflow,
}

fn setup_oob_ptr(case: &MemOobCase) -> Vec<Instruction> {
    let mut ops = set_full_word(0x11, VM_MAX_RAM);
    ops.push(match case {
        MemOobCase::OffByOne => op::addi(0x10, 0x11, 0), // ptr = VM_MAX_RAM
        MemOobCase::Outside => op::addi(0x10, 0x11, 1),  // ptr = VM_MAX_RAM + 1
        MemOobCase::Overflow => op::not(0x10, RegId::ZERO), // ptr = u64::MAX
    });
    ops
}

// ---------------------------------------------------------------------------
// SWW tests
// ---------------------------------------------------------------------------

/// SWW writes the word to the first 8 bytes and zeroes the remaining 24.
/// This tests that when SWW overwrites a slot previously written by SWWQ (which
/// has non-zero data throughout), the trailing bytes are zeroed.
#[test]
fn sww_zeroes_remaining_bytes_on_overwrite() {
    let receipts = call_contract_once(vec![
        // Allocate a 32-byte key (all zeroes)
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        // Allocate a 32-byte source buffer filled with 0xFF words
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        op::not(0x10, RegId::ZERO),
        op::sw(BUF, 0x10, 0),
        op::sw(BUF, 0x10, 1),
        op::sw(BUF, 0x10, 2),
        op::sw(BUF, 0x10, 3),
        // Write the 32-byte buffer to the slot using SWWQ
        op::swwq(KEY, STATUS, BUF, RegId::ONE),
        // Overwrite the slot with a single word (42) using SWW.
        // This must zero the trailing 24 bytes.
        op::movi(0x10, 42),
        op::sww(KEY, STATUS, 0x10),
        // Read back the full 32-byte slot into a fresh buffer
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        op::srwq(BUF, STATUS, KEY, RegId::ONE),
        op::movi(0x15, 32),
        op::logd(RegId::ZERO, RegId::ZERO, BUF, 0x15),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    let mut expected = [0u8; 32];
    expected[7] = 42; // word 42 stored big-endian in first 8 bytes; rest zero

    for r in receipts {
        let Receipt::LogData { data, .. } = r else {
            continue;
        };
        assert_eq!(&**data.as_ref().unwrap(), &expected);
        return;
    }
    panic!("Missing LogData receipt");
}

/// SWW on a new slot returns status 1; SWW on an existing slot returns status 0.
#[test]
fn sww_status_new_vs_existing() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        // First write: new slot → STATUS = 1
        op::movi(0x10, 99),
        op::sww(KEY, STATUS, 0x10),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        // Second write: existing slot → STATUS = 0
        op::movi(0x10, 100),
        op::sww(KEY, STATUS, 0x10),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    let statuses: Vec<u64> = receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .collect();

    assert_eq!(
        statuses,
        vec![1, 0],
        "SWW status: 1 for new, 0 for overwrite"
    );
}

// ---------------------------------------------------------------------------
// SRW tests
// ---------------------------------------------------------------------------

/// SRW correctly reads back a word written by SWW.
#[test]
fn srw_reads_word_written_by_sww() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        op::movi(0x10, 0xABCD),
        op::sww(KEY, STATUS, 0x10),
        op::srw(0x11, STATUS, KEY, 0),
        op::log(0x11, STATUS, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    for r in &receipts {
        let Receipt::Log { ra, rb, .. } = r else {
            continue;
        };
        assert_eq!(*ra, 0xABCD, "SRW should read back written value");
        assert_eq!(*rb, 1, "Status should be 1 (slot exists)");
        return;
    }
    panic!("Missing Log receipt");
}

/// SRW returns 0 and status=0 for a slot that was never written.
#[test]
fn srw_returns_zero_for_nonexistent_slot() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        op::srw(0x11, STATUS, KEY, 0),
        op::log(0x11, STATUS, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    for r in &receipts {
        let Receipt::Log { ra, rb, .. } = r else {
            continue;
        };
        assert_eq!(*ra, 0, "SRW should return 0 for missing slot");
        assert_eq!(*rb, 0, "Status should be 0 (slot not set)");
        return;
    }
    panic!("Missing Log receipt");
}

/// SRW reads words at each valid 8-byte offset within a 32-byte SWWQ slot.
#[test]
fn srw_reads_at_word_offsets() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // Fill buffer: word 0=1, word 1=2, word 2=3, word 3=4
        op::movi(0x10, 1),
        op::sw(BUF, 0x10, 0),
        op::movi(0x10, 2),
        op::sw(BUF, 0x10, 1),
        op::movi(0x10, 3),
        op::sw(BUF, 0x10, 2),
        op::movi(0x10, 4),
        op::sw(BUF, 0x10, 3),
        // Write 32 bytes to storage using SWWQ
        op::swwq(KEY, STATUS, BUF, RegId::ONE),
        // Read at each offset and log
        op::srw(0x10, STATUS, KEY, 0),
        op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::srw(0x10, STATUS, KEY, 1),
        op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::srw(0x10, STATUS, KEY, 2),
        op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::srw(0x10, STATUS, KEY, 3),
        op::log(0x10, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    let values: Vec<u64> = receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .collect();

    assert_eq!(
        values,
        vec![1, 2, 3, 4],
        "SRW should read correct word at each offset"
    );
}

/// SRW panics with StorageOutOfBounds when the slot is too small for the
/// requested offset (slot has 8 bytes; reading at offset 1 needs bytes 8..16).
#[test]
fn srw_panics_when_slot_too_small_for_offset() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        // Write an 8-byte slot via swri
        op::movi(0x15, 8),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        op::swri(KEY, BUF, 8),
        // Try to read at offset 1 (bytes 8..16) — must panic
        op::srw(0x11, STATUS, KEY, 1),
        op::ret(RegId::ONE),
    ]);

    assert_panics(&receipts, PanicReason::StorageOutOfBounds);
}

/// SRW panics with MemoryOverflow when the key pointer is past VM_MAX_RAM.
#[rstest::rstest]
fn srw_key_out_of_memory_panics(
    #[values(MemOobCase::OffByOne, MemOobCase::Outside, MemOobCase::Overflow)]
    case: MemOobCase,
) {
    let mut program = setup_oob_ptr(&case);
    program.extend([op::srw(0x12, STATUS, 0x10, 0), op::ret(RegId::ONE)]);

    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

// ---------------------------------------------------------------------------
// SWWQ tests
// ---------------------------------------------------------------------------

/// SWWQ writes correct data to multiple sequential slots, verified by SRWQ.
#[test]
fn swwq_writes_correct_data_to_multiple_slots() {
    let receipts = call_contract_once(vec![
        // Key (all zeroes = first key)
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        // 64-byte source buffer for two 32-byte slots
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // Slot 0: words 1,2,3,4
        op::movi(0x10, 1),
        op::sw(BUF, 0x10, 0),
        op::movi(0x10, 2),
        op::sw(BUF, 0x10, 1),
        op::movi(0x10, 3),
        op::sw(BUF, 0x10, 2),
        op::movi(0x10, 4),
        op::sw(BUF, 0x10, 3),
        // Slot 1 (at BUF+32): words 5,6,7,8
        op::movi(0x10, 5),
        op::sw(BUF, 0x10, 4),
        op::movi(0x10, 6),
        op::sw(BUF, 0x10, 5),
        op::movi(0x10, 7),
        op::sw(BUF, 0x10, 6),
        op::movi(0x10, 8),
        op::sw(BUF, 0x10, 7),
        // Write both slots
        op::movi(0x10, 2),
        op::swwq(KEY, STATUS, BUF, 0x10),
        // Read back into a fresh 64-byte buffer
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        op::movi(0x10, 2),
        op::srwq(BUF, STATUS, KEY, 0x10),
        op::movi(0x15, 64),
        op::logd(RegId::ZERO, RegId::ZERO, BUF, 0x15),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    let mut expected = [0u8; 64];
    for (i, v) in (1u64..=8).enumerate() {
        expected[i * 8 + 7] = v as u8;
    }

    for r in receipts {
        let Receipt::LogData { data, .. } = r else {
            continue;
        };
        assert_eq!(&**data.as_ref().unwrap(), &expected);
        return;
    }
    panic!("Missing LogData receipt");
}

/// SWWQ status counts newly created slots: 3 on first write, 0 on overwrite.
#[test]
fn swwq_status_counts_new_slots() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        // Source buffer: 3 slots × 32 bytes = 96 bytes
        op::movi(0x15, 96),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // First write: 3 new slots → STATUS = 3
        op::movi(0x10, 3),
        op::swwq(KEY, STATUS, BUF, 0x10),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        // Second write: all 3 already exist → STATUS = 0
        op::movi(0x10, 3),
        op::swwq(KEY, STATUS, BUF, 0x10),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    let statuses: Vec<u64> = receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .collect();

    assert_eq!(statuses, vec![3, 0], "SWWQ status should count new slots");
}

/// SWWQ panics with MemoryOverflow when the key pointer is out of bounds.
#[rstest::rstest]
fn swwq_key_out_of_memory_panics(
    #[values(MemOobCase::OffByOne, MemOobCase::Outside, MemOobCase::Overflow)]
    case: MemOobCase,
) {
    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
    ];
    program.extend(setup_oob_ptr(&case));
    program.extend([
        // 0x10 is the bad key pointer; BUF is source
        op::swwq(0x10, STATUS, BUF, RegId::ONE),
        op::ret(RegId::ONE),
    ]);

    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

/// SWWQ panics with MemoryOverflow when the source data pointer is out of bounds.
#[rstest::rstest]
fn swwq_src_out_of_memory_panics(
    #[values(MemOobCase::OffByOne, MemOobCase::Outside, MemOobCase::Overflow)]
    case: MemOobCase,
) {
    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
    ];
    program.extend(setup_oob_ptr(&case));
    program.extend([
        // KEY is valid; 0x10 is the bad source pointer
        op::swwq(KEY, STATUS, 0x10, RegId::ONE),
        op::ret(RegId::ONE),
    ]);

    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

// ---------------------------------------------------------------------------
// SRWQ tests
// ---------------------------------------------------------------------------

/// SRWQ reads back the exact bytes written by SWWQ.
#[test]
fn srwq_reads_correct_data_from_swwq_slots() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // Slot 0: first 8 bytes = 0xAA
        op::movi(0x10, 0xAA),
        op::sb(BUF, 0x10, 0),
        op::sb(BUF, 0x10, 7),
        // Slot 1 (offset 32): first 8 bytes = 0xBB
        op::movi(0x10, 0xBB),
        op::sb(BUF, 0x10, 32),
        op::sb(BUF, 0x10, 39),
        // Write both slots
        op::movi(0x10, 2),
        op::swwq(KEY, STATUS, BUF, 0x10),
        // Read back into a fresh 64-byte buffer
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        op::movi(0x10, 2),
        op::srwq(BUF, STATUS, KEY, 0x10),
        op::movi(0x15, 64),
        op::logd(RegId::ZERO, RegId::ZERO, BUF, 0x15),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    for r in receipts {
        let Receipt::LogData { data, .. } = r else {
            continue;
        };
        let data = data.as_ref().unwrap();
        assert_eq!(data[0], 0xAA, "slot 0 byte 0");
        assert_eq!(data[7], 0xAA, "slot 0 byte 7");
        assert_eq!(data[32], 0xBB, "slot 1 byte 0");
        assert_eq!(data[39], 0xBB, "slot 1 byte 7");
        // Bytes not explicitly set should be zero
        assert!(data[1..7].iter().all(|&b| b == 0));
        assert!(data[8..32].iter().all(|&b| b == 0));
        assert!(data[33..39].iter().all(|&b| b == 0));
        assert!(data[40..64].iter().all(|&b| b == 0));
        return;
    }
    panic!("Missing LogData receipt");
}

/// SRWQ reads unset slots as zeroes and reports status=0 (not all set).
#[test]
fn srwq_unset_slots_read_as_zeroes_with_false_status() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // Pre-fill BUF with non-zero to confirm it gets overwritten
        op::not(0x10, RegId::ZERO),
        op::sw(BUF, 0x10, 0),
        op::sw(BUF, 0x10, 1),
        op::sw(BUF, 0x10, 2),
        op::sw(BUF, 0x10, 3),
        // Read unset slot
        op::srwq(BUF, STATUS, KEY, RegId::ONE),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::movi(0x15, 32),
        op::logd(RegId::ZERO, RegId::ZERO, BUF, 0x15),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    let mut found_status = false;
    let mut found_data = false;

    for r in &receipts {
        match r {
            Receipt::Log { ra, .. } => {
                assert_eq!(*ra, 0, "SRWQ status should be 0 when any slot is unset");
                found_status = true;
            }
            Receipt::LogData { data, .. } => {
                assert_eq!(
                    &**data.as_ref().unwrap(),
                    &[0u8; 32],
                    "Unset slot should read as zeroes"
                );
                found_data = true;
            }
            _ => {}
        }
    }

    assert!(found_status && found_data);
}

/// SRWQ status is 1 only when ALL slots in the range are set; 0 if any is unset.
#[test]
fn srwq_status_true_only_when_all_slots_set() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // Write only the first of two sequential slots
        op::swwq(KEY, STATUS, BUF, RegId::ONE),
        // Read both slots — second is unset → STATUS = 0
        op::movi(0x10, 2),
        op::srwq(BUF, STATUS, KEY, 0x10),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        // Write both slots
        op::movi(0x10, 2),
        op::swwq(KEY, STATUS, BUF, 0x10),
        // Read both slots — both set → STATUS = 1
        op::movi(0x10, 2),
        op::srwq(BUF, STATUS, KEY, 0x10),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    let statuses: Vec<u64> = receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .collect();

    assert_eq!(
        statuses,
        vec![0, 1],
        "SRWQ status: 0 if any unset, 1 if all set"
    );
}

/// SRWQ panics with MemoryOverflow when the destination buffer is out of bounds.
#[rstest::rstest]
fn srwq_dst_out_of_memory_panics(
    #[values(MemOobCase::OffByOne, MemOobCase::Outside, MemOobCase::Overflow)]
    case: MemOobCase,
) {
    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
    ];
    program.extend(setup_oob_ptr(&case));
    program.extend([
        // 0x10 is the bad destination pointer
        op::srwq(0x10, STATUS, KEY, RegId::ONE),
        op::ret(RegId::ONE),
    ]);

    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

/// SRWQ panics with MemoryOverflow when the key pointer is out of bounds.
#[rstest::rstest]
fn srwq_key_out_of_memory_panics(
    #[values(MemOobCase::OffByOne, MemOobCase::Outside, MemOobCase::Overflow)]
    case: MemOobCase,
) {
    let mut program = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
    ];
    program.extend(setup_oob_ptr(&case));
    program.extend([
        // 0x10 is the bad key pointer
        op::srwq(BUF, STATUS, 0x10, RegId::ONE),
        op::ret(RegId::ONE),
    ]);

    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}

// ---------------------------------------------------------------------------
// SCWQ tests
// ---------------------------------------------------------------------------

/// After SCWQ, the cleared slot is unset: SRW returns 0 with status=0.
#[test]
fn scwq_cleared_slot_is_unset() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        // Write a slot
        op::movi(0x10, 0xFF),
        op::sww(KEY, STATUS, 0x10),
        // Clear it
        op::scwq(KEY, STATUS, RegId::ONE),
        // Read back — should be 0 with status=0
        op::srw(0x11, STATUS, KEY, 0),
        op::log(0x11, STATUS, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    for r in &receipts {
        let Receipt::Log { ra, rb, .. } = r else {
            continue;
        };
        assert_eq!(*ra, 0, "Cleared slot should read as 0");
        assert_eq!(*rb, 0, "Status should be 0 (slot no longer set)");
        return;
    }
    panic!("Missing Log receipt");
}

/// SCWQ with range=N clears exactly N sequential slots. Slots outside the
/// cleared range remain intact.
#[test]
fn scwq_clears_exact_range_of_slots() {
    // Write slots 0..4, clear slots 1..3, then verify slot 0 and slot 4 remain.
    let receipts = call_contract_once(vec![
        // Allocate key buffer (all zeroes = key for slot 0)
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        // Allocate source buffer: 5 slots × 32 bytes = 160 bytes
        op::movi(0x15, 160),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // Write 5 consecutive slots (keys 0..4)
        op::movi(0x10, 5),
        op::swwq(KEY, STATUS, BUF, 0x10),
        // Advance key to slot 1 (last byte = 1)
        op::movi(0x10, 1),
        op::sb(KEY, 0x10, 31),
        // Clear 3 slots starting at key 1 (clears keys 1, 2, 3)
        op::movi(0x10, 3),
        op::scwq(KEY, STATUS, 0x10),
        // Reset key to slot 0 (last byte = 0) and check status
        op::mcli(KEY, 32),
        op::srw(0x11, STATUS, KEY, 0),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO), // slot 0 → should be 1
        // Advance to slot 4 (last byte = 4) and check status
        op::movi(0x10, 4),
        op::sb(KEY, 0x10, 31),
        op::srw(0x11, STATUS, KEY, 0),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO), // slot 4 → should be 1
        // Check slot 2 (should be cleared → status 0)
        op::movi(0x10, 2),
        op::sb(KEY, 0x10, 31),
        op::srw(0x11, STATUS, KEY, 0),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO), // slot 2 → should be 0
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    let statuses: Vec<u64> = receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .collect();

    assert_eq!(
        statuses,
        vec![1, 1, 0],
        "Slots 0 and 4 should remain; slot 2 should be cleared"
    );
}

/// SCWQ status is 1 when all cleared slots were previously set, and 0 when
/// any cleared slot was already unset.
#[test]
fn scwq_status_all_set_vs_any_unset() {
    let receipts = call_contract_once(vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY, RegId::HP),
        // Source buffer: 2 slots × 32 bytes = 64 bytes
        op::movi(0x15, 64),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // Write only slot 0 (not slot 1)
        op::swwq(KEY, STATUS, BUF, RegId::ONE),
        // Try to clear both slots — slot 1 is unset → STATUS = 0
        op::movi(0x10, 2),
        op::scwq(KEY, STATUS, 0x10),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        // Write both slots
        op::movi(0x10, 2),
        op::swwq(KEY, STATUS, BUF, 0x10),
        // Clear both — both were set → STATUS = 1
        op::movi(0x10, 2),
        op::scwq(KEY, STATUS, 0x10),
        op::log(STATUS, RegId::ZERO, RegId::ZERO, RegId::ZERO),
        op::ret(RegId::ONE),
    ]);

    assert_success(&receipts);

    let statuses: Vec<u64> = receipts
        .iter()
        .filter_map(|r| match r {
            Receipt::Log { ra, .. } => Some(*ra),
            _ => None,
        })
        .collect();

    assert_eq!(
        statuses,
        vec![0, 1],
        "SCWQ status: 0 if any was unset, 1 if all were set"
    );
}

/// SCWQ panics with MemoryOverflow when the key pointer is out of bounds.
#[rstest::rstest]
fn scwq_key_out_of_memory_panics(
    #[values(MemOobCase::OffByOne, MemOobCase::Outside, MemOobCase::Overflow)]
    case: MemOobCase,
) {
    let mut program = setup_oob_ptr(&case);
    program.extend([op::scwq(0x10, STATUS, RegId::ONE), op::ret(RegId::ONE)]);

    let receipts = call_contract_once(program);
    assert_panics(&receipts, PanicReason::MemoryOverflow);
}
