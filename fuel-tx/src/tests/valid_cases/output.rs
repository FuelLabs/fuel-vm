use fuel_tx::*;
use rand::{
    rngs::StdRng,
    Rng,
    RngCore,
    SeedableRng,
};

#[test]
fn coin() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    Output::coin(rng.gen(), rng.next_u64(), rng.gen())
        .check(1, &[])
        .unwrap();
}

#[test]
fn contract() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    Output::contract(1, rng.gen(), rng.gen())
        .check(
            2,
            &[
                Input::coin_signed(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                ),
                Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
            ],
        )
        .unwrap();

    let err = Output::contract(0, rng.gen(), rng.gen())
        .check(
            2,
            &[
                Input::coin_signed(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                ),
                Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
            ],
        )
        .err()
        .unwrap();

    assert_eq!(ValidityError::OutputContractInputIndex { index: 2 }, err);

    let err = Output::contract(2, rng.gen(), rng.gen())
        .check(
            2,
            &[
                Input::coin_signed(
                    rng.gen(),
                    rng.gen(),
                    rng.next_u64(),
                    rng.gen(),
                    rng.gen(),
                    rng.gen(),
                ),
                Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen(), rng.gen()),
            ],
        )
        .err()
        .unwrap();

    assert_eq!(ValidityError::OutputContractInputIndex { index: 2 }, err);
}

#[test]
fn change() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    Output::change(rng.gen(), rng.next_u64(), rng.gen())
        .check(1, &[])
        .unwrap();
}

#[test]
fn variable() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    Output::variable(rng.gen(), rng.next_u64(), rng.gen())
        .check(1, &[])
        .unwrap();
}

#[test]
fn contract_created() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    Output::contract_created(rng.gen(), rng.gen())
        .check(1, &[])
        .unwrap();
}
