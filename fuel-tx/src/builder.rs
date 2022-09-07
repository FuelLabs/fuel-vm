use crate::{
    CheckedTransaction, ConsensusParameters, Input, Output, StorageSlot, Transaction, TxPointer,
    Witness,
};

use fuel_crypto::SecretKey;
use fuel_types::{Salt, Word};

use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct TransactionBuilder {
    tx: Transaction,

    should_prepare_script: bool,
    should_prepare_predicate: bool,

    // We take the key by reference so this lib won't have the responsibility to properly zeroize
    // the keys
    sign_keys: Vec<SecretKey>,
}

impl TransactionBuilder {
    fn with_tx(tx: Transaction) -> Self {
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

    pub fn create(bytecode: Witness, salt: Salt, storage_slots: Vec<StorageSlot>) -> Self {
        let mut tx = Transaction::create(
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
            salt,
            storage_slots,
            Default::default(),
            Default::default(),
            Default::default(),
        );

        tx._set_bytecode(bytecode);

        Self::with_tx(tx)
    }

    pub fn script(script: Vec<u8>, script_data: Vec<u8>) -> Self {
        let tx = Transaction::script(
            Default::default(),
            Default::default(),
            Default::default(),
            script,
            script_data,
            Default::default(),
            Default::default(),
            Default::default(),
        );

        let mut slf = Self::with_tx(tx);

        slf.prepare_script(true);

        slf
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
            self.tx
                .prepare_init_script()
                .expect("failed to prepare script");
        }
    }

    #[cfg(feature = "std")]
    pub fn finalize(&mut self) -> Transaction {
        self.prepare_finalize();

        let mut tx = core::mem::take(&mut self.tx);

        self.sign_keys.iter().for_each(|k| tx.sign_inputs(k));

        tx.precompute_metadata();

        tx
    }

    #[cfg(feature = "std")]
    pub fn finalize_without_signature(&mut self) -> Transaction {
        self.prepare_finalize();

        let mut tx = core::mem::take(&mut self.tx);

        tx.precompute_metadata();

        tx
    }

    #[cfg(feature = "std")]
    pub fn finalize_checked(
        &mut self,
        height: Word,
        params: &ConsensusParameters,
    ) -> CheckedTransaction {
        self.finalize()
            .check(height, params)
            .expect("failed to check tx")
    }

    #[cfg(feature = "std")]
    pub fn finalize_checked_without_signature(
        &mut self,
        height: Word,
        params: &ConsensusParameters,
    ) -> CheckedTransaction {
        self.finalize_without_signature()
            .check_without_signature(height, params)
            .expect("failed to check tx")
    }
}
