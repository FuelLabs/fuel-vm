use crate::{Input, Output, StorageSlot, Transaction, Witness};

use fuel_crypto::SecretKey;
use fuel_types::{Salt, Word};

use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct TransactionBuilder<'a> {
    tx: Transaction,

    // We take the key by reference so this lib won't have the responsibility to properly zeroize
    // the keys
    sign_keys: Vec<&'a SecretKey>,
}

impl<'a> TransactionBuilder<'a> {
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

        let sign_keys = Vec::new();

        Self { tx, sign_keys }
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
        let sign_keys = Vec::new();

        Self { tx, sign_keys }
    }

    pub fn sign_keys(&self) -> &[&SecretKey] {
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
        secret: &'a SecretKey,
        utxo_id: crate::UtxoId,
        amount: Word,
        asset_id: fuel_types::AssetId,
        maturity: Word,
    ) -> &mut Self {
        let pk = secret.public_key();

        self.sign_keys.push(secret);
        self.tx
            .add_unsigned_coin_input(utxo_id, &pk, amount, asset_id, maturity);

        self
    }

    #[cfg(feature = "std")]
    pub fn add_unsigned_message_input(
        &mut self,
        secret: &'a SecretKey,
        sender: fuel_types::Address,
        recipient: fuel_types::Address,
        nonce: Word,
        amount: Word,
        data: Vec<u8>,
    ) -> &mut Self {
        let pk = secret.public_key();

        self.sign_keys.push(secret);
        self.tx
            .add_unsigned_message_input(sender, recipient, nonce, &pk, amount, data);

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

    #[cfg(feature = "std")]
    pub fn finalize(&mut self) -> Transaction {
        let mut tx = core::mem::take(&mut self.tx);

        self.sign_keys.iter().for_each(|k| tx.sign_inputs(k));

        tx
    }
}
