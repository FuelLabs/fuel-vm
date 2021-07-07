use fuel_tx::consts::*;
use fuel_tx::*;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

#[test]
fn coin() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let witnesses = vec![Witness::random(rng)];

    Input::coin(
        Bytes32::random(rng),
        Address::random(rng),
        rng.next_u64(),
        Color::random(rng),
        0,
        rng.next_u64(),
        vec![0u8; MAX_PREDICATE_LENGTH as usize],
        vec![],
    )
    .validate(1, &[], witnesses.as_slice())
    .unwrap();

    Input::coin(
        Bytes32::random(rng),
        Address::random(rng),
        rng.next_u64(),
        Color::random(rng),
        0,
        rng.next_u64(),
        vec![],
        vec![0u8; MAX_PREDICATE_DATA_LENGTH as usize],
    )
    .validate(1, &[], witnesses.as_slice())
    .unwrap();

    let err = Input::coin(
        Bytes32::random(rng),
        Address::random(rng),
        rng.next_u64(),
        Color::random(rng),
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
        Bytes32::random(rng),
        Address::random(rng),
        rng.next_u64(),
        Color::random(rng),
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
        Bytes32::random(rng),
        Address::random(rng),
        rng.next_u64(),
        Color::random(rng),
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

    Input::contract(
        Bytes32::random(rng),
        Bytes32::random(rng),
        Bytes32::random(rng),
        ContractId::random(rng),
    )
    .validate(
        1,
        &[Output::contract(
            1,
            Bytes32::random(rng),
            Bytes32::random(rng),
        )],
        &[],
    )
    .unwrap();

    let err = Input::contract(
        Bytes32::random(rng),
        Bytes32::random(rng),
        Bytes32::random(rng),
        ContractId::random(rng),
    )
    .validate(1, &[], &[])
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(
        Bytes32::random(rng),
        Bytes32::random(rng),
        Bytes32::random(rng),
        ContractId::random(rng),
    )
    .validate(
        1,
        &[Output::coin(
            Address::random(rng),
            rng.next_u64(),
            Color::random(rng),
        )],
        &[],
    )
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::InputContractAssociatedOutputContract { index: 1 },
        err
    );

    let err = Input::contract(
        Bytes32::random(rng),
        Bytes32::random(rng),
        Bytes32::random(rng),
        ContractId::random(rng),
    )
    .validate(
        1,
        &[Output::contract(
            2,
            Bytes32::random(rng),
            Bytes32::random(rng),
        )],
        &[],
    )
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::InputContractAssociatedOutputContract { index: 1 },
        err
    );
}
