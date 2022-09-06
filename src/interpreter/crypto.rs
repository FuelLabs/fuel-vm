use super::Interpreter;
use crate::consts::{MEM_MAX_ACCESS_SIZE, VM_MAX_RAM};
use crate::error::RuntimeError;

use fuel_asm::PanicReason;
use fuel_crypto::{Hasher, Message, PublicKey, Signature};
use fuel_types::{Bytes32, Bytes64, Word};

impl<S> Interpreter<S> {
    pub(crate) fn ecrecover(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        if a > VM_MAX_RAM.saturating_sub(Bytes64::LEN as Word)
            || b > VM_MAX_RAM.saturating_sub(Bytes64::LEN as Word)
            || c > VM_MAX_RAM.saturating_sub(Bytes32::LEN as Word)
        {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b, c) = (a as usize, b as usize, c as usize);

        let bx = b.saturating_add(Bytes64::LEN);
        let cx = c.saturating_add(Bytes32::LEN);

        // Safety: memory bounds are checked
        let signature = unsafe { Signature::as_ref_unchecked(&self.memory[b..bx]) };
        let message = unsafe { Message::as_ref_unchecked(&self.memory[c..cx]) };

        match signature.recover(message) {
            Ok(public) => {
                self.try_mem_write(a, public.as_ref())?;
                self.clear_err();
            }
            Err(_) => {
                self.try_zeroize(a, PublicKey::LEN)?;
                self.set_err();
            }
        }

        self.inc_pc()
    }

    pub(crate) fn keccak256(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        use sha3::{Digest, Keccak256};

        if a > VM_MAX_RAM.saturating_sub(Bytes32::LEN as Word)
            || c > MEM_MAX_ACCESS_SIZE
            || b > VM_MAX_RAM.saturating_sub(c)
        {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b, c) = (a as usize, b as usize, c as usize);
        let bc = b.saturating_add(c);

        let mut h = Keccak256::new();

        h.update(&self.memory[b..bc]);

        self.try_mem_write(a, h.finalize().as_slice())?;

        self.inc_pc()
    }

    pub(crate) fn sha256(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        if a > VM_MAX_RAM.saturating_sub(Bytes32::LEN as Word)
            || c > MEM_MAX_ACCESS_SIZE
            || b > VM_MAX_RAM.saturating_sub(c)
        {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b, c) = (a as usize, b as usize, c as usize);
        let bc = b.saturating_add(c);

        self.try_mem_write(a, Hasher::hash(&self.memory[b..bc]).as_ref())?;

        self.inc_pc()
    }
}
