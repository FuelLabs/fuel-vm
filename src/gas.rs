use fuel_types::Word;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GasUnit {
    Atom(Word),
    Arithmetic(Word),
    ArithmeticExpensive(Word),
    RegisterRead(Word),
    RegisterWrite(Word),
    Branching(Word),
    MemoryOwnership(Word),
    MemoryWrite(Word),
    Accumulated(Word),
    Undefined,
}

impl GasUnit {
    pub const fn cost(&self) -> Word {
        use GasUnit::*;

        match self {
            Atom(1) => 1,
            Arithmetic(1) => 5,
            ArithmeticExpensive(1) => 7,
            RegisterRead(1) => 1,
            RegisterWrite(1) => 2,
            Branching(1) => 10,
            MemoryOwnership(1) => 9,
            MemoryWrite(1) => 8,
            Undefined => 20,

            Atom(n) => *n * Atom(1).cost(),
            Arithmetic(n) => *n * Arithmetic(1).cost(),
            ArithmeticExpensive(n) => *n * ArithmeticExpensive(1).cost(),
            RegisterRead(n) => *n * RegisterRead(1).cost(),
            RegisterWrite(n) => *n * RegisterWrite(1).cost(),
            Branching(n) => *n * Branching(1).cost(),
            MemoryOwnership(n) => *n * MemoryOwnership(1).cost(),
            MemoryWrite(n) => *n * MemoryWrite(1).cost(),
            Accumulated(c) => *c,
        }
    }

    pub const fn join(self, other: Self) -> Self {
        Self::Accumulated(self.cost() + other.cost())
    }
}
