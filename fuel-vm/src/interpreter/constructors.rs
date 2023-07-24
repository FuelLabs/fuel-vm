//! Exposed constructors API for the [`Interpreter`]

use super::{
    ExecutableTransaction,
    Interpreter,
    RuntimeBalances,
};
use crate::{
    consts::*,
    context::Context,
    interpreter::{
        InterpreterParams,
        PanicContext,
    },
    state::Debugger,
    storage::MemoryStorage,
};
use fuel_tx::{
    ContractParameters,
    FeeParameters,
    GasCosts,
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
    pub fn with_storage(storage: S, interpreter_params: InterpreterParams) -> Self {
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
            profiler: Profiler::default(),
            interpreter_params,
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
        let params = InterpreterParams {
            gas_costs: Default::default(),
            max_inputs: TxParameters::DEFAULT.max_inputs,
            contract_max_size: ContractParameters::DEFAULT.contract_max_size,
            tx_offset: TxParameters::default().tx_offset(),
            max_message_data_length: PredicateParameters::DEFAULT.max_message_data_length,
            chain_id: ChainId::default(),
            fee_params: FeeParameters::default(),
        };
        Self::with_storage(Default::default(), params)
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
