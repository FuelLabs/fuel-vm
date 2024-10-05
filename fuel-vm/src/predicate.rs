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

#[allow(clippy::cast_possible_truncation)]
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
        checked_transaction::{
            CheckPredicateParams,
            EstimatePredicates,
        },
        constraints::reg_key::{
            HP,
            IS,
            ONE,
            SSP,
            ZERO,
        },
        error::PredicateVerificationFailed,
        interpreter::InterpreterParams,
        prelude::{
            predicates::check_predicates,
            *,
        },
        storage::{
            predicate::empty_predicate_storage,
            BlobData,
        },
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
                empty_predicate_storage(),
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

    fn assert_inputs_are_validated_for_predicates(
        inputs: Vec<(
            Vec<Instruction>,
            bool,
            Result<(), PredicateVerificationFailed>,
        )>,
        blob: Vec<Instruction>,
    ) {
        let rng = &mut StdRng::seed_from_u64(2322u64);

        let height = 1.into();
        let predicate_data =
            b"If you think it's simple, then you have misunderstood the problem."
                .to_vec();

        let mut storage = MemoryStorage::new(Default::default(), Default::default());

        let blob_id = BlobId::zeroed();
        let blob: Vec<u8> = blob.into_iter().collect();
        storage
            .storage_as_mut::<BlobData>()
            .insert(&blob_id, &blob)
            .unwrap();

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
                        0,
                        predicate.clone(),
                        predicate_data.clone(),
                    ),
                    Input::message_coin_predicate(
                        rng.gen(),
                        owner,
                        rng.gen(),
                        rng.gen(),
                        0,
                        predicate.clone(),
                        predicate_data.clone(),
                    ),
                    Input::message_data_predicate(
                        rng.gen(),
                        owner,
                        rng.gen(),
                        rng.gen(),
                        0,
                        vec![rng.gen(); rng.gen_range(1..100)],
                        predicate.clone(),
                        predicate_data.clone(),
                    ),
                ]
            }};
        }

        for (i, (input_predicate, correct_gas, expected)) in
            inputs.into_iter().enumerate()
        {
            let input_group = predicate_input!(input_predicate);
            for mut input in input_group {
                if !correct_gas {
                    input.set_predicate_gas_used(1234);
                }

                let mut script = TransactionBuilder::script(
                    [op::ret(0x01)].into_iter().collect(),
                    vec![],
                )
                .add_input(input)
                .add_fee_input()
                .finalize();

                if correct_gas {
                    script
                        .estimate_predicates(
                            &CheckPredicateParams::default(),
                            MemoryInstance::new(),
                            &storage,
                        )
                        .unwrap();
                }

                let tx = script
                    .into_checked_basic(height, &Default::default())
                    .unwrap();

                let result = check_predicates(
                    &tx,
                    &CheckPredicateParams::default(),
                    MemoryInstance::new(),
                    &storage,
                );

                assert_eq!(result.map(|_| ()), expected, "failed at input {}", i);
            }
        }
    }

    /// Verifies the runtime predicate validation rules outlined in the spec are actually
    /// validated https://github.com/FuelLabs/fuel-specs/blob/master/src/fuel-vm/index.md#predicate-verification
    #[test]
    fn inputs_are_validated_for_good_predicate_inputs() {
        const CORRECT_GAS: bool = true;
        let good_blob = vec![op::noop(), op::ret(0x01)];

        let inputs = vec![
            (
                // A valid predicate
                vec![
                    op::addi(0x10, 0x00, 0x01),
                    op::addi(0x10, 0x10, 0x01),
                    op::ret(0x01),
                ],
                CORRECT_GAS,
                Ok(()),
            ),
            (
                // Use `LDC` with mode `1` to load the blob into the predicate.
                vec![
                    // Allocate 32 byte on the heap.
                    op::movi(0x10, 32),
                    op::aloc(0x10),
                    // This will be our zeroed blob id
                    op::move_(0x10, HP),
                    // Store the size of the blob
                    op::bsiz(0x11, 0x10),
                    // Store start of the blob code
                    op::move_(0x12, SSP),
                    // Subtract the start of the code from the end of the code
                    op::sub(0x12, 0x12, IS),
                    // Divide the code by the instruction size to get the number of
                    // instructions
                    op::divi(0x12, 0x12, Instruction::SIZE as u16),
                    // Load the blob by `0x10` ID with the `0x11` size
                    op::ldc(0x10, ZERO, 0x11, 1),
                    // Jump to a new code location
                    op::jmp(0x12),
                ],
                CORRECT_GAS,
                Ok(()),
            ),
            (
                // Use `LDC` with mode `2` to load the part of the predicate from the
                // transaction.
                vec![
                    // Skip the return opcodes. One of two opcodes is a good opcode that
                    // returns `0x1`. This opcode is our source for the `LDC`
                    // opcode. We will copy return good opcode to the end
                    // of the `ssp` via `LDC`. And jump there to
                    // return `true` from the predicate.
                    op::jmpf(ZERO, 2),
                    // Bad return opcode that we want to skip.
                    op::ret(0x0),
                    // Good return opcode that we want to use for the `LDC`.
                    op::ret(0x1),
                    // Take the start of the code and move it for 2 opcodes to get the
                    // desired opcode to copy.
                    op::move_(0x10, IS),
                    // We don't need to copy `jmpf` and bad `ret` opcodes via `LDC`.
                    op::addi(0x10, 0x10, 2 * Instruction::SIZE as u16),
                    // Store end of the code
                    op::move_(0x12, SSP),
                    // Subtract the start of the code from the end of the code
                    op::sub(0x12, 0x12, IS),
                    // Divide the code by the instruction size to get the number of
                    // instructions
                    op::divi(0x12, 0x12, Instruction::SIZE as u16),
                    // We want to load only on good `ret` opcode.
                    op::movi(0x11, Instruction::SIZE as u32),
                    // Load the code from the memory address `0x10` with the `0x11` size
                    op::ldc(0x10, ZERO, 0x11, 2),
                    // Jump to a new code location
                    op::jmp(0x12),
                ],
                CORRECT_GAS,
                Ok(()),
            ),
        ];

        assert_inputs_are_validated_for_predicates(inputs, good_blob)
    }

    #[test]
    fn inputs_are_validated_for_bad_predicate_inputs() {
        const CORRECT_GAS: bool = true;
        const INCORRECT_GAS: bool = false;

        let bad_blob = vec![op::noop(), op::ret(0x00)];

        let inputs = vec![
            (
                // A valid predicate, but gas amount mismatches
                vec![
                    op::addi(0x10, 0x00, 0x01),
                    op::addi(0x10, 0x10, 0x01),
                    op::ret(0x01),
                ],
                INCORRECT_GAS,
                Err(PredicateVerificationFailed::GasMismatch),
            ),
            (
                // Returning an invalid value
                vec![op::ret(0x0)],
                CORRECT_GAS,
                Err(PredicateVerificationFailed::Panic(
                    PanicReason::PredicateReturnedNonOne,
                )),
            ),
            (
                // Using a contract instruction
                vec![op::time(0x20, 0x1), op::ret(0x1)],
                CORRECT_GAS,
                Err(PredicateVerificationFailed::PanicInstruction(
                    PanicInstruction::error(
                        PanicReason::ContractInstructionNotAllowed,
                        op::time(0x20, 0x1).into(),
                    ),
                )),
            ),
            (
                // Using a contract instruction
                vec![op::ldc(ONE, ONE, ONE, 0)],
                CORRECT_GAS,
                Err(PredicateVerificationFailed::PanicInstruction(
                    PanicInstruction::error(
                        PanicReason::ContractInstructionNotAllowed,
                        op::ldc(ONE, ONE, ONE, 0).into(),
                    ),
                )),
            ),
            (
                // Use `LDC` with mode `1` to load the blob into the predicate.
                vec![
                    // Allocate 32 byte on the heap.
                    op::movi(0x10, 32),
                    op::aloc(0x10),
                    // This will be our zeroed blob id
                    op::move_(0x10, HP),
                    // Store the size of the blob
                    op::bsiz(0x11, 0x10),
                    // Store start of the blob code
                    op::move_(0x12, SSP),
                    // Subtract the start of the code from the end of the code
                    op::sub(0x12, 0x12, IS),
                    // Divide the code by the instruction size to get the number of
                    // instructions
                    op::divi(0x12, 0x12, Instruction::SIZE as u16),
                    // Load the blob by `0x10` ID with the `0x11` size
                    op::ldc(0x10, ZERO, 0x11, 1),
                    // Jump to a new code location
                    op::jmp(0x12),
                ],
                CORRECT_GAS,
                Err(PredicateVerificationFailed::Panic(
                    PanicReason::PredicateReturnedNonOne,
                )),
            ),
            (
                // Use `LDC` with mode `2` to load the part of the predicate from the
                // transaction.
                vec![
                    // Skip the return opcodes. One of two opcodes is a bad opcode that
                    // returns `0x0`. This opcode is our source for the `LDC`
                    // opcode. We will copy return bad opcode to the end
                    // of the `ssp` via `LDC`. And jump there to
                    // return `false` from the predicate adn fail.
                    op::jmpf(ZERO, 2),
                    // Good return opcode that we want to skip.
                    op::ret(0x1),
                    // Bad return opcode that we want to use for the `LDC`.
                    op::ret(0x0),
                    // Take the start of the code and move it for 2 opcodes to get the
                    // desired opcode to copy.
                    op::move_(0x10, IS),
                    // We don't need to copy `jmpf` and bad `ret` opcodes via `LDC`.
                    op::addi(0x10, 0x10, 2 * Instruction::SIZE as u16),
                    // Store end of the code
                    op::move_(0x12, SSP),
                    // Subtract the start of the code from the end of the code
                    op::sub(0x12, 0x12, IS),
                    // Divide the code by the instruction size to get the number of
                    // instructions
                    op::divi(0x12, 0x12, Instruction::SIZE as u16),
                    // We want to load only on bad `ret` opcode.
                    op::movi(0x11, Instruction::SIZE as u32),
                    // Load the code from the memory address `0x10` with the `0x11` size
                    op::ldc(0x10, ZERO, 0x11, 2),
                    // Jump to a new code location
                    op::jmp(0x12),
                ],
                CORRECT_GAS,
                Err(PredicateVerificationFailed::Panic(
                    PanicReason::PredicateReturnedNonOne,
                )),
            ),
        ];

        assert_inputs_are_validated_for_predicates(inputs, bad_blob)
    }
}
