//! Exposed constructors API for the [`Interpreter`]

use super::{
    ExecutableTransaction,
    Interpreter,
    RuntimeBalances,
};
use crate::{
    consts::*,
    context::Context,
    gas::GasCosts,
    interpreter::PanicContext,
    state::Debugger,
    storage::MemoryStorage,
};
use fuel_tx::{
    ContractParameters,
    FeeParameters,
    PredicateParameters,
    TxParameters,
};
use fuel_types::ChainId;

#[cfg(feature = "profile-any")]
use crate::profiler::ProfileReceiver;

use crate::profiler::Profiler;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: Default,
{
    /// Create a new interpreter instance out of a storage implementation.
    ///
    /// If the provided storage implements
    /// [`crate::storage::InterpreterStorage`], the returned interpreter
    /// will provide full functionality.
    pub fn with_storage(
        storage: S,
        gas_costs: GasCosts,
        max_inputs: u64,
        contract_max_size: u64,
        tx_offset: usize,
        max_message_data_length: u64,
        chain_id: ChainId,
        fee_params: FeeParameters,
    ) -> Self {
        Self {
            registers: [0; VM_REGISTER_COUNT],
            memory: vec![0; MEM_SIZE]
                .try_into()
                .expect("Failed to allocate memory"),
            frames: vec![],
            receipts: Default::default(),
            tx: Default::default(),
            initial_balances: Default::default(),
            storage,
            debugger: Debugger::default(),
            context: Context::default(),
            balances: RuntimeBalances::default(),
            gas_costs,
            profiler: Profiler::default(),
            fee_params,
            max_inputs,
            contract_max_size,
            tx_offset,
            chain_id,
            max_message_data_length,
            panic_context: PanicContext::None,
        }
    }

    /// Sets a profiler for the VM
    #[cfg(feature = "profile-any")]
    pub fn with_profiler<P>(&mut self, receiver: P) -> &mut Self
    where
        P: ProfileReceiver + Send + Sync + 'static,
    {
        self.profiler.set_receiver(Box::new(receiver));
        self
    }
}

impl<S, Tx> Interpreter<S, Tx>
where
    S: Clone,
    Tx: ExecutableTransaction,
{
    /// Build the interpreter
    pub fn build(&mut self) -> Self {
        self.clone()
    }
}

impl<S, Tx> Default for Interpreter<S, Tx>
where
    S: Default,
    Tx: ExecutableTransaction,
{
    fn default() -> Self {
        let gas_costs = Default::default();
        let max_inputs = TxParameters::DEFAULT.max_inputs;
        let contract_max_size = ContractParameters::DEFAULT.contract_max_size;
        let tx_offset = TxParameters::default().tx_offset();
        let max_message_data_length =
            PredicateParameters::DEFAULT.max_message_data_length;
        let chain_id = ChainId::default();
        let fee_params = FeeParameters::default();
        Self::with_storage(
            Default::default(),
            gas_costs,
            max_inputs,
            contract_max_size,
            tx_offset,
            max_message_data_length,
            chain_id,
            fee_params,
        )
    }
}

impl<Tx> Interpreter<(), Tx>
where
    Tx: ExecutableTransaction,
{
    /// Create a new interpreter without a storage backend.
    ///
    /// It will have restricted capabilities.
    pub fn without_storage() -> Self {
        Self::default()
    }
}

impl<Tx> Interpreter<MemoryStorage, Tx>
where
    Tx: ExecutableTransaction,
{
    /// Create a new storage with a provided in-memory storage.
    ///
    /// It will have full capabilities.
    pub fn with_memory_storage() -> Self {
        Self::default()
    }
}
