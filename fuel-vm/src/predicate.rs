//! Predicate representations with required data to be executed during VM runtime

use crate::interpreter::MemoryRange;

use fuel_asm::Word;
use fuel_tx::{field, ConsensusParameters};

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
    pub fn from_tx<T>(params: &ConsensusParameters, tx: &T, idx: usize) -> Option<Self>
    where
        T: field::Inputs,
    {
        tx.inputs_predicate_offset_at(idx)
            .map(|(ofs, len)| (ofs as Word + params.tx_offset() as Word, len as Word))
            .map(|(ofs, len)| MemoryRange::new(ofs, len))
            .map(|program| Self { program, idx })
    }
}

#[test]
fn from_tx_works() {
    use fuel_tx::TransactionBuilder;
    use fuel_types::bytes;
    use fuel_vm::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    use std::iter;

    let rng = &mut StdRng::seed_from_u64(2322u64);

    let params = ConsensusParameters::default();
    let height = 1;

    #[rustfmt::skip]
    let predicate: Vec<u8> = vec![
        Opcode::ADDI(0x10, 0x00, 0x01),
        Opcode::ADDI(0x10, 0x10, 0x01),
        Opcode::RET(0x01),
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
        predicate.clone(),
        predicate_data.clone(),
    );

    let b = Input::message_predicate(
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        rng.gen(),
        vec![],
        predicate.clone(),
        predicate_data,
    );

    let inputs = vec![a, b];

    for i in inputs {
        let tx = TransactionBuilder::script(vec![], vec![])
            .add_input(i)
            .finalize_checked_basic(height, &params);

        // assert invalid idx wont panic
        let idx = 1;
        let runtime = RuntimePredicate::from_tx(&params, tx.as_ref(), idx);

        assert!(runtime.is_none());

        // fetch the input predicate
        let idx = 0;
        let runtime =
            RuntimePredicate::from_tx(&params, tx.as_ref(), idx).expect("failed to generate predicate from valid tx");

        assert_eq!(idx, runtime.idx());

        let mut interpreter = Interpreter::without_storage();

        assert!(interpreter.init_predicate(tx));

        let pad = bytes::padded_len(&predicate) - predicate.len();

        // assert we are testing an edge case
        assert_ne!(0, pad);

        let padded_predicate: Vec<u8> = predicate.iter().copied().chain(iter::repeat(0u8).take(pad)).collect();

        let program = runtime.program();
        let program = &interpreter.memory()[program.start() as usize..program.end() as usize];

        // assert the program in the vm memory is the same of the input
        assert_eq!(program, &padded_predicate);
    }
}
