use fuel_asm::*;
use fuel_tx::*;
use std::fmt;
use std::io::{self, Read, Write};

pub fn d<T: Default>() -> T {
    Default::default()
}

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
    assert_encoding_correct(&[Witness::from(vec![0xef]), Witness::from(vec![])]);
}

#[test]
fn input() {
    assert_encoding_correct(&[
        Input::coin(
            [0xaa; 32],
            [0xbb; 32],
            Word::MAX,
            [0xcc; 32],
            0xff,
            Word::MAX >> 1,
            vec![0xdd; 50],
            vec![0xee; 23],
        ),
        Input::coin(
            [0xaa; 32],
            [0xbb; 32],
            Word::MAX,
            [0xcc; 32],
            0xff,
            Word::MAX >> 1,
            vec![],
            vec![0xee; 23],
        ),
        Input::coin(
            [0xaa; 32],
            [0xbb; 32],
            Word::MAX,
            [0xcc; 32],
            0xff,
            Word::MAX >> 1,
            vec![0xdd; 50],
            vec![],
        ),
        Input::coin(
            [0xaa; 32],
            [0xbb; 32],
            Word::MAX,
            [0xcc; 32],
            0xff,
            Word::MAX >> 1,
            vec![],
            vec![],
        ),
        Input::contract([0xaa; 32], [0xbb; 32], [0xcc; 32], [0xdd; 32]),
    ]);
}

#[test]
fn output() {
    assert_encoding_correct(&[
        Output::coin([0xaa; 32], Word::MAX >> 1, [0xbb; 32]),
        Output::contract(0xaa, [0xbb; 32], [0xcc; 32]),
        Output::withdrawal([0xaa; 32], Word::MAX >> 1, [0xbb; 32]),
        Output::change([0xaa; 32], Word::MAX >> 1, [0xbb; 32]),
        Output::variable([0xaa; 32], Word::MAX >> 1, [0xbb; 32]),
        Output::contract_created([0xaa; 32]),
    ]);
}

#[test]
fn transaction() {
    let i = Input::contract([0xaa; 32], [0xbb; 32], [0xcc; 32], [0xdd; 32]);
    let o = Output::coin([0xaa; 32], Word::MAX >> 1, [0xbb; 32]);
    let w = Witness::from(vec![0xbf]);

    assert_encoding_correct(&[
        Transaction::script(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            vec![0xfa],
            vec![0xfb, 0xfc],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            vec![],
            vec![0xfb, 0xfc],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            vec![0xfa],
            vec![],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            vec![],
            vec![],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            vec![],
            vec![],
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        Transaction::create(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            0xba,
            [0xdd; 32],
            vec![[0xce; 32]],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            0xba,
            [0xdd; 32],
            vec![],
            vec![i.clone()],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            0xba,
            [0xdd; 32],
            vec![],
            vec![],
            vec![o.clone()],
            vec![w.clone()],
        ),
        Transaction::create(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            0xba,
            [0xdd; 32],
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::create(
            Word::MAX >> 1,
            Word::MAX >> 2,
            Word::MAX >> 3,
            0xba,
            [0xdd; 32],
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    ]);
}

#[test]
fn create_input_coin_data_offset() {
    let gas_price = 100;
    let gas_limit = 1000;
    let maturity = 10;
    let bytecode_witness_index = 0x00;
    let salt = [0xbb; 32];

    let static_contracts: Vec<Vec<ContractAddress>> = vec![vec![], vec![d()], vec![d(), d()]];
    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(d(), d(), d(), d())],
        vec![Input::contract(d(), d(), d(), d()); 2],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(d(), d(), d())],
        vec![Output::contract(d(), d(), d()), Output::withdrawal(d(), d(), d())],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![vec![0xfau8, 0xfb].into()],
        vec![vec![0xbau8, 0xbb].into(), vec![0xff].into()],
    ];

    let predicate = vec![0x50u8, 0x66, 0x70, 0x71];
    let predicate_data = vec![0xa0u8, 0xb0, 0xc0];
    let input_coin = Input::coin(d(), d(), d(), d(), d(), d(), predicate.clone(), predicate_data.clone());

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
                    tx.read(buffer.as_mut_slice()).expect("Failed to serialize input");

                    let offset = tx.input_coin_predicate_offset(offset).expect("Failed to fetch offset");

                    assert_eq!(predicate.as_slice(), &buffer[offset..offset + predicate.len()]);
                }
            }
        }
    }
}

#[test]
fn script_input_coin_data_offset() {
    let gas_price = 100;
    let gas_limit = 1000;
    let maturity = 10;

    let script: Vec<Vec<u8>> = vec![vec![], vec![0xfa, 0xfb]];

    let script_data: Vec<Vec<u8>> = vec![vec![], vec![0xfa, 0xfb]];

    let inputs: Vec<Vec<Input>> = vec![
        vec![],
        vec![Input::contract(d(), d(), d(), d())],
        vec![Input::contract(d(), d(), d(), d()); 2],
    ];
    let outputs: Vec<Vec<Output>> = vec![
        vec![],
        vec![Output::coin(d(), d(), d())],
        vec![Output::contract(d(), d(), d()), Output::withdrawal(d(), d(), d())],
    ];
    let witnesses: Vec<Vec<Witness>> = vec![
        vec![],
        vec![vec![0xfau8, 0xfb].into()],
        vec![vec![0xbau8, 0xbb].into(), vec![0xff].into()],
    ];

    let predicate = vec![0x50u8, 0x66, 0x70, 0x71];
    let predicate_data = vec![0xa0u8, 0xb0, 0xc0];
    let input_coin = Input::coin(d(), d(), d(), d(), d(), d(), predicate.clone(), predicate_data.clone());

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
                        tx.read(buffer.as_mut_slice()).expect("Failed to serialize input");

                        let script_offset = tx.script_offset();
                        assert_eq!(script.as_slice(), &buffer[script_offset..script_offset + script.len()]);

                        let offset = tx.input_coin_predicate_offset(offset).expect("Failed to fetch offset");
                        assert_eq!(predicate.as_slice(), &buffer[offset..offset + predicate.len()]);
                    }
                }
            }
        }
    }
}
