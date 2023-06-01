use crate::consts::*;
use crate::interpreter::{ExecutableTransaction, InitialBalances, Interpreter};
use crate::prelude::RuntimeError;

use fuel_asm::{RegId, Word};
use fuel_tx::CheckError;
use fuel_types::AssetId;
use itertools::Itertools;

use std::collections::{BTreeMap, HashMap};
use std::ops::Index;

use super::VmMemory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Balance {
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

impl TryFrom<InitialBalances> for RuntimeBalances {
    type Error = CheckError;

    fn try_from(initial_balances: InitialBalances) -> Result<Self, CheckError> {
        let mut balances: BTreeMap<_, _> = initial_balances.non_retryable.into();
        if let Some(retryable_amount) = initial_balances.retryable {
            let entry = balances.entry(AssetId::BASE).or_default();
            *entry = entry
                .checked_add(*retryable_amount)
                .ok_or(CheckError::ArithmeticOverflow)?;
        }
        Self::try_from_iter(balances.into_iter())
    }
}

impl RuntimeBalances {
    /// Attempt to create a set of runtime balances from an iterator of pairs.
    ///
    /// This will fail if, and only if, the provided asset/balance pair isn't consistent or a
    /// balance overflows.
    pub fn try_from_iter<T>(iter: T) -> Result<Self, CheckError>
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
                    .or_insert_with(|| Balance::new(0, offset))
                    .checked_add(balance)
                    .ok_or(CheckError::ArithmeticOverflow)?;

                Ok(state)
            })
            .map(|state| Self { state })
    }

    /// Fetch the balance of a given Id, if set.
    pub fn balance(&self, asset: &AssetId) -> Option<Word> {
        self.state.get(asset).map(Balance::value)
    }

    fn set_memory_balance_inner(balance: &Balance, memory: &mut VmMemory) -> Result<Word, RuntimeError> {
        let value = balance.value();
        let offset = balance.offset();

        let offset = offset.saturating_add(AssetId::LEN);
        memory.write_bytes(offset, &value.to_be_bytes());

        Ok(value)
    }

    #[cfg(test)]
    /// Attempt to add the balance of an asset, updating the VM memory in the appropriate
    /// offset
    ///
    /// Note: This will not append a new asset into the set since all the assets must be created
    /// during VM initialization and any additional asset would imply reordering the memory
    /// representation of the balances since they must always be ordered, as in the protocol.
    pub fn checked_balance_add(&mut self, memory: &mut VmMemory, asset: &AssetId, value: Word) -> Option<Word> {
        self.state
            .get_mut(asset)
            .and_then(|b| b.checked_add(value))
            .map(|balance| Self::set_memory_balance_inner(balance, memory))
            .map_or((value == 0).then_some(0), |r| r.ok())
    }

    /// Attempt to subtract the balance of an asset, updating the VM memory in the appropriate
    /// offset
    pub fn checked_balance_sub(&mut self, memory: &mut VmMemory, asset: &AssetId, value: Word) -> Option<Word> {
        self.state
            .get_mut(asset)
            .and_then(|b| b.checked_sub(value))
            .map(|balance| Self::set_memory_balance_inner(balance, memory))
            .map_or((value == 0).then_some(0), |r| r.ok())
    }

    /// Write all assets into the VM memory.
    /// Requires the VM stack to be initialized until `VM_MEMORY_BALANCES_OFFSET`.
    pub fn to_vm<S, Tx>(self, vm: &mut Interpreter<S, Tx>)
    where
        Tx: ExecutableTransaction,
    {
        let start_sp = vm.registers()[RegId::SP];

        debug_assert_eq!(start_sp, VM_MEMORY_BALANCES_OFFSET as Word);

        // Push balances in order, and fill the rest with zeroes.
        for maybe_balance in self
            .state
            .iter()
            .sorted_by_key(|(_, balance)| balance.offset())
            .map(Some)
            .chain(std::iter::repeat(None))
            .take(vm.params().max_inputs as usize)
        {
            if let Some((asset, balance)) = maybe_balance {
                debug_assert_eq!(balance.offset() as Word, vm.registers()[RegId::SP]);

                vm.init_push_stack(asset.as_ref());
                vm.init_push_stack(&balance.value().to_be_bytes());
            } else {
                vm.init_push_stack(AssetId::zeroed().as_ref());
                vm.init_push_stack(&Word::MIN.to_be_bytes());
            }
        }

        let len = vm.params().max_inputs * (AssetId::LEN + WORD_SIZE) as Word;
        debug_assert_eq!(start_sp + len, vm.registers()[RegId::SP]);

        vm.balances = self;
    }
}

impl Index<&AssetId> for RuntimeBalances {
    type Output = Word;

    fn index(&self, index: &AssetId) -> &Self::Output {
        &self.state[index].value
    }
}

impl AsMut<HashMap<AssetId, Balance>> for RuntimeBalances {
    fn as_mut(&mut self) -> &mut HashMap<AssetId, Balance> {
        &mut self.state
    }
}

impl AsRef<HashMap<AssetId, Balance>> for RuntimeBalances {
    fn as_ref(&self) -> &HashMap<AssetId, Balance> {
        &self.state
    }
}

impl PartialEq for RuntimeBalances {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

#[test]
fn writes_to_memory_correctly() {
    use crate::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    let rng = &mut StdRng::seed_from_u64(2322u64);
    let mut interpreter = Interpreter::<_, Script>::without_storage();
    interpreter.registers_mut()[RegId::CGAS] = 1_000_000;
    interpreter.registers_mut()[RegId::GGAS] = 1_000_000;
    
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

    let balances = assets.into_iter();

    // Init stack section before the balances with random data
    interpreter.init_push_stack(&*ContractId::new(rng.gen()));

    // Push balances
    RuntimeBalances::try_from_iter(balances)
        .expect("failed to generate balances")
        .to_vm(&mut interpreter);

    let memory = interpreter.memory();
    assets_sorted
        .iter()
        .fold(VM_MEMORY_BALANCES_OFFSET, |ofs, (asset, value)| {
            let mem_asset = AssetId::from(memory.read_bytes(ofs));
            let mem_value = Word::from_be_bytes(memory.read_bytes(ofs + AssetId::LEN));
            assert_eq!(asset, &mem_asset);
            assert_eq!(value, &mem_value);

            ofs + AssetId::LEN + WORD_SIZE
        });
}

#[test]
fn try_from_iter_wont_overflow() {
    use crate::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    let rng = &mut StdRng::seed_from_u64(2322u64);

    let a: AssetId = rng.gen();
    let b: AssetId = rng.gen();
    let c: AssetId = rng.gen();

    // Sanity check
    let balances = vec![(a, u64::MAX), (b, 15), (c, 0)];
    let runtime_balances = RuntimeBalances::try_from_iter(balances.clone()).expect("failed to create balance set");

    balances.iter().for_each(|(asset, val)| {
        let bal = runtime_balances.balance(asset).expect("failed to fetch balance");

        assert_eq!(val, &bal);
    });

    // Aggregated sum check
    let balances = vec![(a, u64::MAX), (b, 15), (c, 0), (b, 1)];
    let balances_aggregated = vec![(a, u64::MAX), (b, 16), (c, 0)];
    let runtime_balances = RuntimeBalances::try_from_iter(balances).expect("failed to create balance set");

    balances_aggregated.iter().for_each(|(asset, val)| {
        let bal = runtime_balances.balance(asset).expect("failed to fetch balance");

        assert_eq!(val, &bal);
    });

    // Overflow won't panic
    let balances = vec![(a, u64::MAX), (b, 15), (c, 0), (a, 1)];
    let err = RuntimeBalances::try_from_iter(balances).expect_err("overflow set should fail");

    assert_eq!(CheckError::ArithmeticOverflow, err);
}

#[test]
fn checked_add_and_sub_works() {
    use crate::prelude::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    let rng = &mut StdRng::seed_from_u64(2322u64);

    let mut memory = Interpreter::<_, Script>::without_storage().memory;
    let _ = memory.update_allocations(VM_MAX_RAM, VM_MAX_RAM).unwrap();

    let asset: AssetId = rng.gen();

    let balances = vec![(asset, 0)];
    let mut balances = RuntimeBalances::try_from_iter(balances).expect("failed to create set");

    // Sanity check
    let bal = balances.balance(&asset).expect("failed to fetch balance");
    assert_eq!(bal, 0);

    // Add zero balance not in the set should result in zero and not mutate the set
    let asset_b: AssetId = rng.gen();
    assert_ne!(asset, asset_b);

    let val = balances
        .checked_balance_add(&mut memory, &asset_b, 0)
        .expect("failed to add balance");

    assert_eq!(val, 0);
    assert!(balances.balance(&asset_b).is_none());

    // Normal add balance works
    let val = balances
        .checked_balance_add(&mut memory, &asset, 150)
        .expect("failed to add balance");
    let bal = balances.balance(&asset).expect("failed to fetch balance");

    assert_eq!(val, 150);
    assert_eq!(bal, 150);

    let val = balances
        .checked_balance_add(&mut memory, &asset, 75)
        .expect("failed to add balance");
    let bal = balances.balance(&asset).expect("failed to fetch balance");

    assert_eq!(val, 225);
    assert_eq!(bal, 225);

    // Normal sub balance works
    let val = balances
        .checked_balance_sub(&mut memory, &asset, 30)
        .expect("failed to sub balance");
    let bal = balances.balance(&asset).expect("failed to fetch balance");

    assert_eq!(val, 195);
    assert_eq!(bal, 195);

    let val = balances
        .checked_balance_sub(&mut memory, &asset, 120)
        .expect("failed to sub balance");
    let bal = balances.balance(&asset).expect("failed to fetch balance");

    assert_eq!(val, 75);
    assert_eq!(bal, 75);

    let val = balances
        .checked_balance_sub(&mut memory, &asset, 70)
        .expect("failed to sub balance");
    let bal = balances.balance(&asset).expect("failed to fetch balance");

    assert_eq!(val, 5);
    assert_eq!(bal, 5);

    // Balance won't panic underflow
    assert!(balances.checked_balance_sub(&mut memory, &asset, 10).is_none());

    // Balance won't panic overflow
    let val = balances
        .checked_balance_add(&mut memory, &asset, u64::MAX - 5)
        .expect("failed to add balance");
    let bal = balances.balance(&asset).expect("failed to fetch balance");

    assert_eq!(val, u64::MAX);
    assert_eq!(bal, u64::MAX);

    assert!(balances.checked_balance_add(&mut memory, &asset, 1).is_none());
}
