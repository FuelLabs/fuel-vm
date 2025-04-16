//! # VM State Differences
//! This module provides the ability to generate diffs between two VMs internal states.
//! The diff can then be used to invert a VM to the original state.
//! This module is experimental work in progress and currently only used in testing
//! although it could potentially stabilize to be used in production.

use alloc::{
    sync::Arc,
    vec::Vec,
};
use core::{
    any::Any,
    fmt::Debug,
    hash::Hash,
    ops::AddAssign,
};
use hashbrown::{
    HashMap,
    HashSet,
};

use fuel_asm::Word;
use fuel_storage::{
    Mappable,
    StorageInspect,
    StorageMutate,
};
use fuel_tx::{
    Contract,
    Receipt,
};
use fuel_types::AssetId;

use crate::{
    call::CallFrame,
    context::Context,
    storage::{
        ContractsAssets,
        ContractsRawCode,
        ContractsState,
    },
};

use super::{
    ExecutableTransaction,
    Interpreter,
    Memory,
    PanicContext,
    balances::Balance,
    receipts::ReceiptsCtx,
};
use crate::interpreter::memory::MemoryRollbackData;
use storage::*;

mod storage;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
/// A diff of VM state.
///
/// By default this does not print out the
/// memory bytes but you can supply the `#` flag
/// to get them like:
/// ```ignore
/// println!("{:#}", diff);
/// ```
pub struct Diff<T: VmStateCapture + Clone> {
    changes: Vec<Change<T>>,
}

#[derive(Debug, Clone)]
enum Change<T: VmStateCapture + Clone> {
    /// Holds a snapshot of register state.
    Register(T::State<VecState<Word>>),
    /// Holds a snapshot of memory state.
    Memory(MemoryRollbackData),
    /// Holds a snapshot of storage state.
    Storage(T::State<StorageState>),
    /// Holds a snapshot of the call stack.
    Frame(T::State<VecState<Option<CallFrame>>>),
    /// Holds a snapshot of receipt state.
    Receipt(T::State<VecState<Option<Receipt>>>),
    /// Holds a snapshot of balance state.
    Balance(T::State<MapState<AssetId, Option<Balance>>>),
    /// Holds a snapshot of context state.
    Context(T::State<Context>),
    /// Holds a snapshot of the panic context state.
    PanicContext(T::State<PanicContext>),
    /// Holds a snapshot of the transaction state.
    Txn(T::State<Arc<dyn AnyDebug>>),
}

/// A trait that combines the [`Debug`] and [`Any`] traits.
pub trait AnyDebug: Any + Debug {
    /// Returns a reference to the underlying type as `Any`.
    fn as_any_ref(&self) -> &dyn Any;
}

impl<T> AnyDebug for T
where
    T: Any + Debug,
{
    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}

/// A mapping between the kind of state that is being capture
/// and the concrete data that is collected.
pub trait VmStateCapture {
    /// The actual type is defined by the implementations of
    /// the Capture trait.
    type State<S: core::fmt::Debug + Clone>: core::fmt::Debug + Clone;
}

#[derive(Debug, Clone)]
/// Family of state data that are implemented with the [`Delta`]
/// struct. Captures the difference between the current and previous
/// state of the VM.
pub struct Deltas;

impl VmStateCapture for Deltas {
    type State<S: core::fmt::Debug + Clone> = Delta<S>;
}

#[derive(Debug, Clone)]
/// The Delta struct represents the difference between two states of the VM.
pub struct Delta<S> {
    // Represents the state of the VM before a change.
    from: S,
    // Represents the state of the VM after a change.
    to: S,
}

#[derive(Debug, Clone)]
/// Family of state data that are implemented with the [`Previous`]
/// struct. Captures the initial state of the VM.
pub struct InitialVmState;

impl VmStateCapture for InitialVmState {
    type State<S: core::fmt::Debug + Clone> = Previous<S>;
}
#[derive(Debug, Clone)]
/// The State type when capturing the initial state of the VM.
pub struct Previous<S>(S);

#[derive(Debug, Clone)]
/// The state of a vector at an index.
struct VecState<T> {
    /// Index of the value.
    index: usize,
    /// Value at the index.
    value: T,
}

#[derive(Debug, Clone)]
/// The state of a map at a key.
struct MapState<K, V>
where
    K: Hash,
    V: PartialEq,
{
    /// Key of the value.
    key: K,
    /// Value at the key.
    value: V,
}

fn capture_buffer_state<'iter, I, T>(
    a: I,
    b: I,
    change: fn(Delta<VecState<T>>) -> Change<Deltas>,
) -> impl Iterator<Item = Change<Deltas>> + 'iter
where
    T: 'static + core::cmp::PartialEq + Clone,
    I: Iterator<Item = &'iter T> + 'iter,
{
    a.enumerate()
        .zip(b)
        .filter(|&(a, b)| (a.1 != b))
        .map(move |(a, b)| {
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
}

type ChangeDeltaVariant<S> = fn(Delta<S>) -> Change<Deltas>;

fn capture_map_state<'iter, K, V>(
    a: &'iter HashMap<K, V>,
    b: &'iter HashMap<K, V>,
    change: ChangeDeltaVariant<MapState<K, Option<V>>>,
) -> Vec<Change<Deltas>>
where
    K: 'static + PartialEq + Eq + Clone + Hash + Debug,
    V: 'static + core::cmp::PartialEq + Clone + Debug,
{
    let a_keys: HashSet<_> = a.keys().collect();
    let b_keys: HashSet<_> = b.keys().collect();
    capture_map_state_inner(a, &a_keys, b, &b_keys)
        .map(change)
        .collect()
}

fn capture_map_state_inner<'iter, K, V>(
    a: &'iter HashMap<K, V>,
    a_keys: &'iter HashSet<&K>,
    b: &'iter HashMap<K, V>,
    b_keys: &'iter HashSet<&K>,
) -> impl Iterator<Item = Delta<MapState<K, Option<V>>>> + 'iter
where
    K: 'static + PartialEq + Eq + Clone + Hash + Debug,
    V: 'static + core::cmp::PartialEq + Clone + Debug,
{
    let a_diff = a_keys.difference(b_keys).map(|k| Delta {
        from: MapState {
            key: (*k).clone(),
            value: Some(a[*k].clone()),
        },
        to: MapState {
            key: (*k).clone(),
            value: None,
        },
    });
    let b_diff = b_keys.difference(a_keys).map(|k| Delta {
        from: MapState {
            key: (*k).clone(),
            value: None,
        },
        to: MapState {
            key: (*k).clone(),
            value: Some(b[*k].clone()),
        },
    });
    let intersection = a_keys.intersection(b_keys).filter_map(|k| {
        let value_a = &a[*k];
        let value_b = &b[*k];
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
    change: ChangeDeltaVariant<VecState<Option<T>>>,
) -> impl Iterator<Item = Change<Deltas>> + 'iter
where
    T: 'static + core::cmp::PartialEq + Clone,
    I: Iterator<Item = &'iter T> + 'iter,
{
    capture_vec_state_inner(a, b).map(move |(index, a, b)| {
        change(Delta {
            from: VecState { index, value: a },
            to: VecState { index, value: b },
        })
    })
}
fn capture_vec_state_inner<'iter, I, T>(
    a: I,
    b: I,
) -> impl Iterator<Item = (usize, Option<T>, Option<T>)> + 'iter
where
    T: 'static + core::cmp::PartialEq + Clone,
    I: Iterator<Item = &'iter T> + 'iter,
{
    a.map(Some)
        .chain(core::iter::repeat(None))
        .enumerate()
        .zip(b.map(Some).chain(core::iter::repeat(None)))
        .take_while(|((_, a), b)| a.is_some() || b.is_some())
        .filter(|((_, a), b)| b.map_or(true, |b| a.map_or(true, |a| a != b)))
        .map(|((index, a), b)| (index, a.cloned(), b.cloned()))
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
{
    /// The function generates a diff of VM state, represented by the Diff struct,
    /// between two VMs internal states. The `desired_state` is the desired state
    /// that we expect after rollback is done.
    pub fn rollback_to(&self, desired_state: &Self) -> Diff<Deltas>
    where
        Tx: PartialEq + Clone + Debug + 'static,
    {
        let mut diff = Diff {
            changes: Vec::new(),
        };
        let registers = capture_buffer_state(
            self.registers.iter(),
            desired_state.registers.iter(),
            Change::Register,
        );
        diff.changes.extend(registers);
        let frames = capture_vec_state(
            self.frames.iter(),
            desired_state.frames.iter(),
            Change::Frame,
        );
        diff.changes.extend(frames);
        let receipts = capture_vec_state(
            self.receipts.as_ref().iter(),
            desired_state.receipts.as_ref().iter(),
            Change::Receipt,
        );
        diff.changes.extend(receipts);
        let balances = capture_map_state(
            self.balances.as_ref(),
            desired_state.balances.as_ref(),
            Change::Balance,
        );
        diff.changes.extend(balances);

        let memory_rollback_data =
            self.memory().collect_rollback_data(desired_state.memory());

        if let Some(memory_rollback_data) = memory_rollback_data {
            diff.changes.push(Change::Memory(memory_rollback_data));
        }

        if self.context != desired_state.context {
            diff.changes.push(Change::Context(Delta {
                from: self.context.clone(),
                to: desired_state.context.clone(),
            }))
        }

        if self.panic_context != desired_state.panic_context {
            diff.changes.push(Change::PanicContext(Delta {
                from: self.panic_context.clone(),
                to: desired_state.panic_context.clone(),
            }))
        }

        if self.tx != desired_state.tx {
            let from: Arc<dyn AnyDebug> = Arc::new(self.tx.clone());
            let to: Arc<dyn AnyDebug> = Arc::new(desired_state.tx.clone());
            diff.changes.push(Change::Txn(Delta { from, to }))
        }

        diff
    }
}

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
{
    fn inverse_inner(&mut self, change: &Change<InitialVmState>)
    where
        Tx: Clone + 'static,
    {
        match change {
            Change::Register(Previous(VecState { index, value })) => {
                self.registers[*index] = *value
            }
            Change::Frame(Previous(value)) => invert_vec(&mut self.frames, value),
            Change::Receipt(Previous(value)) => {
                invert_receipts_ctx(&mut self.receipts, value)
            }
            Change::Balance(Previous(value)) => invert_map(self.balances.as_mut(), value),
            Change::Memory(memory_rollback_data) => {
                self.memory_mut().rollback(memory_rollback_data)
            }
            Change::Context(Previous(value)) => self.context = value.clone(),
            Change::PanicContext(Previous(value)) => self.panic_context = value.clone(),
            Change::Txn(Previous(tx)) => {
                self.tx = AsRef::<dyn AnyDebug>::as_ref(tx)
                    .as_any_ref()
                    .downcast_ref::<Tx>()
                    .unwrap()
                    .clone();
            }
            Change::Storage(_) => (),
        }
    }
}

fn invert_vec<T: Clone>(vector: &mut Vec<T>, value: &VecState<Option<T>>) {
    use core::cmp::Ordering;
    match (&value, value.index.cmp(&vector.len())) {
        (
            VecState {
                index,
                value: Some(value),
            },
            Ordering::Equal | Ordering::Greater,
        ) => {
            vector.resize((*index).saturating_add(1), value.clone());
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

fn invert_receipts_ctx(ctx: &mut ReceiptsCtx, value: &VecState<Option<Receipt>>) {
    let mut ctx_mut = ctx.lock();
    invert_vec(ctx_mut.receipts_mut(), value);
}

impl<M, S, Tx, Ecal> PartialEq for Interpreter<M, S, Tx, Ecal>
where
    M: Memory,
    Tx: PartialEq,
{
    /// Does not compare storage or debugger
    fn eq(&self, other: &Self) -> bool {
        self.registers == other.registers
            && self.memory.as_ref() == other.memory.as_ref()
            && self.frames == other.frames
            && self.receipts == other.receipts
            && self.tx == other.tx
            && self.initial_balances == other.initial_balances
            && self.context == other.context
            && self.balances == other.balances
            && self.interpreter_params == other.interpreter_params
            && self.panic_context == other.panic_context
    }
}

impl From<Diff<Deltas>> for Diff<InitialVmState> {
    fn from(d: Diff<Deltas>) -> Self {
        Self {
            changes: d
                .changes
                .into_iter()
                .map(|c| match c {
                    Change::Register(v) => Change::Register(v.into()),
                    Change::Memory(v) => Change::Memory(v),
                    Change::Storage(v) => Change::Storage(v.into()),
                    Change::Frame(v) => Change::Frame(v.into()),
                    Change::Receipt(v) => Change::Receipt(v.into()),
                    Change::Balance(v) => Change::Balance(v.into()),
                    Change::Context(v) => Change::Context(v.into()),
                    Change::PanicContext(v) => Change::PanicContext(v.into()),
                    Change::Txn(v) => Change::Txn(v.into()),
                })
                .collect(),
        }
    }
}

impl<T> From<Delta<T>> for Previous<T> {
    fn from(d: Delta<T>) -> Self {
        Self(d.to)
    }
}

impl<T: VmStateCapture + Clone> AddAssign for Diff<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.changes.extend(rhs.changes);
    }
}
