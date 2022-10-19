//! In-memory client implementation

use crate::backtrace::Backtrace;
use crate::state::StateTransitionRef;
use crate::storage::MemoryStorage;
use crate::transactor::Transactor;

use fuel_tx::{Checked, ConsensusParameters, Create, Receipt, Script, Stage};

#[derive(Default, Debug)]
/// Client implementation with in-memory storage backend.
pub struct MemoryClient {
    transactor: Transactor<MemoryStorage, Script>,
}

impl<'a> AsRef<MemoryStorage> for MemoryClient {
    fn as_ref(&self) -> &MemoryStorage {
        self.transactor.as_ref()
    }
}

impl<'a> AsMut<MemoryStorage> for MemoryClient {
    fn as_mut(&mut self) -> &mut MemoryStorage {
        self.transactor.as_mut()
    }
}

impl MemoryClient {
    /// Create a new instance of the memory client out of a provided storage.
    pub fn new(storage: MemoryStorage, params: ConsensusParameters) -> Self {
        Self {
            transactor: Transactor::new(storage, params),
        }
    }

    /// Create a new instance of the memory client out of a provided storage.
    pub fn from_txtor(transactor: Transactor<MemoryStorage, Script>) -> Self {
        Self { transactor }
    }

    /// If a transaction was executed and produced a VM panic, returns the
    /// backtrace; return `None` otherwise.
    pub fn backtrace(&self) -> Option<Backtrace<Script>> {
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
    pub fn deploy(&mut self, tx: Checked<Create>) -> Option<()> {
        let params = self.transactor.params().clone();
        let mut deploy_transactor = Transactor::new(self.transactor.as_mut(), params);

        if let Ok(state) = deploy_transactor.transact(tx).result() {
            if state.should_revert() {
                self.transactor.as_mut().revert();
            } else {
                self.transactor.as_mut().commit();
            }
            Some(())
        } else {
            // if vm failed to execute, revert storage just in case
            self.transactor.as_mut().revert();
            None
        }
    }

    /// Execute a transaction.
    ///
    /// Since the memory storage is `Infallible`, associatively, the memory
    /// client should also be.
    pub fn transact<St: Stage>(&mut self, tx: Checked<Script, St>) -> &[Receipt] {
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

    /// Consensus parameters
    pub const fn params(&self) -> &ConsensusParameters {
        self.transactor.params()
    }

    /// Tx memory offset
    pub const fn tx_offset(&self) -> usize {
        self.transactor.tx_offset()
    }
}

impl<'a> From<MemoryStorage> for MemoryClient {
    fn from(s: MemoryStorage) -> Self {
        Self::new(s, Default::default())
    }
}

impl<'a> From<MemoryClient> for Transactor<MemoryStorage, Script> {
    fn from(client: MemoryClient) -> Self {
        client.transactor
    }
}
