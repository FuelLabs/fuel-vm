use fuel_tx::bytes::{Deserializable, SerializableVec};
use fuel_tx::*;

use factory::TransactionFactory;

mod factory;

#[test]
fn iow_offset() {
    TransactionFactory::from_seed(3493)
        .take(100)
        .for_each(|mut tx| {
            let bytes = tx.to_bytes();

            tx.inputs().iter().enumerate().for_each(|(x, i)| {
                let offset = tx.input_offset(x).unwrap();
                let input =
                    Input::from_bytes(&bytes[offset..]).expect("Failed to deserialize input!");

                assert_eq!(i, &input);
            });

            tx.outputs().iter().enumerate().for_each(|(x, o)| {
                let offset = tx.output_offset(x).unwrap();
                let output =
                    Output::from_bytes(&bytes[offset..]).expect("Failed to deserialize output!");

                assert_eq!(o, &output);
            });

            tx.witnesses().iter().enumerate().for_each(|(x, w)| {
                let offset = tx.witness_offset(x).unwrap();
                let witness =
                    Witness::from_bytes(&bytes[offset..]).expect("Failed to deserialize witness!");

                assert_eq!(w, &witness);
            });
        });
}
