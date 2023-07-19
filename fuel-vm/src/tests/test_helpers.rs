use fuel_asm::{
    op,
    Instruction,
};
use fuel_tx::TxParameters;
use fuel_types::ChainId;
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
    let script = script.into_iter().collect();
    let mut client = MemoryClient::default();

    let tx_params = TxParameters::default();
    let predicate_params = PredicateParameters::default();
    let script_params = ScriptParameters::default();
    let contract_params = ContractParameters::default();
    let fee_params = FeeParameters::default();
    let chain_id = ChainId::default();
    let gas_costs = GasCosts::default();

    let tx = TransactionBuilder::script(script, vec![])
        .gas_price(0)
        .gas_limit(1_000_000)
        .maturity(Default::default())
        .add_random_fee_input()
        .finalize()
        .into_checked(
            Default::default(),
            &tx_params,
            &predicate_params,
            &script_params,
            &contract_params,
            &fee_params,
            chain_id,
            gas_costs.clone(),
        )
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
