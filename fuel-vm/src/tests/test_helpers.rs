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
#[track_caller]
pub fn assert_success(receipts: &[Receipt]) {
    if let Receipt::ScriptResult { result, .. } = receipts.last().unwrap() {
        if *result != ScriptExecutionResult::Success {
            let Some(Receipt::Panic { reason, .. }) = receipts.get(receipts.len() - 2)
            else {
                panic!("Expected vm success, got {result:?} instead (panic receipt missing!)");
            };

            panic!(
                "Expected vm success, got {result:?} ({:?}) instead",
                reason.reason()
            );
        }
    } else {
        unreachable!("No script result");
    }
}

/// Assert that transaction receipts end in a panic with the given reason
#[track_caller]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunResult<T> {
    Success(T),
    UnableToExtractValue,
    Revert,
    Panic(PanicReason),
    GenericFailure(u64),
}

impl<T> RunResult<T> {
    pub fn is_ok(&self) -> bool {
        matches!(self, RunResult::Success(_))
    }

    pub fn map<F: FnOnce(T) -> R, R>(self, f: F) -> RunResult<R> {
        match self {
            RunResult::Success(v) => RunResult::Success(f(v)),
            RunResult::UnableToExtractValue => RunResult::UnableToExtractValue,
            RunResult::Revert => RunResult::Revert,
            RunResult::Panic(r) => RunResult::Panic(r),
            RunResult::GenericFailure(v) => RunResult::GenericFailure(v),
        }
    }

    /// Extract the value from the receipts, using the provided extractor function
    /// to get even more data about successful runs.
    pub fn extract(
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
    pub fn extract_novalue(receipts: &[Receipt]) -> RunResult<()> {
        Self::extract(receipts, |_| Some(()))
    }
}
