use alloc::{
    vec,
    vec::Vec,
};

use crate::{
    consts::*,
    prelude::*,
};
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};

use core::fmt;
use fuel_tx::policies::Policies;
use fuel_types::{
    canonical::{
        Deserialize,
        Serialize,
    },
    Word,
};

pub fn assert_encoding_correct<T>(data: &[T])
where
    T: Serialize + Deserialize + fmt::Debug + Clone + PartialEq,
{
    let mut buffer;

    for data in data.iter() {
        buffer = vec![0u8; 1024];

        data.encode(&mut &mut buffer[..]).expect("Failed to encode");
        let data_decoded = T::decode(&mut &buffer[..]).expect("Failed to decode");
        assert_eq!(data, &data_decoded);
        assert_eq!(data.size(), data_decoded.size());

        let counted_bytes = {
            let mut v = Vec::new();
            data.encode(&mut v).expect("Failed to encode");
            v.len()
        };

        // Test that insufficine buffer size fails and that partial decoding fails
        buffer.truncate(counted_bytes);
        while buffer.pop().is_some() {
            data.encode(&mut buffer.as_mut_slice())
                .expect_err("Encoding should fail");
            T::decode(&mut &buffer[..]).expect_err("Decoding should fail");
        }
    }
}

/// The function tests that the encoding of the `Transaction` and the specific `T` variant
/// of the transaction are correct.
pub fn assert_transactions_encoding_correct<T>(data: &[T])
where
    T: Serialize + Deserialize + Into<Transaction> + fmt::Debug + Clone + PartialEq,
{
    assert_encoding_correct(data);

    let txs = data
        .iter()
        .map(|d| d.clone().into())
        .collect::<Vec<Transaction>>();
    assert_encoding_correct(&txs);
}

#[test]
fn call() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    assert_encoding_correct(
        (0..10)
            .map(|_| Call::new(rng.gen(), rng.gen(), rng.gen()))
            .collect::<Vec<Call>>()
            .as_slice(),
    );
}

#[test]
fn call_frame() {
    let rng = &mut StdRng::seed_from_u64(2322u64);

    assert_encoding_correct(
        (0..10)
            .map(|_| {
                CallFrame::new(
                    rng.gen(),
                    rng.gen(),
                    [rng.gen(); VM_REGISTER_COUNT],
                    200,
                    rng.gen(),
                    rng.gen(),
                )
                .unwrap()
            })
            .collect::<Vec<CallFrame>>()
            .as_slice(),
    );
}

#[test]
fn witness() {
    assert_encoding_correct(&[Witness::from(vec![0xef]), Witness::from(vec![])]);
}

#[test]
fn input() {
    assert_encoding_correct(&[
        Input::coin_signed(
            UtxoId::new([0xaa; 32].into(), 0),
            [0xbb; 32].into(),
            Word::MAX,
            [0xcc; 32].into(),
            TxPointer::new(0x3802.into(), 0x28),
            0xff,
        ),
        Input::coin_predicate(
            UtxoId::new([0xaa; 32].into(), 0),
            [0xbb; 32].into(),
            Word::MAX,
            [0xcc; 32].into(),
            TxPointer::new(0x3802.into(), 0x28),
            Word::MAX,
            vec![0xdd; 50],
            vec![0xee; 23],
        ),
        Input::coin_predicate(
            UtxoId::new([0xaa; 32].into(), 0),
            [0xbb; 32].into(),
            Word::MAX,
            [0xcc; 32].into(),
            TxPointer::new(0x3802.into(), 0x28),
            Word::MAX,
            vec![0xdd; 50],
            vec![],
        ),
        Input::message_coin_signed(
            [0xaa; 32].into(),
            [0xbb; 32].into(),
            Word::MAX,
            [0xcc; 32].into(),
            0xff,
        ),
        Input::message_coin_predicate(
            [0xaa; 32].into(),
            [0xbb; 32].into(),
            Word::MAX,
            [0xcc; 32].into(),
            Word::MAX,
            vec![0xee; 50],
            vec![0xff; 23],
        ),
        Input::message_coin_predicate(
            [0xaa; 32].into(),
            [0xbb; 32].into(),
            Word::MAX,
            [0xcc; 32].into(),
            Word::MAX,
            vec![0xee; 50],
            vec![],
        ),
        Input::message_data_signed(
            [0xaa; 32].into(),
            [0xbb; 32].into(),
            Word::MAX,
            [0xcc; 32].into(),
            0xff,
            vec![0xdd; 50],
        ),
        Input::message_data_predicate(
            [0xaa; 32].into(),
            [0xbb; 32].into(),
            Word::MAX,
            [0xcc; 32].into(),
            Word::MAX,
            vec![0xdd; 50],
            vec![0xee; 50],
            vec![0xff; 23],
        ),
        Input::message_data_predicate(
            [0xaa; 32].into(),
            [0xbb; 32].into(),
            Word::MAX,
            [0xcc; 32].into(),
            Word::MAX,
            vec![0xdd; 50],
            vec![0xee; 50],
            vec![],
        ),
        Input::contract(
            UtxoId::new([0xaa; 32].into(), 0),
            [0xbb; 32].into(),
            [0xcc; 32].into(),
            TxPointer::new(0x3802.into(), 0x28),
            [0xdd; 32].into(),
        ),
    ]);
}

#[test]
fn output() {
    assert_encoding_correct(&[
        Output::coin([0xaa; 32].into(), Word::MAX >> 1, [0xbb; 32].into()),
        Output::contract(0xaa, [0xbb; 32].into(), [0xcc; 32].into()),
        Output::change([0xaa; 32].into(), Word::MAX >> 1, [0xbb; 32].into()),
        Output::variable([0xaa; 32].into(), Word::MAX >> 1, [0xbb; 32].into()),
        Output::contract_created([0xaa; 32].into(), [0xaa; 32].into()),
    ]);
}

#[test]
fn transaction_canonical_serialization_deserialization() {
    let i = Input::contract(
        UtxoId::new([0xaa; 32].into(), 0),
        [0xbb; 32].into(),
        [0xcc; 32].into(),
        TxPointer::new(0xbeef.into(), 0xeaae),
        [0xdd; 32].into(),
    );
    let o = Output::coin([0xaa; 32].into(), Word::MAX >> 1, [0xbb; 32].into());
    let w = Witness::from(vec![0xbf]);

    assert_transactions_encoding_correct(&[
        Transaction::script(
            Word::MAX >> 2,
            vec![0xfa],
            vec![0xfb, 0xfc],
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 2,
            vec![],
            vec![0xfb, 0xfc],
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 2,
            vec![0xfa],
            vec![],
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 2,
            vec![],
            vec![],
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 2,
            vec![],
            vec![],
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 2,
            vec![],
            vec![],
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::script(
            Word::MAX >> 2,
            vec![],
            vec![],
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
        Transaction::script(
            Word::MAX >> 2,
            vec![],
            vec![],
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
        Transaction::script(
            Word::MAX >> 2,
            vec![],
            vec![],
            Policies::new(),
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_transactions_encoding_correct(&[
        Transaction::create(
            0xba,
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            [0xdd; 32].into(),
            vec![],
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            0xba,
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            [0xdd; 32].into(),
            vec![],
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::create(
            0xba,
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            [0xdd; 32].into(),
            vec![],
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::create(
            0xba,
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            [0xdd; 32].into(),
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        Transaction::create(
            0xba,
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_max_fee(Word::MAX >> 5),
            [0xdd; 32].into(),
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        Transaction::create(
            0xba,
            Policies::new(),
            [0xdd; 32].into(),
            vec![],
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_transactions_encoding_correct(&[
        Transaction::upgrade(
            UpgradePurpose::ConsensusParameters {
                witness_index: 0,
                checksum: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::ConsensusParameters {
                witness_index: 0,
                checksum: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::ConsensusParameters {
                witness_index: 0,
                checksum: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::ConsensusParameters {
                witness_index: 0,
                checksum: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
        Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::upgrade(
            UpgradePurpose::StateTransition {
                root: [0xfa; 32].into(),
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_transactions_encoding_correct(&[
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::upload(
            UploadBody {
                root: [6; 32].into(),
                witness_index: 0,
                subsection_index: 0x1234,
                subsections_number: 0x4321,
                proof_set: vec![[1; 32].into(), [2; 32].into(), [3; 32].into()],
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
    ]);
    assert_transactions_encoding_correct(&[
        Transaction::blob(
            BlobBody {
                id: [6; 32].into(),
                witness_index: 0,
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![i.clone()],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::blob(
            BlobBody {
                id: [6; 32].into(),
                witness_index: 0,
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![o],
            vec![w.clone()],
        ),
        Transaction::blob(
            BlobBody {
                id: [6; 32].into(),
                witness_index: 0,
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![w.clone()],
        ),
        Transaction::blob(
            BlobBody {
                id: [6; 32].into(),
                witness_index: 0,
            },
            Policies::new()
                .with_tip(Word::MAX >> 1)
                .with_maturity((u32::MAX >> 3).into())
                .with_witness_limit(Word::MAX >> 4)
                .with_max_fee(Word::MAX >> 5),
            vec![],
            vec![],
            vec![],
        ),
    ]);
}
