//! Profiler, can be used to export profiling data from VM runs

use std::collections::HashMap;
use std::fmt;

use dyn_clone::DynClone;

use fuel_types::ContractId;

use crate::prelude::*;

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

type PerLocation<T> = HashMap<InstructionLocation, T>;

/// Iterates through location (key, value) pairs
pub struct PerLocationIter<'a, T>(std::collections::hash_map::Iter<'a, InstructionLocation, T>);
impl<'a, T> Iterator for PerLocationIter<'a, T> {
    type Item = (&'a InstructionLocation, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Iterates through location keys
pub struct PerLocationKeys<'a, T>(std::collections::hash_map::Keys<'a, InstructionLocation, T>);
impl<'a, T> Iterator for PerLocationKeys<'a, T> {
    type Item = &'a InstructionLocation;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Iterates through location values
pub struct PerLocationValues<'a, T>(std::collections::hash_map::Values<'a, InstructionLocation, T>);
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
pub struct StderrReceiver;

impl ProfileReceiver for StderrReceiver {
    fn on_transaction(&mut self, state: &Result<ProgramState, InterpreterError>, data: &ProfilingData) {
        eprintln!("PROFILER: {:?} {:?}", state, data);
    }
}

/// Profiler
#[derive(Default, Clone)]
pub struct Profiler {
    /// Settings
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
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ProfilingData {
    #[cfg(feature = "profile-gas")]
    gas: GasProfilingData,
}

impl ProfilingData {
    /// Gas profiling info, immutable
    #[cfg(feature = "profile-gas")]
    pub fn gas(&self) -> &GasProfilingData {
        &self.gas
    }

    /// Gas profiling info, mutable
    #[cfg(feature = "profile-gas")]
    pub fn gas_mut(&mut self) -> &mut GasProfilingData {
        &mut self.gas
    }
}

/// Used gas per memory address
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GasProfilingData {
    gas_use: PerLocation<u64>,
}

impl<'a> GasProfilingData {
    /// Get total gas used at location
    pub fn get(&self, location: &InstructionLocation) -> u64 {
        self.gas_use.get(location).copied().unwrap_or(0)
    }

    /// Increase gas used at location
    pub fn add(&mut self, location: InstructionLocation, amount: u64) {
        *self.gas_use.entry(location).or_insert(0) += amount;
    }

    /// Iterate through locations and gas values
    pub fn iter(&'a self) -> PerLocationIter<'a, u64> {
        PerLocationIter(self.gas_use.iter())
    }

    /// Iterate through locations
    pub fn keys(&'a self) -> PerLocationKeys<'a, u64> {
        PerLocationKeys(self.gas_use.keys())
    }

    /// Iterate through gas values
    /// Can be used to get whole gas usage with `.sum()`
    pub fn values(&'a self) -> PerLocationValues<'a, u64> {
        PerLocationValues(self.gas_use.values())
    }
}

impl fmt::Display for GasProfilingData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut items: Vec<(_, _)> = self.iter().collect();
        items.sort();
        for (addr, count) in items {
            writeln!(f, "{}: {}", addr, count)?;
        }
        Ok(())
    }
}
