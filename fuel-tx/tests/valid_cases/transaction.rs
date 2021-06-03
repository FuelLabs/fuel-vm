use fuel_tx::consts::*;
use fuel_tx::*;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

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
        Witness::random(rng).into_inner(),
        Witness::random(rng).into_inner(),
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
        Salt::random(rng),
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
        Witness::random(rng).into_inner(),
        Witness::random(rng).into_inner(),
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
        Salt::random(rng),
        vec![],
        vec![],
        vec![],
        vec![Witness::random(rng)],
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
        Salt::random(rng),
        vec![],
        vec![],
        vec![],
        vec![Witness::random(rng)],
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
        Salt::random(rng),
        vec![],
        vec![],
        vec![],
        vec![Witness::random(rng)],
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
        Witness::random(rng).into_inner(),
        Witness::random(rng).into_inner(),
        vec![
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                Color::random(rng),
                0,
                rng.next_u64(),
                vec![],
                vec![]
            );
            MAX_INPUTS as usize
        ],
        vec![Output::coin(Address::random(rng), rng.next_u64(), Color::random(rng)); MAX_OUTPUTS as usize],
        vec![Witness::random(rng); MAX_WITNESSES as usize],
    )
    .validate(block_height)
    .unwrap();

    Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        Salt::random(rng),
        vec![],
        vec![
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                Color::random(rng),
                0,
                rng.next_u64(),
                vec![],
                vec![]
            );
            MAX_INPUTS as usize
        ],
        vec![Output::coin(Address::random(rng), rng.next_u64(), Color::random(rng)); MAX_OUTPUTS as usize],
        vec![Witness::random(rng); MAX_WITNESSES as usize],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        Witness::random(rng).into_inner(),
        Witness::random(rng).into_inner(),
        vec![
            Input::contract(
                Hash::random(rng),
                Hash::random(rng),
                Hash::random(rng),
                ContractAddress::random(rng)
            );
            MAX_INPUTS as usize + 1
        ],
        vec![Output::variable(Address::random(rng), rng.next_u64(), Color::random(rng)); MAX_OUTPUTS as usize],
        vec![Witness::random(rng); MAX_WITNESSES as usize],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionInputsMax, err);

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        Witness::random(rng).into_inner(),
        Witness::random(rng).into_inner(),
        vec![
            Input::contract(
                Hash::random(rng),
                Hash::random(rng),
                Hash::random(rng),
                ContractAddress::random(rng)
            );
            MAX_INPUTS as usize
        ],
        vec![Output::variable(Address::random(rng), rng.next_u64(), Color::random(rng)); MAX_OUTPUTS as usize + 1],
        vec![Witness::random(rng); MAX_WITNESSES as usize],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionOutputsMax, err);

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        Witness::random(rng).into_inner(),
        Witness::random(rng).into_inner(),
        vec![
            Input::contract(
                Hash::random(rng),
                Hash::random(rng),
                Hash::random(rng),
                ContractAddress::random(rng)
            );
            MAX_INPUTS as usize
        ],
        vec![Output::variable(Address::random(rng), rng.next_u64(), Color::random(rng)); MAX_OUTPUTS as usize],
        vec![Witness::random(rng); MAX_WITNESSES as usize + 1],
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

    let a = Color::random(rng);
    let b = Color::random(rng);
    let c = Color::random(rng);

    Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        Witness::random(rng).into_inner(),
        Witness::random(rng).into_inner(),
        vec![
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                a,
                0,
                rng.next_u64(),
                Witness::random(rng).into_inner(),
                Witness::random(rng).into_inner(),
            ),
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                b,
                0,
                rng.next_u64(),
                Witness::random(rng).into_inner(),
                Witness::random(rng).into_inner(),
            ),
        ],
        vec![
            Output::change(Address::random(rng), rng.next_u64(), a),
            Output::change(Address::random(rng), rng.next_u64(), b),
        ],
        vec![Witness::random(rng)],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        Witness::random(rng).into_inner(),
        Witness::random(rng).into_inner(),
        vec![
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                a,
                0,
                rng.next_u64(),
                Witness::random(rng).into_inner(),
                Witness::random(rng).into_inner(),
            ),
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                b,
                0,
                rng.next_u64(),
                Witness::random(rng).into_inner(),
                Witness::random(rng).into_inner(),
            ),
        ],
        vec![
            Output::change(Address::random(rng), rng.next_u64(), a),
            Output::change(Address::random(rng), rng.next_u64(), a),
        ],
        vec![Witness::random(rng)],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionOutputChangeColorDuplicated, err);

    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        Witness::random(rng).into_inner(),
        Witness::random(rng).into_inner(),
        vec![
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                a,
                0,
                rng.next_u64(),
                Witness::random(rng).into_inner(),
                Witness::random(rng).into_inner(),
            ),
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                b,
                0,
                rng.next_u64(),
                Witness::random(rng).into_inner(),
                Witness::random(rng).into_inner(),
            ),
        ],
        vec![
            Output::change(Address::random(rng), rng.next_u64(), a),
            Output::change(Address::random(rng), rng.next_u64(), c),
        ],
        vec![Witness::random(rng)],
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

    let color = Color::random(rng);
    Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        vec![0xfa; MAX_SCRIPT_LENGTH as usize],
        vec![0xfb; MAX_SCRIPT_DATA_LENGTH as usize],
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            color,
            0,
            rng.next_u64(),
            Witness::random(rng).into_inner(),
            Witness::random(rng).into_inner(),
        )],
        vec![Output::change(Address::random(rng), rng.next_u64(), color)],
        vec![Witness::random(rng)],
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
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::random(rng),
            0,
            rng.next_u64(),
            Witness::random(rng).into_inner(),
            Witness::random(rng).into_inner(),
        )],
        vec![Output::contract_created(ContractAddress::random(rng))],
        vec![Witness::random(rng)],
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
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::random(rng),
            0,
            rng.next_u64(),
            Witness::random(rng).into_inner(),
            Witness::random(rng).into_inner(),
        )],
        vec![Output::contract_created(ContractAddress::random(rng))],
        vec![Witness::random(rng)],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionScriptLength, err);

    let color = Color::random(rng);
    let err = Transaction::script(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        vec![0xfa; MAX_SCRIPT_LENGTH as usize],
        vec![0xfb; MAX_SCRIPT_DATA_LENGTH as usize + 1],
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            color,
            0,
            rng.next_u64(),
            Witness::random(rng).into_inner(),
            Witness::random(rng).into_inner(),
        )],
        vec![Output::change(Address::random(rng), rng.next_u64(), color)],
        vec![Witness::random(rng)],
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
        Salt::random(rng),
        vec![],
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(Address::random(rng), rng.next_u64(), Color::default())],
        vec![Witness::random(rng)],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        Salt::random(rng),
        vec![],
        vec![Input::contract(
            Hash::random(rng),
            Hash::random(rng),
            Hash::random(rng),
            ContractAddress::random(rng),
        )],
        vec![Output::contract(0, Hash::random(rng), Hash::random(rng))],
        vec![Witness::random(rng)],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionCreateInputContract { index: 0 }, err);

    let color = Color::random(rng);
    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        Salt::random(rng),
        vec![],
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            color,
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::variable(Address::random(rng), rng.next_u64(), color)],
        vec![Witness::random(rng)],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionCreateOutputVariable { index: 0 }, err);

    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        Salt::random(rng),
        vec![],
        vec![
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                Color::default(),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                Color::random(rng),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
        ],
        vec![
            Output::change(Address::random(rng), rng.next_u64(), Color::default()),
            Output::change(Address::random(rng), rng.next_u64(), Color::default()),
        ],
        vec![Witness::random(rng)],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(
        ValidationError::TransactionCreateOutputChangeColorZero { index: 1 },
        err
    );

    let color = Color::random(rng);
    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        Salt::random(rng),
        vec![],
        vec![
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                Color::default(),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                color,
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
        ],
        vec![
            Output::change(Address::random(rng), rng.next_u64(), Color::default()),
            Output::change(Address::random(rng), rng.next_u64(), color),
        ],
        vec![Witness::random(rng)],
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
        Salt::random(rng),
        vec![],
        vec![
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                Color::default(),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
            Input::coin(
                Hash::random(rng),
                Address::random(rng),
                rng.next_u64(),
                Color::random(rng),
                0,
                rng.next_u64(),
                vec![],
                vec![],
            ),
        ],
        vec![
            Output::contract_created(ContractAddress::random(rng)),
            Output::contract_created(ContractAddress::random(rng)),
        ],
        vec![Witness::random(rng)],
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
        Salt::random(rng),
        vec![],
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(Address::random(rng), rng.next_u64(), Color::default())],
        vec![vec![0xfau8; CONTRACT_MAX_SIZE as usize / 4].into()],
    )
    .validate(block_height)
    .unwrap();

    let err = Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        Salt::random(rng),
        vec![],
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(Address::random(rng), rng.next_u64(), Color::default())],
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
        Salt::random(rng),
        vec![],
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(Address::random(rng), rng.next_u64(), Color::default())],
        vec![Witness::random(rng)],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionCreateBytecodeWitnessIndex, err);

    let mut id = ContractAddress::default();
    let mut static_contracts = (0..MAX_STATIC_CONTRACTS as u64)
        .map(|i| {
            id[..8].copy_from_slice(&i.to_be_bytes());
            id
        })
        .collect::<Vec<ContractAddress>>();

    Transaction::create(
        MAX_GAS_PER_TX,
        rng.next_u64(),
        maturity,
        0,
        Salt::random(rng),
        static_contracts.clone(),
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(Address::random(rng), rng.next_u64(), Color::default())],
        vec![Witness::random(rng)],
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
        Salt::random(rng),
        static_contracts.clone(),
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(Address::random(rng), rng.next_u64(), Color::default())],
        vec![Witness::random(rng)],
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
        Salt::random(rng),
        static_contracts,
        vec![Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::default(),
            0,
            rng.next_u64(),
            vec![],
            vec![],
        )],
        vec![Output::change(Address::random(rng), rng.next_u64(), Color::default())],
        vec![Witness::random(rng)],
    )
    .validate(block_height)
    .err()
    .unwrap();
    assert_eq!(ValidationError::TransactionCreateStaticContractsOrder, err);
}
