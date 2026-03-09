use crate::{
    prelude::*,
    script_with_data_offset,
    tests::test_helpers::assert_success,
    util::test_helpers::TestBuilder,
};
use alloc::{
    vec,
    vec::Vec,
};
use consensus_parameters::gas::{
    GasCostsValues,
    GasCostsValuesV7,
};
use fuel_asm::{
    RegId,
    op,
};
use fuel_types::canonical::Serialize;

// Register aliases
const KEY1: RegId = RegId::new(0x21);
const KEY2: RegId = RegId::new(0x22);
const BUF: RegId = RegId::new(0x23);
const DST: RegId = RegId::new(0x24);

// Gas cost constants. Cold reads are much more expensive than hot reads so the
// difference is clearly visible in gas_used measurements.
const COLD_BASE: u64 = 100;
const HOT_BASE: u64 = 1;

// Number of bytes written to / read from storage in these tests.
// Must be ≤ 63 so that it fits in an Imm06 for `srdi`.
const SLOT_LEN: u8 = 32;

/// Build a [`GasCosts`] with distinguishable hot vs. cold read costs.
/// All other gas costs are zero (free) so that only storage operations
/// contribute to `gas_used`, making assertions exact.
fn hot_cold_gas_costs() -> GasCosts {
    let v7 = GasCostsValuesV7 {
        storage_read_cold: DependentCost::LightOperation {
            base: COLD_BASE,
            // units_per_gas = u64::MAX means floor(bytes / u64::MAX) = 0 for
            // any realistic slot size, so only the base cost is charged.
            units_per_gas: u64::MAX,
        },
        storage_read_hot: DependentCost::LightOperation {
            base: HOT_BASE,
            units_per_gas: u64::MAX,
        },
        ..GasCostsValuesV7::free()
    };
    GasCosts::new(GasCostsValues::V7(v7))
}

/// Deploy `program` in a fresh contract and execute it once inside a script.
/// Returns all receipts.  The TestBuilder is configured with `gas_costs`.
fn run_contract(program: Vec<Instruction>, gas_costs: GasCosts) -> Vec<Receipt> {
    let mut test_context = TestBuilder::new(2322u64);
    test_context.with_gas_costs(gas_costs);

    let contract_id = test_context.setup_contract(program, None, None).contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS),
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_data = Call::new(contract_id, 0, 0).to_bytes();

    let result = test_context
        .start_script(script, script_data)
        .script_gas_limit(10_000_000)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute();

    result.receipts().to_vec()
}

/// Extract `gas_used` from the final ScriptResult receipt.
fn gas_used(receipts: &[Receipt]) -> u64 {
    let Some(Receipt::ScriptResult { gas_used, .. }) = receipts.last() else {
        panic!("Last receipt must be ScriptResult");
    };
    *gas_used
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

/// Writing to a slot populates the slot cache.  A subsequent read of the same
/// slot should be served from the cache (hot), not from backing storage (cold).
///
/// The test runs two programs that are structurally identical except for which
/// key is used in the final `srdi`:
///
/// * **hot program** – writes K1, then reads K1 → the read is a cache hit.
/// * **cold program** – writes K1, then reads K2 (never accessed) → cold miss.
///
/// With `COLD_BASE = 100` and `HOT_BASE = 1` and all other costs free, the gas
/// accounting works out as:
///
/// | operation              | hot program | cold program |
/// |------------------------|-------------|--------------|
/// | swri K1 (internal read)| 100 (cold)  | 100 (cold)   |
/// | swri K1 (write cost)   | 0 (free)    | 0 (free)     |
/// | srdi (explicit read)   | 1 (hot)     | 100 (cold)   |
/// | **total**              | **101**     | **200**      |
///
/// The gap of 99 equals `COLD_BASE - HOT_BASE`.
#[test]
fn read_after_write_is_hot() {
    // Shared setup: allocate KEY1 (zeros), KEY2 (last byte = 1), and a source
    // data buffer (zeros).  Write SLOT_LEN bytes to K1 to populate the cache.
    // Allocate a destination buffer for the upcoming read.
    let common: Vec<Instruction> = vec![
        // KEY1 – all-zero 32-byte key (slot 0)
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY1, RegId::HP),
        // KEY2 – 32-byte key with last byte = 1 (slot 1, a different key)
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY2, RegId::HP),
        op::movi(0x10, 1),
        op::sb(KEY2, 0x10, 31),
        // BUF – source data buffer (SLOT_LEN zero bytes)
        op::movi(0x15, SLOT_LEN as _),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // Write SLOT_LEN bytes of BUF to slot K1.
        // Internally this calls storage_read_slot(K1) → cold miss (charges
        // COLD_BASE), then writes the slot and inserts K1 into the cache.
        op::swri(KEY1, BUF, SLOT_LEN as _),
        // DST – destination buffer for the read
        op::movi(0x15, SLOT_LEN as _),
        op::aloc(0x15),
        op::move_(DST, RegId::HP),
    ];

    // Hot program: read K1 → cache hit → hot gas.
    let hot_program: Vec<Instruction> = common
        .iter()
        .cloned()
        .chain([
            op::srdi(DST, KEY1, RegId::ZERO, SLOT_LEN),
            op::ret(RegId::ONE),
        ])
        .collect();

    // Cold program: read K2 (never written) → cache miss → cold gas.
    let cold_program: Vec<Instruction> = common
        .into_iter()
        .chain([
            op::srdi(DST, KEY2, RegId::ZERO, SLOT_LEN),
            op::ret(RegId::ONE),
        ])
        .collect();

    let gas_costs = hot_cold_gas_costs();

    let receipts_hot = run_contract(hot_program, gas_costs.clone());
    let receipts_cold = run_contract(cold_program, gas_costs);

    assert_success(&receipts_hot);
    assert_success(&receipts_cold);

    let hot = gas_used(&receipts_hot);
    let cold = gas_used(&receipts_cold);

    assert!(
        cold > hot,
        "Cold read ({cold}) should cost more gas than hot read ({hot})"
    );
    assert_eq!(
        cold - hot,
        COLD_BASE - HOT_BASE,
        "Gas difference should be exactly COLD_BASE - HOT_BASE"
    );
}

/// The hot/cold cache is keyed by `(ContractId, key)`.  Reading the same key
/// through two different contracts results in two independent cold misses —
/// contract A warming its cache entry does not warm contract B's.
///
/// | operation          | gas              |
/// |--------------------|------------------|
/// | contract A: srdi K | 100 (cold)       |
/// | contract B: srdi K | 100 (cold, miss) |
/// | **total**          | **200**          |
///
/// Without isolation (shared flat cache) the total would be 101.
#[test]
fn cache_not_shared_across_contracts() {
    // A minimal contract that reads SLOT_LEN bytes from the all-zeros key.
    // The slot is empty so ERR=1, but cold gas is charged regardless.
    let read_program: Vec<Instruction> = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY1, RegId::HP),
        op::movi(0x15, SLOT_LEN as _),
        op::aloc(0x15),
        op::move_(DST, RegId::HP),
        op::srdi(DST, KEY1, RegId::ZERO, SLOT_LEN),
        op::ret(RegId::ONE),
    ];

    let gas_costs = hot_cold_gas_costs();
    let mut test_context = TestBuilder::new(2322u64);
    test_context.with_gas_costs(gas_costs);

    let contract_a = test_context
        .setup_contract(read_program.clone(), None, None)
        .contract_id;
    let contract_b = test_context
        .setup_contract(read_program, None, None)
        .contract_id;

    // Call data is 48 bytes: ContractId (32) + param1 (8) + param2 (8).
    let call_size = Call::new(contract_a, 0, 0).to_bytes().len() as u32;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS), // → contract A
            op::movi(0x10, (data_offset + call_size) as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS), // → contract B
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let mut script_data = Call::new(contract_a, 0, 0).to_bytes();
    script_data.extend(Call::new(contract_b, 0, 0).to_bytes());

    let receipts = test_context
        .start_script(script, script_data)
        .script_gas_limit(10_000_000)
        .contract_input(contract_a)
        .contract_input(contract_b)
        .fee_input()
        .contract_output(&contract_a)
        .contract_output(&contract_b)
        .execute()
        .receipts()
        .to_vec();

    assert_success(&receipts);

    // Each contract has its own independent cache entry, so both reads are cold.
    assert_eq!(
        gas_used(&receipts),
        2 * COLD_BASE,
        "Two different contracts reading the same key should both pay cold gas"
    );
}

/// SCWQ's pre-read loop warms the slot cache, so a read that immediately
/// follows a clear is served from cache (hot) rather than from backing storage
/// (cold).
///
/// Both programs: SWWQ K1 (write 1 slot), SCWQ K1 (clear 1 slot), then read.
///
/// * **hot program** – reads K1 after clearing → cache holds `None` → hot
/// * **cold program** – reads K2 (never accessed) → cold miss
///
/// | operation                       | hot program | cold program |
/// |---------------------------------|-------------|--------------|
/// | swwq K1 pre-read (cold miss)    | 100         | 100          |
/// | scwq K1 pre-read (cache hit)    |   1         |   1          |
/// | srdi (K1 hot / K2 cold)         |   1         | 100          |
/// | **total**                       | **102**     | **201**      |
///
/// If SCWQ's pre-read loop did not populate the cache, the read after the
/// clear would be a cold miss and the totals would both be 201.
#[test]
fn scwq_clears_populate_cache() {
    const STATUS: RegId = RegId::new(0x25);

    // Common prefix: allocate K1 (zeros), K2 (last byte = 1), a 32-byte write
    // buffer (zeros), write 1 slot to K1, then clear it via SCWQ.
    let common: Vec<Instruction> = vec![
        // KEY1 – all-zero 32-byte key
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY1, RegId::HP),
        // KEY2 – 32-byte key with last byte = 1 (a distinct, untouched key)
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY2, RegId::HP),
        op::movi(0x10, 1),
        op::sb(KEY2, 0x10, 31),
        // BUF – 32-byte source buffer (all zeros)
        op::movi(0x15, SLOT_LEN as _),
        op::aloc(0x15),
        op::move_(BUF, RegId::HP),
        // SWWQ: write 1 slot to K1.
        //   → pre-read K1: cold miss, charges COLD_BASE; warms cache.
        //   → internal storage_slot_len_no_gas: cache hit, no gas.
        op::swwq(KEY1, STATUS, BUF, RegId::ONE),
        // SCWQ: clear 1 slot starting at K1.
        //   → pre-read K1: cache hit, charges HOT_BASE.
        //   → storage_clear_slot_range: sets cache[K1] = None.
        op::scwq(KEY1, STATUS, RegId::ONE),
        // DST – destination buffer for the upcoming read
        op::movi(0x15, SLOT_LEN as _),
        op::aloc(0x15),
        op::move_(DST, RegId::HP),
    ];

    // Hot: read K1 — in cache as None after the clear → HOT_BASE.
    let hot_program: Vec<Instruction> = common
        .iter()
        .cloned()
        .chain([
            op::srdi(DST, KEY1, RegId::ZERO, SLOT_LEN),
            op::ret(RegId::ONE),
        ])
        .collect();

    // Cold: read K2 — never accessed, cold miss → COLD_BASE.
    let cold_program: Vec<Instruction> = common
        .into_iter()
        .chain([
            op::srdi(DST, KEY2, RegId::ZERO, SLOT_LEN),
            op::ret(RegId::ONE),
        ])
        .collect();

    let gas_costs = hot_cold_gas_costs();
    let receipts_hot = run_contract(hot_program, gas_costs.clone());
    let receipts_cold = run_contract(cold_program, gas_costs);

    assert_success(&receipts_hot);
    assert_success(&receipts_cold);

    let hot = gas_used(&receipts_hot);
    let cold = gas_used(&receipts_cold);

    assert_eq!(
        cold - hot,
        COLD_BASE - HOT_BASE,
        "After SCWQ, reading a cleared slot should be a hot cache hit"
    );
}

/// The slot cache persists across the call–return boundary within a single
/// transaction.  Calling the same contract twice and reading the same key in
/// each call: the first call is a cold miss, the second is a hot hit.
///
/// | operation      | gas        |
/// |----------------|------------|
/// | call 1: srdi K | 100 (cold) |
/// | call 2: srdi K | 1   (hot)  |
/// | **total**      | **101**    |
///
/// If the cache were cleared on return the total would be 200.
#[test]
fn cache_persists_across_calls() {
    let read_program: Vec<Instruction> = vec![
        op::movi(0x15, 32),
        op::aloc(0x15),
        op::move_(KEY1, RegId::HP),
        op::movi(0x15, SLOT_LEN as _),
        op::aloc(0x15),
        op::move_(DST, RegId::HP),
        op::srdi(DST, KEY1, RegId::ZERO, SLOT_LEN),
        op::ret(RegId::ONE),
    ];

    let gas_costs = hot_cold_gas_costs();
    let mut test_context = TestBuilder::new(2322u64);
    test_context.with_gas_costs(gas_costs);

    let contract_id = test_context
        .setup_contract(read_program, None, None)
        .contract_id;

    let (script, _) = script_with_data_offset!(
        data_offset,
        vec![
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS), // first call: cold miss
            op::movi(0x10, data_offset as Immediate18),
            op::call(0x10, RegId::ZERO, 0x10, RegId::CGAS), // second call: hot hit
            op::ret(RegId::ONE),
        ],
        test_context.get_tx_params().tx_offset()
    );
    let script_data = Call::new(contract_id, 0, 0).to_bytes();

    let receipts = test_context
        .start_script(script, script_data)
        .script_gas_limit(10_000_000)
        .contract_input(contract_id)
        .fee_input()
        .contract_output(&contract_id)
        .execute()
        .receipts()
        .to_vec();

    assert_success(&receipts);

    // First call: cold (100). Second call: hot (1). Total: 101.
    assert_eq!(
        gas_used(&receipts),
        COLD_BASE + HOT_BASE,
        "Cache should persist across the call-return boundary"
    );
}
