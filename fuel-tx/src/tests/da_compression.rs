use crate::{
    builder::Finalizable,
    input::{
        coin::{
            self,
            Coin,
            CompressedCoin,
        },
        message::{
            self,
            CompressedMessage,
            Message,
        },
        Empty,
        PredicateCode,
    },
    test_helper::generate_bytes,
    BlobBody,
    BlobId,
    ConsensusParameters,
    Input,
    Output,
    ScriptCode,
    Transaction,
    TransactionBuilder,
    TxPointer,
    UpgradePurpose,
    UploadBody,
    UtxoId,
};
use bimap::BiMap;
use fuel_compression::{
    Compress,
    CompressibleBy,
    Decompress,
    DecompressibleBy,
    RegistryKey,
};
use fuel_crypto::SecretKey;
use fuel_types::{
    Address,
    AssetId,
    ContractId,
    Word,
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

/// When a coin is created, this data is stored
struct CoinInfo {
    owner: Address,
    amount: u64,
    asset_id: AssetId,
    tx_pointer: TxPointer,
}

/// When a message is created, this data is stored
struct MessageInfo {
    pub _sender: Address,
    pub _recipient: Address,
    pub _amount: Word,
    pub _data: Vec<u8>,
}

/// A simple and inefficient registry for testing purposes
#[derive(Default)]
struct TestCompressionCtx {
    registry: HashMap<Keyspace, BiMap<RegistryKey, Vec<u8>>>,
    tx_blocks: BiMap<(TxPointer, u16), UtxoId>,
    coins: HashMap<UtxoId, CoinInfo>,
    _messages: HashMap<usize, MessageInfo>,
}

macro_rules! impl_substitutable_key {
    ($t:ty) => {
        impl CompressibleBy<TestCompressionCtx, Infallible> for $t {
            async fn compress_with(
                &self,
                ctx: &mut TestCompressionCtx,
            ) -> Result<RegistryKey, Infallible> {
                let keyspace = stringify!($t);
                let value = postcard::to_stdvec(self).expect("failed to serialize");
                let key_seed = ctx.registry.len(); // Just get an unique integer key

                let entry = ctx.registry.entry(keyspace).or_default();
                if let Some(key) = entry.get_by_right(&value) {
                    return Ok(*key);
                }

                let key = RegistryKey::try_from(key_seed as u32).expect("key too large");
                entry.insert(key, value);
                Ok(key)
            }
        }

        impl DecompressibleBy<TestCompressionCtx, Infallible> for $t {
            async fn decompress_with(
                key: &RegistryKey,
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
impl_substitutable_key!(PredicateCode);

impl CompressibleBy<TestCompressionCtx, Infallible> for UtxoId {
    async fn compress_with(
        &self,
        ctx: &mut TestCompressionCtx,
    ) -> Result<(TxPointer, u16), Infallible> {
        if let Some(key) = ctx.tx_blocks.get_by_right(self) {
            return Ok(*key);
        }

        let key_seed = ctx.tx_blocks.len(); // Just get an unique integer key
        let key = (TxPointer::new((key_seed as u32).into(), 0), 0);
        ctx.tx_blocks.insert(key, *self);
        Ok(key)
    }
}

impl DecompressibleBy<TestCompressionCtx, Infallible> for UtxoId {
    async fn decompress_with(
        key: &(TxPointer, u16),
        ctx: &TestCompressionCtx,
    ) -> Result<UtxoId, Infallible> {
        Ok(*ctx.tx_blocks.get_by_left(key).expect("key not found"))
    }
}

impl DecompressibleBy<TestCompressionCtx, Infallible> for Coin<coin::Full> {
    async fn decompress_with(
        c: &CompressedCoin<coin::Full>,
        ctx: &TestCompressionCtx,
    ) -> Result<Coin<coin::Full>, Infallible> {
        let utxo_id = UtxoId::decompress_with(&c.utxo_id, ctx).await?;
        let coin_info = ctx.coins.get(&utxo_id).expect("coin not found");
        Ok(Coin {
            utxo_id,
            owner: coin_info.owner,
            amount: coin_info.amount,
            asset_id: coin_info.asset_id,
            tx_pointer: coin_info.tx_pointer,
            witness_index: c.witness_index,
            predicate_gas_used: c.predicate_gas_used,
            predicate:
                <coin::Full as coin::CoinSpecification>::Predicate::decompress_with(
                    &c.predicate,
                    ctx,
                )
                .await?,
            predicate_data: c.predicate_data.clone(),
        })
    }
}

impl DecompressibleBy<TestCompressionCtx, Infallible> for Coin<coin::Signed> {
    async fn decompress_with(
        c: &CompressedCoin<coin::Signed>,
        ctx: &TestCompressionCtx,
    ) -> Result<Coin<coin::Signed>, Infallible> {
        let utxo_id = UtxoId::decompress_with(&c.utxo_id, ctx).await?;
        let coin_info = ctx.coins.get(&utxo_id).expect("coin not found");
        Ok(Coin {
            utxo_id,
            owner: coin_info.owner,
            amount: coin_info.amount,
            asset_id: coin_info.asset_id,
            tx_pointer: coin_info.tx_pointer,
            witness_index: c.witness_index,
            predicate_gas_used: Empty::default(),
            predicate: Empty::default(),
            predicate_data: Empty::default(),
        })
    }
}

impl DecompressibleBy<TestCompressionCtx, Infallible> for Coin<coin::Predicate> {
    async fn decompress_with(
        c: &CompressedCoin<coin::Predicate>,
        ctx: &TestCompressionCtx,
    ) -> Result<Coin<coin::Predicate>, Infallible> {
        let utxo_id = UtxoId::decompress_with(&c.utxo_id, ctx).await?;
        let coin_info = ctx.coins.get(&utxo_id).expect("coin not found");
        Ok(Coin {
            utxo_id,
            owner: coin_info.owner,
            amount: coin_info.amount,
            asset_id: coin_info.asset_id,
            tx_pointer: coin_info.tx_pointer,
            witness_index: Empty::default(),
            predicate_gas_used: c.predicate_gas_used,
            predicate:
                <coin::Full as coin::CoinSpecification>::Predicate::decompress_with(
                    &c.predicate,
                    ctx,
                )
                .await?,
            predicate_data: c.predicate_data.clone(),
        })
    }
}

macro_rules! impl_for_message {
    ($spec:ty) => {
        impl DecompressibleBy<TestCompressionCtx, Infallible> for Message<$spec> {
            async fn decompress_with(
                _c: &CompressedMessage<$spec>,
                _ctx: &TestCompressionCtx,
            ) -> Result<Message<$spec>, Infallible> {
                // let msg_info = ctx.messages.get(&utxo_id).expect("message not found");
                // let msg_info = todo!();
                // Ok(Message {
                //     sender: msg_info.sender,
                //     recipient: msg_info.sender,
                //     amount: msg_info.sender,
                //     nonce: msg_info.sender,
                //     witness_index: msg_info.sender,
                //     predicate_gas_used: msg_info.sender,
                //     data: msg_info.sender,
                //     predicate: msg_info.sender,
                //     predicate_data: msg_info.sender,
                // })
                todo!();
            }
        }
    };
}

impl_for_message!(message::specifications::Full);
impl_for_message!(message::specifications::MessageData<message::specifications::Signed>);
impl_for_message!(
    message::specifications::MessageData<message::specifications::Predicate>
);
impl_for_message!(message::specifications::MessageCoin<message::specifications::Signed>);
impl_for_message!(
    message::specifications::MessageCoin<message::specifications::Predicate>
);

#[derive(Debug, PartialEq, Default, Compress, Decompress)]
pub struct ExampleStruct {
    pub asset_id: AssetId,
    pub array: [u8; 32],
    pub vec: Vec<u8>,
    pub integer: u32,
}

#[derive(Debug, PartialEq, Compress, Decompress)]
pub struct InnerStruct {
    pub asset_id: AssetId,
    pub count: u64,
    #[compress(skip)]
    pub cached: [u8; 32],
}

#[tokio::test]
async fn example_struct_roundtrip_simple() {
    let mut ctx = TestCompressionCtx::default();
    let original = ExampleStruct::default();
    let compressed = original
        .compress_with(&mut ctx)
        .await
        .expect("compression failed");
    let decompressed = ExampleStruct::decompress_with(&compressed, &ctx)
        .await
        .expect("decompression failed");
    assert_eq!(original, decompressed);
}

#[tokio::test]
async fn example_struct_postcard_roundtrip_multiple() {
    let rng = &mut StdRng::seed_from_u64(8586);

    let mut ctx = TestCompressionCtx::default();
    for _ in 0..10 {
        let original = ExampleStruct {
            asset_id: AssetId::new(rng.gen()),
            array: rng.gen(),
            vec: (0..rng.gen_range(0..32)).map(|_| rng.gen::<u8>()).collect(),
            integer: rng.gen(),
        };
        let compressed = original
            .compress_with(&mut ctx)
            .await
            .expect("compression failed");
        let postcard_compressed =
            postcard::to_stdvec(&compressed).expect("failed to serialize");
        let postcard_decompressed =
            postcard::from_bytes(&postcard_compressed).expect("failed to deserialize");
        let decompressed = ExampleStruct::decompress_with(&postcard_decompressed, &ctx)
            .await
            .expect("decompression failed");
        assert_eq!(original, decompressed);
    }
}

#[tokio::test]
async fn transaction_postcard_roundtrip() {
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
        let compressed = tx
            .compress_with(&mut ctx)
            .await
            .expect("compression failed");
        let postcard_compressed =
            postcard::to_stdvec(&compressed).expect("failed to serialize");
        let postcard_decompressed =
            postcard::from_bytes(&postcard_compressed).expect("failed to deserialize");
        let decompressed = <Transaction as DecompressibleBy<_, _>>::decompress_with(
            &postcard_decompressed,
            &ctx,
        )
        .await
        .expect("decompression failed");
        assert_eq!(tx, decompressed);
    }
}
