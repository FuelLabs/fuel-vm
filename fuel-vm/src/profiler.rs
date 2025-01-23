//! Profiler, can be used to export profiling data from VM runs

use alloc::{
    boxed::Box,
    format,
    string::{
        String,
        ToString,
    },
    vec::Vec,
};
use core::fmt;
use hashbrown::HashMap;

use dyn_clone::DynClone;

use fuel_types::ContractId;

use crate::prelude::*;

pub use crate::constraints::InstructionLocation;

#[cfg(feature = "serde")]
impl serde::Serialize for InstructionLocation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(ctx) = self.context {
            serializer.serialize_str(&format!("{}:{}", ctx, self.offset))
        } else {
            serializer.serialize_str(&format!("{}", self.offset))
        }
    }
}

#[cfg(feature = "serde")]
struct InstructionLocationVisitor;

#[cfg(feature = "serde")]
impl serde::de::Visitor<'_> for InstructionLocationVisitor {
    type Value = InstructionLocation;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A valid instruction location")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        use core::str::FromStr;

        Ok(if let Some((l, r)) = value.split_once(':') {
            let context = Some(ContractId::from_str(l).map_err(|_| {
                serde::de::Error::custom("Invalid ContractId in InstructionLocation")
            })?);
            let offset = r.parse().unwrap();
            InstructionLocation { context, offset }
        } else {
            let offset = value.parse().unwrap();
            InstructionLocation {
                context: None,
                offset,
            }
        })
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for InstructionLocation {
    fn deserialize<D>(deserializer: D) -> Result<InstructionLocation, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(InstructionLocationVisitor)
    }
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
                    contract_id.iter().fold(String::new(), |mut output, b| {
                        use core::fmt::Write;
                        let _ = write!(output, "{b:02x?}");
                        output
                    })
                ),)
                .unwrap_or_else(|| "script".to_string()),
            self.offset
        )
    }
}

type PerLocation<T> = HashMap<InstructionLocation, T>;

/// Iterates through location (key, value) pairs
pub struct PerLocationIter<'a, T>(hashbrown::hash_map::Iter<'a, InstructionLocation, T>);
impl<'a, T> Iterator for PerLocationIter<'a, T> {
    type Item = (&'a InstructionLocation, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Iterates through location keys
pub struct PerLocationKeys<'a, T>(hashbrown::hash_map::Keys<'a, InstructionLocation, T>);
impl<'a, T> Iterator for PerLocationKeys<'a, T> {
    type Item = &'a InstructionLocation;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Iterates through location values
pub struct PerLocationValues<'a, T>(
    hashbrown::hash_map::Values<'a, InstructionLocation, T>,
);
impl<'a, T> Iterator for PerLocationValues<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

/// Used to receive profile information from the interpreter
pub trait ProfileReceiver: DynClone {
    /// Called after a transaction has completed
    fn on_transaction(
        &mut self,
        state: Result<&ProgramState, InterpreterError<String>>,
        data: &ProfilingData,
    );
}

dyn_clone::clone_trait_object!(ProfileReceiver);

/// Prints profiling info to stderr
#[derive(Clone)]
pub struct StderrReceiver;

impl ProfileReceiver for StderrReceiver {
    #[cfg_attr(not(feature = "std"), allow(unused_variables))]
    fn on_transaction(
        &mut self,
        state: Result<&ProgramState, InterpreterError<String>>,
        data: &ProfilingData,
    ) {
        #[cfg(feature = "std")]
        eprintln!("PROFILER: {state:?} {data:?}");
    }
}

/// Profiler
#[derive(Default, Clone)]
pub struct Profiler {
    /// Settings
    receiver: Option<Box<dyn ProfileReceiver + Send + Sync>>,
    /// Collected profiling data
    data: ProfilingData,
}

impl Profiler {
    /// Called by the VM after a transaction, send collected data to receiver
    pub fn on_transaction(
        &mut self,
        state_result: Result<&ProgramState, InterpreterError<String>>,
    ) {
        if let Some(r) = &mut self.receiver {
            r.on_transaction(state_result, &self.data);
        }
    }

    /// Sets profiling data receiver
    pub fn set_receiver(&mut self, receiver: Box<dyn ProfileReceiver + Send + Sync>) {
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

impl Profiler {
    /// Set the current coverage location.
    pub fn set_coverage(&mut self, location: InstructionLocation) {
        self.data_mut().coverage_mut().set(location);
    }

    /// Add gas to the current coverage location.
    pub fn add_gas(&mut self, location: InstructionLocation, gas_use: u64) {
        self.data_mut().gas_mut().add(location, gas_use);
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
    #[cfg(feature = "profile-coverage")]
    coverage: CoverageProfilingData,
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

    /// Coverage profiling info, immutable
    #[cfg(feature = "profile-coverage")]
    pub fn coverage(&self) -> &CoverageProfilingData {
        &self.coverage
    }

    /// Coverage profiling info, mutable
    #[cfg(feature = "profile-coverage")]
    pub fn coverage_mut(&mut self) -> &mut CoverageProfilingData {
        &mut self.coverage
    }
}

/// Excuted memory addresses
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CoverageProfilingData {
    executed: PerLocation<()>,
}

impl<'a> CoverageProfilingData {
    /// Get total gas used at location
    pub fn get(&self, location: &InstructionLocation) -> bool {
        self.executed.contains_key(location)
    }

    /// Increase gas used at location
    pub fn set(&mut self, location: InstructionLocation) {
        self.executed.insert(location, ());
    }

    /// Iterate through locations
    pub fn iter(&'a self) -> PerLocationKeys<'a, ()> {
        PerLocationKeys(self.executed.keys())
    }
}

impl fmt::Display for CoverageProfilingData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut items: Vec<_> = self.iter().collect();
        items.sort();
        writeln!(f, "{items:?}")
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
        let gas_use = self.gas_use.entry(location).or_insert(0);
        // Saturating is ok for profiling.
        // This should never matter, as gas is deducted on each iteration.
        *gas_use = gas_use.saturating_add(amount);
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
            writeln!(f, "{addr}: {count}")?;
        }
        Ok(())
    }
}
