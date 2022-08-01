use fuel_tx::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

#[test]
fn output_message_prepare_init_zeroes_recipient_and_amount() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let message = Output::message(rng.gen(), rng.gen());
    let zeroed = Output::message(Address::zeroed(), 0);

    let tx = TransactionBuilder::script(vec![], vec![])
        .add_output(message)
        .finalize();

    let output = tx
        .clone()
        .prepare_init_predicate()
        .outputs()
        .first()
        .cloned()
        .expect("failed to fetch output");

    let output_p = tx
        .outputs()
        .first()
        .cloned()
        .expect("failed to fetch output");

    assert_ne!(zeroed, message);
    assert_eq!(zeroed, output);
    assert_eq!(message, output_p);
}

#[test]
fn output_variable_prepare_init_zeroes_recipient_and_amount() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let variable = Output::variable(rng.gen(), rng.gen(), rng.gen());
    let zeroed = Output::variable(Address::zeroed(), 0, AssetId::zeroed());

    let tx = TransactionBuilder::script(vec![], vec![])
        .add_output(variable)
        .finalize();

    let output = tx
        .clone()
        .prepare_init_predicate()
        .outputs()
        .first()
        .cloned()
        .expect("failed to fetch output");

    let output_p = tx
        .outputs()
        .first()
        .cloned()
        .expect("failed to fetch output");

    assert_ne!(zeroed, variable);
    assert_eq!(zeroed, output);
    assert_eq!(variable, output_p);
}
