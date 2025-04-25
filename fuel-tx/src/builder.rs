use crate::{
    Blob,
    BlobBody,
    ConsensusParameters,
    ContractParameters,
    CreateMetadata,
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
    Upgrade,
    UpgradePurpose,
    Upload,
    UploadBody,
    Witness,
    input,
    output,
    transaction::{
        Chargeable,
        Create,
        Executable,
        Script,
        field::{
            self,
            BytecodeWitnessIndex,
            Expiration,
            Maturity,
            Tip,
            Witnesses,
        },
    },
};

use crate::{
    Cacheable,
    Signable,
};

use crate::{
    field::{
        MaxFeeLimit,
        Outputs,
        WitnessLimit,
    },
    policies::Policies,
    transaction::{
        CreateBody,
        ScriptBody,
        UpgradeBody,
    },
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
#[cfg(feature = "rand")]
use rand::{
    Rng,
    rngs::StdRng,
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

    params: ConsensusParameters,

    // We take the key by reference so this lib won't have the responsibility to properly
    // zeroize the keys
    // Maps signing keys -> witness indexes
    sign_keys: BTreeMap<SecretKey, u16>,
}

impl TransactionBuilder<Script> {
    pub fn script(script: Vec<u8>, script_data: Vec<u8>) -> Self {
        let tx = Script {
            body: ScriptBody {
                script_gas_limit: Default::default(),
                receipts_root: Default::default(),
                script: script.into(),
                script_data,
            },
            policies: Policies::new().with_max_fee(0),
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            metadata: None,
        };
        Self::with_tx(tx)
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
            body: CreateBody {
                bytecode_witness_index: Default::default(),
                salt,
                storage_slots,
            },
            policies: Policies::new().with_max_fee(0),
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            metadata: None,
        };

        *tx.bytecode_witness_index_mut() = 0;

        tx.witnesses_mut().push(bytecode);

        Self::with_tx(tx)
    }

    pub fn add_contract_created(&mut self) -> &mut Self {
        let create_metadata = CreateMetadata::compute(&self.tx)
            .expect("Should be able to compute metadata");

        self.tx.outputs_mut().push(Output::contract_created(
            create_metadata.contract_id,
            create_metadata.state_root,
        ));
        self
    }
}

impl TransactionBuilder<Upgrade> {
    pub fn upgrade(purpose: UpgradePurpose) -> Self {
        let tx = Upgrade {
            body: UpgradeBody { purpose },
            policies: Policies::new().with_max_fee(0),
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            metadata: None,
        };
        Self::with_tx(tx)
    }
}

impl TransactionBuilder<Upload> {
    pub fn upload(body: UploadBody) -> Self {
        let tx = Upload {
            body,
            policies: Policies::new().with_max_fee(0),
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            metadata: None,
        };
        Self::with_tx(tx)
    }
}

impl TransactionBuilder<Blob> {
    pub fn blob(body: BlobBody) -> Self {
        let tx = Blob {
            body,
            policies: Policies::new().with_max_fee(0),
            inputs: Default::default(),
            outputs: Default::default(),
            witnesses: Default::default(),
            metadata: None,
        };
        Self::with_tx(tx)
    }
}

impl TransactionBuilder<Mint> {
    pub fn mint(
        block_height: BlockHeight,
        #[cfg(feature = "u32-tx-pointer")] tx_index: u32,
        #[cfg(not(feature = "u32-tx-pointer"))] tx_index: u16,
        input_contract: input::contract::Contract,
        output_contract: output::contract::Contract,
        mint_amount: Word,
        mint_asset_id: AssetId,
        gas_price: Word,
    ) -> Self {
        let tx = Mint {
            tx_pointer: TxPointer::new(block_height, tx_index),
            input_contract,
            output_contract,
            mint_amount,
            mint_asset_id,
            gas_price,
            metadata: None,
        };

        Self::with_tx(tx)
    }
}

impl<Tx> TransactionBuilder<Tx> {
    fn with_tx(tx: Tx) -> Self {
        let sign_keys = BTreeMap::new();

        Self {
            tx,
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
        self.params.set_tx_params(tx_params);
        self
    }

    pub fn with_predicate_params(
        &mut self,
        predicate_params: PredicateParameters,
    ) -> &mut Self {
        self.params.set_predicate_params(predicate_params);
        self
    }

    pub fn with_script_params(&mut self, script_params: ScriptParameters) -> &mut Self {
        self.params.set_script_params(script_params);
        self
    }

    pub fn with_contract_params(
        &mut self,
        contract_params: ContractParameters,
    ) -> &mut Self {
        self.params.set_contract_params(contract_params);
        self
    }

    pub fn with_fee_params(&mut self, fee_params: FeeParameters) -> &mut Self {
        self.params.set_fee_params(fee_params);
        self
    }

    pub fn with_chain_id(&mut self, chain_id: ChainId) -> &mut Self {
        self.params.set_chain_id(chain_id);
        self
    }

    pub fn with_gas_costs(&mut self, gas_costs: GasCosts) -> &mut Self {
        self.params.set_gas_costs(gas_costs);
        self
    }

    pub fn with_base_asset_id(&mut self, base_asset_id: AssetId) -> &mut Self {
        self.params.set_base_asset_id(base_asset_id);
        self
    }

    pub fn with_block_gas_limit(&mut self, block_gas_limit: u64) -> &mut Self {
        self.params.set_block_gas_limit(block_gas_limit);
        self
    }
}

impl<Tx: Buildable> TransactionBuilder<Tx> {
    pub fn sign_keys(&self) -> impl Iterator<Item = &SecretKey> {
        self.sign_keys.keys()
    }

    pub fn tip(&mut self, tip: Word) -> &mut Self {
        self.tx.set_tip(tip);
        self
    }

    pub fn script_gas_limit(&mut self, gas_limit: Word) -> &mut Self
    where
        Tx: field::ScriptGasLimit,
    {
        self.tx.set_script_gas_limit(gas_limit);

        self
    }

    pub fn maturity(&mut self, maturity: BlockHeight) -> &mut Self {
        self.tx.set_maturity(maturity);

        self
    }

    pub fn expiration(&mut self, expiration: BlockHeight) -> &mut Self {
        self.tx.set_expiration(expiration);

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
    ) -> &mut Self {
        let pk = secret.public_key();

        let witness_index = self.upsert_secret(secret);

        self.tx.add_unsigned_coin_input(
            utxo_id,
            &pk,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
        );

        self
    }


    pub fn add_unsigned_data_coin_input(
        &mut self,
        secret: SecretKey,
        utxo_id: crate::UtxoId,
        amount: Word,
        asset_id: fuel_types::AssetId,
        tx_pointer: TxPointer,
    ) -> &mut Self {
        let pk = secret.public_key();

        let witness_index = self.upsert_secret(secret);

        self.tx.add_unsigned_coin_input(
            utxo_id,
            &pk,
            amount,
            asset_id,
            tx_pointer,
            witness_index,
        );

        self
    }

    #[cfg(feature = "rand")]
    pub fn add_random_fee_input(&mut self, rng: &mut StdRng) -> &mut Self {
        self.add_unsigned_coin_input(
            SecretKey::random(rng),
            rng.r#gen(),
            u32::MAX as u64,
            *self.params.base_asset_id(),
            Default::default(),
        )
    }

    #[cfg(feature = "rand")]
    pub fn add_fee_input(&mut self) -> &mut Self {
        use rand::{
            Rng,
            SeedableRng,
        };
        let mut rng = rand::rngs::StdRng::seed_from_u64(2322u64);
        self.add_unsigned_coin_input(
            SecretKey::random(&mut rng),
            rng.r#gen(),
            u32::MAX as u64,
            *self.params.base_asset_id(),
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
    fn upsert_secret(&mut self, secret_key: SecretKey) -> u16 {
        let witness_len = u16::try_from(self.witnesses().len())
            .expect("The number of witnesses can't exceed `u16::MAX`");

        if u32::from(witness_len) > self.params.tx_params().max_witnesses() {
            panic!("Max witnesses exceeded");
        }

        let witness_index = self.sign_keys.entry(secret_key).or_insert_with(|| {
            // if this private key hasn't been used before,
            // add a new witness entry and return its index
            self.tx.witnesses_mut().push(Witness::default());
            witness_len
        });
        *witness_index
    }

    fn finalize_inner(&self) -> Tx {
        let mut tx = self.tx.clone();

        self.sign_keys
            .iter()
            .for_each(|(k, _)| tx.sign_inputs(k, &self.get_chain_id()));

        tx.precompute(&self.get_chain_id())
            .expect("Should be able to calculate cache");

        tx
    }

    pub fn finalize_without_signature_inner(&self) -> Tx {
        let mut tx = self.tx.clone();

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
    fn finalize(&self) -> Tx;

    fn finalize_without_signature(&self) -> Tx;
}

impl Finalizable<Mint> for TransactionBuilder<Mint> {
    fn finalize(&self) -> Mint {
        let mut tx = self.tx.clone();
        tx.precompute(&self.get_chain_id())
            .expect("Should be able to calculate cache");
        tx
    }

    fn finalize_without_signature(&self) -> Mint {
        self.finalize()
    }
}

impl<Tx> Finalizable<Tx> for TransactionBuilder<Tx>
where
    Tx: Buildable,
{
    fn finalize(&self) -> Tx {
        self.finalize_inner()
    }

    fn finalize_without_signature(&self) -> Tx {
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
