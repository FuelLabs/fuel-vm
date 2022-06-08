use fuel_tx::*;
use fuel_tx_test_helpers::TransactionFactory;
use fuel_types::bytes::{Deserializable, SerializableVec};
use rand::{rngs::StdRng, Rng, SeedableRng};

#[test]
fn iow_offset() {
    let rng = &mut StdRng::seed_from_u64(8586);

    TransactionFactory::from_seed(3493)
        .take(100)
        .for_each(|(mut tx, _)| {
            let bytes = tx.to_bytes();

            let mut tx_p = tx.clone();
            tx_p.precompute_metadata();

            tx.inputs().iter().enumerate().for_each(|(x, i)| {
                let offset = tx.input_offset(x).unwrap();
                let offset_p = tx_p.input_offset(x).unwrap();

                let input =
                    Input::from_bytes(&bytes[offset..]).expect("Failed to deserialize input!");

                assert_eq!(i, &input);
                assert_eq!(offset, offset_p);
            });

            tx.outputs().iter().enumerate().for_each(|(x, o)| {
                let offset = tx.output_offset(x).unwrap();
                let offset_p = tx_p.output_offset(x).unwrap();

                let output =
                    Output::from_bytes(&bytes[offset..]).expect("Failed to deserialize output!");

                assert_eq!(o, &output);
                assert_eq!(offset, offset_p);
            });

            tx.witnesses().iter().enumerate().for_each(|(x, w)| {
                let offset = tx.witness_offset(x).unwrap();
                let offset_p = tx_p.witness_offset(x).unwrap();

                let witness =
                    Witness::from_bytes(&bytes[offset..]).expect("Failed to deserialize witness!");

                assert_eq!(w, &witness);
                assert_eq!(offset, offset_p);
            });

            tx.receipts_root_offset().map(|offset| {
                let receipts_root = rng.gen();

                tx.set_receipts_root(receipts_root);

                let bytes = tx.to_bytes();
                let receipts_root_p = &bytes[offset..offset + Bytes32::LEN];

                assert_eq!(&receipts_root[..], receipts_root_p);
            });
        });
}
