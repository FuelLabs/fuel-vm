use crate::consts::*;
use crate::interpreter::Interpreter;

use fuel_asm::Word;
use fuel_tx::{CheckedTransaction, ValidationError};
use fuel_types::AssetId;
use itertools::Itertools;

use std::{collections::HashMap, ops::Index};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Balance {
    value: Word,
    offset: usize,
}

impl Balance {
    pub const fn new(value: Word, offset: usize) -> Self {
        Self { value, offset }
    }

    pub const fn offset(&self) -> usize {
        self.offset
    }

    pub const fn value(&self) -> Word {
        self.value
    }

    pub fn checked_add(&mut self, value: Word) -> Option<&mut Self> {
        self.value.checked_add(value).map(|v| self.value = v).map(|_| self)
    }

    pub fn checked_sub(&mut self, value: Word) -> Option<&mut Self> {
        self.value.checked_sub(value).map(|v| self.value = v).map(|_| self)
    }
}

/// Structure to encapsulate asset balances for VM runtime
#[derive(Debug, Default, Clone)]
pub struct RuntimeBalances {
    state: HashMap<AssetId, Balance>,
}

impl From<&CheckedTransaction> for RuntimeBalances {
    fn from(tx: &CheckedTransaction) -> Self {
        let iter = tx.free_balances().map(|(asset, value)| (*asset, *value));

        Self::try_from_iter(iter).expect(
r#"This is a bug!

A checked transaction shouldn't produce a malformed initial free balances set.

Please, file a report mentioning an incorrect transaction validation implementation that allowed a type-safe checked transaction to be created from a malformed inputs set."#)
    }
}

impl RuntimeBalances {
    /// Attempt to create a set of runtime balances from an iterator of pairs.
    ///
    /// This will fail if, and only if, the provided asset/balance pair isn't consistent or a
    /// balance overflows.
    pub fn try_from_iter<T>(iter: T) -> Result<Self, ValidationError>
    where
        T: IntoIterator<Item = (AssetId, Word)>,
    {
        iter.into_iter()
            .sorted_by_key(|k| k.0)
            .enumerate()
            .try_fold(HashMap::new(), |mut state, (i, (asset, balance))| {
                let offset = VM_MEMORY_BALANCES_OFFSET + i * (AssetId::LEN + WORD_SIZE);

                state
                    .entry(asset)
                    .or_insert(Balance::new(0, offset))
                    .checked_add(balance)
                    .ok_or(ValidationError::ArithmeticOverflow)?;

                Ok(state)
            })
            .map(|state| Self { state })
    }

    /// Fetch the balance of a given Id, if set.
    pub fn balance(&self, asset: &AssetId) -> Option<Word> {
        self.state.get(asset).map(Balance::value)
    }

    fn _set_memory_balance(balance: &Balance, memory: &mut [u8]) -> Word {
        let value = balance.value();
        let offset = balance.offset();

        let offset = offset + AssetId::LEN;
        let memory = &mut memory[offset..offset + WORD_SIZE];

        memory.copy_from_slice(&value.to_be_bytes());

        value
    }

    /// Attempt to add the balance of an asset, updating the VM memory in the appropriate
    /// offset
    pub fn checked_balance_add(&mut self, memory: &mut [u8], asset: &AssetId, value: Word) -> Option<Word> {
        self.state
            .get_mut(asset)
            .and_then(|b| b.checked_sub(value))
            .map(|balance| Self::_set_memory_balance(balance, memory))
            .or((value == 0).then(|| 0))
    }

    /// Attempt to subtract the balance of an asset, updating the VM memory in the appropriate
    /// offset
    pub fn checked_balance_sub(&mut self, memory: &mut [u8], asset: &AssetId, value: Word) -> Option<Word> {
        self.state
            .get_mut(asset)
            .and_then(|b| b.checked_sub(value))
            .map(|balance| Self::_set_memory_balance(balance, memory))
            .or((value == 0).then(|| 0))
    }

    /// Write all assets into the VM memory.
    pub fn to_vm<S>(self, vm: &mut Interpreter<S>) {
        let len = vm.params().max_inputs * (AssetId::LEN + WORD_SIZE) as Word;

        vm.registers[REG_SP] += len;
        vm.reserve_stack(len)
            .expect("consensus parameters won't allow stack overflow for VM initialization");

        self.state.iter().for_each(|(asset, balance)| {
            let value = balance.value();
            let ofs = balance.offset();

            (&mut vm.memory[ofs..ofs + AssetId::LEN]).copy_from_slice(asset.as_ref());
            (&mut vm.memory[ofs + AssetId::LEN..ofs + AssetId::LEN + WORD_SIZE]).copy_from_slice(&value.to_be_bytes());
        });

        vm.balances = self;
    }
}

impl Index<&AssetId> for RuntimeBalances {
    type Output = Word;

    fn index(&self, index: &AssetId) -> &Self::Output {
        &self.state[index].value
    }
}

#[test]
fn runtime_balances_writes_to_memory_correctly() {
    use crate::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    let rng = &mut StdRng::seed_from_u64(2322u64);
    let mut interpreter = Interpreter::without_storage();

    let base = AssetId::zeroed();
    let base_balance = 950;
    let assets = vec![
        (rng.gen(), 10),
        (rng.gen(), 25),
        (rng.gen(), 50),
        (base, base_balance),
        (rng.gen(), 100),
    ];

    let mut assets_sorted = assets.clone();
    assets_sorted.as_mut_slice().sort_by(|a, b| a.0.cmp(&b.0));

    assert_ne!(assets_sorted, assets);

    let balances = assets.clone().into_iter();

    RuntimeBalances::try_from_iter(balances)
        .expect("failed to generate balances")
        .to_vm(&mut interpreter);

    let memory = interpreter.memory();
    assets_sorted
        .iter()
        .fold(VM_MEMORY_BALANCES_OFFSET, |ofs, (asset, value)| {
            assert_eq!(asset.as_ref(), &memory[ofs..ofs + AssetId::LEN]);
            assert_eq!(
                &value.to_be_bytes(),
                &memory[ofs + AssetId::LEN..ofs + AssetId::LEN + WORD_SIZE]
            );

            ofs + AssetId::LEN + WORD_SIZE
        });
}
