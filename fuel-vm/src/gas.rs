//! Tools for gas instrumentalization

use fuel_types::Word;

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
