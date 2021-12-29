//! Profiler, can be used to export profiling data from VM runs

use crate::consts::*;
use crate::error::InterpreterError;
use crate::interpreter::Interpreter;
use crate::state::ProgramState;

use dyn_clone::DynClone;
use fuel_types::ContractId;

use std::collections::{hash_map, HashMap};
use std::fmt;

#[cfg(feature = "profiler-gas")]
mod gas;

#[cfg(feature = "profiler-gas")]
pub use gas::GasProfilingData;

/// Location of an instructing collected during runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InstructionLocation {
    /// Context, i.e. current contract. None if running a script.
    context: Option<ContractId>,
    /// Offset from the IS register
    offset: u64,
}

impl InstructionLocation {
    /// New location from context and offset
    pub const fn new(context: Option<ContractId>, offset: u64) -> Self {
        Self { context, offset }
    }

    /// Create an instruction location from a given VM context.
    pub fn from_vm_context<S>(vm: &Interpreter<S>) -> Self {
        let context = vm.internal_contract().ok().copied();

        let registers = vm.registers();
        let offset = registers[REG_PC] - registers[REG_IS];

        Self::new(context, offset)
    }

    /// Context, i.e. current contract
    pub const fn context(&self) -> Option<ContractId> {
        self.context
    }

    /// Offset from the IS register
    pub const fn offset(&self) -> u64 {
        self.offset
    }
}

impl fmt::Display for InstructionLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Location({}, offset={})",
            self.context
                .map(|contract_id| format!(
                    "contract_id={}",
                    contract_id.iter().map(|b| format!("{:02x?}", b)).collect::<String>()
                ),)
                .unwrap_or_else(|| "script".to_string()),
            self.offset
        )
    }
}

/// Mapping from a contextualized instruction location to a concrete profiler
/// instance
pub type PerLocation<T> = HashMap<InstructionLocation, T>;

/// Iterates through location (key, value) pairs
pub struct PerLocationIter<'a, T>(hash_map::Iter<'a, InstructionLocation, T>);

impl<'a, T> Iterator for PerLocationIter<'a, T> {
    type Item = (&'a InstructionLocation, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Iterates through location keys
pub struct PerLocationKeys<'a, T>(hash_map::Keys<'a, InstructionLocation, T>);

impl<'a, T> Iterator for PerLocationKeys<'a, T> {
    type Item = &'a InstructionLocation;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Iterates through location values
pub struct PerLocationValues<'a, T>(hash_map::Values<'a, InstructionLocation, T>);

impl<'a, T> Iterator for PerLocationValues<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Used to receive profile information from the interpreter
pub trait ProfileReceiver: DynClone {
    /// Called after a transaction has completed
    fn on_transaction(&mut self, state: &Result<ProgramState, InterpreterError>, data: &ProfilingData);
}

dyn_clone::clone_trait_object!(ProfileReceiver);

/// Prints profiling info to stderr
#[derive(Clone)]
pub struct ProfilerStderrReceiver;

impl ProfileReceiver for ProfilerStderrReceiver {
    fn on_transaction(&mut self, state: &Result<ProgramState, InterpreterError>, data: &ProfilingData) {
        eprintln!("PROFILER: {:?} {:?}", state, data);
    }
}

/// Profiler
#[derive(Default, Clone)]
pub struct Profiler {
    /// Settings
    // TODO document why we are avoiding generics here
    receiver: Option<Box<dyn ProfileReceiver>>,
    /// Collected profiling data
    data: ProfilingData,
}

impl Profiler {
    /// Called by the VM after a transaction, send collected data to receiver
    pub fn on_transaction(&mut self, state_result: &Result<ProgramState, InterpreterError>) {
        if let Some(r) = &mut self.receiver {
            r.on_transaction(&state_result, &self.data);
        }
    }

    /// Sets profiling data receiver
    pub fn set_receiver(&mut self, receiver: Box<dyn ProfileReceiver>) {
        self.receiver = Some(receiver);
    }

    /// Read-only access to the data
    pub fn data(&self) -> &ProfilingData {
        &self.data
    }

    /// Write access to the data
    pub fn data_mut(&mut self) -> &mut ProfilingData {
        &mut self.data
    }
}

impl fmt::Debug for Profiler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Profiler(receiver={:?}, data=",
            match self.receiver {
                Some(_) => "enabled",
                None => "disabled",
            }
        )?;
        self.data.fmt(f)?;
        write!(f, ")")
    }
}

/// Profiling data separated by profiler
// TODO external consumers should be able to inject custom profilers into `ProfilingData`
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProfilingData {
    #[cfg(feature = "profiler-gas")]
    gas: GasProfilingData,
}
