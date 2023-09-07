//! Predicate representations with required data to be executed during VM runtime

use crate::interpreter::MemoryRange;

use fuel_tx::field;

/// Runtime representation of a predicate
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RuntimePredicate {
    program: MemoryRange,
    idx: usize,
}

impl RuntimePredicate {
    /// Memory slice with the program representation of the predicate
    pub const fn program(&self) -> &MemoryRange {
        &self.program
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
        let range = MemoryRange::new(addr, len).expect("Invalid memory range");
        Some(Self {
            program: range,
            idx,
        })
    }
}

#[cfg(feature = "random")]
#[test]
fn from_tx_works() {
    use fuel_asm::op;
    use fuel_tx::TransactionBuilder;
    use fuel_types::bytes;
    use fuel_vm::prelude::*;
    use rand::{
        rngs::StdRng,
        Rng,
        SeedableRng,
    };

    use core::iter;

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
        rng.gen(),
        rng.gen(),
        predicate.clone(),
        predicate_data.clone(),
    );

    let b = Input::message_coin_predicate(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        predicate.clone(),
        predicate_data.clone(),
    );

    let c = Input::message_data_predicate(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        vec![0xff; 10],
        predicate.clone(),
        predicate_data,
    );

    let inputs = vec![a, b, c];

    for i in inputs {
        let tx = TransactionBuilder::script(vec![], vec![])
            .add_input(i)
            .add_random_fee_input()
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

        let mut interpreter = Interpreter::without_storage();

        assert!(interpreter
            .init_predicate(
                fuel_vm::context::Context::PredicateVerification {
                    program: Default::default()
                },
                tx.transaction().clone(),
                tx.transaction().limit()
            )
            .is_ok());

        let pad = bytes::padded_len(&predicate) - predicate.len();

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
