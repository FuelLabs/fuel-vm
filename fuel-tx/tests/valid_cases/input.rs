use fuel_tx::consts::*;
use fuel_tx::*;
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

#[test]
fn coin() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let witnesses = vec![rng.gen()];

    Input::coin(
        rng.gen(),
        rng.gen(),
        rng.next_u64(),
        rng.gen(),
        0,
        rng.next_u64(),
        vec![0u8; MAX_PREDICATE_LENGTH as usize],
        vec![],
    )
    .validate(1, &[], witnesses.as_slice())
    .unwrap();

    Input::coin(
        rng.gen(),
        rng.gen(),
        rng.next_u64(),
        rng.gen(),
        0,
        rng.next_u64(),
        vec![],
        vec![0u8; MAX_PREDICATE_DATA_LENGTH as usize],
    )
    .validate(1, &[], witnesses.as_slice())
    .unwrap();

    let err = Input::coin(
        rng.gen(),
        rng.gen(),
        rng.next_u64(),
        rng.gen(),
        0,
        rng.next_u64(),
        vec![0u8; MAX_PREDICATE_LENGTH as usize + 1],
        vec![],
    )
    .validate(1, &[], witnesses.as_slice())
    .err()
    .unwrap();
    assert_eq!(ValidationError::InputCoinPredicateLength { index: 1 }, err);

    let err = Input::coin(
        rng.gen(),
        rng.gen(),
        rng.next_u64(),
        rng.gen(),
        0,
        rng.next_u64(),
        vec![],
        vec![0u8; MAX_PREDICATE_DATA_LENGTH as usize + 1],
    )
    .validate(1, &[], witnesses.as_slice())
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::InputCoinPredicateDataLength { index: 1 },
        err
    );

    let err = Input::coin(
        rng.gen(),
        rng.gen(),
        rng.next_u64(),
        rng.gen(),
        1,
        rng.next_u64(),
        vec![],
        vec![],
    )
    .validate(1, &[], witnesses.as_slice())
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::InputCoinWitnessIndexBounds { index: 1 },
        err
    );
}

#[test]
fn contract() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .validate(1, &[Output::contract(1, rng.gen(), rng.gen())], &[])
        .unwrap();

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .validate(1, &[], &[])
        .err()
        .unwrap();
    assert_eq!(
        ValidationError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .validate(
            1,
            &[Output::coin(rng.gen(), rng.next_u64(), rng.gen())],
            &[],
        )
        .err()
        .unwrap();
    assert_eq!(
        ValidationError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())
        .validate(1, &[Output::contract(2, rng.gen(), rng.gen())], &[])
        .err()
        .unwrap();
    assert_eq!(
        ValidationError::InputContractAssociatedOutputContract { index: 1 },
        err
    );
}
