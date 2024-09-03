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
    UpgradePurpose,
    UploadBody,
};
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

/// A simple and inefficient registry for testing purposes
#[derive(Default)]
struct TestCompressionCtx {
    registry: HashMap<(&'static str, RawKey), Vec<u8>>,
}

macro_rules! impl_substitutable {
    ($t:ty) => {
        impl RegistrySubstitutableBy<TestCompressionCtx, Infallible> for $t {
            fn substitute(
                &self,
                ctx: &mut TestCompressionCtx,
            ) -> Result<RawKey, Infallible> {
                let keyspace = stringify!($t);
                for ((ks, rk), v) in ctx.registry.iter() {
                    if *ks != keyspace {
                        continue;
                    }
                    let d: $t = postcard::from_bytes(v).expect("failed to deserialize");
                    if d == *self {
                        return Ok(*rk);
                    }
                }

                let key = ctx.registry.len(); // Just get an unique integer key
                let key = RawKey::try_from(key as u32).expect("key too large");
                let value = postcard::to_stdvec(self).expect("failed to serialize");
                ctx.registry.insert((keyspace, key), value);
                Ok(key)
            }
        }

        impl RegistryDesubstitutableBy<TestCompressionCtx, Infallible> for $t {
            fn desubstitute(
                key: &RawKey,
                ctx: &TestCompressionCtx,
            ) -> Result<$t, Infallible> {
                let keyspace = stringify!($t);
                let value = ctx.registry.get(&(keyspace, *key)).expect("key not found");
                Ok(postcard::from_bytes(value).expect("failed to deserialize"))
            }
        }
    };
}

impl_substitutable!(Address);
impl_substitutable!(AssetId);
impl_substitutable!(ContractId);
impl_substitutable!(ScriptCode);

#[derive(Debug, PartialEq, Default, Compressed)]
pub struct ExampleStruct {
    pub asset_id_bare: AssetId,
    #[da_compress(registry)]
    pub asset_id_ref: AssetId,
    pub array: [u8; 32],
    pub vec: Vec<u8>,
    pub integer: u32,
}

#[derive(Debug, PartialEq, Compressed)]
pub struct InnerStruct {
    #[da_compress(registry)]
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
