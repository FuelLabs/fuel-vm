use alloc::{
    vec,
    vec::Vec,
};

use fuel_asm::{
    op,
    Instruction,
};
use fuel_crypto::SecretKey;
use fuel_tx::ConsensusParameters;
use fuel_vm::prelude::*;

/// Set a register `r` to a Word-sized number value using left-shifts
pub fn set_full_word(r: RegisterId, v: Word) -> Vec<Instruction> {
    let r = u8::try_from(r).unwrap();
    let mut ops = vec![op::movi(r, 0)];
    for byte in v.to_be_bytes() {
        ops.push(op::ori(r, r, byte as Immediate12));
        ops.push(op::slli(r, r, 8));
    }
    ops.pop().unwrap(); // Remove last shift
    ops
}

/// Run a instructions-only script with reasonable defaults, and return receipts
pub fn run_script(script: Vec<Instruction>) -> Vec<Receipt> {
    use rand::{
        Rng,
        SeedableRng,
    };
    let script = script.into_iter().collect();
    let mut client = MemoryClient::default();
    let arb_max_fee = 1000;

    let consensus_params = ConsensusParameters::standard();

    let mut rng = rand::rngs::StdRng::seed_from_u64(2322u64);
    let tx = TransactionBuilder::script(script, vec![])
        .max_fee_limit(arb_max_fee)
        .script_gas_limit(1_000_000)
        .maturity(Default::default())
        .add_unsigned_coin_input(
            SecretKey::random(&mut rng),
            rng.gen(),
            arb_max_fee,
            *consensus_params.base_asset_id(),
            Default::default(),
        )
        .finalize()
        .into_checked(Default::default(), &consensus_params)
        .expect("failed to generate a checked tx");
    client.transact(tx);
    client.receipts().expect("Expected receipts").to_vec()
}

/// Assert that transaction didn't panic
pub fn assert_success(receipts: &[Receipt]) {
    if let Receipt::ScriptResult { result, .. } = receipts.last().unwrap() {
        if *result != ScriptExecutionResult::Success {
            panic!("Expected vm success, got {result:?} instead");
        }
    } else {
        unreachable!("No script result");
    }
}

/// Assert that transaction receipts end in a panic with the given reason
pub fn assert_panics(receipts: &[Receipt], reason: PanicReason) {
    if let Receipt::ScriptResult { result, .. } = receipts.last().unwrap() {
        if *result != ScriptExecutionResult::Panic {
            panic!("Expected vm panic, got {result:?} instead");
        }
    } else {
        unreachable!("No script result");
    }

    let n = receipts.len();
    assert!(n >= 2, "Invalid receipts len");
    if let Receipt::Panic { reason: pr, .. } = receipts.get(n - 2).unwrap() {
        assert_eq!(
            *pr.reason(),
            reason,
            "Panic reason differs for the expected reason"
        );
    } else {
        unreachable!("No script receipt for a paniced tx");
    }
}
