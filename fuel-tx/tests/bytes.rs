use fuel_data::{bytes, ContractId};
use fuel_tx::*;
use rand::rngs::StdRng;
use rand::{Rng, RngCore, SeedableRng};
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

    assert_encoding_correct(&[rng.gen(), Witness::default()]);
}

#[test]
fn input() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    assert_encoding_correct(&[
        Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
        ),
        Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            vec![],
            rng.gen::<Witness>().into_inner(),
        ),
        Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            vec![],
        ),
        Input::coin(
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            rng.next_u64(),
            vec![],
            vec![],
        ),
        Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
    ]);
}

#[test]
fn output() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    assert_encoding_correct(&[
        Output::coin(rng.gen(), rng.next_u64(), rng.gen()),
        Output::contract(rng.gen(), rng.gen(), rng.gen()),
        Output::withdrawal(rng.gen(), rng.next_u64(), rng.gen()),
        Output::change(rng.gen(), rng.next_u64(), rng.gen()),
        Output::variable(rng.gen(), rng.next_u64(), rng.gen()),
        Output::contract_created(rng.gen()),
    ]);
}

#[test]
fn receipt() {
    let rng = &mut StdRng::seed_from_u64(8586);

    assert_encoding_correct(&[
        Receipt::call(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::ret(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Receipt::return_data(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::revert(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        Receipt::log(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::log_data(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::transfer(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
        Receipt::transfer_out(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
        ),
    ]);
}

#[test]
fn transaction() {
    let mut rng_base = StdRng::seed_from_u64(8586);
    let rng = &mut rng_base;

    let i = Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen());
    let o = Output::coin(rng.gen(), rng.next_u64(), rng.gen());
    let w = rng.gen::<Witness>();

    assert_encoding_correct(&[
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
            rng.gen::<Witness>().into_inner(),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            vec![],
            rng.gen::<Witness>().into_inner(),
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen::<Witness>().into_inner(),
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
            rng.gen(),
            rng.gen(),
            vec![rng.gen()],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::create(
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.gen(),
            rng.gen(),
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
    let salt = rng.gen();

    let static_contracts: Vec<Vec<ContractId>> =
        vec![vec![], vec![rng.gen()], vec![rng.gen(), rng.gen()]];
    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()); 2],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen())],
        vec![
            Output::contract(rng.gen(), rng.gen(), rng.gen()),
            Output::withdrawal(rng.gen(), rng.next_u64(), rng.gen()),
        ],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![vec![], vec![rng.gen()], vec![rng.gen(), rng.gen()]];

    let predicate = rng.gen::<Witness>().into_inner();
    let predicate_data = rng.gen::<Witness>().into_inner();
    let input_coin = Input::coin(
        rng.gen(),
        rng.gen(),
        rng.next_u64(),
        rng.gen(),
        rng.gen(),
        rng.next_u64(),
        predicate.clone(),
        predicate_data.clone(),
    );

    let mut buffer = vec![0u8; 4096];
    for static_contracts in static_contracts.iter() {
        for inputs in inputs.iter() {
            for outputs in outputs.iter() {
                for witnesses in witnesses.iter() {
                    let mut inputs = inputs.clone();
                    let last_input = inputs.len();
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

                    let mut tx_p = tx.clone();
                    tx_p.precompute_metadata();

                    buffer.iter_mut().for_each(|b| *b = 0x00);
                    tx.read(buffer.as_mut_slice())
                        .expect("Failed to serialize input");

                    let offset = tx
                        .input_coin_predicate_offset(last_input)
                        .expect("Failed to fetch offset");

                    let offset_p = tx_p
                        .input_coin_predicate_offset(last_input)
                        .expect("Failed to fetch offset from tx with precomputed metadata!");

                    assert_eq!(offset, offset_p);
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

    let script: Vec<Vec<u8>> = vec![vec![], rng.gen::<Witness>().into_inner()];

    let script_data: Vec<Vec<u8>> = vec![vec![], rng.gen::<Witness>().into_inner()];

    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen())],
        vec![
            Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
            Input::contract(rng.gen(), rng.gen(), rng.gen(), rng.gen()),
        ],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(rng.gen(), rng.next_u64(), rng.gen())],
        vec![
            Output::contract(rng.gen(), rng.gen(), rng.gen()),
            Output::withdrawal(rng.gen(), rng.next_u64(), rng.gen()),
        ],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![vec![], vec![rng.gen()], vec![rng.gen(), rng.gen()]];

    let predicate = rng.gen::<Witness>().into_inner();
    let predicate_data = rng.gen::<Witness>().into_inner();
    let input_coin = Input::coin(
        rng.gen(),
        rng.gen(),
        rng.next_u64(),
        rng.gen(),
        rng.gen(),
        rng.next_u64(),
        predicate.clone(),
        predicate_data.clone(),
    );

    let mut buffer = vec![0u8; 4096];
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

                        let mut tx_p = tx.clone();
                        tx_p.precompute_metadata();

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

                        let script_data_offset_p = tx_p.script_data_offset().expect(
                            "Transaction metadata script data offset failed after precompute!",
                        );

                        assert_eq!(script_data_offset, script_data_offset_p);
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
