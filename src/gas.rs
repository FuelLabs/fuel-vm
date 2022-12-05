//! Tools for gas instrumentalization

use std::ops::Deref;
use std::sync::Arc;

use fuel_types::Word;

#[allow(dead_code)]
/// Default gas costs are generated from the
/// `fuel-core` repo using the `collect` bin
/// in the `fuel-core-benches` crate.
/// The git sha is included in the file to
/// show what version of `fuel-core` was used
/// to generate the costs.
mod default_gas_costs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Gas unit cost that embeds a unit price and operations count.
///
/// The operations count will be the argument of every variant except
/// `Accumulated`, that will hold the total acumulated gas.
pub enum GasUnit {
    /// Atomic operation.
    Atom(Word),
    /// Arithmetic operation.
    Arithmetic(Word),
    /// Expensive arithmetic operation.
    ArithmeticExpensive(Word),
    /// Write to a register.
    RegisterWrite(Word),
    /// Branching cost.
    Branching(Word),
    /// Hash crypto operation.
    Hash(Word),
    /// Memory ownership test cost.
    MemoryOwnership(Word),
    /// Cost of memory read, per byte.
    MemoryRead(Word),
    /// Cost of memory write, per byte.
    MemoryWrite(Word),
    /// Crypto public key recover.
    Recover(Word),
    /// Cost to read bytes from a storage tree
    StorageReadTree(Word),
    /// Cost to write bytes to a storage tree
    StorageWriteTree(Word),
    /// Cost to write a word to the storage
    StorageWriteWord(Word),
    /// Accumulated cost of several operations.
    Accumulated(Word),
}

impl GasUnit {
    /// Return the `cost := price · N`.
    pub const fn cost(&self) -> Word {
        use GasUnit::*;

        match self {
            Atom(1) => self.unit_price(),
            Arithmetic(1) => self.unit_price(),
            ArithmeticExpensive(1) => self.unit_price(),
            RegisterWrite(1) => self.unit_price(),
            Branching(1) => self.unit_price(),
            Hash(1) => self.unit_price(),
            MemoryOwnership(1) => self.unit_price(),
            MemoryRead(1) => self.unit_price(),
            MemoryWrite(1) => self.unit_price(),
            Recover(1) => self.unit_price(),
            StorageReadTree(1) => self.unit_price(),
            StorageWriteTree(1) => self.unit_price(),
            StorageWriteWord(1) => self.unit_price(),

            Atom(n) => *n * Atom(1).cost(),
            Arithmetic(n) => *n * Arithmetic(1).cost(),
            ArithmeticExpensive(n) => *n * ArithmeticExpensive(1).cost(),
            RegisterWrite(n) => *n * RegisterWrite(1).cost(),
            Branching(n) => *n * Branching(1).cost(),
            Hash(n) => *n * Hash(1).cost(),
            MemoryOwnership(n) => *n * MemoryOwnership(1).cost(),
            MemoryRead(n) => *n * MemoryRead(1).cost(),
            MemoryWrite(n) => *n * MemoryWrite(1).cost(),
            Recover(n) => *n * Recover(1).cost(),
            StorageReadTree(n) => *n * StorageReadTree(1).cost(),
            StorageWriteTree(n) => *n * StorageWriteTree(1).cost(),
            StorageWriteWord(n) => *n * StorageWriteWord(1).cost(),
            Accumulated(c) => *c,
        }
    }

    /// Return the price per unit.
    pub const fn unit_price(&self) -> Word {
        use GasUnit::*;

        // the values are defined empirically from tests performed in fuel-core-benches.
        //
        // the worst case scenario of execution is a memory write for chunks larger than the OS
        // page size, that is commonly set to `4096` bytes.
        //
        // the storage, as expected from a production-ready implementation, didn't present alarming
        // computing power demand from increased operations because tree-seek should be, in worst
        // case scenario, logarithmic.
        match self {
            // base price for pc inc
            Atom(_) => 10,
            // arithmetic operations
            Arithmetic(_) => 15,
            // expensive arith operations
            ArithmeticExpensive(_) => 100,
            // write a register with reserved branching check
            RegisterWrite(_) => 20,
            // branching different than reserved reg
            Branching(_) => 20,
            // native hash operation
            Hash(_) => 300,
            // memory ownership branching check
            MemoryOwnership(_) => 20,
            // memory read per page. should increase exponentially to the number of used pages
            MemoryRead(_) => 15,
            // memory write per page. should increase exponentially to the number of used pages
            MemoryWrite(_) => 20,
            // native ecrecover operation
            Recover(_) => 950,
            // storage read. the storage backend should offer logarithmic worst case scenarios
            StorageReadTree(_) => 75,
            // storage write. the storage backend should offer logarithmic worst case scenarios
            StorageWriteTree(_) => 150,
            // storage write word. the storage backend should offer logarithmic worst case scenarios
            StorageWriteWord(_) => 130,
            // accumulated cost for different operations
            Accumulated(c) => *c,
        }
    }

    /// Combine two gas computations, accumulating their cost.
    pub const fn join(self, other: Self) -> Self {
        Self::Accumulated(self.cost() + other.cost())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Gas costings for every op.
/// The inner values are wrapped in an [`Arc`]
/// so this is cheap to clone.
pub struct GasCosts(Arc<GasCostsValues>);

impl GasCosts {
    /// Create new cost values wrapped in an [`Arc`].
    pub fn new(costs: GasCostsValues) -> Self {
        Self(Arc::new(costs))
    }
}

impl Default for GasCosts {
    fn default() -> Self {
        Self(Arc::new(GasCostsValues::default()))
    }
}

impl Default for GasCostsValues {
    fn default() -> Self {
        // The default values for gas costs
        // are generated from fuel-core-benches.
        default_gas_costs::default_gas_costs()
    }
}

#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default = "GasCostsValues::unit"))]
pub struct GasCostsValues {
    pub add: Word,
    pub addi: Word,
    pub and: Word,
    pub andi: Word,
    pub div: Word,
    pub divi: Word,
    pub eq: Word,
    pub exp: Word,
    pub expi: Word,
    pub gt: Word,
    pub lt: Word,
    pub mlog: Word,
    pub mod_op: Word,
    pub modi: Word,
    pub move_op: Word,
    pub movi: Word,
    pub mroo: Word,
    pub mul: Word,
    pub muli: Word,
    pub noop: Word,
    pub not: Word,
    pub or: Word,
    pub ori: Word,
    pub sll: Word,
    pub slli: Word,
    pub srl: Word,
    pub srli: Word,
    pub sub: Word,
    pub subi: Word,
    pub xor: Word,
    pub xori: Word,
    pub ji: Word,
    pub jnei: Word,
    pub jnzi: Word,
    pub jmp: Word,
    pub jne: Word,
    pub ret: Word,
    pub retd: Word,
    pub rvrt: Word,
    pub smo: Word,
    pub aloc: Word,
    pub cfei: Word,
    pub cfsi: Word,
    pub lb: Word,
    pub lw: Word,
    pub sb: Word,
    pub sw: Word,
    pub bal: Word,
    pub bhei: Word,
    pub bhsh: Word,
    pub burn: Word,
    pub call: Word,
    pub cb: Word,
    pub croo: Word,
    pub csiz: Word,
    pub ldc: Word,
    pub log: Word,
    pub logd: Word,
    pub mint: Word,
    pub scwq: Word,
    pub srw: Word,
    pub srwq: Word,
    pub sww: Word,
    pub swwq: Word,
    pub time: Word,
    pub ecr: Word,
    pub k256: Word,
    pub s256: Word,
    pub flag: Word,
    pub gm: Word,
    pub gtf: Word,
    pub tr: Word,
    pub tro: Word,

    // Dependant
    pub mcl: DependantCost,
    pub mcli: DependantCost,
    pub mcp: DependantCost,
    pub mcpi: DependantCost,
    pub ccp: DependantCost,
    pub meq: DependantCost,
}

#[allow(missing_docs)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DependantCost {
    pub base: Word,
    pub dep_per_unit: Word,
}

impl GasCosts {
    /// Create costs that are all set to zero.
    pub fn free() -> Self {
        Self(Arc::new(GasCostsValues::free()))
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        Self(Arc::new(GasCostsValues::unit()))
    }
}

impl GasCostsValues {
    /// Create costs that are all set to zero.
    pub fn free() -> Self {
        Self {
            add: 0,
            addi: 0,
            and: 0,
            andi: 0,
            div: 0,
            divi: 0,
            eq: 0,
            exp: 0,
            expi: 0,
            gt: 0,
            lt: 0,
            mlog: 0,
            mod_op: 0,
            modi: 0,
            move_op: 0,
            movi: 0,
            mroo: 0,
            mul: 0,
            muli: 0,
            noop: 0,
            not: 0,
            or: 0,
            ori: 0,
            sll: 0,
            slli: 0,
            srl: 0,
            srli: 0,
            sub: 0,
            subi: 0,
            xor: 0,
            xori: 0,
            ji: 0,
            jnei: 0,
            jnzi: 0,
            jmp: 0,
            jne: 0,
            ret: 0,
            retd: 0,
            rvrt: 0,
            smo: 0,
            aloc: 0,
            cfei: 0,
            cfsi: 0,
            lb: 0,
            lw: 0,
            sb: 0,
            sw: 0,
            bal: 0,
            bhei: 0,
            bhsh: 0,
            burn: 0,
            call: 0,
            cb: 0,
            croo: 0,
            csiz: 0,
            ldc: 0,
            log: 0,
            logd: 0,
            mint: 0,
            scwq: 0,
            srw: 0,
            srwq: 0,
            sww: 0,
            swwq: 0,
            time: 0,
            ecr: 0,
            k256: 0,
            s256: 0,
            flag: 0,
            gm: 0,
            gtf: 0,
            tr: 0,
            tro: 0,
            mcl: DependantCost::free(),
            mcli: DependantCost::free(),
            mcp: DependantCost::free(),
            mcpi: DependantCost::free(),
            ccp: DependantCost::free(),
            meq: DependantCost::free(),
        }
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        Self {
            add: 1,
            addi: 1,
            and: 1,
            andi: 1,
            div: 1,
            divi: 1,
            eq: 1,
            exp: 1,
            expi: 1,
            gt: 1,
            lt: 1,
            mlog: 1,
            mod_op: 1,
            modi: 1,
            move_op: 1,
            movi: 1,
            mroo: 1,
            mul: 1,
            muli: 1,
            noop: 1,
            not: 1,
            or: 1,
            ori: 1,
            sll: 1,
            slli: 1,
            srl: 1,
            srli: 1,
            sub: 1,
            subi: 1,
            xor: 1,
            xori: 1,
            ji: 1,
            jnei: 1,
            jnzi: 1,
            jmp: 1,
            jne: 1,
            ret: 1,
            retd: 1,
            rvrt: 1,
            smo: 1,
            aloc: 1,
            cfei: 1,
            cfsi: 1,
            lb: 1,
            lw: 1,
            sb: 1,
            sw: 1,
            bal: 1,
            bhei: 1,
            bhsh: 1,
            burn: 1,
            call: 1,
            cb: 1,
            croo: 1,
            csiz: 1,
            ldc: 1,
            log: 1,
            logd: 1,
            mint: 1,
            scwq: 1,
            srw: 1,
            srwq: 1,
            sww: 1,
            swwq: 1,
            time: 1,
            ecr: 1,
            k256: 1,
            s256: 1,
            flag: 1,
            gm: 1,
            gtf: 1,
            tr: 1,
            tro: 1,
            mcl: DependantCost::unit(),
            mcli: DependantCost::unit(),
            mcp: DependantCost::unit(),
            mcpi: DependantCost::unit(),
            ccp: DependantCost::unit(),
            meq: DependantCost::unit(),
        }
    }
}

impl DependantCost {
    /// Create costs that are all set to zero.
    pub fn free() -> Self {
        Self {
            base: 0,
            dep_per_unit: 0,
        }
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        Self {
            base: 1,
            dep_per_unit: 0,
        }
    }
}

impl Deref for GasCosts {
    type Target = GasCostsValues;

    fn deref(&self) -> &Self::Target {
        &*(self.0)
    }
}

impl From<GasCostsValues> for GasCosts {
    fn from(i: GasCostsValues) -> Self {
        Self(Arc::new(i))
    }
}

impl From<GasCosts> for GasCostsValues {
    fn from(i: GasCosts) -> Self {
        (*i.0).clone()
    }
}
