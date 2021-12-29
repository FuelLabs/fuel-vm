use super::{InstructionLocation, PerLocation, PerLocationIter, PerLocationKeys, PerLocationValues, ProfilingData};

use std::fmt;

impl ProfilingData {
    /// Gas profiling info, immutable
    pub fn gas(&self) -> &GasProfilingData {
        &self.gas
    }

    /// Gas profiling info, mutable
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
