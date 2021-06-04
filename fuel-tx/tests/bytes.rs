use fuel_tx::*;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::fmt;
use std::io::{self, Read, Write};

pub fn assert_encoding_correct<T>(data: &[T])
where
    T: Read
        + Write
        + fmt::Debug
        + Clone
        + PartialEq
        + bytes::SizedBytes
        + bytes::SerializableVec
        + bytes::Deserializable,
{
    let mut buffer;

    for data in data.iter() {
        let mut d = data.clone();

        let d_bytes = data.clone().to_bytes();
        let d_p = T::from_bytes(d_bytes.as_slice()).expect("Failed to deserialize T");
        assert_eq!(d, d_p);

        let mut d_p = data.clone();

        buffer = vec![0u8; 1024];
        let read_size = d.read(buffer.as_mut_slice()).expect("Failed to read");
        let write_size = d_p.write(buffer.as_slice()).expect("Failed to write");

        // Simple RW assertion
        assert_eq!(d, d_p);
        assert_eq!(read_size, write_size);

        buffer = vec![0u8; read_size];

        // Minimum size buffer assertion
        d.read(buffer.as_mut_slice()).expect("Failed to read");
        d_p.write(buffer.as_slice()).expect("Failed to write");
        assert_eq!(d, d_p);
        assert_eq!(d_bytes.as_slice(), buffer.as_slice());

        // No panic assertion
        loop {
            buffer.pop();

            let err = d
                .read(buffer.as_mut_slice())
                .err()
                .expect("Insufficient buffer should fail!");
            assert_eq!(io::ErrorKind::UnexpectedEof, err.kind());

            let err = d_p
                .write(buffer.as_slice())
                .err()
                .expect("Insufficient buffer should fail!");
            assert_eq!(io::ErrorKind::UnexpectedEof, err.kind());

            if buffer.is_empty() {
                break;
            }
        }
    }
}

#[test]
fn witness() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    assert_encoding_correct(&[Witness::random(rng), Witness::default()]);
}

#[test]
fn input() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    assert_encoding_correct(&[
        Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::random(rng),
            rng.next_u32().to_be_bytes()[0],
            rng.next_u64(),
            Witness::random(rng).into_inner(),
            Witness::random(rng).into_inner(),
        ),
        Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::random(rng),
            rng.next_u32().to_be_bytes()[0],
            rng.next_u64(),
            vec![],
            Witness::random(rng).into_inner(),
        ),
        Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::random(rng),
            rng.next_u32().to_be_bytes()[0],
            rng.next_u64(),
            Witness::random(rng).into_inner(),
            vec![],
        ),
        Input::coin(
            Hash::random(rng),
            Address::random(rng),
            rng.next_u64(),
            Color::random(rng),
            rng.next_u32().to_be_bytes()[0],
            rng.next_u64(),
            vec![],
            vec![],
        ),
        Input::contract(
            Hash::random(rng),
            Hash::random(rng),
            Hash::random(rng),
            ContractAddress::random(rng),
        ),
    ]);
}

#[test]
fn output() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    assert_encoding_correct(&[
        Output::coin(Address::random(rng), rng.next_u64(), Color::random(rng)),
        Output::contract(
            rng.next_u32().to_be_bytes()[0],
            Hash::random(rng),
            Hash::random(rng),
        ),
        Output::withdrawal(Address::random(rng), rng.next_u64(), Color::random(rng)),
        Output::change(Address::random(rng), rng.next_u64(), Color::random(rng)),
        Output::variable(Address::random(rng), rng.next_u64(), Color::random(rng)),
        Output::contract_created(ContractAddress::random(rng)),
    ]);
}

#[test]
fn transaction() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let i = Input::contract(
        Hash::random(rng),
        Hash::random(rng),
        Hash::random(rng),
        ContractAddress::random(rng),
    );
    let o = Output::coin(Address::random(rng), rng.next_u64(), Color::random(rng));
    let w = Witness::random(rng);

    assert_encoding_correct(&[
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            Witness::random(rng).into_inner(),
            Witness::random(rng).into_inner(),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            Witness::random(rng).into_inner(),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            Witness::random(rng).into_inner(),
            vec![],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        Transaction::create(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u32().to_be_bytes()[0],
            Salt::random(rng),
            vec![ContractAddress::random(rng)],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u32().to_be_bytes()[0],
            Salt::random(rng),
            vec![],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u32().to_be_bytes()[0],
            Salt::random(rng),
            vec![],
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u32().to_be_bytes()[0],
            Salt::random(rng),
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u32().to_be_bytes()[0],
            Salt::random(rng),
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    ]);
}

#[test]
fn create_input_coin_data_offset() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let gas_price = 100;
    let gas_limit = 1000;
    let maturity = 10;
    let bytecode_witness_index = 0x00;
    let salt = Salt::random(rng);

    let static_contracts: Vec<Vec<ContractAddress>> = vec![
        vec![],
        vec![ContractAddress::random(rng)],
        vec![ContractAddress::random(rng), ContractAddress::random(rng)],
    ];
    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(
            Hash::random(rng),
            Hash::random(rng),
            Hash::random(rng),
            ContractAddress::random(rng),
        )],
        vec![
            Input::contract(
                Hash::random(rng),
                Hash::random(rng),
                Hash::random(rng),
                ContractAddress::random(rng)
            );
            2
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(
            Address::random(rng),
            rng.next_u64(),
            Color::random(rng),
        )],
        vec![
            Output::contract(
                rng.next_u32().to_be_bytes()[0],
                Hash::random(rng),
                Hash::random(rng),
            ),
            Output::withdrawal(Address::random(rng), rng.next_u64(), Color::random(rng)),
        ],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![Witness::random(rng)],
        vec![Witness::random(rng), Witness::random(rng)],
    ];

    let predicate = Witness::random(rng).into_inner();
    let predicate_data = Witness::random(rng).into_inner();
    let input_coin = Input::coin(
        Hash::random(rng),
        Address::random(rng),
        rng.next_u64(),
        Color::random(rng),
        rng.next_u32().to_be_bytes()[0],
        rng.next_u64(),
        predicate.clone(),
        predicate_data.clone(),
    );

    let mut buffer = vec![0u8; 1024];
    for static_contracts in static_contracts.iter() {
        for inputs in inputs.iter() {
            for outputs in outputs.iter() {
                for witnesses in witnesses.iter() {
                    let mut inputs = inputs.clone();
                    let offset = inputs.len();
                    inputs.push(input_coin.clone());

                    let mut tx = Transaction::create(
                        gas_price,
                        gas_limit,
                        maturity,
                        bytecode_witness_index,
                        salt,
                        static_contracts.clone(),
                        inputs,
                        outputs.clone(),
                        witnesses.clone(),
                    );

                    buffer.iter_mut().for_each(|b| *b = 0x00);
                    tx.read(buffer.as_mut_slice())
                        .expect("Failed to serialize input");

                    let offset = tx
                        .input_coin_predicate_offset(offset)
                        .expect("Failed to fetch offset");

                    assert_eq!(
                        predicate.as_slice(),
                        &buffer[offset..offset + predicate.len()]
                    );
                }
            }
        }
    }
}

#[test]
fn script_input_coin_data_offset() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let gas_price = 100;
    let gas_limit = 1000;
    let maturity = 10;

    let script: Vec<Vec<u8>> = vec![vec![], Witness::random(rng).into_inner()];

    let script_data: Vec<Vec<u8>> = vec![vec![], Witness::random(rng).into_inner()];

    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(
            Hash::random(rng),
            Hash::random(rng),
            Hash::random(rng),
            ContractAddress::random(rng),
        )],
        vec![
            Input::contract(
                Hash::random(rng),
                Hash::random(rng),
                Hash::random(rng),
                ContractAddress::random(rng),
            ),
            Input::contract(
                Hash::random(rng),
                Hash::random(rng),
                Hash::random(rng),
                ContractAddress::random(rng),
            ),
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(
            Address::random(rng),
            rng.next_u64(),
            Color::random(rng),
        )],
        vec![
            Output::contract(
                rng.next_u32().to_be_bytes()[0],
                Hash::random(rng),
                Hash::random(rng),
            ),
            Output::withdrawal(Address::random(rng), rng.next_u64(), Color::random(rng)),
        ],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![Witness::random(rng)],
        vec![Witness::random(rng), Witness::random(rng)],
    ];

    let predicate = Witness::random(rng).into_inner();
    let predicate_data = Witness::random(rng).into_inner();
    let input_coin = Input::coin(
        Hash::random(rng),
        Address::random(rng),
        rng.next_u64(),
        Color::random(rng),
        rng.next_u32().to_be_bytes()[0],
        rng.next_u64(),
        predicate.clone(),
        predicate_data.clone(),
    );

    let mut buffer = vec![0u8; 1024];
    for script in script.iter() {
        for script_data in script_data.iter() {
            for inputs in inputs.iter() {
                for outputs in outputs.iter() {
                    for witnesses in witnesses.iter() {
                        let mut inputs = inputs.clone();
                        let offset = inputs.len();
                        inputs.push(input_coin.clone());

                        let mut tx = Transaction::script(
                            gas_price,
                            gas_limit,
                            maturity,
                            script.clone(),
                            script_data.clone(),
                            inputs,
                            outputs.clone(),
                            witnesses.clone(),
                        );

                        buffer.iter_mut().for_each(|b| *b = 0x00);
                        tx.read(buffer.as_mut_slice())
                            .expect("Failed to serialize input");

                        let script_offset = Transaction::script_offset();
                        assert_eq!(
                            script.as_slice(),
                            &buffer[script_offset..script_offset + script.len()]
                        );

                        let script_data_offset = tx
                            .script_data_offset()
                            .expect("Transaction is Script and should return data offset");
                        assert_eq!(
                            script_data.as_slice(),
                            &buffer[script_data_offset..script_data_offset + script_data.len()]
                        );

                        let offset = tx
                            .input_coin_predicate_offset(offset)
                            .expect("Failed to fetch offset");
                        assert_eq!(
                            predicate.as_slice(),
                            &buffer[offset..offset + predicate.len()]
                        );
                    }
                }
            }
        }
    }
}
