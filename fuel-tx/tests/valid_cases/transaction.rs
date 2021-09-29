use fuel_tx::consts::*;
use fuel_tx::*;
use fuel_types::*;
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};

#[test]
fn gas_price() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let maturity = 100;
    let block_height = 1000;

    Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        rng.gen::<Witness>().into_inner(),
        rng.gen::<Witness>().into_inner(),
        vec![],
        vec![],
        vec![],
    )
    .validate(block_height)
    .unwrap();

    Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![vec![0xfau8].into()],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::script(
        MAX_GAS_PER_TX + 1,
        rng.next_u64(),
        maturity,
        rng.gen::<Witness>().into_inner(),
        rng.gen::<Witness>().into_inner(),
        vec![],
        vec![],
        vec![],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionGasLimit, err);

    let err = Transaction::create(
        MAX_GAS_PER_TX + 1,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionGasLimit, err);
}

#[test]
fn maturity() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let block_height = 1000;

    Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        block_height,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .validate(block_height)
    .unwrap();

    Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        1000,
        0,
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![rng.gen()],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        1001,
        vec![],
        vec![],
        vec![],
        vec![],
        vec![],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionMaturity, err);

    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        1001,
        0,
        rng.gen(),
        vec![],
        vec![],
        vec![],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionMaturity, err);
}

#[test]
fn max_iow() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let maturity = 100;
    let block_height = 1000;

    Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        rng.gen::<Witness>().into_inner(),
        rng.gen::<Witness>().into_inner(),
        vec![
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                rng.gen(),
                0,
                rng.next_u64(),
                vec![],
                vec![]
            );
            MAX_INPUTS as usize
        ],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen()); MAX_OUTPUTS as usize],
        vec![rng.gen(); MAX_WITNESSES as usize],
    )
    .validate(block_height)
    .unwrap();

    Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                rng.gen(),
                0,
                rng.next_u64(),
                vec![],
                vec![]
            );
            MAX_INPUTS as usize
        ],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen()); MAX_OUTPUTS as usize],
        vec![rng.gen(); MAX_WITNESSES as usize],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        rng.gen::<Witness>().into_inner(),
        rng.gen::<Witness>().into_inner(),
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()); MAX_INPUTS as usize + 1],
        vec![Output::variable(rng.gen(), rng.next_u64(), rng.gen()); MAX_OUTPUTS as usize],
        vec![rng.gen(); MAX_WITNESSES as usize],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionInputsMax, err);

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        rng.gen::<Witness>().into_inner(),
        rng.gen::<Witness>().into_inner(),
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()); MAX_INPUTS as usize],
        vec![Output::variable(rng.gen(), rng.next_u64(), rng.gen()); MAX_OUTPUTS as usize + 1],
        vec![rng.gen(); MAX_WITNESSES as usize],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionOutputsMax, err);

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        rng.gen::<Witness>().into_inner(),
        rng.gen::<Witness>().into_inner(),
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()); MAX_INPUTS as usize],
        vec![Output::variable(rng.gen(), rng.next_u64(), rng.gen()); MAX_OUTPUTS as usize],
        vec![rng.gen(); MAX_WITNESSES as usize + 1],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionWitnessesMax, err);
}

#[test]
fn output_change_color() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let maturity = 100;
    let block_height = 1000;

    let a = rng.gen();
    let b = rng.gen();
    let c = rng.gen();

    Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        rng.gen::<Witness>().into_inner(),
        rng.gen::<Witness>().into_inner(),
        vec![
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                a,
                0,
                rng.next_u64(),
                rng.gen::<Witness>().into_inner(),
                rng.gen::<Witness>().into_inner(),
            ),
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                b,
                0,
                rng.next_u64(),
                rng.gen::<Witness>().into_inner(),
                rng.gen::<Witness>().into_inner(),
            ),
        ],
        vec![
            Output::change(rng.gen(), rng.next_u64(), a),
            Output::change(rng.gen(), rng.next_u64(), b),
        ],
        vec![rng.gen()],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        rng.gen::<Witness>().into_inner(),
        rng.gen::<Witness>().into_inner(),
        vec![
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                a,
                0,
                rng.next_u64(),
                rng.gen::<Witness>().into_inner(),
                rng.gen::<Witness>().into_inner(),
            ),
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                b,
                0,
                rng.next_u64(),
                rng.gen::<Witness>().into_inner(),
                rng.gen::<Witness>().into_inner(),
            ),
        ],
        vec![
            Output::change(rng.gen(), rng.next_u64(), a),
            Output::change(rng.gen(), rng.next_u64(), a),
        ],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionOutputChangeColorDuplicated, err);

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        rng.gen::<Witness>().into_inner(),
        rng.gen::<Witness>().into_inner(),
        vec![
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                a,
                0,
                rng.next_u64(),
                rng.gen::<Witness>().into_inner(),
                rng.gen::<Witness>().into_inner(),
            ),
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                b,
                0,
                rng.next_u64(),
                rng.gen::<Witness>().into_inner(),
                rng.gen::<Witness>().into_inner(),
            ),
        ],
        vec![
            Output::change(rng.gen(), rng.next_u64(), a),
            Output::change(rng.gen(), rng.next_u64(), c),
        ],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionOutputChangeColorNotFound, err);
}

#[test]
fn script() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let maturity = 100;
    let block_height = 1000;

    let color = rng.gen();
    Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        vec![0xfa; MAX_SCRIPT_LENGTH as usize],
        vec![0xfb; MAX_SCRIPT_DATA_LENGTH as usize],
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            color,
            0,
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
        )],
        vec![Output::change(rng.gen(), rng.next_u64(), color)],
        vec![rng.gen()],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        vec![0xfa; MAX_SCRIPT_LENGTH as usize],
        vec![0xfb; MAX_SCRIPT_DATA_LENGTH as usize],
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            0,
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
        )],
        vec![Output::contract_created(rng.gen())],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::TransactionScriptOutputContractCreated { index: 0 },
        err
    );

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        vec![0xfa; MAX_SCRIPT_LENGTH as usize + 1],
        vec![0xfb; MAX_SCRIPT_DATA_LENGTH as usize],
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            0,
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
        )],
        vec![Output::contract_created(rng.gen())],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionScriptLength, err);

    let color = rng.gen();
    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        vec![0xfa; MAX_SCRIPT_LENGTH as usize],
        vec![0xfb; MAX_SCRIPT_DATA_LENGTH as usize + 1],
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            color,
            0,
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
        )],
        vec![Output::change(rng.gen(), rng.next_u64(), color)],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionScriptDataLength, err);
}

#[test]
fn create() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let maturity = 100;
    let block_height = 1000;

    Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(rng.gen(), rng.next_u64(), Color::default())],
        vec![rng.gen()],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())],
        vec![Output::contract(0, rng.gen(), rng.gen())],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::TransactionCreateInputContract { index: 0 },
        err
    );

    let color = rng.gen();
    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            color,
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::variable(rng.gen(), rng.next_u64(), color)],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::TransactionCreateOutputVariable { index: 0 },
        err
    );

    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                Color::default(),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                rng.gen(),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
        ],
        vec![
            Output::change(rng.gen(), rng.next_u64(), Color::default()),
            Output::change(rng.gen(), rng.next_u64(), Color::default()),
        ],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::TransactionCreateOutputChangeColorZero { index: 1 },
        err
    );

    let color = rng.gen();
    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                Color::default(),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                color,
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
        ],
        vec![
            Output::change(rng.gen(), rng.next_u64(), Color::default()),
            Output::change(rng.gen(), rng.next_u64(), color),
        ],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::TransactionCreateOutputChangeColorNonZero { index: 1 },
        err
    );

    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                Color::default(),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
            Input::coin(
                rng.gen(),
                rng.gen(),
                rng.next_u64(),
                rng.gen(),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
        ],
        vec![
            Output::contract_created(rng.gen()),
            Output::contract_created(rng.gen()),
        ],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::TransactionCreateOutputContractCreatedMultiple { index: 1 },
        err
    );

    Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(rng.gen(), rng.next_u64(), Color::default())],
        vec![vec![0xfau8; CONTRACT_MAX_SIZE as usize / 4].into()],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        vec![],
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(rng.gen(), rng.next_u64(), Color::default())],
        vec![vec![0xfau8; 1 + CONTRACT_MAX_SIZE as usize / 4].into()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionCreateBytecodeLen, err);

    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        1,
        rng.gen(),
        vec![],
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(rng.gen(), rng.next_u64(), Color::default())],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionCreateBytecodeWitnessIndex, err);

    let mut id = ContractId::default();
    let mut static_contracts = (0..MAX_STATIC_CONTRACTS as u64)
        .map(|i| {
            id[..8].copy_from_slice(&i.to_be_bytes());
            id
        })
        .collect::<Vec<ContractId>>();

    Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        static_contracts.clone(),
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(rng.gen(), rng.next_u64(), Color::default())],
        vec![rng.gen()],
    )
    .validate(block_height)
    .unwrap();

    id.iter_mut().for_each(|i| *i = 0xff);
    static_contracts.push(id);
    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        static_contracts.clone(),
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(rng.gen(), rng.next_u64(), Color::default())],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionCreateStaticContractsMax, err);

    static_contracts.pop();
    static_contracts[0][0] = 0xff;
    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        rng.gen(),
        static_contracts,
        vec![Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(rng.gen(), rng.next_u64(), Color::default())],
        vec![rng.gen()],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionCreateStaticContractsOrder, err);
}
