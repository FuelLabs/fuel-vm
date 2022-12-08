use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::AddAssign;

use fuel_asm::Word;
use fuel_storage::Mappable;
use fuel_storage::MerkleRootStorage;
use fuel_storage::StorageInspect;
use fuel_storage::StorageMutate;
use fuel_tx::Contract;
use fuel_tx::Receipt;
use fuel_types::AssetId;

use crate::call::CallFrame;
use crate::context::Context;
use crate::storage::ContractsAssets;
use crate::storage::ContractsInfo;
use crate::storage::ContractsRawCode;
use crate::storage::ContractsState;

use super::PanicContext;
use super::balances::Balance;
use super::ExecutableTransaction;
use super::Interpreter;
use storage::*;

mod storage;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct Diff<T: Capture + Clone> {
    changes: Vec<Change<T>>,
}

#[derive(Debug, Clone)]
enum Change<T: Capture + Clone> {
    Register(T::State<VecState<Word>>),
    Memory(T::State<Memory>),
    Storage(T::State<StorageState>),
    Frame(T::State<VecState<Option<CallFrame>>>),
    Receipt(T::State<VecState<Option<Receipt>>>),
    Balance(T::State<MapState<AssetId, Option<Balance>>>),
    Context(T::State<Context>),
    PanicContext(T::State<PanicContext>),
}

pub trait Capture {
    type State<S: std::fmt::Debug + Clone>: std::fmt::Debug + Clone;
}

#[derive(Debug, Clone)]
pub struct Delta<S> {
    from: S,
    to: S,
}

#[derive(Debug, Clone)]
pub struct Deltas;
impl Capture for Deltas {
    type State<S: std::fmt::Debug + Clone> = Delta<S>;
}

#[derive(Debug, Clone)]
pub struct Previous<S>(S);

#[derive(Debug, Clone)]
pub struct Beginning;
impl Capture for Beginning {
    type State<S: std::fmt::Debug + Clone> = Previous<S>;
}

#[derive(Debug, Clone)]
struct Next<S>(S);

#[derive(Debug, Clone)]
struct VecState<T> {
    index: usize,
    value: T,
}

#[derive(Debug, Clone)]
struct MapState<K, V>
where
    K: Hash,
    V: PartialEq,
{
    key: K,
    value: V,
}

#[derive(Debug, Clone)]
struct Memory {
    start: usize,
    bytes: Vec<u8>,
}

fn capture_buffer_state<'iter, I, T>(
    a: I,
    b: I,
    change: fn(Delta<VecState<T>>) -> Change<Deltas>,
) -> impl Iterator<Item = Change<Deltas>> + 'iter
where
    T: 'static + std::cmp::PartialEq + Clone,
    I: Iterator<Item = &'iter T> + 'iter,
{
    a.enumerate().zip(b).filter_map(move |(a, b)| {
        (a.1 != b).then(|| {
            change(Delta {
                from: VecState {
                    index: a.0,
                    value: a.1.clone(),
                },
                to: VecState {
                    index: a.0,
                    value: b.clone(),
                },
            })
        })
    })
}

fn capture_map_state<'iter, K, V>(
    a: &'iter HashMap<K, V>,
    b: &'iter HashMap<K, V>,
    change: fn(Delta<MapState<K, Option<V>>>) -> Change<Deltas>,
) -> Vec<Change<Deltas>>
where
    K: 'static + std::cmp::PartialEq + Eq + Clone + Hash + Debug,
    V: 'static + std::cmp::PartialEq + Clone + Debug,
{
    let a_keys: HashSet<_> = a.keys().collect();
    let b_keys: HashSet<_> = b.keys().collect();
    capture_map_state_inner(a, &a_keys, b, &b_keys).map(change).collect()
}

fn capture_map_state_inner<'iter, K, V>(
    a: &'iter HashMap<K, V>,
    a_keys: &'iter HashSet<&K>,
    b: &'iter HashMap<K, V>,
    b_keys: &'iter HashSet<&K>,
) -> impl Iterator<Item = Delta<MapState<K, Option<V>>>> + 'iter
where
    K: 'static + std::cmp::PartialEq + Eq + Clone + Hash + Debug,
    V: 'static + std::cmp::PartialEq + Clone + Debug,
{
    let a_diff = a_keys.difference(&b_keys).map(|k| Delta {
        from: MapState {
            key: (*k).clone(),
            value: Some(a[k].clone()),
        },
        to: MapState {
            key: (*k).clone(),
            value: None,
        },
    });
    let b_diff = b_keys.difference(&a_keys).map(|k| Delta {
        from: MapState {
            key: (*k).clone(),
            value: None,
        },
        to: MapState {
            key: (*k).clone(),
            value: Some(b[k].clone()),
        },
    });
    let intersection = a_keys.intersection(&b_keys).filter_map(|k| {
        let value_a = &a[k];
        let value_b = &a[k];
        (value_a != value_b).then(|| Delta {
            from: MapState {
                key: (*k).clone(),
                value: Some(value_a.clone()),
            },
            to: MapState {
                key: (*k).clone(),
                value: Some(value_b.clone()),
            },
        })
    });

    a_diff.chain(b_diff).chain(intersection)
}

fn capture_vec_state<'iter, I, T>(
    a: I,
    b: I,
    change: fn(Delta<VecState<Option<T>>>) -> Change<Deltas>,
) -> impl Iterator<Item = Change<Deltas>> + 'iter
where
    T: 'static + std::cmp::PartialEq + Clone,
    I: Iterator<Item = &'iter T> + 'iter,
{
    capture_vec_state_inner(a, b).map(move |(index, a, b)| {
        change(Delta {
            from: VecState { index, value: a },
            to: VecState { index, value: b },
        })
    })
}
fn capture_vec_state_inner<'iter, I, T>(a: I, b: I) -> impl Iterator<Item = (usize, Option<T>, Option<T>)> + 'iter
where
    T: 'static + std::cmp::PartialEq + Clone,
    I: Iterator<Item = &'iter T> + 'iter,
{
    a.map(Some)
        .chain(std::iter::repeat(None))
        .enumerate()
        .zip(b.map(Some).chain(std::iter::repeat(None)))
        .take_while(|((_, a), b)| a.is_some() || b.is_some())
        .filter_map(|((index, a), b)| {
            b.map_or(true, |b| a.map_or(true, |a| a != b))
                .then(|| (index, a.cloned(), b.cloned()))
        })
}

impl<S, Tx> Interpreter<S, Tx> {
    pub fn diff(&self, other: &Self) -> Diff<Deltas> {
        let mut diff = Diff { changes: Vec::new() };
        let registers = capture_buffer_state(self.registers.iter(), other.registers.iter(), Change::Register);
        diff.changes.extend(registers);
        let frames = capture_vec_state(self.frames.iter(), other.frames.iter(), Change::Frame);
        diff.changes.extend(frames);
        let receipts = capture_vec_state(self.receipts.iter(), other.receipts.iter(), Change::Receipt);
        diff.changes.extend(receipts);
        let balances = capture_map_state(self.balances.as_ref(), other.balances.as_ref(), Change::Balance);
        diff.changes.extend(balances);

        let mut memory = self.memory.iter().enumerate().zip(other.memory.iter()).peekable();

        memory.by_ref().take_while(|((_, a), b)| a == b).for_each(|_| ());
        while let Some(((start, _), _)) = memory.peek().cloned() {
            let (from, to) = memory
                .by_ref()
                .take_while(|((_, a), b)| a != b)
                .map(|((_, a), b)| (*a, *b))
                .unzip();
            diff.changes.push(Change::Memory(Delta {
                from: Memory { start, bytes: from },
                to: Memory { start, bytes: to },
            }));
            memory.by_ref().take_while(|((_, a), b)| a == b).for_each(|_| ());
        }

        if self.context != other.context {
            diff.changes.push(Change::Context(Delta {
                from: self.context.clone(),
                to: other.context.clone(),
            }))
        }
        
        if self.panic_context != other.panic_context {
            diff.changes.push(Change::PanicContext(Delta {
                from: self.panic_context.clone(),
                to: other.panic_context.clone(),
            }))
        }

        diff
    }

    fn inverse_inner(&mut self, diff: &Diff<Beginning>) {
        for change in &diff.changes {
            match change {
                Change::Register(Previous(VecState { index, value })) => self.registers[*index] = *value,
                Change::Frame(Previous(value)) => invert_vec(&mut self.frames, value),
                Change::Receipt(Previous(value)) => invert_vec(&mut self.receipts, value),
                Change::Balance(Previous(value)) => invert_map(self.balances.as_mut(), value),
                Change::Memory(Previous(Memory { start, bytes })) => {
                    self.memory[*start..(*start + bytes.len())].copy_from_slice(&bytes[..])
                }
                Change::Context(Previous(value)) => self.context = value.clone(),
                Change::PanicContext(Previous(value)) => self.panic_context = value.clone(),
                Change::Storage(_) => (),
            }
        }
    }
}

fn invert_vec<T: Clone>(vector: &mut Vec<T>, value: &VecState<Option<T>>) {
    use std::cmp::Ordering;
    match (&value, value.index.cmp(&vector.len())) {
        (
            VecState {
                index,
                value: Some(value),
            },
            Ordering::Equal | Ordering::Greater,
        ) => {
            vector.resize(*index + 1, value.clone());
            vector[*index] = value.clone();
        }
        (
            VecState {
                index,
                value: Some(value),
            },
            Ordering::Less,
        ) => vector[*index] = value.clone(),
        (VecState { value: None, .. }, Ordering::Equal | Ordering::Greater) => (),
        (VecState { index, value: None }, Ordering::Less) => vector.truncate(*index),
    }
}

fn invert_map<K: Hash + PartialEq + Eq + Clone, V: Clone + PartialEq>(
    map: &mut HashMap<K, V>,
    value: &MapState<K, Option<V>>,
) {
    match value {
        MapState {
            key,
            value: Some(value),
        } => {
            map.insert(key.clone(), value.clone());
        }
        MapState { key, value: None } => {
            map.remove(key);
        }
    }
}

impl<S, Tx> PartialEq for Interpreter<S, Tx> {
    fn eq(&self, other: &Self) -> bool {
        self.registers == other.registers
            && self.memory == other.memory
            && self.frames == other.frames
            && self.receipts == other.receipts
            // && self.tx == other.tx
            && self.initial_balances == other.initial_balances
            // && self.storage == other.storage
            // && self.debugger == other.debugger
            && self.context == other.context
            // && self.balances == other.balances
            && self.gas_costs == other.gas_costs
            // && self.profiler == other.profiler
            && self.params == other.params
        // && self.panic_context == other.panic_context
    }
}

impl From<Diff<Deltas>> for Diff<Beginning> {
    fn from(d: Diff<Deltas>) -> Self {
        Self {
            changes: d
                .changes
                .into_iter()
                .map(|c| match c {
                    Change::Register(v) => Change::Register(v.into()),
                    Change::Memory(v) => Change::Memory(v.into()),
                    Change::Storage(v) => Change::Storage(v.into()),
                    Change::Frame(v) => Change::Frame(v.into()),
                    Change::Receipt(v) => Change::Receipt(v.into()),
                    Change::Balance(v) => Change::Balance(v.into()),
                    Change::Context(v) => Change::Context(v.into()),
                    Change::PanicContext(v) => Change::PanicContext(v.into()),
                })
                .collect(),
        }
    }
}

impl<T> From<Delta<T>> for Previous<T> {
    fn from(d: Delta<T>) -> Self {
        Self(d.from)
    }
}

impl<T: Capture + Clone> AddAssign for Diff<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.changes.extend(rhs.changes);
    }
}