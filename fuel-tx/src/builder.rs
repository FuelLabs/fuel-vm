use crate::{
    transaction::{
        field,
        field::{
            BytecodeLength,
            BytecodeWitnessIndex,
            Witnesses,
        },
        Chargeable,
        Create,
        Executable,
        Script,
    },
    Cacheable,
    ConsensusParameters,
    Input,
    Mint,
    Output,
    StorageSlot,
    Transaction,
    TxPointer,
    Witness,
};

#[cfg(feature = "std")]
use crate::Signable;

use fuel_crypto::SecretKey;
use fuel_types::{
    BlockHeight,
    Nonce,
    Salt,
    Word,
};

use alloc::vec::Vec;
use std::collections::HashMap;

pub trait BuildableAloc
where
    Self: Default
        + Clone
        + Executable
        + Chargeable
        + field::GasLimit
        + field::GasPrice
        + field::Maturity
        + Into<Transaction>,
{
}

#[cfg(feature = "std")]
pub trait BuildableStd: Signable + Cacheable {}

#[cfg(not(feature = "std"))]
pub trait BuildableSet: BuildableAloc {}

#[cfg(feature = "std")]
pub trait BuildableSet: BuildableAloc + BuildableStd {}

pub trait Buildable
where
    Self: BuildableSet,
{
    /// Append an input to the transaction
    fn add_input(&mut self, input: Input) {
        self.inputs_mut().push(input)
    }

    /// Append a witness to the transaction
    fn add_witness(&mut self, witness: Witness) {
        self.witnesses_mut().push(witness);
    }

    /// Set the gas price
    fn set_gas_price(&mut self, price: Word) {
        *self.gas_price_mut() = price;
    }

    /// Set the gas limit
    fn set_gas_limit(&mut self, limit: Word) {
        *self.gas_limit_mut() = limit;
    }

    /// Set the maturity
    fn set_maturity(&mut self, maturity: BlockHeight) {
        *self.maturity_mut() = maturity;
    }
}

impl<T> BuildableAloc for T where
    Self: Default
        + Clone
        + Executable
        + Chargeable
        + field::GasLimit
        + field::GasPrice
        + field::Maturity
        + Into<Transaction>
{
}

#[cfg(feature = "std")]
impl<T> BuildableStd for T where T: Signable + Cacheable {}
#[cfg(feature = "std")]
impl<T> BuildableSet for T where T: BuildableAloc + BuildableStd {}
#[cfg(not(feature = "std"))]
impl<T> BuildableSet for T where T: BuildableAloc {}
impl<T> Buildable for T where T: BuildableSet {}

#[derive(Debug, Clone)]
pub struct TransactionBuilder<Tx> {
    tx: Tx,

    should_prepare_script: bool,
    should_prepare_predicate: bool,
    parameters: ConsensusParameters,

    // We take the key by reference so this lib won't have the responsibility to properly
    // zeroize the keys
    // Maps signing keys -> witness indexes
    sign_keys: HashMap<SecretKey, u8>,
}

impl TransactionBuilder<Script> {
    pub fn script(script: Vec<u8>, script_data: Vec<u8>) -> Self {
        let tx = Script {
            gas_price: Default::default(),
            gas_limit: Default::default(),
            maturity: Default::default(),
            script,
            script_data,
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            receipts_root: Default::default(),
            metadata: None,
        };

        let mut slf = Self::with_tx(tx);

        slf.prepare_script(true);

        slf
    }
}

impl TransactionBuilder<Create> {
    pub fn create(
        bytecode: Witness,
        salt: Salt,
        mut storage_slots: Vec<StorageSlot>,
    ) -> Self {
        // sort the storage slots before initializing the builder
        storage_slots.sort();
        let mut tx = Create {
            gas_price: Default::default(),
            gas_limit: Default::default(),
            maturity: Default::default(),
            bytecode_length: Default::default(),
            bytecode_witness_index: Default::default(),
            salt,
            storage_slots,
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            metadata: None,
        };

        *tx.bytecode_length_mut() = (bytecode.as_ref().len() / 4) as Word;
        *tx.bytecode_witness_index_mut() = tx.witnesses().len() as u8;

        tx.witnesses_mut().push(bytecode);

        Self::with_tx(tx)
    }
}

impl TransactionBuilder<Mint> {
    pub fn mint(block_height: BlockHeight, tx_index: u16) -> Self {
        let tx = Mint {
            tx_pointer: TxPointer::new(block_height, tx_index),
            outputs: Default::default(),
            metadata: None,
        };

        Self::with_tx(tx)
    }
}

impl<Tx> TransactionBuilder<Tx> {
    fn with_tx(tx: Tx) -> Self {
        let should_prepare_script = false;
        let should_prepare_predicate = false;
        let sign_keys = HashMap::new();

        Self {
            tx,
            should_prepare_script,
            should_prepare_predicate,
            sign_keys,
            parameters: ConsensusParameters::DEFAULT,
        }
    }

    pub fn get_params(&self) -> &ConsensusParameters {
        &self.parameters
    }

    pub fn with_params(&mut self, parameters: ConsensusParameters) -> &mut Self {
        self.parameters = parameters;
        self
    }
}

impl<Tx: Buildable> TransactionBuilder<Tx> {
    pub fn prepare_script(&mut self, should_prepare_script: bool) -> &mut Self {
        self.should_prepare_script = should_prepare_script;
        self
    }

    pub fn prepare_predicate(&mut self, should_prepare_predicate: bool) -> &mut Self {
        self.should_prepare_predicate = should_prepare_predicate;
        self
    }

    pub fn sign_keys(&self) -> impl Iterator<Item = &SecretKey> {
        self.sign_keys.keys()
    }

    pub fn gas_price(&mut self, gas_price: Word) -> &mut Self {
        self.tx.set_gas_price(gas_price);

        self
    }

    pub fn gas_limit(&mut self, gas_limit: Word) -> &mut Self {
        self.tx.set_gas_limit(gas_limit);

        self
    }

    pub fn maturity(&mut self, maturity: BlockHeight) -> &mut Self {
        self.tx.set_maturity(maturity);

        self
    }

    #[cfg(feature = "std")]
    pub fn add_unsigned_coin_input(
        &mut self,
        secret: SecretKey,
        utxo_id: crate::UtxoId,
        amount: Word,
        asset_id: fuel_types::AssetId,
        tx_pointer: TxPointer,
        maturity: BlockHeight,
    ) -> &mut Self {
        let pk = secret.public_key();

        let witness_index = self.upsert_secret(secret);

        self.tx.add_unsigned_coin_input(
            utxo_id,
            &pk,
            amount,
            asset_id,
            tx_pointer,
            maturity,
            witness_index,
        );

        self
    }

    #[cfg(all(feature = "rand", feature = "std"))]
    pub fn add_random_fee_input(&mut self) -> &mut Self {
        use rand::{
            Rng,
            SeedableRng,
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(2322u64);
        self.add_unsigned_coin_input(
            rng.gen(),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            Default::default(),
            Default::default(),
        )
    }

    #[cfg(feature = "std")]
    pub fn add_unsigned_message_input(
        &mut self,
        secret: SecretKey,
        sender: fuel_types::Address,
        nonce: Nonce,
        amount: Word,
        data: Vec<u8>,
    ) -> &mut Self {
        let pk = secret.public_key();
        let recipient = Input::owner(&pk);

        let witness_index = self.upsert_secret(secret);

        self.tx.add_unsigned_message_input(
            sender,
            recipient,
            nonce,
            amount,
            data,
            witness_index,
        );

        self
    }

    pub fn inputs(&self) -> &[Input] {
        self.tx.inputs()
    }

    pub fn outputs(&self) -> &[Output] {
        self.tx.outputs()
    }

    pub fn witnesses(&self) -> &[Witness] {
        self.tx.witnesses()
    }

    pub fn add_input(&mut self, input: Input) -> &mut Self {
        self.tx.add_input(input);

        self
    }

    pub fn add_witness(&mut self, witness: Witness) -> &mut Self {
        self.tx.add_witness(witness);

        self
    }

    /// Adds a secret to the builder, and adds a corresponding witness if it's a new entry
    #[cfg(feature = "std")]
    fn upsert_secret(&mut self, secret_key: SecretKey) -> u8 {
        let witness_len = self.witnesses().len() as u8;

        let witness_index = self.sign_keys.entry(secret_key).or_insert_with(|| {
            // if this private key hasn't been used before,
            // add a new witness entry and return its index
            self.tx.witnesses_mut().push(Witness::default());
            witness_len
        });
        *witness_index
    }

    #[cfg(feature = "std")]
    fn prepare_finalize(&mut self) {
        if self.should_prepare_predicate {
            self.tx.prepare_init_predicate();
        }

        if self.should_prepare_script {
            self.tx.prepare_init_script();
        }
    }

    #[cfg(feature = "std")]
    fn finalize_inner(&mut self) -> Tx {
        self.prepare_finalize();

        let mut tx = core::mem::take(&mut self.tx);

        self.sign_keys
            .iter()
            .for_each(|(k, _)| tx.sign_inputs(k, &self.parameters.chain_id));

        tx.precompute(&self.parameters.chain_id)
            .expect("Should be able to calculate cache");

        tx
    }

    #[cfg(feature = "std")]
    pub fn finalize_without_signature_inner(&mut self) -> Tx {
        self.prepare_finalize();

        let mut tx = core::mem::take(&mut self.tx);

        tx.precompute(&self.parameters.chain_id)
            .expect("Should be able to calculate cache");

        tx
    }
}

impl<Tx: field::Outputs> TransactionBuilder<Tx> {
    pub fn add_output(&mut self, output: Output) -> &mut Self {
        self.tx.outputs_mut().push(output);
        self
    }
}

pub trait Finalizable<Tx> {
    fn finalize(&mut self) -> Tx;

    fn finalize_without_signature(&mut self) -> Tx;
}

#[cfg(feature = "std")]
impl Finalizable<Mint> for TransactionBuilder<Mint> {
    fn finalize(&mut self) -> Mint {
        let mut tx = core::mem::take(&mut self.tx);
        tx.precompute(&self.parameters.chain_id)
            .expect("Should be able to calculate cache");
        tx
    }

    fn finalize_without_signature(&mut self) -> Mint {
        self.finalize()
    }
}

#[cfg(feature = "std")]
impl Finalizable<Create> for TransactionBuilder<Create> {
    fn finalize(&mut self) -> Create {
        self.finalize_inner()
    }

    fn finalize_without_signature(&mut self) -> Create {
        self.finalize_without_signature_inner()
    }
}

#[cfg(feature = "std")]
impl Finalizable<Script> for TransactionBuilder<Script> {
    fn finalize(&mut self) -> Script {
        self.finalize_inner()
    }

    fn finalize_without_signature(&mut self) -> Script {
        self.finalize_without_signature_inner()
    }
}

impl<Tx> TransactionBuilder<Tx>
where
    Self: Finalizable<Tx>,
    Transaction: From<Tx>,
{
    #[cfg(feature = "std")]
    pub fn finalize_as_transaction(&mut self) -> Transaction {
        self.finalize().into()
    }

    #[cfg(feature = "std")]
    pub fn finalize_without_signature_as_transaction(&mut self) -> Transaction {
        self.finalize_without_signature().into()
    }
}
