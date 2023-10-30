//! Tools for gas instrumentalization

use core::ops::Deref;

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

use fuel_types::Word;

/// Default gas costs are generated from the
/// `fuel-core` repo using the `collect` bin
/// in the `fuel-core-benches` crate.
/// The git sha is included in the file to
/// show what version of `fuel-core` was used
/// to generate the costs.
#[allow(dead_code)]
mod default_gas_costs;

/// Gas unit cost that embeds a unit price and operations count.
///
/// The operations count will be the argument of every variant except
/// `Accumulated`, that will hold the total acumulated gas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        // the worst case scenario of execution is a memory write for chunks larger than
        // the OS page size, that is commonly set to `4096` bytes.
        //
        // the storage, as expected from a production-ready implementation, didn't present
        // alarming computing power demand from increased operations because
        // tree-seek should be, in worst case scenario, logarithmic.
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
            // memory read per page. should increase exponentially to the number of used
            // pages
            MemoryRead(_) => 15,
            // memory write per page. should increase exponentially to the number of used
            // pages
            MemoryWrite(_) => 20,
            // native ecrecover operation
            Recover(_) => 950,
            // storage read. the storage backend should offer logarithmic worst case
            // scenarios
            StorageReadTree(_) => 75,
            // storage write. the storage backend should offer logarithmic worst case
            // scenarios
            StorageWriteTree(_) => 150,
            // storage write word. the storage backend should offer logarithmic worst case
            // scenarios
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

/// Gas costings for every op.
/// The inner values are wrapped in an [`Arc`]
/// so this is cheap to clone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg(feature = "alloc")]
pub struct GasCosts(Arc<GasCostsValues>);

#[cfg(feature = "serde")]
#[cfg(feature = "alloc")]
impl<'de> serde::Deserialize<'de> for GasCosts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(GasCosts(Arc::new(serde::Deserialize::deserialize(
            deserializer,
        )?)))
    }
}

#[cfg(feature = "serde")]
#[cfg(feature = "alloc")]
impl serde::Serialize for GasCosts {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(self.0.as_ref(), serializer)
    }
}

#[cfg(feature = "alloc")]
impl GasCosts {
    /// Create new cost values wrapped in an [`Arc`].
    pub fn new(costs: GasCostsValues) -> Self {
        Self(Arc::new(costs))
    }
}

#[cfg(feature = "alloc")]
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

/// Gas costs for every op.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default = "GasCostsValues::unit"))]
pub struct GasCostsValues {
    pub add: Word,
    pub addi: Word,
    pub aloc: Word,
    pub and: Word,
    pub andi: Word,
    pub bal: Word,
    pub bhei: Word,
    pub bhsh: Word,
    pub burn: Word,
    pub cb: Word,
    pub cfei: Word,
    pub cfsi: Word,
    pub croo: Word,
    pub div: Word,
    pub divi: Word,
    pub eck1: Word,
    pub ecr1: Word,
    pub ed19: Word,
    pub eq: Word,
    pub exp: Word,
    pub expi: Word,
    pub flag: Word,
    pub gm: Word,
    pub gt: Word,
    pub gtf: Word,
    pub ji: Word,
    pub jmp: Word,
    pub jne: Word,
    pub jnei: Word,
    pub jnzi: Word,
    pub jmpf: Word,
    pub jmpb: Word,
    pub jnzf: Word,
    pub jnzb: Word,
    pub jnef: Word,
    pub jneb: Word,
    pub lb: Word,
    pub log: Word,
    pub lt: Word,
    pub lw: Word,
    pub mint: Word,
    pub mlog: Word,
    #[cfg_attr(feature = "serde", serde(rename = "mod"))]
    pub mod_op: Word,
    pub modi: Word,
    #[cfg_attr(feature = "serde", serde(rename = "move"))]
    pub move_op: Word,
    pub movi: Word,
    pub mroo: Word,
    pub mul: Word,
    pub muli: Word,
    pub mldv: Word,
    pub noop: Word,
    pub not: Word,
    pub or: Word,
    pub ori: Word,
    pub poph: Word,
    pub popl: Word,
    pub pshh: Word,
    pub pshl: Word,
    #[cfg_attr(feature = "serde", serde(rename = "ret_contract"))]
    pub ret: Word,
    #[cfg_attr(feature = "serde", serde(rename = "rvrt_contract"))]
    pub rvrt: Word,
    pub sb: Word,
    pub sll: Word,
    pub slli: Word,
    pub srl: Word,
    pub srli: Word,
    pub srw: Word,
    pub sub: Word,
    pub subi: Word,
    pub sw: Word,
    pub sww: Word,
    pub time: Word,
    pub tr: Word,
    pub tro: Word,
    pub wdcm: Word,
    pub wqcm: Word,
    pub wdop: Word,
    pub wqop: Word,
    pub wdml: Word,
    pub wqml: Word,
    pub wddv: Word,
    pub wqdv: Word,
    pub wdmd: Word,
    pub wqmd: Word,
    pub wdam: Word,
    pub wqam: Word,
    pub wdmm: Word,
    pub wqmm: Word,
    pub xor: Word,
    pub xori: Word,

    // Dependent
    pub call: DependentCost,
    pub ccp: DependentCost,
    pub csiz: DependentCost,
    pub k256: DependentCost,
    pub ldc: DependentCost,
    pub logd: DependentCost,
    pub mcl: DependentCost,
    pub mcli: DependentCost,
    pub mcp: DependentCost,
    pub mcpi: DependentCost,
    pub meq: DependentCost,
    #[cfg_attr(feature = "serde", serde(rename = "retd_contract"))]
    pub retd: DependentCost,
    pub s256: DependentCost,
    pub scwq: DependentCost,
    pub smo: DependentCost,
    pub srwq: DependentCost,
    pub swwq: DependentCost,

    // Non-opcode dependent costs
    pub contract_root: DependentCost,
}

/// Dependent cost is a cost that depends on the number of units.
/// The cost starts at the base and grows by `dep_per_unit` for every unit.
///
/// For example, if the base is 10 and the `dep_per_unit` is 2,
/// then the cost for 0 units is 10, 1 unit is 12, 2 units is 14, etc.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DependentCost {
    /// The minimum that this operation can cost.
    pub base: Word,
    /// The amount that this operation costs per
    /// increase in unit.
    pub dep_per_unit: Word,
}

#[cfg(feature = "alloc")]
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
            aloc: 0,
            and: 0,
            andi: 0,
            bal: 0,
            bhei: 0,
            bhsh: 0,
            burn: 0,
            cb: 0,
            cfei: 0,
            cfsi: 0,
            croo: 0,
            div: 0,
            divi: 0,
            eck1: 0,
            ecr1: 0,
            ed19: 0,
            eq: 0,
            exp: 0,
            expi: 0,
            flag: 0,
            gm: 0,
            gt: 0,
            gtf: 0,
            ji: 0,
            jmp: 0,
            jne: 0,
            jnei: 0,
            jnzi: 0,
            jmpf: 0,
            jmpb: 0,
            jnzf: 0,
            jnzb: 0,
            jnef: 0,
            jneb: 0,
            lb: 0,
            log: 0,
            lt: 0,
            lw: 0,
            mint: 0,
            mlog: 0,
            mod_op: 0,
            modi: 0,
            move_op: 0,
            movi: 0,
            mroo: 0,
            mul: 0,
            muli: 0,
            mldv: 0,
            noop: 0,
            not: 0,
            or: 0,
            ori: 0,
            poph: 0,
            popl: 0,
            pshh: 0,
            pshl: 0,
            ret: 0,
            rvrt: 0,
            sb: 0,
            sll: 0,
            slli: 0,
            srl: 0,
            srli: 0,
            srw: 0,
            sub: 0,
            subi: 0,
            sw: 0,
            sww: 0,
            time: 0,
            tr: 0,
            tro: 0,
            wdcm: 0,
            wqcm: 0,
            wdop: 0,
            wqop: 0,
            wdml: 0,
            wqml: 0,
            wddv: 0,
            wqdv: 0,
            wdmd: 0,
            wqmd: 0,
            wdam: 0,
            wqam: 0,
            wdmm: 0,
            wqmm: 0,
            xor: 0,
            xori: 0,
            call: DependentCost::free(),
            ccp: DependentCost::free(),
            csiz: DependentCost::free(),
            k256: DependentCost::free(),
            ldc: DependentCost::free(),
            logd: DependentCost::free(),
            mcl: DependentCost::free(),
            mcli: DependentCost::free(),
            mcp: DependentCost::free(),
            mcpi: DependentCost::free(),
            meq: DependentCost::free(),
            retd: DependentCost::free(),
            s256: DependentCost::free(),
            scwq: DependentCost::free(),
            smo: DependentCost::free(),
            srwq: DependentCost::free(),
            swwq: DependentCost::free(),
            contract_root: DependentCost::free(),
        }
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        Self {
            add: 1,
            addi: 1,
            aloc: 1,
            and: 1,
            andi: 1,
            bal: 1,
            bhei: 1,
            bhsh: 1,
            burn: 1,
            cb: 1,
            cfei: 1,
            cfsi: 1,
            croo: 1,
            div: 1,
            divi: 1,
            eck1: 1,
            ecr1: 1,
            ed19: 1,
            eq: 1,
            exp: 1,
            expi: 1,
            flag: 1,
            gm: 1,
            gt: 1,
            gtf: 1,
            ji: 1,
            jmp: 1,
            jne: 1,
            jnei: 1,
            jnzi: 1,
            jmpf: 1,
            jmpb: 1,
            jnzf: 1,
            jnzb: 1,
            jnef: 1,
            jneb: 1,
            lb: 1,
            log: 1,
            lt: 1,
            lw: 1,
            mint: 1,
            mlog: 1,
            mod_op: 1,
            modi: 1,
            move_op: 1,
            movi: 1,
            mroo: 1,
            mul: 1,
            muli: 1,
            mldv: 1,
            noop: 1,
            not: 1,
            or: 1,
            ori: 1,
            ret: 1,
            poph: 1,
            popl: 1,
            pshh: 1,
            pshl: 1,
            rvrt: 1,
            sb: 1,
            sll: 1,
            slli: 1,
            srl: 1,
            srli: 1,
            srw: 1,
            sub: 1,
            subi: 1,
            sw: 1,
            sww: 1,
            time: 1,
            tr: 1,
            tro: 1,
            wdcm: 1,
            wqcm: 1,
            wdop: 1,
            wqop: 1,
            wdml: 1,
            wqml: 1,
            wddv: 1,
            wqdv: 1,
            wdmd: 1,
            wqmd: 1,
            wdam: 1,
            wqam: 1,
            wdmm: 1,
            wqmm: 1,
            xor: 1,
            xori: 1,
            call: DependentCost::unit(),
            ccp: DependentCost::unit(),
            csiz: DependentCost::unit(),
            k256: DependentCost::unit(),
            ldc: DependentCost::unit(),
            logd: DependentCost::unit(),
            mcl: DependentCost::unit(),
            mcli: DependentCost::unit(),
            mcp: DependentCost::unit(),
            mcpi: DependentCost::unit(),
            meq: DependentCost::unit(),
            retd: DependentCost::unit(),
            s256: DependentCost::unit(),
            scwq: DependentCost::unit(),
            smo: DependentCost::unit(),
            srwq: DependentCost::unit(),
            swwq: DependentCost::unit(),
            contract_root: DependentCost::unit(),
        }
    }
}

impl DependentCost {
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

    pub fn resolve(&self, units: Word) -> Word {
        self.base + units.saturating_div(self.dep_per_unit)
    }
}

#[cfg(feature = "alloc")]
impl Deref for GasCosts {
    type Target = GasCostsValues;

    fn deref(&self) -> &Self::Target {
        &(self.0)
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
