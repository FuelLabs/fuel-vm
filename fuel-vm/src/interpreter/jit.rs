//! Cranelift-based JIT acceleration for straight-line ALU basic blocks.
//!
//! Design (see `devlog/`): the interpreter is the source of truth. The JIT only
//! compiles *maximal straight-line runs of "simple" ALU opcodes* (no control flow,
//! no memory/storage/crypto/calls). A compiled block:
//!   * operates directly on the real `[u64; VM_REGISTER_COUNT]` register array
//!     (CGAS/GGAS/PC/OF/ERR are just entries in that array),
//!   * charges gas inline per instruction, exactly like `gas_charge`,
//!   * replicates `alu_set` / `alu_capture_overflow` semantics for the no-overflow
//!     fast path, and
//!   * *bails to the interpreter* for anything it cannot prove safe at compile time
//!     (overflow/underflow, out-of-gas), by leaving registers/gas/PC untouched for
//!     the bailing instruction and returning the count of instructions it completed.
//!
//! Because every uncertain case bails to the interpreter, semantics are bit-identical
//! to pure interpretation; worst case the JIT executes zero instructions and the
//! interpreter runs as before. This is what lets the full test-suite stay green with
//! the JIT enabled.
//!
//! This module necessarily transmutes JIT-compiled code pointers and dereferences the
//! raw register-array pointer, so `unsafe_code` is allowed here only.
#![allow(unsafe_code)]
// Codegen translates register indices (0..64), small byte offsets, gas costs and
// instruction counts into Cranelift's i64/i32 immediate operands. These casts are
// intentional and bounded; the strict numeric clippy lints add only noise here.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]

use alloc::vec::Vec;
use core::mem;
use hashbrown::HashMap;

use cranelift_codegen::{
    ir::{AbiParam, InstBuilder, MemFlags, Value, condcodes::IntCC, types},
    settings::{self, Configurable},
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};

use fuel_asm::{Instruction, Opcode, RegId};
use fuel_tx::GasCosts;

use crate::{consts::MEM_SIZE, interpreter::MemoryInstance};

// `Vec`'s in-memory header layout is not a stable language guarantee; on the toolchain
// this was built against (rustc 1.96, post-`RawVecInner` refactor) it is `{cap@0, ptr@8,
// len@16}` on 64-bit. Both offsets are pinned by `tests::vec_layout_matches_jit_assumption`
// — a layout change fails that gate loudly instead of miscompiling loads into OOB reads.
/// Byte offset of `Vec`'s data pointer within its header.
const VEC_PTR_OFF: i32 = 8;
/// Byte offset of `Vec`'s `len` within its header.
const VEC_LEN_OFF: i32 = 16;

// Register file indices (these are just offsets into `registers: [u64; 64]`).
const R_OF: u32 = RegId::OF.to_u8() as u32;
const R_PC: u32 = RegId::PC.to_u8() as u32;
const R_ERR: u32 = RegId::ERR.to_u8() as u32;
const R_GGAS: u32 = RegId::GGAS.to_u8() as u32;
const R_CGAS: u32 = RegId::CGAS.to_u8() as u32;
/// First writable (program) register. Writes below this index error in the
/// interpreter, so such instructions are not JIT-eligible.
const FIRST_WRITABLE: u8 = RegId::WRITABLE.to_u8();

const INSTR_SIZE: i64 = Instruction::SIZE as i64;

/// A compiled block: `fn(regs_base_ptr) -> instructions_executed`.
pub type BlockFn = unsafe extern "C" fn(*const JitCtx) -> u64;

// ---- JIT v2 scaffolding: host-fn (thunk) inlining of non-ALU ops --------------
// A compiled block v2 receives a `*const JitCtx` instead of the bare regs pointer, so
// it can both (a) do native ALU/memory work on the register array and (b) call back into
// the interpreter via a specialized thunk (`spec`) for ops we don't emit as IR — staying
// in native code across them instead of returning to the dispatcher. Registers live in
// memory (the shared array), so a thunk call needs no register marshalling.
#[repr(C)]
pub struct JitCtx {
    /// `&mut interpreter.registers[0]` — the live `[u64; 64]` array.
    pub regs: *mut u64,
    /// Type-erased `&mut Interpreter<..>`, passed back to `step`.
    pub interp: *mut core::ffi::c_void,
    /// Caller-owned slot a thunk writes a non-`Proceed`/error result into. Typed as
    /// `*mut Option<Result<ExecuteState, InterpreterError<S::DataError>>>` by the runner
    /// (the specialized thunk knows the concrete `S`). Type-erased here so `JitCtx` stays
    /// non-generic — and so no `'static` bound leaks into the public API.
    pub exit_out: *mut core::ffi::c_void,
    /// Base pointer of the per-monomorphization [`SpecThunkFn`] table (see `spec_table`).
    /// A compiled `Thunk`/`Term` step loads `spec[idx]` and calls it — skipping the
    /// opcode re-decode + dispatch match `Interpreter::instruction` would do.
    pub spec: *const SpecThunkFn,
    /// `&interpreter.memory` (the live [`MemoryInstance`]). Native bounds-checked loads
    /// (LW/LB/LQW/LHW) re-read the stack/heap `Vec` base+len and `hp` watermark from this
    /// on every access — the struct address is stable for the block, but the `Vec`s
    /// inside reallocate on growth, so a base captured once would go stale mid-block.
    pub mem: *const crate::interpreter::MemoryInstance,
    /// Monomorphized [`chain_dispatch`]: after a block's trailing jump sets `regs[PC]`, the
    /// block calls this to run the *next* already-compiled block in-place (reusing this same
    /// `ctx`) instead of returning to the dispatcher — threaded dispatch across the block
    /// edge. Returns 0 if it didn't chain (next block not cached / depth-capped / disabled),
    /// else the chained run's instruction count or [`EXIT_SENTINEL`].
    pub chain: ChainFn,
}

/// Block return value with this bit set ⇒ a thunk exited the block; the result is in the
/// caller's `exit_out` slot. Otherwise the value is the number of instructions run.
pub const EXIT_SENTINEL: u64 = 1 << 63;

/// A *specialized* thunk bound to one concrete opcode (and one `Interpreter<..>`
/// monomorphization), so it skips the per-execution `Opcode::try_from` + the ~80-arm
/// dispatch match that `Interpreter::instruction` walks. ABI:
/// `extern "C" fn(*const JitCtx, u32) -> u64`; takes the raw instruction word, runs that
/// opcode's audited `op::X::from_raw_args(..).execute(..)` directly, returns 0 on `Proceed`
/// or [`EXIT_SENTINEL`] after stashing a non-`Proceed`/error result into `ctx.exit_out`.
pub type SpecThunkFn = unsafe extern "C" fn(*const JitCtx, u32) -> u64;

/// The exit-stash type written through `JitCtx.exit_out`.
type SpecExit<S> = Option<
    Result<
        crate::state::ExecuteState,
        crate::error::InterpreterError<<S as crate::storage::InterpreterStorage>::DataError>,
    >,
>;

// ---- Native block chaining (threaded dispatch across block edges) ---------------
//
// After a block's trailing jump runs (its specialized thunk sets `regs[PC]` to the
// resolved target), the block calls [`chain_dispatch`] to run the *next* block in native
// code — reusing the same `JitCtx` — instead of returning to the interpreter's dispatch
// loop. This is correct-by-construction and *safe*: it reuses the same content-validated
// `get_block` lookup the dispatcher uses (so stack-reused code can't alias the wrong
// block), only chains to **already-compiled** blocks (never compiles mid-execution), and
// is depth-capped to bound native-stack growth on cyclic chains (loops). Gas/PC/exit
// semantics are identical to returning to the dispatcher — only the round-trip is elided.

/// Block-chaining helper installed in `JitCtx.chain`. ABI:
/// `extern "C" fn(*const JitCtx) -> u64`. Returns 0 if it did not chain (next block not
/// cached, depth cap hit, or chaining disabled), else the chained run's instruction count
/// or [`EXIT_SENTINEL`].
pub type ChainFn = unsafe extern "C" fn(*const JitCtx) -> u64;

/// Max native-frame depth for chained blocks before falling back to the dispatcher. Bounds
/// native-stack growth on cyclic chains (e.g. a JAL loop): the chain runs in bursts of this
/// many blocks, re-entering the dispatcher (at depth 0) between bursts.
pub(crate) const CHAIN_DEPTH_CAP: u32 = 32;

thread_local! {
    /// Current native chain-call nesting depth (see [`CHAIN_DEPTH_CAP`]). Incremented around
    /// each chained `f(ctx)` call and restored after; balanced, so it is 0 at every
    /// top-level dispatch entry.
    static CHAIN_DEPTH: core::cell::Cell<u32> = const { core::cell::Cell::new(0) };
}

/// Runtime on/off switch for chaining (for A/B measurement via `FUEL_VM_JIT_CHAIN`). When
/// off, [`chain_dispatch`] always returns 0 (every block returns to the dispatcher), so the
/// only residual cost vs. the no-chain build is one predictable, returns-0 call per edge.
/// 0 = unread, 1 = on, 2 = off.
static CHAIN_ENABLED: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);

fn chain_enabled() -> bool {
    use core::sync::atomic::Ordering;
    match CHAIN_ENABLED.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            // Default ON; only "0" disables.
            let on = std::env::var("FUEL_VM_JIT_CHAIN").as_deref() != Ok("0");
            CHAIN_ENABLED.store(if on { 1 } else { 2 }, Ordering::Relaxed);
            on
        }
    }
}

/// Current chain depth; if it is below [`CHAIN_DEPTH_CAP`], the caller may run one more
/// chained block (after which it must restore the depth via [`set_chain_depth`]).
pub(crate) fn chain_depth() -> u32 {
    CHAIN_DEPTH.with(core::cell::Cell::get)
}

pub(crate) fn set_chain_depth(d: u32) {
    CHAIN_DEPTH.with(|c| c.set(d));
}

/// `JitCtx.chain` entry point: thin `extern "C"` wrapper that reconstructs the concrete
/// `Interpreter` from `ctx.interp` and runs one chained block via `jit_chain_step`.
///
/// # Safety
/// Same contract as the thunks: `(*ctx).interp` is a valid `&mut Interpreter<M,S,Tx,Ecal,V>`
/// for this monomorphization (the fn pointer was stored from it), and `ctx` outlives the
/// call. Only invoked from compiled blocks after a trailing jump has set `regs[PC]`.
pub unsafe extern "C" fn chain_dispatch<M, S, Tx, Ecal, V>(ctx: *const JitCtx) -> u64
where
    M: super::Memory,
    S: crate::storage::InterpreterStorage,
    Tx: super::ExecutableTransaction,
    Ecal: super::EcalHandler,
    V: crate::verification::Verifier,
{
    if !chain_enabled() {
        return 0;
    }
    // SAFETY: caller guarantees `interp` matches this monomorphization.
    let interp =
        unsafe { &mut *((*ctx).interp as *mut super::Interpreter<M, S, Tx, Ecal, V>) };
    interp.jit_chain_step(ctx)
}

/// Generates, for each listed opcode, a specialized `extern "C"` thunk that runs exactly
/// that opcode via its audited interpreter impl (`op::X::from_raw_args(..).execute(..)`),
/// replicating `Interpreter::instruction`'s error wrapping — but WITHOUT the opcode
/// re-decode + giant match it performs on every call. Also emits `spec_table` (the
/// per-monomorphization fn-pointer table; a constant array ⇒ promoted to static rodata,
/// so building it per dispatch is free) and `spec_index` (opcode → dense table slot).
///
/// Semantics are identical to the generic thunk: it reuses the same op impls and the same
/// `InterpreterError::from_runtime(.., raw)` wrapping, so it cannot diverge. The only
/// elided work is decode/dispatch the JIT already did at compile time (and the inactive
/// debugger check, which the JIT dispatch path guarantees is off).
macro_rules! define_spec_thunks {
    ($($op:ident),+ $(,)?) => { paste::paste! {
        $(
            #[allow(non_snake_case)]
            unsafe extern "C" fn [<spec_thunk_ $op>]<M, S, Tx, Ecal, V>(
                ctx: *const JitCtx,
                raw: u32,
            ) -> u64
            where
                M: super::Memory,
                S: crate::storage::InterpreterStorage,
                Tx: super::ExecutableTransaction,
                Ecal: super::EcalHandler,
                V: crate::verification::Verifier,
            {
                use crate::interpreter::executors::instruction::Execute;
                // SAFETY: `interp`/`exit_out` match this monomorphization (the table
                // pointer in `ctx.spec` was built from it).
                let interp = unsafe {
                    &mut *((*ctx).interp as *mut super::Interpreter<M, S, Tx, Ecal, V>)
                };
                let b = raw.to_be_bytes();
                // Mirror `Interpreter::instruction`: run the op, then wrap any
                // `RuntimeError` into an `InterpreterError` carrying the raw word.
                let res: Result<
                    crate::state::ExecuteState,
                    crate::error::InterpreterError<S::DataError>,
                > = match fuel_asm::op::$op::from_raw_args([b[1], b[2], b[3]]) {
                    Ok(o) => o.execute(interp),
                    Err(_) => Err(crate::error::RuntimeError::from(
                        fuel_asm::PanicReason::InvalidInstruction,
                    )),
                }
                .map_err(|e| crate::error::InterpreterError::from_runtime(e, raw));
                match res {
                    Ok(crate::state::ExecuteState::Proceed) => 0,
                    other => {
                        // SAFETY: `exit_out` is a `*mut SpecExit<S>` for this same `S`.
                        unsafe {
                            *((*ctx).exit_out as *mut SpecExit<S>) = Some(other);
                        }
                        EXIT_SENTINEL
                    }
                }
            }
        )+

        /// Count of specialized opcodes (size of the per-monomorph table).
        const N_SPEC: usize = [ $( { let _ = stringify!($op); 1usize } ),+ ].len();

        /// Per-monomorphization table of specialized thunks, in `spec_index` order.
        pub fn spec_table<M, S, Tx, Ecal, V>() -> [SpecThunkFn; N_SPEC]
        where
            M: super::Memory,
            S: crate::storage::InterpreterStorage,
            Tx: super::ExecutableTransaction,
            Ecal: super::EcalHandler,
            V: crate::verification::Verifier,
        {
            [ $( [<spec_thunk_ $op>]::<M, S, Tx, Ecal, V> ),+ ]
        }

        /// Dense table index of an opcode the JIT runs via a specialized thunk, if any.
        fn spec_index(op: Opcode) -> Option<u32> {
            let mut i = 0u32;
            $(
                if op == Opcode::$op {
                    return Some(i);
                }
                i = i.wrapping_add(1);
            )+
            let _ = i;
            None
        }
    } };
}

// Union of `is_thunkable` (mid-block) and `is_terminator_thunkable` (trailing control
// flow). Every opcode that can appear as `BlockStep::Thunk`/`Term` MUST be listed here;
// `scan_block` `.expect()`s a slot, so any omission fails loudly in the test suite.
define_spec_thunks!(
    // memory load/store/copy/compare/alloc + stack frame adjust
    LB, LW, LQW, LHW, SB, SW, SQW, SHW, MCL, MCLI, MCP, MCPI, MEQ,
    ALOC, CFEI, CFSI, CFE, CFS, POPL, POPH, PSHL, PSHH,
    // transaction/VM field reads
    GTF, GM,
    // arithmetic not emitted as native IR
    MUL, MULI, DIV, DIVI, MOD, MODI, EXP, EXPI, MLOG, MROO,
    // 256/128-bit wide-int math
    WDCM, WQCM, WDOP, WQOP, WDML, WQML, WDDV, WQDV, WDMD, WQMD,
    WDAM, WQAM, WDMM, WQMM, MLDV,
    // misc register/flag ops
    FLAG, MOVE, NOOP,
    // control-flow terminators (run as the block's last step)
    JAL, JMP, JI, JNE, JNEI, JNZI, JNZF, JNZB, JMPF, JMPB, JNEF,
);

/// Field embedded in the `Interpreter`. Lazily creates a [`JitRuntime`] on first
/// use. Cloning an interpreter yields a fresh (empty) JIT — the cache is a pure
/// runtime accelerator, so it is safe to drop on clone and rebuild lazily.
pub struct JitState {
    enabled: bool,
    verify: bool,
}

impl Default for JitState {
    fn default() -> Self {
        Self {
            enabled: true,
            verify: std::env::var("FUEL_VM_JIT_VERIFY").as_deref() == Ok("1"),
        }
    }
}

impl Clone for JitState {
    fn clone(&self) -> Self {
        Self {
            enabled: self.enabled,
            verify: self.verify,
        }
    }
}

impl core::fmt::Debug for JitState {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("JitState")
            .field("enabled", &self.enabled)
            .field("verify", &self.verify)
            .finish()
    }
}

thread_local! {
    /// Thread-wide JIT runtime so compiled blocks are reused across `Interpreter`
    /// instances — the executor builds a fresh interpreter per tx, and we don't want to
    /// recompile every time. `(tried, runtime)`: `tried` guards one-shot Cranelift init.
    static RUNTIME: core::cell::RefCell<(bool, Option<JitRuntime>)> =
        const { core::cell::RefCell::new((false, None)) };
}

impl JitState {
    /// Try to run a compiled block whose bytecode begins at `window`. See
    /// [`JitRuntime::run_block`].
    ///
    /// # Safety
    /// `regs` must point to a valid `[u64; VM_REGISTER_COUNT]` array and `window` must
    /// be the executable bytecode at `regs[PC]`.
    /// Compile/look up the block at `pc` (no execution). Returns a `Copy` fn pointer so
    /// the caller can drop all interpreter borrows before running it.
    pub fn get_block(
        &mut self,
        window: &[u8],
        g: &GasCosts,
        pc: u64,
        allow_thunks: bool,
    ) -> Option<BlockFn> {
        if !self.enabled {
            return None;
        }
        RUNTIME.with(|cell| {
            let mut slot = cell.borrow_mut();
            if !slot.0 {
                slot.0 = true;
                slot.1 = JitRuntime::new();
            }
            slot.1.as_mut()?.get_block(window, g, pc, allow_thunks)
        })
    }

    /// Memo-only lookup of an **already-compiled** block at `pc` (no compilation). Used by
    /// block chaining, which must never compile while another block is executing natively.
    /// Returns `None` on any miss (uncompiled, content mismatch, or stale gas schedule),
    /// in which case the caller returns to the dispatcher (which compiles safely).
    pub fn get_block_cached(
        &mut self,
        window: &[u8],
        g: &GasCosts,
        pc: u64,
        allow_thunks: bool,
    ) -> Option<BlockFn> {
        if !self.enabled {
            return None;
        }
        RUNTIME.with(|cell| {
            cell.borrow_mut()
                .1
                .as_mut()?
                .get_block_cached(window, g, pc, allow_thunks)
        })
    }

    /// Add to the executed-instruction telemetry counter (post block call).
    pub fn add_executed(&self, n: u64) {
        RUNTIME.with(|cell| {
            if let Some(rt) = cell.borrow_mut().1.as_mut() {
                rt.add_executed(n);
            }
        });
    }

    /// Enable/disable the JIT at runtime (e.g. for A/B benchmarking). When
    /// disabled, every block falls through to the interpreter.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Whether the JIT is currently enabled (false ⇒ pure interpretation).
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// True when FUEL_VM_JIT_VERIFY=1: run each JIT block, then re-run the same
    /// instructions in the interpreter and compare register state to catch codegen bugs.
    pub fn verify_enabled(&self) -> bool {
        self.verify
    }

    /// Total instructions executed through compiled blocks (0 if never initialized).
    pub fn executed_instrs(&self) -> u64 {
        RUNTIME.with(|cell| cell.borrow().1.as_ref().map_or(0, JitRuntime::executed_instrs))
    }
}

/// Diagnostic histogram of which opcodes terminate JIT blocks (i.e. force a return to
/// the interpreter dispatcher). Env-gated by `FUEL_VM_JIT_STATS=1`; off by default and
/// off any hot path. Used to size the control-flow-JIT opportunity on real workloads.
pub mod stats {
    use super::{HashMap, Instruction, Opcode};
    use core::{
        cell::RefCell,
        sync::atomic::{AtomicU8, Ordering},
    };

    static ENABLED: AtomicU8 = AtomicU8::new(0);

    pub fn enabled() -> bool {
        match ENABLED.load(Ordering::Relaxed) {
            1 => true,
            2 => false,
            _ => {
                let on = std::env::var("FUEL_VM_JIT_STATS").as_deref() == Ok("1");
                ENABLED.store(if on { 1 } else { 2 }, Ordering::Relaxed);
                on
            }
        }
    }

    thread_local! {
        // key: (opcode byte, dyn-reg class), val: count.
        // class: 0 = not a relative jump, 1 = dynamic reg is ZERO (statically mappable),
        //        2 = dynamic reg is non-zero (runtime target).
        static HIST: RefCell<HashMap<(u8, u8), u64>> = RefCell::new(HashMap::new());
    }

    /// The dynamic (register) operand of a relative jump, used to decide if the target is
    /// statically computable (reg == ZERO ⇒ pure-immediate offset). `None` for non-jumps.
    fn dyn_reg(instr: Instruction) -> Option<u8> {
        use fuel_asm::Instruction::*;
        Some(match instr {
            JMPF(op) => op.unpack().0.to_u8(),
            JMPB(op) => op.unpack().0.to_u8(),
            JNZF(op) => op.unpack().1.to_u8(),
            JNZB(op) => op.unpack().1.to_u8(),
            JNEF(op) => op.unpack().2.to_u8(),
            JNEB(op) => op.unpack().2.to_u8(),
            _ => return None,
        })
    }

    thread_local! {
        // JAL call-site PC -> (target PC -> count). Sized to gauge the block-linking win.
        static JAL: RefCell<HashMap<u64, HashMap<u64, u64>>> =
            RefCell::new(HashMap::new());
    }

    pub fn record(raw: u32) {
        let Ok(instr) = Instruction::try_from(raw.to_be_bytes()) else {
            return;
        };
        let op = instr.opcode() as u8;
        let class = match dyn_reg(instr) {
            None => 0,
            Some(0) => 1,
            Some(_) => 2,
        };
        HIST.with(|h| *h.borrow_mut().entry((op, class)).or_insert(0) += 1);
    }

    /// Record a JAL call-site PC and its (runtime-computed) target PC.
    pub fn record_jal(site: u64, target: u64) {
        JAL.with(|j| {
            *j.borrow_mut()
                .entry(site)
                .or_default()
                .entry(target)
                .or_insert(0) += 1;
        });
    }

    pub fn dump() {
        HIST.with(|h| {
            let h = h.borrow();
            if h.is_empty() {
                return;
            }
            let mut rows: alloc::vec::Vec<((u8, u8), u64)> =
                h.iter().map(|(k, v)| (*k, *v)).collect();
            rows.sort_by_key(|(_, n)| core::cmp::Reverse(*n));
            let total: u64 = rows.iter().map(|(_, v)| *v).sum();
            std::eprintln!("=== JIT block terminators (total {total}) ===");
            for ((op, class), n) in rows {
                let opcode = Opcode::try_from(op)
                    .map(|o| alloc::format!("{o:?}"))
                    .unwrap_or_else(|_| alloc::format!("op{op}"));
                let tag = match class {
                    1 => " [dyn-reg=ZERO → static target]",
                    2 => " [dyn-reg≠0 → runtime target]",
                    _ => "",
                };
                let pct = 100.0 * n as f64 / total as f64;
                std::eprintln!("  {opcode:<8} {n:>8}  {pct:5.1}%{tag}");
            }
        });
        JAL.with(|j| {
            let j = j.borrow();
            if j.is_empty() {
                return;
            }
            let sites = j.len();
            let mut targets = HashMap::new();
            let mut total = 0u64;
            for tmap in j.values() {
                for (t, c) in tmap {
                    *targets.entry(*t).or_insert(0u64) += *c;
                    total += *c;
                }
            }
            // How many call-sites resolve to a single (monomorphic) target?
            let mono = j.values().filter(|t| t.len() == 1).count();
            std::eprintln!(
                "=== JAL linking potential: {total} calls, {sites} distinct call-sites ({mono} monomorphic), {} distinct targets ===",
                targets.len()
            );
        });
    }
}

/// One JIT-eligible instruction, decoded into the minimal info codegen needs.
#[derive(Clone, Copy, Debug)]
struct DecodedOp {
    kind: OpKind,
    gas: u64,
}

#[derive(Clone, Copy, Debug)]
enum Rhs {
    Reg(u8),
    Imm(u64),
}

/// The "pure" ops use `alu_set` semantics (OF=0, ERR=0, dest=val, never overflow).
/// The "overflowing" ops (`Add`/`Sub`) bail to the interpreter on carry/borrow.
#[derive(Clone, Copy, Debug)]
enum OpKind {
    /// dest = b (+/-) rhs, with bail-on-overflow. `sub=false` => add.
    AddSub {
        dest: u8,
        b: u8,
        rhs: Rhs,
        sub: bool,
    },
    /// dest = constant immediate (MOVI).
    Movi { dest: u8, imm: u64 },
    /// dest = reg (MOVE).
    Move { dest: u8, src: u8 },
    /// dest = !reg (NOT).
    Not { dest: u8, src: u8 },
    /// dest = b <bitop> rhs (AND/OR/XOR + immediate variants).
    Bit {
        dest: u8,
        b: u8,
        rhs: Rhs,
        op: BitOp,
    },
    /// dest = (b <cmp> c) as u64 (EQ/LT/GT).
    Cmp { dest: u8, b: u8, c: u8, cc: Cmp },
    /// dest = shift(b, rhs) with >=64 => 0 (SLL/SRL + immediate variants).
    Shift {
        dest: u8,
        b: u8,
        rhs: Rhs,
        right: bool,
    },
    /// NOOP: alu_clear (OF=0, ERR=0, no dest write).
    Noop,
    /// Native bounds-checked memory load (LB/LQW/LHW/LW): `dest = mem[base + offset]`,
    /// `nbytes` wide, big-endian, zero-extended. Bails to the interpreter (preserving the
    /// exact panic) for anything not trivially in-bounds. Does NOT touch OF/ERR.
    Load {
        dest: u8,
        base: u8,
        /// `imm * nbytes`, precomputed (≤ 4095*8, never overflows a Word).
        offset: u64,
        /// 1 (LB), 2 (LQW), 4 (LHW), or 8 (LW).
        nbytes: u8,
    },
}

#[derive(Clone, Copy, Debug)]
enum BitOp {
    And,
    Or,
    Xor,
}

#[derive(Clone, Copy, Debug)]
enum Cmp {
    Eq,
    Lt,
    Gt,
}

/// Decode a single instruction into a JIT-eligible op, or `None` if it is not in
/// the supported straight-line subset (the block terminates before it).
fn decode_op(instr: Instruction, g: &GasCosts) -> Option<DecodedOp> {
    use fuel_asm::Instruction as I;

    // Reject if dest is not a writable program register (interpreter would error).
    fn w(reg: RegId) -> Option<u8> {
        let r = reg.to_u8();
        (r >= FIRST_WRITABLE).then_some(r)
    }

    // Build a `Load` op. `offset = imm * nbytes` exactly as the interpreter's `store_load!`
    // macro computes it (`imm * size_of::<$t>()`); imm is u12, so this never overflows.
    fn load_kind(dest: u8, base: u8, imm: fuel_asm::Imm12, nbytes: u8) -> OpKind {
        OpKind::Load {
            dest,
            base,
            offset: u64::from(imm) * nbytes as u64,
            nbytes,
        }
    }

    let (kind, gas) = match instr {
        I::ADD(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::AddSub {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Reg(c.to_u8()),
                    sub: false,
                },
                g.add(),
            )
        }
        I::ADDI(op) => {
            let (a, b, imm) = op.unpack();
            (
                OpKind::AddSub {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Imm(u64::from(imm)),
                    sub: false,
                },
                g.addi(),
            )
        }
        I::SUB(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::AddSub {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Reg(c.to_u8()),
                    sub: true,
                },
                g.sub(),
            )
        }
        I::SUBI(op) => {
            let (a, b, imm) = op.unpack();
            (
                OpKind::AddSub {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Imm(u64::from(imm)),
                    sub: true,
                },
                g.subi(),
            )
        }
        I::MOVI(op) => {
            let (a, imm) = op.unpack();
            (
                OpKind::Movi {
                    dest: w(a)?,
                    imm: u64::from(imm),
                },
                g.movi(),
            )
        }
        I::MOVE(op) => {
            let (a, b) = op.unpack();
            (
                OpKind::Move {
                    dest: w(a)?,
                    src: b.to_u8(),
                },
                g.move_op(),
            )
        }
        I::NOT(op) => {
            let (a, b) = op.unpack();
            (
                OpKind::Not {
                    dest: w(a)?,
                    src: b.to_u8(),
                },
                g.not(),
            )
        }
        I::AND(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::Bit {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Reg(c.to_u8()),
                    op: BitOp::And,
                },
                g.and(),
            )
        }
        I::ANDI(op) => {
            let (a, b, imm) = op.unpack();
            (
                OpKind::Bit {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Imm(u64::from(imm)),
                    op: BitOp::And,
                },
                g.andi(),
            )
        }
        I::OR(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::Bit {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Reg(c.to_u8()),
                    op: BitOp::Or,
                },
                g.or(),
            )
        }
        I::ORI(op) => {
            let (a, b, imm) = op.unpack();
            (
                OpKind::Bit {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Imm(u64::from(imm)),
                    op: BitOp::Or,
                },
                g.ori(),
            )
        }
        I::XOR(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::Bit {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Reg(c.to_u8()),
                    op: BitOp::Xor,
                },
                g.xor(),
            )
        }
        I::XORI(op) => {
            let (a, b, imm) = op.unpack();
            (
                OpKind::Bit {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Imm(u64::from(imm)),
                    op: BitOp::Xor,
                },
                g.xori(),
            )
        }
        I::EQ(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::Cmp {
                    dest: w(a)?,
                    b: b.to_u8(),
                    c: c.to_u8(),
                    cc: Cmp::Eq,
                },
                g.eq_(),
            )
        }
        I::LT(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::Cmp {
                    dest: w(a)?,
                    b: b.to_u8(),
                    c: c.to_u8(),
                    cc: Cmp::Lt,
                },
                g.lt(),
            )
        }
        I::GT(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::Cmp {
                    dest: w(a)?,
                    b: b.to_u8(),
                    c: c.to_u8(),
                    cc: Cmp::Gt,
                },
                g.gt(),
            )
        }
        I::SLL(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::Shift {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Reg(c.to_u8()),
                    right: false,
                },
                g.sll(),
            )
        }
        I::SLLI(op) => {
            let (a, b, imm) = op.unpack();
            (
                OpKind::Shift {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Imm(u64::from(u32::from(imm))),
                    right: false,
                },
                g.slli(),
            )
        }
        I::SRL(op) => {
            let (a, b, c) = op.unpack();
            (
                OpKind::Shift {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Reg(c.to_u8()),
                    right: true,
                },
                g.srl(),
            )
        }
        I::SRLI(op) => {
            let (a, b, imm) = op.unpack();
            (
                OpKind::Shift {
                    dest: w(a)?,
                    b: b.to_u8(),
                    rhs: Rhs::Imm(u64::from(u32::from(imm))),
                    right: true,
                },
                g.srli(),
            )
        }
        I::NOOP(_) => (OpKind::Noop, g.noop()),
        // Native bounds-checked loads. `dest` must be a writable program register (the
        // interpreter's `WriteRegKey::try_from` errors otherwise); if not, we return
        // `None` here and the op is run via its thunk, which raises that exact error.
        // nbytes & gas mirror the interpreter: LB→u8/lb(), LQW→u16, LHW→u32, LW→u64/lw().
        I::LB(op) => {
            let (a, b, imm) = op.unpack();
            (load_kind(w(a)?, b.to_u8(), imm, 1), g.lb())
        }
        I::LQW(op) => {
            let (a, b, imm) = op.unpack();
            (load_kind(w(a)?, b.to_u8(), imm, 2), g.lw())
        }
        I::LHW(op) => {
            let (a, b, imm) = op.unpack();
            (load_kind(w(a)?, b.to_u8(), imm, 4), g.lw())
        }
        I::LW(op) => {
            let (a, b, imm) = op.unpack();
            (load_kind(w(a)?, b.to_u8(), imm, 8), g.lw())
        }
        _ => return None,
    };
    Some(DecodedOp { kind, gas })
}

/// Largest block we will scan/compile in one go (instructions). Longer eligible runs
/// are simply split across consecutive blocks.
const MAX_BLOCK_OPS: usize = 256;

/// One step of a compiled block: either native ALU/memory IR, or a callback into the
/// interpreter (a specialized thunk) for an op we don't emit as IR but which is safe to run
/// mid-block (advances PC by 4, no control-flow / no frame push / no code-window change).
#[derive(Clone, Copy, Debug)]
enum BlockStep {
    Native(DecodedOp),
    /// `spec` = index into the [`SpecThunkFn`] table; `raw` = the instruction word.
    Thunk { spec: u32, raw: u32 },
    /// A control-flow *terminator* run via the interpreter thunk as the block's last
    /// step (threaded dispatch). Unlike `Thunk`, the interpreter sets PC to the jump
    /// target/fallthrough itself, so the block returns immediately afterwards WITHOUT
    /// overwriting PC. This lets a block absorb its trailing JAL / relative jump instead
    /// of bouncing through a separate `NotEligible` dispatch + interpreter step per
    /// terminator — see [`is_terminator_thunkable`]. Semantics come for free (the audited
    /// interpreter impl runs), so this cannot diverge from the interpreter.
    /// `spec`/`raw` as in [`BlockStep::Thunk`].
    Term { spec: u32, raw: u32 },
}

/// Is `op` a control-flow terminator we can run as a block's final thunk step? Allowlist
/// of ops that (a) produce `ExecuteState::Proceed`, (b) set PC themselves (jump target or
/// fallthrough), and (c) never push/pop a call frame or change which contract code window
/// is executing. JAL is an intra-contract call (writes the return register + jumps within
/// the same code), so it qualifies; CALL/RET/RETD/RVRT do not (frame/window changes).
fn is_terminator_thunkable(op: Opcode) -> bool {
    use Opcode::*;
    matches!(
        op,
        JAL | JMP | JI | JNE | JNEI | JNZI | JNZF | JNZB | JMPF | JMPB | JNEF
    )
}

/// Is `opcode` safe to execute via a host thunk in the middle of a compiled block?
/// Allowlist of ops that (a) always advance PC by exactly 4 on success, (b) never push a
/// call frame or change which code window is executing, and (c) are not control flow.
/// Anything not listed terminates the block (handled by the interpreter dispatcher).
fn is_thunkable(op: Opcode) -> bool {
    use Opcode::*;
    matches!(
        op,
        // memory load/store/copy/compare/alloc + stack frame adjust
        LB | LW | LQW | LHW | SB | SW | SQW | SHW | MCL | MCLI | MCP | MCPI | MEQ
        | ALOC | CFEI | CFSI | CFE | CFS | POPL | POPH | PSHL | PSHH
        // transaction/VM field reads
        | GTF | GM
        // arithmetic not yet emitted as native IR
        | MUL | MULI | DIV | DIVI | MOD | MODI | EXP | EXPI | MLOG | MROO
        // 256/128-bit wide-int math (operate on memory, advance pc by 4)
        | WDCM | WQCM | WDOP | WQOP | WDML | WQML | WDDV | WQDV | WDMD | WQMD
        | WDAM | WQAM | WDMM | WQMM | MLDV
        // misc register/flag ops
        | FLAG | MOVE | NOOP
    )
}

/// Scan a maximal block from the start of `window`: native ALU ops plus (when
/// `allow_thunks`) thunkable non-ALU ops. Terminates at the first control-flow / call /
/// return / unknown op. `window` is the executable bytecode at the current PC.
fn scan_block(window: &[u8], g: &GasCosts, allow_thunks: bool) -> Vec<BlockStep> {
    let mut steps: Vec<BlockStep> = Vec::new();
    let mut pos = 0;
    while pos + 4 <= window.len() && steps.len() < MAX_BLOCK_OPS {
        let raw = [window[pos], window[pos + 1], window[pos + 2], window[pos + 3]];
        let Ok(instr) = Instruction::try_from(raw) else {
            break;
        };
        if let Some(op) = decode_op(instr, g) {
            steps.push(BlockStep::Native(op));
            pos += 4;
            continue;
        }
        if allow_thunks && is_thunkable(instr.opcode()) {
            let spec = spec_index(instr.opcode())
                .expect("is_thunkable opcode missing from define_spec_thunks!");
            steps.push(BlockStep::Thunk { spec, raw: u32::from_be_bytes(raw) });
            pos += 4;
            continue;
        }
        // Threaded dispatch: absorb a trailing control-flow terminator (JAL / relative
        // jump) into this block so we don't pay a separate dispatch round-trip + interp
        // step for it. Only worthwhile when the block already has real work before it (a
        // lone terminator is cheaper to run via the interpreter than to wrap in a block
        // call), so gate on a non-empty block.
        if allow_thunks && !steps.is_empty() && is_terminator_thunkable(instr.opcode()) {
            let spec = spec_index(instr.opcode())
                .expect("terminator opcode missing from define_spec_thunks!");
            steps.push(BlockStep::Term { spec, raw: u32::from_be_bytes(raw) });
        }
        break;
    }
    steps
}

/// Cheap fingerprint of the gas costs baked into compiled blocks, mixed into the cache
/// key so a block compiled under one gas schedule is never reused under another.
fn gas_fingerprint(g: &GasCosts) -> u64 {
    let costs = [
        g.add(), g.addi(), g.sub(), g.subi(), g.and(), g.andi(), g.or(), g.ori(),
        g.xor(), g.xori(), g.not(), g.move_op(), g.movi(), g.eq_(), g.lt(), g.gt(),
        g.sll(), g.slli(), g.srl(), g.srli(), g.noop(),
        // memory loads emitted as native IR (LB uses lb(); LQW/LHW/LW use lw())
        g.lb(), g.lw(),
    ];
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for c in costs {
        h ^= c;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// JIT runtime owned by an `Interpreter`. Holds the Cranelift module (and thus the
/// executable code) plus a block cache keyed by the block's raw bytecode.
///
/// Keying by bytecode content (rather than address) is what lets the JIT accelerate
/// *contract code run inside CALL frames*: compiled blocks are position-independent
/// (PC is read from the register array at runtime), so identical bytecode reuses the
/// same compiled block no matter where in memory — or in which call frame — it runs.
pub struct JitRuntime {
    module: JITModule,
    ctx: cranelift_codegen::Context,
    fctx: FunctionBuilderContext,
    /// Cache: block bytecode -> compiled block. Content-keyed, position-independent.
    cache: HashMap<Vec<u8>, BlockFn>,
    /// PC-indexed dispatch memo so steady-state dispatch is an O(1) lookup instead of
    /// re-scanning the bytecode every instruction. Verified by the first instruction's
    /// bytes (safe across stack reuse) and scoped to one gas schedule (`memo_gas_fp`).
    /// Value: (verified bytecode, compiled block or None if not eligible here). The
    /// bytecode is the full block for compiled entries (so stack-reused code at the same
    /// PC can't alias a wrong block) or the 4 bytes of the non-eligible head otherwise.
    memo: HashMap<u64, (Vec<u8>, Option<BlockFn>)>,
    memo_gas_fp: u64,
    gas_ptr: usize,
    gas_fp: u64,
    func_counter: u32,
    /// Total instructions executed via compiled blocks (telemetry / test proof).
    executed_instrs: u64,
}

impl core::fmt::Debug for JitRuntime {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("JitRuntime")
            .field("cached_blocks", &self.cache.len())
            .field("executed_instrs", &self.executed_instrs)
            .finish()
    }
}

impl JitRuntime {
    pub fn new() -> Option<Self> {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").ok()?;
        flag_builder.set("is_pic", "false").ok()?;
        // Speed up codegen a little; correctness does not depend on opt level.
        flag_builder.set("opt_level", "speed").ok()?;
        let isa_builder = cranelift_native::builder().ok()?;
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .ok()?;
        let builder =
            JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);
        let ctx = module.make_context();
        Some(Self {
            module,
            ctx,
            fctx: FunctionBuilderContext::new(),
            cache: HashMap::new(),
            memo: HashMap::new(),
            memo_gas_fp: 0,
            gas_ptr: 0,
            gas_fp: 0,
            func_counter: 0,
            executed_instrs: 0,
        })
    }

    /// Run a straight-line block starting at the beginning of `window` (the bytecode
    /// at the current PC, read from VM memory and bounded by the executable region).
    /// Returns the number of instructions executed, or `None` if nothing eligible.
    ///
    /// # Safety
    /// `regs` must point to a valid `[u64; VM_REGISTER_COUNT]` array, and `window`
    /// must be the bytecode at `regs[PC]`.
    /// Gas-costs fingerprint, cached by the `&GasCosts` pointer so the 21-accessor hash
    /// runs only when the gas schedule actually changes (not every block).
    fn fp(&mut self, g: &GasCosts) -> u64 {
        let ptr = core::ptr::from_ref(g).addr();
        if ptr != self.gas_ptr {
            self.gas_ptr = ptr;
            self.gas_fp = gas_fingerprint(g);
        }
        self.gas_fp
    }

    /// Look up (compiling on demand) the compiled block whose bytecode begins at
    /// `window`, or `None` if no eligible block starts there. Pure compile/lookup — the
    /// returned fn pointer is `Copy`, so the caller can drop all interpreter borrows
    /// before executing it (required: the block re-enters the interpreter via thunks).
    fn get_block(
        &mut self,
        window: &[u8],
        g: &GasCosts,
        pc: u64,
        allow_thunks: bool,
    ) -> Option<BlockFn> {
        if window.len() < 4 {
            return None;
        }
        // Fold `allow_thunks` into the schedule fingerprint so memo/cache never mix
        // thunk-enabled and native-only compilations.
        let fp = self.fp(g) ^ if allow_thunks { 0xA11 } else { 0 };
        if fp != self.memo_gas_fp {
            self.memo.clear();
            self.memo_gas_fp = fp;
        }
        // Fast path: PC already classified for this fingerprint and the bytecode still
        // matches (full block for compiled entries, head for non-eligible ones — guards
        // against stack-reused code at the same address).
        if let Some((code, blk)) = self.memo.get(&pc)
            && window.len() >= code.len()
            && window[..code.len()] == code[..]
        {
            return *blk;
        }
        let steps = scan_block(window, g, allow_thunks);
        let (code, blk): (Vec<u8>, Option<BlockFn>) = if steps.is_empty() {
            // First instruction fully determines non-eligibility: 4-byte head.
            (window[..4].to_vec(), None)
        } else {
            let blen = steps.len() * 4;
            let mut key = Vec::with_capacity(8 + blen);
            key.extend_from_slice(&fp.to_le_bytes());
            key.extend_from_slice(&window[..blen]);
            let f = match self.cache.get(&key) {
                Some(f) => *f,
                None => {
                    let f = self.compile(&steps)?;
                    self.cache.insert(key, f);
                    f
                }
            };
            (window[..blen].to_vec(), Some(f))
        };
        self.memo.insert(pc, (code, blk));
        blk
    }

    /// Memo-only variant of [`Self::get_block`] for chaining: returns an already-compiled
    /// block at `pc` validated against `window`, or `None` on any miss — never scans or
    /// compiles (compiling while another block executes natively could re-protect live code
    /// pages). A stale gas-schedule memo also returns `None` (the dispatcher rebuilds it).
    fn get_block_cached(
        &mut self,
        window: &[u8],
        g: &GasCosts,
        pc: u64,
        allow_thunks: bool,
    ) -> Option<BlockFn> {
        if window.len() < 4 {
            return None;
        }
        let fp = self.fp(g) ^ if allow_thunks { 0xA11 } else { 0 };
        if fp != self.memo_gas_fp {
            return None;
        }
        if let Some((code, blk)) = self.memo.get(&pc)
            && window.len() >= code.len()
            && window[..code.len()] == code[..]
        {
            return *blk;
        }
        None
    }

    fn add_executed(&mut self, n: u64) {
        self.executed_instrs = self.executed_instrs.saturating_add(n);
    }

    /// Total instructions executed through compiled blocks so far.
    pub fn executed_instrs(&self) -> u64 {
        self.executed_instrs
    }

    fn compile(&mut self, steps: &[BlockStep]) -> Option<BlockFn> {
        let ptr_ty = self.module.target_config().pointer_type();
        let call_conv = self.module.target_config().default_call_conv;
        self.module.clear_context(&mut self.ctx);
        // Param: *const JitCtx (the block reads regs/interp/step from it).
        self.ctx.func.signature.params.push(AbiParam::new(ptr_ty));
        self.ctx
            .func
            .signature
            .returns
            .push(AbiParam::new(types::I64));

        {
            let mut b = FunctionBuilder::new(&mut self.ctx.func, &mut self.fctx);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            b.seal_block(entry);
            let ctx = b.block_params(entry)[0];
            let flags = MemFlags::trusted();
            // JitCtx (repr(C)) = { regs@0, interp@8, exit_out@16, spec@24, mem@32, chain@40 }.
            // regs = ctx.regs (offset 0).
            let regs = b.ins().load(ptr_ty, flags, ctx, 0);
            // mem = ctx.mem (offset 32): the live `MemoryInstance`. Native loads re-read
            // its stack/heap Vec base+len + hp from this on every access (the struct's
            // address is stable; the Vecs inside reallocate on growth). Unused (DCE'd) for
            // blocks with no native loads.
            let mem = b.ins().load(ptr_ty, flags, ctx, 32);
            // Signature of a specialized thunk: extern "C" fn(*const JitCtx, u32) -> u64.
            let thunk_sig = {
                let mut sig = cranelift_codegen::ir::Signature::new(call_conv);
                sig.params.push(AbiParam::new(ptr_ty));
                sig.params.push(AbiParam::new(types::I32));
                sig.returns.push(AbiParam::new(types::I64));
                b.import_signature(sig)
            };
            // Signature of the chain helper: extern "C" fn(*const JitCtx) -> u64.
            let chain_sig = {
                let mut sig = cranelift_codegen::ir::Signature::new(call_conv);
                sig.params.push(AbiParam::new(ptr_ty));
                sig.returns.push(AbiParam::new(types::I64));
                b.import_signature(sig)
            };
            let load = |b: &mut FunctionBuilder, reg: u32| -> Value {
                b.ins().load(types::I64, flags, regs, (reg * 8) as i32)
            };
            let store = |b: &mut FunctionBuilder, reg: u32, v: Value| {
                b.ins().store(flags, v, regs, (reg * 8) as i32);
            };

            // base_pc = regs[PC] at block entry; addr of instr i = base_pc + 4*i.
            let base_pc = load(&mut b, R_PC);

            // Emit a bail: store the bailing instruction's pc, return `done` count.
            let bail = |b: &mut FunctionBuilder, done: usize| {
                let pc_i = b.ins().iadd_imm(base_pc, done as i64 * INSTR_SIZE);
                b.ins().store(flags, pc_i, regs, (R_PC * 8) as i32);
                let n = b.ins().iconst(types::I64, done as i64);
                b.ins().return_(&[n]);
            };

            let rhs_val = |b: &mut FunctionBuilder, rhs: Rhs| -> Value {
                match rhs {
                    Rhs::Reg(r) => load(b, r as u32),
                    Rhs::Imm(i) => b.ins().iconst(types::I64, i as i64),
                }
            };

            // Set when the final step is a `Term` (it returns inside the loop with PC
            // already set by the interpreter), so the trailing PC-store/return below is
            // skipped (and would be unreachable codegen into a terminated block).
            let mut ends_with_term = false;
            for (i, step) in steps.iter().enumerate() {
                // Keep `regs[PC]` at the *current* instruction's address so that ops
                // reading `$pc` (reg 0x3) as a source operand — e.g. Sway's PC-relative
                // `ADD r, $pc, imm` — and the thunked interpreter ops observe the same
                // PC the interpreter would (it increments PC per instruction).
                let pc_cur = b.ins().iadd_imm(base_pc, i as i64 * INSTR_SIZE);
                store(&mut b, R_PC, pc_cur);

                // Thunk step: call back into the interpreter for one instruction. On a
                // non-Proceed/error result the thunk returns EXIT_SENTINEL and we hand
                // control back to the dispatcher (which takes the stashed result).
                let op = match *step {
                    BlockStep::Native(o) => o,
                    BlockStep::Thunk { spec, raw } => {
                        // Load the specialized thunk `spec_base[spec]` and call it; it
                        // reads `interp`/`exit_out` from `ctx`, so pass `ctx` first.
                        emit_thunk_call(&mut b, ptr_ty, flags, thunk_sig, ctx, spec, raw);
                        continue;
                    }
                    BlockStep::Term { spec, raw } => {
                        // Threaded-dispatch terminator: run the jump via its specialized
                        // thunk (the interpreter sets PC to the resolved target/
                        // fallthrough). Do NOT overwrite PC with base+4*n — `pc_cur` (stored
                        // above) is this instruction's address, which the jump reads for
                        // PC-relative targets. Term is always the last step.
                        emit_thunk_call(&mut b, ptr_ty, flags, thunk_sig, ctx, spec, raw);
                        // Block chaining: with PC now at the resolved target, try to run the
                        // next already-compiled block in-place (reusing `ctx`) instead of
                        // bouncing through the dispatcher. `chain` returns 0 if it didn't
                        // chain, else the chained run's count or EXIT_SENTINEL.
                        let own = b.ins().iconst(types::I64, (i + 1) as i64);
                        let chain_fn = b.ins().load(ptr_ty, flags, ctx, 40);
                        let call = b.ins().call_indirect(chain_sig, chain_fn, &[ctx]);
                        let r = b.inst_results(call)[0];
                        // r == 0  -> didn't chain: return our own count.
                        // r has EXIT_SENTINEL -> a chained block exited: propagate it.
                        // else -> chained normally: return own + chained count (telemetry;
                        //         the magnitude is only used for the executed-instr counter,
                        //         and own+r can't set bit 63).
                        let no_chain = b.create_block();
                        let chained = b.create_block();
                        b.ins().brif(r, chained, &[], no_chain, &[]);

                        b.switch_to_block(no_chain);
                        b.seal_block(no_chain);
                        b.ins().return_(&[own]);

                        b.switch_to_block(chained);
                        b.seal_block(chained);
                        let sentinel = b.ins().iconst(types::I64, EXIT_SENTINEL as i64);
                        let exit_bit = b.ins().band(r, sentinel);
                        let prop_exit = b.create_block();
                        let sum_count = b.create_block();
                        b.ins().brif(exit_bit, prop_exit, &[], sum_count, &[]);
                        b.switch_to_block(prop_exit);
                        b.seal_block(prop_exit);
                        b.ins().return_(&[r]);
                        b.switch_to_block(sum_count);
                        b.seal_block(sum_count);
                        let total = b.ins().iadd(own, r);
                        b.ins().return_(&[total]);

                        ends_with_term = true;
                        continue;
                    }
                };
                let cost = op.gas as i64;

                // --- overflow bail (AddSub only), computed BEFORE charging gas ---
                if let OpKind::AddSub {
                    b: rb, rhs, sub, ..
                } = op.kind
                {
                    let bv = load(&mut b, rb as u32);
                    let cv = rhs_val(&mut b, rhs);
                    let (res, of_cond) = if sub {
                        // borrow = bv < cv (unsigned)
                        let r = b.ins().isub(bv, cv);
                        let c = b.ins().icmp(IntCC::UnsignedLessThan, bv, cv);
                        (r, c)
                    } else {
                        // carry = (bv + cv) < bv (unsigned)
                        let r = b.ins().iadd(bv, cv);
                        let c = b.ins().icmp(IntCC::UnsignedLessThan, r, bv);
                        (r, c)
                    };
                    let cont = b.create_block();
                    let bail_b = b.create_block();
                    b.ins().brif(of_cond, bail_b, &[], cont, &[]);
                    b.switch_to_block(bail_b);
                    b.seal_block(bail_b);
                    bail(&mut b, i);
                    b.switch_to_block(cont);
                    b.seal_block(cont);

                    // gas check
                    emit_gas_or_bail(&mut b, regs, flags, cost, &bail, i);
                    // commit: dest, OF=0, ERR=0
                    if let OpKind::AddSub { dest, .. } = op.kind {
                        let zero = b.ins().iconst(types::I64, 0);
                        store(&mut b, dest as u32, res);
                        store(&mut b, R_OF, zero);
                        store(&mut b, R_ERR, zero);
                    }
                    continue;
                }

                // --- native bounds-checked load (LB/LQW/LHW/LW) ---
                // Bails to the interpreter for anything not trivially in-bounds, so the
                // exact `MemoryOverflow`/`UninitializedMemoryAccess` panic is preserved.
                if let OpKind::Load {
                    dest,
                    base,
                    offset,
                    nbytes,
                } = op.kind
                {
                    let int_ty = match nbytes {
                        1 => types::I8,
                        2 => types::I16,
                        4 => types::I32,
                        _ => types::I64,
                    };
                    // VM-memory loads are at arbitrary (unaligned) addresses; non-trapping
                    // because we have just bounds-checked the access.
                    let mem_flags = MemFlags::new().with_notrap();

                    // addr = regs[base] + offset, with the interpreter's `checked_add` —
                    // bail on wrap (would be `MemoryOverflow`).
                    let bv = load(&mut b, base as u32);
                    let off_v = b.ins().iconst(types::I64, offset as i64);
                    let addr = b.ins().iadd(bv, off_v);
                    let carry = b.ins().icmp(IntCC::UnsignedLessThan, addr, bv);
                    let cont0 = b.create_block();
                    let bail0 = b.create_block();
                    b.ins().brif(carry, bail0, &[], cont0, &[]);
                    b.switch_to_block(bail0);
                    b.seal_block(bail0);
                    bail(&mut b, i);
                    b.switch_to_block(cont0);
                    b.seal_block(cont0);

                    // Require addr + nbytes <= MEM_SIZE (mirrors verify()'s end>MEM_SIZE →
                    // Overflow) so addr+nbytes can't wrap and spuriously pass a bound.
                    let max_addr =
                        b.ins().iconst(types::I64, (MEM_SIZE - nbytes as usize) as i64);
                    let in_ram = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, addr, max_addr);
                    let cont1 = b.create_block();
                    let bail1 = b.create_block();
                    b.ins().brif(in_ram, cont1, &[], bail1, &[]);
                    b.switch_to_block(bail1);
                    b.seal_block(bail1);
                    bail(&mut b, i);
                    b.switch_to_block(cont1);
                    b.seal_block(cont1);

                    // Re-read the LIVE stack/heap Vec base+len and hp (they move on growth).
                    let s_off = MemoryInstance::JIT_STACK_OFFSET as i32;
                    let h_off = MemoryInstance::JIT_HEAP_OFFSET as i32;
                    let hp_off = MemoryInstance::JIT_HP_OFFSET as i32;
                    let stack_ptr = b.ins().load(ptr_ty, flags, mem, s_off + VEC_PTR_OFF);
                    let stack_len = b.ins().load(types::I64, flags, mem, s_off + VEC_LEN_OFF);
                    let heap_ptr = b.ins().load(ptr_ty, flags, mem, h_off + VEC_PTR_OFF);
                    let heap_len = b.ins().load(types::I64, flags, mem, h_off + VEC_LEN_OFF);
                    let hp = b.ins().load(types::I64, flags, mem, hp_off);

                    // verify(): valid iff `end <= stack.len()` (stack) or `start >= hp`
                    // (heap). read() prefers the stack branch when both hold.
                    let nb = b.ins().iconst(types::I64, nbytes as i64);
                    let end = b.ins().iadd(addr, nb);
                    let stack_hit =
                        b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, stack_len);
                    let heap_hit =
                        b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, addr, hp);
                    let valid = b.ins().bor(stack_hit, heap_hit);
                    let cont2 = b.create_block();
                    let bail2 = b.create_block();
                    b.ins().brif(valid, cont2, &[], bail2, &[]);
                    b.switch_to_block(bail2);
                    b.seal_block(bail2);
                    bail(&mut b, i);
                    b.switch_to_block(cont2);
                    b.seal_block(cont2);

                    // Charge gas after the bounds check (the read is side-effect-free; the
                    // interpreter charges gas before the read, but bailing on OOG defers the
                    // whole instruction to it, so any ordering is preserved).
                    emit_gas_or_bail(&mut b, regs, flags, cost, &bail, i);

                    // Effective byte pointer. Stack: base+addr. Heap: base+(addr-heap_off),
                    // heap_off = MEM_SIZE - heap.len(). Both computed branchlessly; `select`
                    // takes the live one (heap arm is dead arithmetic when stack_hit).
                    let stack_eff = b.ins().iadd(stack_ptr, addr);
                    let mem_size_v = b.ins().iconst(types::I64, MEM_SIZE as i64);
                    let heap_base = b.ins().isub(mem_size_v, heap_len);
                    let heap_idx = b.ins().isub(addr, heap_base);
                    let heap_eff = b.ins().iadd(heap_ptr, heap_idx);
                    let eff = b.ins().select(stack_hit, stack_eff, heap_eff);

                    // `$t::from_be_bytes(..) as u64`: native (LE) load, byte-swap to
                    // big-endian, zero-extend to 64 bits.
                    let raw = b.ins().load(int_ty, mem_flags, eff, 0);
                    let val = if nbytes == 1 {
                        b.ins().uextend(types::I64, raw)
                    } else {
                        let swapped = b.ins().bswap(raw);
                        if nbytes == 8 {
                            swapped
                        } else {
                            b.ins().uextend(types::I64, swapped)
                        }
                    };
                    store(&mut b, dest as u32, val);
                    // Load touches neither OF nor ERR.
                    continue;
                }

                // --- pure ops: gas check, then compute + commit ---
                emit_gas_or_bail(&mut b, regs, flags, cost, &bail, i);

                let zero = b.ins().iconst(types::I64, 0);
                match op.kind {
                    OpKind::Movi { dest, imm } => {
                        let v = b.ins().iconst(types::I64, imm as i64);
                        store(&mut b, dest as u32, v);
                        store(&mut b, R_OF, zero);
                        store(&mut b, R_ERR, zero);
                    }
                    OpKind::Move { dest, src } => {
                        let v = load(&mut b, src as u32);
                        store(&mut b, dest as u32, v);
                        store(&mut b, R_OF, zero);
                        store(&mut b, R_ERR, zero);
                    }
                    OpKind::Not { dest, src } => {
                        let v = load(&mut b, src as u32);
                        let v = b.ins().bnot(v);
                        store(&mut b, dest as u32, v);
                        store(&mut b, R_OF, zero);
                        store(&mut b, R_ERR, zero);
                    }
                    OpKind::Bit {
                        dest,
                        b: rb,
                        rhs,
                        op: bop,
                    } => {
                        let bv = load(&mut b, rb as u32);
                        let cv = rhs_val(&mut b, rhs);
                        let v = match bop {
                            BitOp::And => b.ins().band(bv, cv),
                            BitOp::Or => b.ins().bor(bv, cv),
                            BitOp::Xor => b.ins().bxor(bv, cv),
                        };
                        store(&mut b, dest as u32, v);
                        store(&mut b, R_OF, zero);
                        store(&mut b, R_ERR, zero);
                    }
                    OpKind::Cmp {
                        dest,
                        b: rb,
                        c: rc,
                        cc,
                    } => {
                        let bv = load(&mut b, rb as u32);
                        let cv = load(&mut b, rc as u32);
                        let icc = match cc {
                            Cmp::Eq => IntCC::Equal,
                            Cmp::Lt => IntCC::UnsignedLessThan,
                            Cmp::Gt => IntCC::UnsignedGreaterThan,
                        };
                        let cmp = b.ins().icmp(icc, bv, cv);
                        let v = b.ins().uextend(types::I64, cmp);
                        store(&mut b, dest as u32, v);
                        store(&mut b, R_OF, zero);
                        store(&mut b, R_ERR, zero);
                    }
                    OpKind::Shift {
                        dest,
                        b: rb,
                        rhs,
                        right,
                    } => {
                        let bv = load(&mut b, rb as u32);
                        let v = match rhs {
                            Rhs::Imm(sh) => {
                                if sh >= 64 {
                                    b.ins().iconst(types::I64, 0)
                                } else {
                                    let amt = b.ins().iconst(types::I64, sh as i64);
                                    if right {
                                        b.ins().ushr(bv, amt)
                                    } else {
                                        b.ins().ishl(bv, amt)
                                    }
                                }
                            }
                            Rhs::Reg(r) => {
                                let cv = load(&mut b, r as u32);
                                let shifted = if right {
                                    b.ins().ushr(bv, cv)
                                } else {
                                    b.ins().ishl(bv, cv)
                                };
                                let sixtyfour = b.ins().iconst(types::I64, 64);
                                let in_range =
                                    b.ins().icmp(IntCC::UnsignedLessThan, cv, sixtyfour);
                                let z = b.ins().iconst(types::I64, 0);
                                b.ins().select(in_range, shifted, z)
                            }
                        };
                        store(&mut b, dest as u32, v);
                        store(&mut b, R_OF, zero);
                        store(&mut b, R_ERR, zero);
                    }
                    OpKind::Noop => {
                        // alu_clear: OF=0, ERR=0, no dest write.
                        store(&mut b, R_OF, zero);
                        store(&mut b, R_ERR, zero);
                    }
                    OpKind::AddSub { .. } | OpKind::Load { .. } => {
                        unreachable!("handled above")
                    }
                }
            }

            // Block completed: pc advanced past all steps; return count. Skipped when the
            // final step was a `Term` (it already returned with PC set by the interpreter).
            if !ends_with_term {
                let n = steps.len();
                let final_pc = b.ins().iadd_imm(base_pc, n as i64 * INSTR_SIZE);
                b.ins().store(flags, final_pc, regs, (R_PC * 8) as i32);
                let nval = b.ins().iconst(types::I64, n as i64);
                b.ins().return_(&[nval]);
            }

            b.finalize();
        }

        let name = {
            self.func_counter += 1;
            alloc::format!("jit_block_{}", self.func_counter)
        };
        let id = self
            .module
            .declare_function(&name, Linkage::Export, &self.ctx.func.signature)
            .ok()?;
        self.module.define_function(id, &mut self.ctx).ok()?;
        self.module.clear_context(&mut self.ctx);
        self.module.finalize_definitions().ok()?;
        let code = self.module.get_finalized_function(id);
        // SAFETY: signature matches BlockFn by construction.
        Some(unsafe { mem::transmute::<*const u8, BlockFn>(code) })
    }
}

/// Emit a call to the specialized thunk `ctx.spec[spec]` with `(ctx, raw)`. On a non-zero
/// (EXIT_SENTINEL) result the block returns immediately (the thunk stashed the exit);
/// otherwise the builder is left positioned in the fall-through block so the caller can
/// continue emitting. `ctx.spec` is at byte offset 24; entries are 8-byte fn pointers.
#[allow(clippy::too_many_arguments)]
fn emit_thunk_call(
    b: &mut FunctionBuilder,
    ptr_ty: types::Type,
    flags: MemFlags,
    thunk_sig: cranelift_codegen::ir::SigRef,
    ctx: Value,
    spec: u32,
    raw: u32,
) {
    let spec_base = b.ins().load(ptr_ty, flags, ctx, 24);
    let f = b.ins().load(ptr_ty, flags, spec_base, (spec * 8) as i32);
    let rawv = b.ins().iconst(types::I32, raw as i64);
    let call = b.ins().call_indirect(thunk_sig, f, &[ctx, rawv]);
    let r = b.inst_results(call)[0];
    let cont = b.create_block();
    let exit_b = b.create_block();
    b.ins().brif(r, exit_b, &[], cont, &[]);
    b.switch_to_block(exit_b);
    b.seal_block(exit_b);
    let sentinel = b.ins().iconst(types::I64, EXIT_SENTINEL as i64);
    b.ins().return_(&[sentinel]);
    b.switch_to_block(cont);
    b.seal_block(cont);
}

/// Emit an inline gas check matching `gas_charge`'s happy path; bail on OOG.
fn emit_gas_or_bail(
    b: &mut FunctionBuilder,
    regs: Value,
    flags: MemFlags,
    cost: i64,
    bail: &dyn Fn(&mut FunctionBuilder, usize),
    i: usize,
) {
    let cgas = b.ins().load(types::I64, flags, regs, (R_CGAS * 8) as i32);
    let ggas = b.ins().load(types::I64, flags, regs, (R_GGAS * 8) as i32);
    let cost_v = b.ins().iconst(types::I64, cost);
    // OOG when cost > cgas
    let oog = b.ins().icmp(IntCC::UnsignedGreaterThan, cost_v, cgas);
    let cont = b.create_block();
    let bail_b = b.create_block();
    b.ins().brif(oog, bail_b, &[], cont, &[]);
    b.switch_to_block(bail_b);
    b.seal_block(bail_b);
    bail(b, i);
    b.switch_to_block(cont);
    b.seal_block(cont);
    let new_cgas = b.ins().isub(cgas, cost_v);
    let new_ggas = b.ins().isub(ggas, cost_v);
    b.ins().store(flags, new_cgas, regs, (R_CGAS * 8) as i32);
    b.ins().store(flags, new_ggas, regs, (R_GGAS * 8) as i32);
}

#[cfg(test)]
mod tests {
    /// The native-load codegen reads a `Vec`'s data pointer at byte 0 and `len` at byte 16
    /// of its header (64-bit: `ptr, cap, len`). That layout is not a language guarantee, so
    /// pin it here: any future `Vec` layout change fails this (a required gate) loudly
    /// instead of miscompiling LW/LB/LQW/LHW into out-of-bounds native reads.
    #[test]
    fn vec_layout_matches_jit_assumption() {
        let mut v: alloc::vec::Vec<u8> = alloc::vec::Vec::with_capacity(8);
        v.push(0xAB);
        v.push(0xCD);
        v.push(0xEF);
        let base = core::ptr::from_ref(&v).cast::<u8>();
        // SAFETY: reading the machine words backing the `Vec` header in-place, at the byte
        // offsets the native-load codegen uses.
        let (raw_ptr, raw_len) = unsafe {
            (
                base.add(super::VEC_PTR_OFF as usize).cast::<usize>().read(),
                base.add(super::VEC_LEN_OFF as usize).cast::<usize>().read(),
            )
        };
        assert_eq!(raw_ptr, v.as_ptr() as usize, "Vec data ptr not at VEC_PTR_OFF");
        assert_eq!(raw_len, v.len(), "Vec len not at VEC_LEN_OFF");
    }
}
