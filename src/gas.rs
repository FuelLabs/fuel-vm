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
    /// Memory ownership test cost.
    MemoryOwnership(Word),
    /// Cost of memory write, per byte.
    MemoryWrite(Word),
    /// Accumulated cost of several operations.
    Accumulated(Word),
    /// Undefined gas cost.
    Undefined,
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
            MemoryOwnership(1) => self.unit_price(),
            MemoryWrite(1) => self.unit_price(),
            Undefined => self.unit_price(),

            Atom(n) => *n * Atom(1).cost(),
            Arithmetic(n) => *n * Arithmetic(1).cost(),
            ArithmeticExpensive(n) => *n * ArithmeticExpensive(1).cost(),
            RegisterWrite(n) => *n * RegisterWrite(1).cost(),
            Branching(n) => *n * Branching(1).cost(),
            MemoryOwnership(n) => *n * MemoryOwnership(1).cost(),
            MemoryWrite(n) => *n * MemoryWrite(1).cost(),
            Accumulated(c) => *c,
        }
    }

    /// Return the price per unit.
    pub const fn unit_price(&self) -> Word {
        use GasUnit::*;

        match self {
            Atom(_) => 1,
            Arithmetic(_) => 5,
            ArithmeticExpensive(_) => 7,
            RegisterWrite(_) => 2,
            Branching(_) => 10,
            MemoryOwnership(_) => 9,
            MemoryWrite(_) => 8,
            Undefined => 20,
            Accumulated(c) => *c,
        }
    }

    /// Combine two gas computations, accumulating their cost.
    pub const fn join(self, other: Self) -> Self {
        Self::Accumulated(self.cost() + other.cost())
    }
}
