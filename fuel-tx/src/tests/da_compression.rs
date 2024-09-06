use crate::{
    input::{
        self,
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
    output,
    test_helper::TransactionFactory,
    transaction::field::Inputs,
    Blob,
    CompressedUtxoId,
    Create,
    Mint,
    PrepareSign,
    Script,
    ScriptCode,
    Transaction,
    TxPointer,
    Upgrade,
    Upload,
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
use fuel_types::{
    Address,
    AssetId,
    ContractId,
    Nonce,
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
}

/// When a message is created, this data is stored
struct MessageInfo {
    pub sender: Address,
    pub recipient: Address,
    pub amount: Word,
    pub data: Vec<u8>,
}

/// A simple and inefficient registry for testing purposes.
/// Also just stores the latest given transaction to just return it back.
#[derive(Default)]
struct TestCompressionCtx {
    next_key: u32,
    registry: HashMap<Keyspace, BiMap<RegistryKey, Vec<u8>>>,
    tx_blocks: BiMap<CompressedUtxoId, UtxoId>,
    latest_tx_coins: HashMap<UtxoId, CoinInfo>,
    latest_tx_pointer: Option<TxPointer>,
    latest_tx_messages: HashMap<Nonce, MessageInfo>,
}

impl TestCompressionCtx {
    fn store_data_for_mint(&mut self, tx: &Mint) {
        self.latest_tx_pointer = Some(tx.tx_pointer);
    }

    fn store_tx_info<Tx>(&mut self, tx: &Tx)
    where
        Tx: Inputs,
    {
        self.latest_tx_coins = tx
            .inputs()
            .iter()
            .filter(|input| input.is_coin())
            .map(|input| {
                (
                    *input.utxo_id().unwrap(),
                    CoinInfo {
                        owner: *input.input_owner().unwrap(),
                        amount: input.amount().unwrap(),
                        asset_id: *input.asset_id(&AssetId::default()).unwrap(),
                    },
                )
            })
            .collect();
        self.latest_tx_messages = tx
            .inputs()
            .iter()
            .filter(|input| input.is_message())
            .map(|input| {
                (
                    *input.nonce().unwrap(),
                    MessageInfo {
                        sender: *input.sender().unwrap(),
                        recipient: *input.recipient().unwrap(),
                        amount: input.amount().unwrap(),
                        data: input.input_data().unwrap_or_default().to_vec(),
                    },
                )
            })
            .collect();
    }
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

                let entry = ctx.registry.entry(keyspace).or_default();
                if let Some(key) = entry.get_by_right(&value) {
                    return Ok(*key);
                }

                let key =
                    RegistryKey::try_from(ctx.next_key as u32).expect("key too large");
                ctx.next_key += 1;
                entry
                    .insert_no_overwrite(key, value)
                    .expect("duplicate key");
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
    ) -> Result<CompressedUtxoId, Infallible> {
        if let Some(key) = ctx.tx_blocks.get_by_right(self) {
            return Ok(*key);
        }

        let key_seed = ctx.tx_blocks.len(); // Just get an unique integer key
        let key = CompressedUtxoId {
            tx_pointer: TxPointer::new((key_seed as u32).into(), 0),
            output_index: 0,
        };
        ctx.tx_blocks.insert(key, *self);
        Ok(key)
    }
}

impl DecompressibleBy<TestCompressionCtx, Infallible> for UtxoId {
    async fn decompress_with(
        key: &CompressedUtxoId,
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
        let coin_info = ctx.latest_tx_coins.get(&utxo_id).expect("coin not found");
        Ok(Coin {
            utxo_id,
            owner: coin_info.owner,
            amount: coin_info.amount,
            asset_id: coin_info.asset_id,
            tx_pointer: Default::default(),
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
        let coin_info = ctx.latest_tx_coins.get(&utxo_id).expect("coin not found");
        Ok(Coin {
            utxo_id,
            owner: coin_info.owner,
            amount: coin_info.amount,
            asset_id: coin_info.asset_id,
            tx_pointer: Default::default(),
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
        let coin_info = ctx.latest_tx_coins.get(&utxo_id).expect("coin not found");
        Ok(Coin {
            utxo_id,
            owner: coin_info.owner,
            amount: coin_info.amount,
            asset_id: coin_info.asset_id,
            tx_pointer: Default::default(),
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

impl DecompressibleBy<TestCompressionCtx, Infallible>
    for Message<message::specifications::Full>
{
    async fn decompress_with(
        c: &CompressedMessage<message::specifications::Full>,
        ctx: &TestCompressionCtx,
    ) -> Result<Message<message::specifications::Full>, Infallible> {
        let msg = ctx
            .latest_tx_messages
            .get(&c.nonce)
            .expect("message not found");
        Ok(Message {
            sender: msg.sender,
            recipient: msg.recipient,
            amount: msg.amount,
            nonce: c.nonce,
            witness_index: c.witness_index,
            predicate_gas_used: c.predicate_gas_used,
            data: msg.data.clone(),
            predicate:
                <message::specifications::Full as message::MessageSpecification>::Predicate::decompress_with(
                    &c.predicate,
                    ctx,
                )
                .await?,
            predicate_data: c.predicate_data.clone(),
        })
    }
}
impl DecompressibleBy<TestCompressionCtx, Infallible>
    for Message<message::specifications::MessageData<message::specifications::Signed>>
{
    async fn decompress_with(
        c: &CompressedMessage<
            message::specifications::MessageData<message::specifications::Signed>,
        >,
        ctx: &TestCompressionCtx,
    ) -> Result<
        Message<message::specifications::MessageData<message::specifications::Signed>>,
        Infallible,
    > {
        let msg = ctx
            .latest_tx_messages
            .get(&c.nonce)
            .expect("message not found");
        Ok(Message {
            sender: msg.sender,
            recipient: msg.recipient,
            amount: msg.amount,
            nonce: c.nonce,
            witness_index: c.witness_index,
            predicate_gas_used: Empty::default(),
            data: msg.data.clone(),
            predicate: <<message::specifications::MessageData<
                message::specifications::Signed,
            > as message::MessageSpecification>::Predicate as DecompressibleBy<
                _,
                Infallible,
            >>::decompress_with(&c.predicate, ctx)
            .await?,
            predicate_data: Empty::default(),
        })
    }
}
impl DecompressibleBy<TestCompressionCtx, Infallible>
    for Message<message::specifications::MessageData<message::specifications::Predicate>>
{
    async fn decompress_with(
        c: &CompressedMessage<
            message::specifications::MessageData<message::specifications::Predicate>,
        >,
        ctx: &TestCompressionCtx,
    ) -> Result<
        Message<message::specifications::MessageData<message::specifications::Predicate>>,
        Infallible,
    > {
        let msg = ctx
            .latest_tx_messages
            .get(&c.nonce)
            .expect("message not found");
        Ok(Message {
            sender: msg.sender,
            recipient: msg.recipient,
            amount: msg.amount,
            nonce: c.nonce,
            witness_index: Empty::default(),
            predicate_gas_used: c.predicate_gas_used,
            data: msg.data.clone(),
            predicate: <message::specifications::MessageData<
                message::specifications::Predicate,
            > as message::MessageSpecification>::Predicate::decompress_with(
                &c.predicate,
                ctx,
            )
            .await?,
            predicate_data: c.predicate_data.clone(),
        })
    }
}
impl DecompressibleBy<TestCompressionCtx, Infallible>
    for Message<message::specifications::MessageCoin<message::specifications::Signed>>
{
    async fn decompress_with(
        c: &CompressedMessage<
            message::specifications::MessageCoin<message::specifications::Signed>,
        >,
        ctx: &TestCompressionCtx,
    ) -> Result<
        Message<message::specifications::MessageCoin<message::specifications::Signed>>,
        Infallible,
    > {
        let msg = ctx
            .latest_tx_messages
            .get(&c.nonce)
            .expect("message not found");
        Ok(Message {
            sender: msg.sender,
            recipient: msg.recipient,
            amount: msg.amount,
            nonce: c.nonce,
            witness_index: c.witness_index,
            predicate_gas_used: Empty::default(),
            data: Empty::default(),
            predicate: <<message::specifications::MessageCoin<
                message::specifications::Signed,
            > as message::MessageSpecification>::Predicate as DecompressibleBy<
                _,
                Infallible,
            >>::decompress_with(&c.predicate, ctx)
            .await?,
            predicate_data: Empty::default(),
        })
    }
}
impl DecompressibleBy<TestCompressionCtx, Infallible>
    for Message<message::specifications::MessageCoin<message::specifications::Predicate>>
{
    async fn decompress_with(
        c: &CompressedMessage<
            message::specifications::MessageCoin<message::specifications::Predicate>,
        >,
        ctx: &TestCompressionCtx,
    ) -> Result<
        Message<message::specifications::MessageCoin<message::specifications::Predicate>>,
        Infallible,
    > {
        let msg = ctx
            .latest_tx_messages
            .get(&c.nonce)
            .expect("message not found");
        Ok(Message {
            sender: msg.sender,
            recipient: msg.recipient,
            amount: msg.amount,
            nonce: c.nonce,
            witness_index: Empty::default(),
            predicate_gas_used: c.predicate_gas_used,
            data: Empty::default(),
            predicate: <message::specifications::MessageCoin<
                message::specifications::Predicate,
            > as message::MessageSpecification>::Predicate::decompress_with(
                &c.predicate,
                ctx,
            )
            .await?,
            predicate_data: c.predicate_data.clone(),
        })
    }
}

impl DecompressibleBy<TestCompressionCtx, Infallible> for Mint {
    async fn decompress_with(
        c: &Self::Compressed,
        ctx: &TestCompressionCtx,
    ) -> Result<Self, Infallible> {
        Ok(Transaction::mint(
            ctx.latest_tx_pointer.expect("no latest tx pointer"),
            <input::contract::Contract as DecompressibleBy<_, Infallible>>::decompress_with(
                &c.input_contract,
                ctx,
            )
            .await?,
            <output::contract::Contract as DecompressibleBy<_, Infallible>>::decompress_with(
                &c.output_contract,
                ctx,
            )
            .await?,
            <Word as DecompressibleBy<_, Infallible>>::decompress_with(&c.mint_amount, ctx).await?,
            <AssetId as DecompressibleBy<_, Infallible>>::decompress_with(&c.mint_asset_id, ctx).await?,
            <Word as DecompressibleBy<_, Infallible>>::decompress_with(&c.gas_price, ctx).await?,
        ))
    }
}

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

async fn verify_tx_roundtrip(tx: Transaction, ctx: &mut TestCompressionCtx) {
    let compressed = tx.compress_with(ctx).await.expect("compression failed");
    let postcard_compressed =
        postcard::to_stdvec(&compressed).expect("failed to serialize");
    let postcard_decompressed =
        postcard::from_bytes(&postcard_compressed).expect("failed to deserialize");
    let decompressed = <Transaction as DecompressibleBy<_, _>>::decompress_with(
        &postcard_decompressed,
        ctx,
    )
    .await
    .expect("decompression failed");
    pretty_assertions::assert_eq!(tx, decompressed);
}

#[tokio::test]
async fn test_tx_roundtrip() {
    let number_cases = 100;
    let mut ctx = TestCompressionCtx::default();

    for mut tx in TransactionFactory::<_, Mint>::from_seed(1234).take(number_cases) {
        ctx.store_data_for_mint(&tx);
        tx.prepare_sign();
        verify_tx_roundtrip(tx.into(), &mut ctx).await;
    }
    for (mut tx, _) in TransactionFactory::<_, Script>::from_seed(1234).take(number_cases)
    {
        ctx.store_tx_info(&tx);
        tx.prepare_sign();
        verify_tx_roundtrip(tx.into(), &mut ctx).await;
    }
    for (mut tx, _) in TransactionFactory::<_, Create>::from_seed(1234).take(number_cases)
    {
        ctx.store_tx_info(&tx);
        tx.prepare_sign();
        verify_tx_roundtrip(tx.into(), &mut ctx).await;
    }
    for (mut tx, _) in
        TransactionFactory::<_, Upgrade>::from_seed(1234).take(number_cases)
    {
        ctx.store_tx_info(&tx);
        tx.prepare_sign();
        verify_tx_roundtrip(tx.into(), &mut ctx).await;
    }
    for (mut tx, _) in TransactionFactory::<_, Upload>::from_seed(1234).take(number_cases)
    {
        ctx.store_tx_info(&tx);
        tx.prepare_sign();
        verify_tx_roundtrip(tx.into(), &mut ctx).await;
    }
    for (mut tx, _) in TransactionFactory::<_, Blob>::from_seed(1234).take(number_cases) {
        ctx.store_tx_info(&tx);
        tx.prepare_sign();
        verify_tx_roundtrip(tx.into(), &mut ctx).await;
    }
}
