//! In-memory client implementation

use crate::{
    backtrace::Backtrace,
    checked_transaction::Checked,
    gas::GasCosts,
    state::StateTransitionRef,
    storage::MemoryStorage,
    transactor::Transactor,
};

use crate::interpreter::InterpreterParams;
use fuel_tx::{
    ContractParameters,
    Create,
    FeeParameters,
    PredicateParameters,
    Receipt,
    Script,
    TxParameters,
};
use fuel_types::ChainId;

#[derive(Default, Debug)]
/// Client implementation with in-memory storage backend.
pub struct MemoryClient {
    transactor: Transactor<MemoryStorage, Script>,
}

impl AsRef<MemoryStorage> for MemoryClient {
    fn as_ref(&self) -> &MemoryStorage {
        self.transactor.as_ref()
    }
}

impl AsMut<MemoryStorage> for MemoryClient {
    fn as_mut(&mut self) -> &mut MemoryStorage {
        self.transactor.as_mut()
    }
}

impl MemoryClient {
    /// Create a new instance of the memory client out of a provided storage.
    pub fn new(storage: MemoryStorage, interpreter_params: InterpreterParams) -> Self {
        Self {
            transactor: Transactor::new(storage, interpreter_params),
        }
    }

    /// Create a new instance of the memory client out of a provided storage.
    pub fn from_txtor(transactor: Transactor<MemoryStorage, Script>) -> Self {
        Self { transactor }
    }

    /// If a transaction was executed and produced a VM panic, returns the
    /// backtrace; return `None` otherwise.
    pub fn backtrace(&self) -> Option<Backtrace> {
        self.transactor.backtrace()
    }

    /// If a transaction was successfully executed, returns the produced
    /// receipts; return `None` otherwise.
    pub fn receipts(&self) -> Option<&[Receipt]> {
        self.transactor.receipts()
    }

    /// State transition representation after the execution of a transaction.
    pub fn state_transition(&self) -> Option<StateTransitionRef<'_, Script>> {
        self.transactor.state_transition()
    }

    /// Deploys a `Create` transaction.
    pub fn deploy(&mut self, tx: Checked<Create>) -> Option<Create> {
        self.transactor.deploy(tx).ok()
    }

    /// Execute a transaction.
    ///
    /// Since the memory storage is `Infallible`, associatively, the memory
    /// client should also be.
    pub fn transact(&mut self, tx: Checked<Script>) -> &[Receipt] {
        self.transactor.transact(tx);

        // TODO `Transactor::result` should accept error as generic so compile-time
        // constraints can be applied.
        //
        // In this case, we should expect `Infallible` error.
        if let Ok(state) = self.transactor.result() {
            if state.should_revert() {
                self.transactor.as_mut().revert();
            } else {
                self.transactor.as_mut().commit();
            }
        } else {
            // if vm failed to execute, revert storage just in case
            self.transactor.as_mut().revert();
        }

        self.transactor.receipts().unwrap_or_default()
    }

    /// Persist the changes caused by [`Self::transact`].
    pub fn persist(&mut self) {
        self.as_mut().persist();
    }

    // /// Consensus parameters
    // pub const fn params(&self) -> &ConsensusParameters {
    //     self.transactor.params()
    // }

    /// Tx memory offset
    pub const fn tx_offset(&self) -> usize {
        self.transactor.tx_offset()
    }

    /// Gas costs for opcodes
    pub fn gas_costs(&self) -> &GasCosts {
        self.transactor.gas_costs()
    }
}

impl From<MemoryStorage> for MemoryClient {
    fn from(s: MemoryStorage) -> Self {
        let interpreter_params = InterpreterParams {
            gas_costs: Default::default(),
            max_inputs: TxParameters::DEFAULT.max_inputs,
            contract_max_size: ContractParameters::DEFAULT.contract_max_size,
            tx_offset: TxParameters::default().tx_offset(),
            max_message_data_length: PredicateParameters::DEFAULT.max_message_data_length,
            chain_id: ChainId::default(),
            fee_params: FeeParameters::default(),
        };
        Self::new(s, interpreter_params)
    }
}

impl From<MemoryClient> for Transactor<MemoryStorage, Script> {
    fn from(client: MemoryClient) -> Self {
        client.transactor
    }
}
