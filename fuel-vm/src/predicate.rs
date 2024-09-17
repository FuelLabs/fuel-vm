//! Predicate representations with required data to be executed during VM runtime

use fuel_tx::field;

use crate::interpreter::MemoryRange;

/// Runtime representation of a predicate
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RuntimePredicate {
    range: MemoryRange,
    idx: usize,
}

impl RuntimePredicate {
    /// Empty predicate for testing
    #[cfg(test)]
    pub const fn empty() -> Self {
        Self {
            range: MemoryRange::new(0, 0),
            idx: 0,
        }
    }

    /// Memory slice with the program representation of the predicate
    pub const fn program(&self) -> &MemoryRange {
        &self.range
    }

    /// Index of the transaction input that maps to this predicate
    pub const fn idx(&self) -> usize {
        self.idx
    }

    /// Create a new runtime predicate from a transaction, given the input index
    ///
    /// Return `None` if the tx input doesn't map to an input with a predicate
    pub fn from_tx<T>(tx: &T, tx_offset: usize, idx: usize) -> Option<Self>
    where
        T: field::Inputs,
    {
        let (ofs, len) = tx.inputs_predicate_offset_at(idx)?;
        let addr = ofs.saturating_add(tx_offset);
        Some(Self {
            range: MemoryRange::new(addr, len),
            idx,
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::{
        vec,
        vec::Vec,
    };
    use core::iter;
    use fuel_asm::op;
    use fuel_tx::{
        field::ScriptGasLimit,
        TransactionBuilder,
    };
    use fuel_types::bytes;
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
    };

    use crate::{
        checked_transaction::CheckPredicateParams,
        error::PredicateVerificationFailed,
        interpreter::InterpreterParams,
        prelude::*,
        storage::PredicateStorage,
    };

    #[test]
    fn from_tx_works() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let height = 1.into();

        #[rustfmt::skip]
        let predicate: Vec<u8> = vec![
            op::addi(0x10, 0x00, 0x01),
            op::addi(0x10, 0x10, 0x01),
            op::ret(0x01),
        ].into_iter().collect();

        let predicate_data = b"If people do not believe that mathematics is simple, it is only because they do not realize how complicated life is.".to_vec();

        let owner = (*Contract::root_from_code(&predicate)).into();
        let a = Input::coin_predicate(
            rng.gen(),
            owner,
            rng.gen(),
            rng.gen(),
            rng.gen(),
            0,
            predicate.clone(),
            predicate_data.clone(),
        );

        let b = Input::message_coin_predicate(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            0,
            predicate.clone(),
            predicate_data.clone(),
        );

        let c = Input::message_data_predicate(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            0,
            vec![0xff; 10],
            predicate.clone(),
            predicate_data,
        );

        let inputs = vec![a, b, c];

        for i in inputs {
            let tx = TransactionBuilder::script(vec![], vec![])
                .add_input(i)
                .add_fee_input()
                .finalize_checked_basic(height);

            // assert invalid idx wont panic
            let idx = 1;
            let tx_offset = TxParameters::DEFAULT.tx_offset();
            let runtime = RuntimePredicate::from_tx(tx.as_ref(), tx_offset, idx);

            assert!(runtime.is_none());

            // fetch the input predicate
            let idx = 0;
            let runtime = RuntimePredicate::from_tx(tx.as_ref(), tx_offset, idx)
                .expect("failed to generate predicate from valid tx");

            assert_eq!(idx, runtime.idx());

            let mut interpreter = Interpreter::<_, _, _>::with_storage(
                MemoryInstance::new(),
                PredicateStorage,
                InterpreterParams::default(),
            );

            assert!(interpreter
                .init_predicate(
                    Context::PredicateVerification {
                        program: RuntimePredicate::empty(),
                    },
                    tx.transaction().clone(),
                    *tx.transaction().script_gas_limit(),
                )
                .is_ok());

            let pad = bytes::padded_len(&predicate).unwrap() - predicate.len();

            // assert we are testing an edge case
            assert_ne!(0, pad);

            let padded_predicate: Vec<u8> = predicate
                .iter()
                .copied()
                .chain(iter::repeat(0u8).take(pad))
                .collect();

            let program = runtime.program();
            let program = &interpreter.memory()[program.usizes()];

            // assert the program in the vm memory is the same of the input
            assert_eq!(program, &padded_predicate);
        }
    }

    /// Verifies the runtime predicate validation rules outlined in the spec are actually
    /// validated https://github.com/FuelLabs/fuel-specs/blob/master/src/fuel-vm/index.md#predicate-verification
    #[test]
    fn inputs_are_validated() {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let height = 1.into();
        let predicate_data =
            b"If you think it's simple, then you have misunderstood the problem."
                .to_vec();

        macro_rules! predicate_input {
            ($predicate:expr) => {{
                let predicate: Vec<u8> = $predicate.into_iter().collect();
                let owner = Input::predicate_owner(&predicate);
                [
                    Input::coin_predicate(
                        rng.gen(),
                        owner,
                        rng.gen(),
                        rng.gen(),
                        rng.gen(),
                        15,
                        predicate.clone(),
                        predicate_data.clone(),
                    ),
                    Input::message_coin_predicate(
                        rng.gen(),
                        owner,
                        rng.gen(),
                        rng.gen(),
                        15,
                        predicate.clone(),
                        predicate_data.clone(),
                    ),
                    Input::message_data_predicate(
                        rng.gen(),
                        owner,
                        rng.gen(),
                        rng.gen(),
                        15,
                        vec![rng.gen(); rng.gen_range(1..100)],
                        predicate.clone(),
                        predicate_data.clone(),
                    ),
                ]
            }};
        }

        let inputs = vec![
            (
                // A valid predicate
                predicate_input!(vec![
                    op::addi(0x10, 0x00, 0x01),
                    op::addi(0x10, 0x10, 0x01),
                    op::ret(0x01),
                ]),
                Ok(()),
            ),
            (
                // A valid predicate, but gas amount mismatches
                predicate_input!(vec![op::ret(0x01),]),
                Err(PredicateVerificationFailed::GasMismatch),
            ),
            (
                // Returning an invalid value
                predicate_input!(vec![op::ret(0x0)]),
                Err(PredicateVerificationFailed::Panic(
                    PanicReason::PredicateReturnedNonOne,
                )),
            ),
            (
                // Using a contract instruction
                predicate_input!(vec![op::time(0x20, 0x1), op::ret(0x1)]),
                Err(PredicateVerificationFailed::PanicInstruction(
                    PanicInstruction::error(
                        PanicReason::ContractInstructionNotAllowed,
                        op::time(0x20, 0x1).into(),
                    ),
                )),
            ),
            (
                // PC exceeding predicate bounds
                predicate_input!(vec![op::ji(0x100), op::ret(0x1)]),
                Err(PredicateVerificationFailed::Panic(
                    PanicReason::MemoryOverflow,
                )),
            ),
        ];

        for (input_group, expected) in inputs {
            for input in input_group {
                let tx = TransactionBuilder::script(
                    [op::ret(0x01)].into_iter().collect(),
                    vec![],
                )
                .add_input(input)
                .add_fee_input()
                .finalize_checked_basic(height);

                let result = Interpreter::check_predicates(
                    &tx,
                    &CheckPredicateParams::default(),
                    MemoryInstance::new(),
                );

                assert_eq!(result.map(|_| ()), expected);
            }
        }
    }
}
