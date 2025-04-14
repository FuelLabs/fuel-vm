use fuel_asm::{
    RegId,
    op,
};
use fuel_tx::{
    ConsensusParameters,
    TransactionBuilder,
};
use fuel_vm::prelude::*;
use rand::{
    Rng,
    SeedableRng,
    rngs::StdRng,
};

#[cfg(feature = "alloc")]
use alloc::vec;

#[test]
fn cannot_exceed_max_inputs() {
    let rng = &mut StdRng::seed_from_u64(1234);
    let params = ConsensusParameters::default();

    let mut script = TransactionBuilder::script(
        vec![op::ret(RegId::ONE)].into_iter().collect(),
        vec![],
    );
    for _ in 0..=params.tx_params().max_inputs() {
        script.add_input(Input::coin_signed(
            rng.r#gen(),
            rng.r#gen(),
            0,
            rng.r#gen(),
            rng.r#gen(),
            rng.r#gen(),
        ));
    }
    script
        .finalize()
        .into_checked(0u32.into(), &params)
        .expect_err("Tx is invalid and shouldn't validate");
}

#[test]
fn cannot_exceed_max_outputs() {
    let rng = &mut StdRng::seed_from_u64(1234);
    let params = ConsensusParameters::default();

    let mut script = TransactionBuilder::script(
        vec![op::ret(RegId::ONE)].into_iter().collect(),
        vec![],
    );
    for _ in 0..=params.tx_params().max_outputs() {
        script.add_output(Output::variable(rng.r#gen(), rng.r#gen(), rng.r#gen()));
    }
    script
        .finalize()
        .into_checked(0u32.into(), &params)
        .expect_err("Tx is invalid and shouldn't validate");
}

#[test]
fn cannot_exceed_max_witnesses() {
    let rng = &mut StdRng::seed_from_u64(1234);
    let params = ConsensusParameters::default();

    let mut script = TransactionBuilder::script(
        vec![op::ret(RegId::ONE)].into_iter().collect(),
        vec![],
    );
    for _ in 0..=params.tx_params().max_witnesses() {
        script.add_witness(Witness::from(vec![rng.r#gen::<u8>(); 1]));
    }
    script
        .finalize()
        .into_checked(0u32.into(), &params)
        .expect_err("Tx is invalid and shouldn't validate");
}
