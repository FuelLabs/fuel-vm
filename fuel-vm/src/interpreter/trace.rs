//! Execution traces

use fuel_asm::Word;

use alloc::vec::Vec;

use super::{
    memory::MemoryRollbackData,
    Interpreter,
    Memory,
    VM_REGISTER_COUNT,
};

/// When to record a new snapshot
#[derive(Debug, Clone, Copy)]
pub enum Trigger {
    /// Capture state after an instruction adds a new receipt
    OnReceipt,
    /// Capture state after each instruction
    OnInstruction,
}

/// Used to capture an execution trace
#[derive(Debug, Clone)]
pub struct ExecutionTracer<M> {
    /// When should we take a new snapshot, i.e. insert a frame?
    trigger: Trigger,
    /// Append-only set of frames
    frames: Vec<Frame>,
    /// Memory at the time of the previous snapshot
    previous_memory: M,
}

/// Snapshot of the execution state, with some delta compression applied
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Frame {
    /// Registers at this point
    #[serde(with = "serde_big_array::BigArray")]
    pub registers: [Word; VM_REGISTER_COUNT],
    /// Memory delta from the previous snapshot
    pub memory_diff: Option<MemoryRollbackData>,
    /// How many of the receipts have been added by now
    pub receipt_count: usize,
}

impl<M, S, Tx, Ecal> Interpreter<M, S, Tx, Ecal>
where
    M: Memory,
{
    /// This is called after each instruction, and it should record a new snapshot
    /// if `trigger` condition is met.
    pub fn with_trace_recording(mut self, trigger: Trigger, memory: M) -> Self {
        self.trace = Some(ExecutionTracer {
            trigger,
            frames: Vec::new(),
            previous_memory: memory,
        });
        self
    }

    /// This is called after each instruction, and it should record a new snapshot
    /// if `trigger` condition is met.
    pub(crate) fn record_trace_after_instruction(&mut self) {
        let Some(trace) = self.trace.as_mut() else {
            return; // Trace disabled
        };

        let take_snapshot = match trace.trigger {
            Trigger::OnReceipt => {
                trace.frames.last().map(|s| s.receipt_count).unwrap_or(0)
                    < self.receipts.len()
            }
            Trigger::OnInstruction => true,
        };

        if take_snapshot {
            let memory_diff = trace
                .previous_memory
                .as_ref()
                .collect_rollback_data(self.memory.as_ref());
            if let Some(diff) = memory_diff.as_ref() {
                trace.previous_memory.as_mut().rollback(&diff);
            }

            trace.frames.push(Frame {
                memory_diff,
                registers: self.registers,
                receipt_count: self.receipts.len(),
            })
        }
    }

    /// Get trace frames at the current moment.
    /// Mostly useful after the execution.
    pub fn trace_frames(&self) -> &[Frame] {
        if let Some(trace) = self.trace.as_ref() {
            &trace.frames
        } else {
            &[]
        }
    }
}
