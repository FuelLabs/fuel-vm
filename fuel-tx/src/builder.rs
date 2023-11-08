use crate::{
    input,
    output,
    transaction::{
        field,
        field::{
            BytecodeLength,
            BytecodeWitnessIndex,
            GasPrice,
            Maturity,
            Witnesses,
        },
        Chargeable,
        Create,
        Executable,
        Script,
    },
    ConsensusParameters,
    ContractParameters,
    FeeParameters,
    GasCosts,
    Input,
    Mint,
    Output,
    PredicateParameters,
    ScriptParameters,
    StorageSlot,
    Transaction,
    TxParameters,
    TxPointer,
    Witness,
};

use crate::{
    Cacheable,
    Signable,
};

use crate::{
    field::{
        MaxFeeLimit,
        WitnessLimit,
    },
    policies::Policies,
};
use alloc::{
    collections::BTreeMap,
    vec::Vec,
};
use fuel_crypto::SecretKey;
use fuel_types::{
    AssetId,
    BlockHeight,
    ChainId,
    Nonce,
    Salt,
    Word,
};

pub trait BuildableAloc
where
    Self: Default + Clone + Executable + Chargeable + field::Policies + Into<Transaction>,
{
}

pub trait BuildableStd: Signable + Cacheable {}

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

    /// Set the `Script`'s gas limit
    fn set_script_gas_limit(&mut self, limit: Word)
    where
        Self: field::ScriptGasLimit,
    {
        *self.script_gas_limit_mut() = limit;
    }
}

impl<T> BuildableAloc for T where
    Self: Default + Clone + Executable + Chargeable + field::Policies + Into<Transaction>
{
}

impl<T> BuildableStd for T where T: Signable + Cacheable {}
impl<T> BuildableSet for T where T: BuildableAloc + BuildableStd {}
impl<T> Buildable for T where T: BuildableSet {}

#[derive(Debug, Clone)]
pub struct TransactionBuilder<Tx> {
    tx: Tx,

    should_prepare_script: bool,
    should_prepare_predicate: bool,
    params: ConsensusParameters,

    // We take the key by reference so this lib won't have the responsibility to properly
    // zeroize the keys
    // Maps signing keys -> witness indexes
    sign_keys: BTreeMap<SecretKey, u8>,
}

impl TransactionBuilder<Script> {
    pub fn script(script: Vec<u8>, script_data: Vec<u8>) -> Self {
        let tx = Script {
            gas_limit: Default::default(),
            script,
            script_data,
            policies: Policies::new().with_gas_price(0),
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
            bytecode_length: Default::default(),
            bytecode_witness_index: Default::default(),
            salt,
            storage_slots,
            policies: Policies::new().with_gas_price(0),
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            metadata: None,
        };

        *tx.bytecode_length_mut() = (bytecode.as_ref().len() / 4) as Word;
        *tx.bytecode_witness_index_mut() = 0;

        tx.witnesses_mut().push(bytecode);

        Self::with_tx(tx)
    }
}

impl TransactionBuilder<Mint> {
    pub fn mint(
        block_height: BlockHeight,
        tx_index: u16,
        input_contract: input::contract::Contract,
        output_contract: output::contract::Contract,
        mint_amount: Word,
        mint_asset_id: AssetId,
    ) -> Self {
        let tx = Mint {
            tx_pointer: TxPointer::new(block_height, tx_index),
            input_contract,
            output_contract,
            mint_amount,
            mint_asset_id,
            metadata: None,
        };

        Self::with_tx(tx)
    }
}

impl<Tx> TransactionBuilder<Tx> {
    fn with_tx(tx: Tx) -> Self {
        let should_prepare_script = false;
        let should_prepare_predicate = false;
        let sign_keys = BTreeMap::new();

        Self {
            tx,
            should_prepare_script,
            should_prepare_predicate,
            params: ConsensusParameters::standard(),
            sign_keys,
        }
    }

    pub fn get_params(&self) -> &ConsensusParameters {
        &self.params
    }

    pub fn get_tx_params(&self) -> &TxParameters {
        self.params.tx_params()
    }

    pub fn get_predicate_params(&self) -> &PredicateParameters {
        self.params.predicate_params()
    }

    pub fn get_script_params(&self) -> &ScriptParameters {
        self.params.script_params()
    }

    pub fn get_contract_params(&self) -> &ContractParameters {
        self.params.contract_params()
    }

    pub fn get_fee_params(&self) -> &FeeParameters {
        self.params.fee_params()
    }

    pub fn get_chain_id(&self) -> ChainId {
        self.params.chain_id()
    }

    pub fn with_params(&mut self, params: ConsensusParameters) -> &mut Self {
        self.params = params;
        self
    }

    pub fn with_tx_params(&mut self, tx_params: TxParameters) -> &mut Self {
        self.params.tx_params = tx_params;
        self
    }

    pub fn with_predicate_params(
        &mut self,
        predicate_params: PredicateParameters,
    ) -> &mut Self {
        self.params.predicate_params = predicate_params;
        self
    }

    pub fn with_script_params(&mut self, script_params: ScriptParameters) -> &mut Self {
        self.params.script_params = script_params;
        self
    }

    pub fn with_contract_params(
        &mut self,
        contract_params: ContractParameters,
    ) -> &mut Self {
        self.params.contract_params = contract_params;
        self
    }

    pub fn with_fee_params(&mut self, fee_params: FeeParameters) -> &mut Self {
        self.params.fee_params = fee_params;
        self
    }

    pub fn with_base_asset_id(&mut self, base_asset_id: AssetId) -> &mut Self {
        self.params.base_asset_id = base_asset_id;
        self
    }

    pub fn with_gas_costs(&mut self, gas_costs: GasCosts) -> &mut Self {
        self.params.gas_costs = gas_costs;
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

    pub fn script_gas_limit(&mut self, gas_limit: Word) -> &mut Self
    where
        Tx: field::ScriptGasLimit,
    {
        self.tx.set_script_gas_limit(gas_limit);

        self
    }

    pub fn with_chain_id(&mut self, chain_id: ChainId) -> &mut Self {
        self.params.chain_id = chain_id;
        self
    }

    pub fn maturity(&mut self, maturity: BlockHeight) -> &mut Self {
        self.tx.set_maturity(maturity);

        self
    }

    pub fn witness_limit(&mut self, witness_limit: Word) -> &mut Self {
        self.tx.set_witness_limit(witness_limit);

        self
    }

    pub fn max_fee_limit(&mut self, max_fee: Word) -> &mut Self {
        self.tx.set_max_fee_limit(max_fee);

        self
    }

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

    #[cfg(feature = "rand")]
    pub fn add_random_fee_input(&mut self) -> &mut Self {
        use rand::{
            Rng,
            SeedableRng,
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(2322u64);
        self.add_unsigned_coin_input(
            SecretKey::random(&mut rng),
            rng.gen(),
            rng.gen(),
            rng.gen(),
            Default::default(),
            Default::default(),
        )
    }

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
    fn upsert_secret(&mut self, secret_key: SecretKey) -> u8 {
        let witness_len = u8::try_from(self.witnesses().len())
            .expect("The number of witnesses can't exceed `u8::MAX`");

        let witness_index = self.sign_keys.entry(secret_key).or_insert_with(|| {
            // if this private key hasn't been used before,
            // add a new witness entry and return its index
            self.tx.witnesses_mut().push(Witness::default());
            witness_len
        });
        *witness_index
    }

    fn prepare_finalize(&mut self) {
        if self.should_prepare_predicate {
            self.tx.prepare_init_predicate();
        }

        if self.should_prepare_script {
            self.tx.prepare_init_script();
        }
    }

    fn finalize_inner(&mut self) -> Tx {
        self.prepare_finalize();

        let mut tx = core::mem::take(&mut self.tx);

        self.sign_keys
            .iter()
            .for_each(|(k, _)| tx.sign_inputs(k, &self.get_chain_id()));

        tx.precompute(&self.get_chain_id())
            .expect("Should be able to calculate cache");

        tx
    }

    pub fn finalize_without_signature_inner(&mut self) -> Tx {
        self.prepare_finalize();

        let mut tx = core::mem::take(&mut self.tx);

        tx.precompute(&self.get_chain_id())
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

impl Finalizable<Mint> for TransactionBuilder<Mint> {
    fn finalize(&mut self) -> Mint {
        let mut tx = core::mem::take(&mut self.tx);
        tx.precompute(&self.get_chain_id())
            .expect("Should be able to calculate cache");
        tx
    }

    fn finalize_without_signature(&mut self) -> Mint {
        self.finalize()
    }
}

impl Finalizable<Create> for TransactionBuilder<Create> {
    fn finalize(&mut self) -> Create {
        self.finalize_inner()
    }

    fn finalize_without_signature(&mut self) -> Create {
        self.finalize_without_signature_inner()
    }
}

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
    pub fn finalize_as_transaction(&mut self) -> Transaction {
        self.finalize().into()
    }

    pub fn finalize_without_signature_as_transaction(&mut self) -> Transaction {
        self.finalize_without_signature().into()
    }
}
