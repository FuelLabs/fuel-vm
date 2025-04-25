use crate::{
    field,
    input::{
        coin::{
            Coin,
            DataCoin,
            CoinSpecification,
        },
        message::{
            Message,
            MessageSpecification,
        },
        AsField,
        PredicateCode,
    },
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
    field,
    input::{
        AsField,
        PredicateCode,
        coin::{
            Coin,
            CoinSpecification,
        },
        message::{
            Message,
            MessageSpecification,
        },
    },
    test_helper::TransactionFactory,
    transaction::field::Inputs,
};
use bimap::BiMap;
use fuel_compression::{
    Compress,
    Compressible,
    CompressibleBy,
    ContextError,
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
    Rng,
    SeedableRng,
    rngs::StdRng,
};
use std::{
    collections::HashMap,
    convert::Infallible,
};

type Keyspace = &'static str;

/// When a coin is created, this data is stored
#[derive(Debug, Default, Clone, PartialEq)]
struct CoinInfo {
    owner: Address,
    amount: u64,
    asset_id: AssetId,
}

/// When a coin is created, this data is stored
#[derive(Debug, Default, Clone, PartialEq)]
struct DataCoinInfo {
    owner: Address,
    amount: u64,
    asset_id: AssetId,
    data: Vec<u8>,
}

/// When a message is created, this data is stored
#[derive(Debug, Default, Clone, PartialEq)]
struct MessageInfo {
    pub sender: Address,
    pub recipient: Address,
    pub amount: Word,
    pub data: Vec<u8>,
}

/// A simple and inefficient registry for testing purposes.
/// Also just stores the latest given transaction to just return it back.
#[derive(Debug, Default, Clone, PartialEq)]
struct TestCompressionCtx {
    next_key: u32,
    registry: HashMap<Keyspace, BiMap<RegistryKey, Vec<u8>>>,
    tx_blocks: BiMap<CompressedUtxoId, UtxoId>,
    latest_tx_coins: HashMap<UtxoId, CoinInfo>,
    latest_tx_data_coins: HashMap<UtxoId, DataCoinInfo>,
    latest_tx_pointer: Option<TxPointer>,
    latest_tx_messages: HashMap<Nonce, MessageInfo>,
}

impl TestCompressionCtx {
    fn store_data_for_mint(tx: &Mint) -> Self {
        Self {
            latest_tx_pointer: Some(tx.tx_pointer),
            ..Default::default()
        }
    }

    fn store_tx_info<Tx>(&mut self, tx: &Tx)
    where
        Tx: Inputs,
    {
        let latest_tx_coins =
            tx.inputs()
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
                });
        let latest_tx_data_coins =
            tx.inputs()
                .iter()
                .filter(|input| input.is_data_coin())
                .map(|input| {
                    (
                        *input.utxo_id().unwrap(),
                        DataCoinInfo {
                            owner: *input.input_owner().unwrap(),
                            amount: input.amount().unwrap(),
                            asset_id: *input.asset_id(&AssetId::default()).unwrap(),
                            data: input.input_data().unwrap_or_default().to_vec(),
                        },
                    )
                });
        let latest_tx_messages = tx
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
            });

        self.latest_tx_coins.extend(latest_tx_coins);
        self.latest_tx_data_coins
            .extend(latest_tx_data_coins);
        self.latest_tx_messages.extend(latest_tx_messages);
    }
}

impl ContextError for TestCompressionCtx {
    type Error = Infallible;
}

macro_rules! impl_substitutable_key {
    ($t:ty) => {
        impl CompressibleBy<TestCompressionCtx> for $t {
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

        impl DecompressibleBy<TestCompressionCtx> for $t {
            async fn decompress_with(
                key: RegistryKey,
                ctx: &TestCompressionCtx,
            ) -> Result<$t, Infallible> {
                let keyspace = stringify!($t);
                let values = ctx.registry.get(&keyspace).expect("key not found");
                let value = values.get_by_left(&key).expect("key not found");
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

impl CompressibleBy<TestCompressionCtx> for UtxoId {
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

impl DecompressibleBy<TestCompressionCtx> for UtxoId {
    async fn decompress_with(
        key: CompressedUtxoId,
        ctx: &TestCompressionCtx,
    ) -> Result<UtxoId, Infallible> {
        Ok(*ctx.tx_blocks.get_by_left(&key).expect("key not found"))
    }
}

impl<Specification> DecompressibleBy<TestCompressionCtx> for Coin<Specification>
where
    Specification: CoinSpecification,
    Specification::Predicate: DecompressibleBy<TestCompressionCtx>,
    Specification::PredicateData: DecompressibleBy<TestCompressionCtx>,
    Specification::PredicateGasUsed: DecompressibleBy<TestCompressionCtx>,
    Specification::Witness: DecompressibleBy<TestCompressionCtx>,
{
    async fn decompress_with(
        c: <Coin<Specification> as Compressible>::Compressed,
        ctx: &TestCompressionCtx,
    ) -> Result<Coin<Specification>, Infallible> {
        let utxo_id = UtxoId::decompress_with(c.utxo_id, ctx).await?;
        let coin_info = ctx.latest_tx_coins.get(&utxo_id).expect("coin not found");
        let witness_index = c.witness_index.decompress(ctx).await?;
        let predicate_gas_used = c.predicate_gas_used.decompress(ctx).await?;
        let predicate = c.predicate.decompress(ctx).await?;
        let predicate_data = c.predicate_data.decompress(ctx).await?;

        Ok(Self {
            utxo_id,
            owner: coin_info.owner,
            amount: coin_info.amount,
            asset_id: coin_info.asset_id,
            tx_pointer: Default::default(),
            witness_index,
            predicate_gas_used,
            predicate,
            predicate_data,
        })
    }
}


impl<Specification> DecompressibleBy<TestCompressionCtx> for DataCoin<Specification>
where
    Specification: CoinSpecification,
    Specification::Predicate: DecompressibleBy<TestCompressionCtx>,
    Specification::PredicateData: DecompressibleBy<TestCompressionCtx>,
    Specification::PredicateGasUsed: DecompressibleBy<TestCompressionCtx>,
    Specification::Witness: DecompressibleBy<TestCompressionCtx>,
{
    async fn decompress_with(
        c: <DataCoin<Specification> as Compressible>::Compressed,
        ctx: &TestCompressionCtx,
    ) -> Result<DataCoin<Specification>, Infallible> {
        let utxo_id = UtxoId::decompress_with(c.utxo_id, ctx).await?;
        let coin_info = ctx.latest_tx_data_coins.get(&utxo_id).expect("data coin not found");
        let witness_index = c.witness_index.decompress(ctx).await?;
        let predicate_gas_used = c.predicate_gas_used.decompress(ctx).await?;
        let predicate = c.predicate.decompress(ctx).await?;
        let predicate_data = c.predicate_data.decompress(ctx).await?;
        let data = c.data.decompress(ctx).await?;

        Ok(Self {
            utxo_id,
            owner: coin_info.owner,
            amount: coin_info.amount,
            asset_id: coin_info.asset_id,
            tx_pointer: Default::default(),
            witness_index,
            predicate_gas_used,
            predicate,
            predicate_data,
            data,
        })
    }
}

impl<Specification> DecompressibleBy<TestCompressionCtx> for Message<Specification>
where
    Specification: MessageSpecification,
    Specification::Data: DecompressibleBy<TestCompressionCtx> + Default,
    Specification::Predicate: DecompressibleBy<TestCompressionCtx>,
    Specification::PredicateData: DecompressibleBy<TestCompressionCtx>,
    Specification::PredicateGasUsed: DecompressibleBy<TestCompressionCtx>,
    Specification::Witness: DecompressibleBy<TestCompressionCtx>,
{
    async fn decompress_with(
        c: <Message<Specification> as Compressible>::Compressed,
        ctx: &TestCompressionCtx,
    ) -> Result<Message<Specification>, Infallible> {
        let msg = ctx
            .latest_tx_messages
            .get(&c.nonce)
            .expect("message not found");
        let witness_index = c.witness_index.decompress(ctx).await?;
        let predicate_gas_used = c.predicate_gas_used.decompress(ctx).await?;
        let predicate = c.predicate.decompress(ctx).await?;
        let predicate_data = c.predicate_data.decompress(ctx).await?;
        let mut message: Message<Specification> = Message {
            sender: msg.sender,
            recipient: msg.recipient,
            amount: msg.amount,
            nonce: c.nonce,
            witness_index,
            predicate_gas_used,
            data: Default::default(),
            predicate,
            predicate_data,
        };

        if let Some(data) = message.data.as_mut_field() {
            data.clone_from(&msg.data)
        }

        Ok(message)
    }
}

impl DecompressibleBy<TestCompressionCtx> for Mint {
    async fn decompress_with(
        c: Self::Compressed,
        ctx: &TestCompressionCtx,
    ) -> Result<Self, Infallible> {
        Ok(Transaction::mint(
            ctx.latest_tx_pointer.expect("no latest tx pointer"),
            c.input_contract.decompress(ctx).await?,
            c.output_contract.decompress(ctx).await?,
            c.mint_amount.decompress(ctx).await?,
            c.mint_asset_id.decompress(ctx).await?,
            c.gas_price.decompress(ctx).await?,
        ))
    }
}

#[tokio::test]
async fn example_struct_postcard_roundtrip_multiple() {
    #[derive(Debug, PartialEq, Default, Compress, Decompress)]
    pub struct Example {
        pub asset_id: AssetId,
        pub array: [u8; 32],
        pub vec: Vec<u8>,
        pub integer: u32,
        pub inner: Inner,
    }

    #[derive(Debug, PartialEq, Default, Compress, Decompress)]
    pub struct Inner {
        pub asset_id: AssetId,
        pub count: u64,
    }

    let rng = &mut StdRng::seed_from_u64(8586);

    let mut ctx = TestCompressionCtx::default();
    for _ in 0..10 {
        let original = Example {
            asset_id: AssetId::new(rng.r#gen()),
            array: rng.r#gen(),
            vec: (0..rng.gen_range(0..32))
                .map(|_| rng.r#gen::<u8>())
                .collect(),
            integer: rng.r#gen(),
            inner: Inner {
                asset_id: AssetId::new(rng.r#gen()),
                count: rng.r#gen(),
            },
        };
        let compressed = original
            .compress_with(&mut ctx)
            .await
            .expect("compression failed");
        let compressed_serialized =
            postcard::to_stdvec(&compressed).expect("failed to serialize");
        let compressed_deserialized =
            postcard::from_bytes(&compressed_serialized).expect("failed to deserialize");
        let decompressed = Example::decompress_with(compressed_deserialized, &ctx)
            .await
            .expect("decompression failed");
        assert_eq!(original, decompressed);
    }
}

#[tokio::test]
async fn skipped_fields_are_not_part_of_the_compressed_output() {
    #[derive(Debug, PartialEq, Default, Compress, Decompress)]
    pub struct NoSkip {
        pub common: u64,
    }
    #[derive(Debug, PartialEq, Default, Compress, Decompress)]
    pub struct Skip {
        pub common: u64,
        #[compress(skip)]
        pub skipped: u64,
    }

    let noskip = NoSkip { common: 123 };
    let skip = Skip {
        common: 123,
        skipped: 456,
    };

    let mut ctx = TestCompressionCtx::default();
    let noskip_compressed = noskip
        .compress_with(&mut ctx)
        .await
        .expect("compression failed");
    let noskip_compressed_serialized =
        postcard::to_stdvec(&noskip_compressed).expect("failed to serialize");

    let mut ctx = TestCompressionCtx::default();
    let skip_compressed = skip
        .compress_with(&mut ctx)
        .await
        .expect("compression failed");
    let skip_compressed_serialized =
        postcard::to_stdvec(&skip_compressed).expect("failed to serialize");

    assert_eq!(noskip_compressed_serialized, skip_compressed_serialized);
}

#[tokio::test]
async fn skipped_fields_are_set_to_default_when_deserializing() {
    #[derive(Debug, PartialEq, Default, Compress, Decompress)]
    pub struct Example {
        pub not_skipped: u64,
        #[compress(skip)]
        pub automatic: u64,
        #[compress(skip)]
        pub manual: HasManualDefault,
    }

    #[derive(Debug, PartialEq)]
    pub struct HasManualDefault {
        pub value: u64,
    }
    impl Default for HasManualDefault {
        fn default() -> Self {
            Self { value: 42 }
        }
    }

    let mut ctx = TestCompressionCtx::default();
    let original = Example {
        not_skipped: 123,
        automatic: 456,
        manual: HasManualDefault { value: 789 },
    };
    let compressed = original
        .compress_with(&mut ctx)
        .await
        .expect("compression failed");
    let decompressed = Example::decompress_with(compressed, &ctx)
        .await
        .expect("decompression failed");
    assert_eq!(
        decompressed,
        Example {
            not_skipped: 123,
            automatic: 0,
            manual: HasManualDefault::default(),
        }
    );
}

async fn verify_tx_roundtrip(tx: Transaction, ctx: &mut TestCompressionCtx) {
    let compressed = tx.compress_with(ctx).await.expect("compression failed");

    let postcard_compressed =
        postcard::to_stdvec(&compressed).expect("failed to serialize");
    let postcard_decompressed =
        postcard::from_bytes(&postcard_compressed).expect("failed to deserialize");
    pretty_assertions::assert_eq!(compressed, postcard_decompressed);

    let decompressed = postcard_decompressed
        .decompress(ctx)
        .await
        .expect("decompression failed");
    pretty_assertions::assert_eq!(tx, decompressed);
}

const NUMBER_CASES: usize = 100;

#[tokio::test]
async fn can_decompress_compressed_transaction_mint() {
    for mut tx in TransactionFactory::<_, Mint>::from_seed(1234).take(NUMBER_CASES) {
        let mut ctx = TestCompressionCtx::store_data_for_mint(&tx);
        tx.prepare_sign();
        verify_tx_roundtrip(tx.into(), &mut ctx).await;
    }
}

async fn assert_can_decompress_compressed_transaction<Tx, Iterator>(iterator: Iterator)
where
    Iterator: core::iter::Iterator<Item = Tx>,
    Tx: PrepareSign + field::Inputs + Clone + Into<Transaction>,
{
    let mut ctx = TestCompressionCtx::default();
    let txs = iterator
        .take(NUMBER_CASES)
        .map(|mut tx| {
            tx.prepare_sign();
            tx
        })
        .collect::<Vec<_>>();
    for tx in txs.iter() {
        ctx.store_tx_info(tx);
        verify_tx_roundtrip(tx.clone().into(), &mut ctx).await;
    }

    // Given
    let original_ctx = ctx.clone();

    // When
    for tx in txs.iter() {
        verify_tx_roundtrip(tx.clone().into(), &mut ctx).await;
    }

    // Then
    assert_eq!(original_ctx, ctx);
}

#[tokio::test]
async fn can_decompress_compressed_transaction_script() {
    let iter = TransactionFactory::<_, Script>::from_seed(1234).map(|(tx, _)| tx);
    assert_can_decompress_compressed_transaction(iter).await;
}

#[tokio::test]
async fn can_decompress_compressed_transaction_create() {
    let iter = TransactionFactory::<_, Create>::from_seed(1234).map(|(tx, _)| tx);
    assert_can_decompress_compressed_transaction(iter).await;
}

#[tokio::test]
async fn can_decompress_compressed_transaction_upgrade() {
    let iter = TransactionFactory::<_, Upgrade>::from_seed(1234).map(|(tx, _)| tx);
    assert_can_decompress_compressed_transaction(iter).await;
}

#[tokio::test]
async fn can_decompress_compressed_transaction_upload() {
    let iter = TransactionFactory::<_, Upload>::from_seed(1234).map(|(tx, _)| tx);
    assert_can_decompress_compressed_transaction(iter).await;
}

#[tokio::test]
async fn can_decompress_compressed_transaction_blob() {
    let iter = TransactionFactory::<_, Blob>::from_seed(1234).map(|(tx, _)| tx);
    assert_can_decompress_compressed_transaction(iter).await;
}
