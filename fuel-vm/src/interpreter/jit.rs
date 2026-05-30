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
// it can both (a) do native ALU on the register array and (b) call back into the
// interpreter (`step`) for ops we don't emit as IR — staying in native code across them
// instead of returning to the dispatcher. Registers live in memory (the shared array),
// so a `step` call needs no register marshalling.
#[repr(C)]
pub struct JitCtx {
    /// `&mut interpreter.registers[0]` — the live `[u64; 64]` array.
    pub regs: *mut u64,
    /// Type-erased `&mut Interpreter<..>`, passed back to `step`.
    pub interp: *mut core::ffi::c_void,
    /// Caller-owned slot the thunk writes a non-`Proceed`/error result into. Typed as
    /// `*mut Option<Result<ExecuteState, InterpreterError<S::DataError>>>` by the runner
    /// (`step_thunk` knows the concrete `S`). Type-erased here so `JitCtx` stays
    /// non-generic — and so no `'static` bound leaks into the public API.
    pub exit_out: *mut core::ffi::c_void,
    /// Monomorphized thunk that runs ONE instruction via the interpreter (generic
    /// fallback; the specialized table below is the hot path).
    pub step: StepFn,
    /// Base pointer of the per-monomorphization [`SpecThunkFn`] table (see `spec_table`).
    /// A compiled `Thunk`/`Term` step loads `spec[idx]` and calls it — skipping the
    /// opcode re-decode + dispatch match the generic `step` would do.
    pub spec: *const SpecThunkFn,
}

/// Runs one instruction (reading `interp`/`exit_out` from the passed `JitCtx`); returns 0
/// on `Proceed`, or [`EXIT_SENTINEL`] when the op produced a non-`Proceed`/error result
/// (written into `ctx.exit_out`).
pub type StepFn = unsafe extern "C" fn(*const JitCtx, u32) -> u64;

/// Block return value with this bit set ⇒ a thunk exited the block; the result is in the
/// caller's `exit_out` slot. Otherwise the value is the number of instructions run.
pub const EXIT_SENTINEL: u64 = 1 << 63;

/// Monomorphized per concrete `Interpreter<..>`: runs one decoded instruction, writing any
/// non-`Proceed`/error result into the caller's typed `exit_out` slot.
///
/// # Safety
/// `(*ctx).interp` must be a valid `&mut Interpreter<M,S,Tx,Ecal,V>` and `(*ctx).exit_out`
/// a valid `*mut Option<Result<ExecuteState, InterpreterError<S::DataError>>>`. Only
/// called with the matching monomorphization (the fn pointer is stored in `JitCtx.step`
/// by code that knows the concrete type).
pub unsafe extern "C" fn step_thunk<M, S, Tx, Ecal, V>(
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
    type Exit<S> = Option<
        Result<
            crate::state::ExecuteState,
            crate::error::InterpreterError<<S as crate::storage::InterpreterStorage>::DataError>,
        >,
    >;
    // SAFETY: caller guarantees `ctx` and its `interp`/`exit_out` match this
    // monomorphization.
    let interp = unsafe {
        &mut *((*ctx).interp as *mut super::Interpreter<M, S, Tx, Ecal, V>)
    };
    match interp.instruction::<u32, false>(raw) {
        Ok(crate::state::ExecuteState::Proceed) => 0,
        other => {
            // SAFETY: `exit_out` is a `*mut Exit<S>` for this same `S`.
            unsafe {
                *((*ctx).exit_out as *mut Exit<S>) = Some(other);
            }
            EXIT_SENTINEL
        }
    }
}


/// A *specialized* thunk: like [`StepFn`] but bound to one concrete opcode, so it skips
/// the per-execution `Opcode::try_from` + the ~80-arm dispatch match that
/// `Interpreter::instruction` walks. Takes the raw instruction word; runs that opcode's
/// audited `op::X::from_raw_args(..).execute(..)` directly. Same ABI as [`StepFn`].
pub type SpecThunkFn = unsafe extern "C" fn(*const JitCtx, u32) -> u64;

/// The exit-stash type written through `JitCtx.exit_out` (must match `step_thunk`'s).
type SpecExit<S> = Option<
    Result<
        crate::state::ExecuteState,
        crate::error::InterpreterError<<S as crate::storage::InterpreterStorage>::DataError>,
    >,
>;

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
                // SAFETY: same contract as `step_thunk` — `interp`/`exit_out` match this
                // monomorphization (the table pointer was built from it).
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
        _ => return None,
    };
    Some(DecodedOp { kind, gas })
}

/// Largest block we will scan/compile in one go (instructions). Longer eligible runs
/// are simply split across consecutive blocks.
const MAX_BLOCK_OPS: usize = 256;

/// One step of a compiled block: either native ALU IR, or a callback into the
/// interpreter (`step_thunk`) for an op we don't emit as IR but which is safe to run
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
            // regs = ctx.regs (offset 0); step loaded per-thunk (offset 24).
            let regs = b.ins().load(ptr_ty, flags, ctx, 0);
            // Signature of `StepFn`: extern "C" fn(*const JitCtx, u32) -> u64.
            let thunk_sig = {
                let mut sig = cranelift_codegen::ir::Signature::new(call_conv);
                sig.params.push(AbiParam::new(ptr_ty));
                sig.params.push(AbiParam::new(types::I32));
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
                        // `JitCtx` = { regs@0, interp@8, exit_out@16, step@24, spec@32 }.
                        // Load the specialized thunk `spec_base[spec]` and call it; it
                        // reads `interp`/`exit_out` from `ctx`, so pass `ctx` first.
                        emit_thunk_call(&mut b, ptr_ty, flags, thunk_sig, ctx, spec, raw);
                        continue;
                    }
                    BlockStep::Term { spec, raw } => {
                        // Threaded-dispatch terminator: run the jump via its specialized
                        // thunk (the interpreter sets PC to the resolved target/
                        // fallthrough) and return the count immediately — do NOT overwrite
                        // PC with base+4*n. `pc_cur` (stored above) is this instruction's
                        // address, which the jump reads for PC-relative targets. Term is
                        // always the last step.
                        emit_thunk_call(&mut b, ptr_ty, flags, thunk_sig, ctx, spec, raw);
                        let n = b.ins().iconst(types::I64, (i + 1) as i64);
                        b.ins().return_(&[n]);
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
                    OpKind::AddSub { .. } => unreachable!("handled above"),
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
/// continue emitting. `ctx.spec` is at byte offset 32; entries are 8-byte fn pointers.
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
    let spec_base = b.ins().load(ptr_ty, flags, ctx, 32);
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
