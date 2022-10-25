use crate::transaction::field::{BytecodeLength, BytecodeWitnessIndex, Witnesses};
use crate::transaction::{field, Chargeable, Create, Executable, Script, Signable};
use crate::{
    Cacheable, Checked, ConsensusParameters, Input, IntoChecked, Output, StorageSlot, Transaction,
    TxPointer, Witness,
};

use fuel_crypto::SecretKey;
use fuel_types::{Salt, Word};

use alloc::vec::Vec;

pub trait Buildable
where
    Self: Default
        + Clone
        + Cacheable
        + Executable
        + Chargeable
        + Signable
        + field::GasLimit
        + field::GasPrice
        + field::Maturity
        + IntoChecked
        + Into<Transaction>,
{
    /// Append an input to the transaction
    fn add_input(&mut self, input: Input) {
        self.inputs_mut().push(input)
    }

    /// Append an output to the transaction
    fn add_output(&mut self, output: Output) {
        self.outputs_mut().push(output)
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
    fn set_maturity(&mut self, maturity: Word) {
        *self.maturity_mut() = maturity;
    }
}

impl<T> Buildable for T where
    Self: Default
        + Clone
        + Cacheable
        + Executable
        + Chargeable
        + Signable
        + field::GasPrice
        + field::GasLimit
        + field::Maturity
        + IntoChecked
        + Into<Transaction>
{
}

#[derive(Debug, Clone)]
pub struct TransactionBuilder<Tx: Buildable> {
    tx: Tx,

    should_prepare_script: bool,
    should_prepare_predicate: bool,

    // We take the key by reference so this lib won't have the responsibility to properly zeroize
    // the keys
    sign_keys: Vec<SecretKey>,
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
    pub fn create(bytecode: Witness, salt: Salt, storage_slots: Vec<StorageSlot>) -> Self {
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

impl<Tx: Buildable> TransactionBuilder<Tx> {
    fn with_tx(tx: Tx) -> Self {
        let should_prepare_script = false;
        let should_prepare_predicate = false;
        let sign_keys = Vec::new();

        Self {
            tx,
            should_prepare_script,
            should_prepare_predicate,
            sign_keys,
        }
    }

    pub fn prepare_script(&mut self, should_prepare_script: bool) -> &mut Self {
        self.should_prepare_script = should_prepare_script;
        self
    }

    pub fn prepare_predicate(&mut self, should_prepare_predicate: bool) -> &mut Self {
        self.should_prepare_predicate = should_prepare_predicate;
        self
    }

    pub fn sign_keys(&self) -> &[SecretKey] {
        self.sign_keys.as_slice()
    }

    pub fn gas_price(&mut self, gas_price: Word) -> &mut Self {
        self.tx.set_gas_price(gas_price);

        self
    }

    pub fn gas_limit(&mut self, gas_limit: Word) -> &mut Self {
        self.tx.set_gas_limit(gas_limit);

        self
    }

    pub fn maturity(&mut self, maturity: Word) -> &mut Self {
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
        maturity: Word,
    ) -> &mut Self {
        let pk = secret.public_key();

        self.sign_keys.push(secret);
        self.tx
            .add_unsigned_coin_input(utxo_id, &pk, amount, asset_id, tx_pointer, maturity);

        self
    }

    #[cfg(feature = "std")]
    pub fn add_unsigned_message_input(
        &mut self,
        secret: SecretKey,
        sender: fuel_types::Address,
        nonce: Word,
        amount: Word,
        data: Vec<u8>,
    ) -> &mut Self {
        let pk = secret.public_key();
        self.sign_keys.push(secret);

        let recipient = Input::owner(&pk);

        self.tx
            .add_unsigned_message_input(sender, recipient, nonce, amount, data);

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

    pub fn add_output(&mut self, output: Output) -> &mut Self {
        self.tx.add_output(output);

        self
    }

    pub fn add_witness(&mut self, witness: Witness) -> &mut Self {
        self.tx.add_witness(witness);

        self
    }

    fn prepare_finalize(&mut self) {
        if self.should_prepare_predicate {
            self.tx.prepare_init_predicate();
        }

        if self.should_prepare_script {
            self.tx.prepare_init_script();
        }
    }

    #[cfg(feature = "std")]
    pub fn finalize(&mut self) -> Tx {
        self.prepare_finalize();

        let mut tx = core::mem::take(&mut self.tx);

        self.sign_keys.iter().for_each(|k| tx.sign_inputs(k));

        tx.precompute();

        tx
    }

    #[cfg(feature = "std")]
    pub fn finalize_as_transaction(&mut self) -> Transaction {
        self.finalize().into()
    }

    #[cfg(feature = "std")]
    pub fn finalize_without_signature(&mut self) -> Tx {
        self.prepare_finalize();

        let mut tx = core::mem::take(&mut self.tx);

        tx.precompute();

        tx
    }

    #[cfg(feature = "std")]
    pub fn finalize_without_signature_as_transaction(&mut self) -> Transaction {
        self.finalize_without_signature().into()
    }

    #[cfg(feature = "std")]
    pub fn finalize_checked(&mut self, height: Word, params: &ConsensusParameters) -> Checked<Tx> {
        self.finalize()
            .into_checked(height, params)
            .expect("failed to check tx")
    }

    #[cfg(feature = "std")]
    pub fn finalize_checked_basic(
        &mut self,
        height: Word,
        params: &ConsensusParameters,
    ) -> Checked<Tx> {
        self.finalize()
            .into_checked_basic(height, params)
            .expect("failed to check tx")
    }
}
