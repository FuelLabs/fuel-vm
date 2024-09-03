use crate::{
    builder::Finalizable,
    test_helper::generate_bytes,
    BlobBody,
    BlobId,
    ConsensusParameters,
    Input,
    Output,
    ScriptCode,
    Transaction,
    TransactionBuilder,
    TxId,
    TxPointer,
    UpgradePurpose,
    UploadBody,
};
use bimap::BiMap;
use fuel_compression::{
    Compressed,
    CompressibleBy,
    DecompressibleBy,
    RawKey,
    RegistryDesubstitutableBy,
    RegistrySubstitutableBy,
};
use fuel_crypto::SecretKey;
use fuel_types::{
    Address,
    AssetId,
    ContractId,
};
use rand::{
    rngs::StdRng,
    Rng,
    SeedableRng,
};
use std::{
    collections::HashMap,
    convert::Infallible,
};

type Keyspace = &'static str;

/// A simple and inefficient registry for testing purposes
#[derive(Default)]
struct TestCompressionCtx {
    registry: HashMap<Keyspace, BiMap<RawKey, Vec<u8>>>,
    tx_blocks: BiMap<TxPointer, TxId>,
}

macro_rules! impl_substitutable_key {
    ($t:ty) => {
        impl RegistrySubstitutableBy<RawKey, TestCompressionCtx, Infallible> for $t {
            fn substitute(
                &self,
                ctx: &mut TestCompressionCtx,
            ) -> Result<RawKey, Infallible> {
                let keyspace = stringify!($t);
                let value = postcard::to_stdvec(self).expect("failed to serialize");
                let key_seed = ctx.registry.len(); // Just get an unique integer key

                let entry = ctx.registry.entry(keyspace).or_default();
                if let Some(key) = entry.get_by_right(&value) {
                    return Ok(*key);
                }

                let key = RawKey::try_from(key_seed as u32).expect("key too large");
                entry.insert(key, value);
                Ok(key)
            }
        }

        impl RegistryDesubstitutableBy<RawKey, TestCompressionCtx, Infallible> for $t {
            fn desubstitute(
                key: &RawKey,
                ctx: &TestCompressionCtx,
            ) -> Result<$t, Infallible> {
                let keyspace = stringify!($t);
                let values = ctx.registry.get(&keyspace).expect("key not found");
                let value = values.get_by_left(key).expect("key not found");
                Ok(postcard::from_bytes(value).expect("failed to deserialize"))
            }
        }
    };
}

impl_substitutable_key!(Address);
impl_substitutable_key!(AssetId);
impl_substitutable_key!(ContractId);
impl_substitutable_key!(ScriptCode);

impl RegistrySubstitutableBy<TxPointer, TestCompressionCtx, Infallible> for TxId {
    fn substitute(&self, ctx: &mut TestCompressionCtx) -> Result<TxPointer, Infallible> {
        if let Some(key) = ctx.tx_blocks.get_by_right(self) {
            return Ok(*key);
        }

        let key_seed = ctx.tx_blocks.len(); // Just get an unique integer key
        let key = TxPointer::new((key_seed as u32).into(), 0);
        ctx.tx_blocks.insert(key, *self);
        Ok(key)
    }
}

impl RegistryDesubstitutableBy<TxPointer, TestCompressionCtx, Infallible> for TxId {
    fn desubstitute(
        key: &TxPointer,
        ctx: &TestCompressionCtx,
    ) -> Result<TxId, Infallible> {
        Ok(*ctx.tx_blocks.get_by_left(key).expect("key not found"))
    }
}

#[derive(Debug, PartialEq, Default, Compressed)]
pub struct ExampleStruct {
    pub asset_id_bare: AssetId,
    #[da_compress(substitute = RawKey)]
    pub asset_id_ref: AssetId,
    pub array: [u8; 32],
    pub vec: Vec<u8>,
    pub integer: u32,
}

#[derive(Debug, PartialEq, Compressed)]
pub struct InnerStruct {
    #[da_compress(substitute = RawKey)]
    pub asset_id: AssetId,
    pub count: u64,
    #[da_compress(skip)]
    pub cached: [u8; 32],
}

#[test]
fn example_struct_roundtrip_simple() {
    let mut ctx = TestCompressionCtx::default();
    let original = ExampleStruct::default();
    let compressed = original.compress(&mut ctx).expect("compression failed");
    let decompressed =
        ExampleStruct::decompress(&compressed, &ctx).expect("decompression failed");
    assert_eq!(original, decompressed);
}

#[test]
fn example_struct_postcard_roundtrip_multiple() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let mut ctx = TestCompressionCtx::default();
    for _ in 0..10 {
        let original = ExampleStruct {
            asset_id_bare: AssetId::new(rng.gen()),
            asset_id_ref: AssetId::new(rng.gen()),
            array: rng.gen(),
            vec: (0..rng.gen_range(0..32)).map(|_| rng.gen::<u8>()).collect(),
            integer: rng.gen(),
        };
        let compressed = original.compress(&mut ctx).expect("compression failed");
        let postcard_compressed =
            postcard::to_stdvec(&compressed).expect("failed to serialize");
        let postcard_decompressed =
            postcard::from_bytes(&postcard_compressed).expect("failed to deserialize");
        let decompressed = ExampleStruct::decompress(&postcard_decompressed, &ctx)
            .expect("decompression failed");
        assert_eq!(original, decompressed);
    }
}

#[test]
fn transaction_postcard_roundtrip() {
    let rng = &mut StdRng::seed_from_u64(8586);

    // Malleable fields zero, others randomized.
    let txs: Vec<Transaction> = vec![
        TransactionBuilder::script(generate_bytes(rng), generate_bytes(rng))
            .maturity(100u32.into())
            .add_random_fee_input()
            .finalize()
            .into(),
        TransactionBuilder::create(generate_bytes(rng).into(), rng.gen(), vec![])
            .maturity(100u32.into())
            .add_unsigned_coin_input(
                SecretKey::random(rng),
                rng.gen(),
                0,
                rng.gen(),
                rng.gen(),
            )
            .add_contract_created()
            .add_output(Output::change(rng.gen(), 0, AssetId::default()))
            .finalize()
            .into(),
        TransactionBuilder::upload(UploadBody {
            root: rng.gen(),
            witness_index: 0,
            subsection_index: rng.gen(),
            subsections_number: rng.gen(),
            proof_set: Default::default(),
        })
        .add_random_fee_input()
        .finalize()
        .into(),
        TransactionBuilder::upgrade(UpgradePurpose::StateTransition {
            root: Default::default(),
        })
        .add_input(Input::coin_signed(
            Default::default(),
            *ConsensusParameters::standard().privileged_address(),
            rng.gen(),
            AssetId::BASE,
            Default::default(),
            0,
        ))
        .add_random_fee_input()
        .finalize()
        .into(),
        TransactionBuilder::blob(BlobBody {
            id: BlobId::new(rng.gen()),
            witness_index: 0,
        })
        .add_witness(generate_bytes(rng).into())
        .maturity(Default::default())
        .add_random_fee_input()
        .finalize()
        .into(),
    ];

    let mut ctx = TestCompressionCtx::default();
    for tx in txs {
        let compressed = tx.compress(&mut ctx).expect("compression failed");
        let postcard_compressed =
            postcard::to_stdvec(&compressed).expect("failed to serialize");
        let postcard_decompressed =
            postcard::from_bytes(&postcard_compressed).expect("failed to deserialize");
        let decompressed = Transaction::decompress(&postcard_decompressed, &ctx)
            .expect("decompression failed");
        assert_eq!(tx, decompressed);
    }
}
