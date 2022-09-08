use super::Interpreter;
use crate::consts::{MEM_MAX_ACCESS_SIZE, VM_MAX_RAM};
use crate::error::{Bug, BugId, BugVariant, RuntimeError};

use fuel_asm::PanicReason;
use fuel_crypto::{Hasher, Message, PublicKey, Signature};
use fuel_types::{Bytes32, Bytes64, Word};

impl<S> Interpreter<S> {
    pub(crate) fn ecrecover(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        if a > VM_MAX_RAM - Bytes64::LEN as Word
            || b > VM_MAX_RAM - Bytes64::LEN as Word
            || c > VM_MAX_RAM - Bytes32::LEN as Word
        {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b, c) = (a as usize, b as usize, c as usize);

        let bx = b
            .checked_add(Bytes64::LEN)
            .ok_or_else(|| Bug::new(BugId::ID010, BugVariant::CheckedRegisterOverflow))?;

        let cx = c
            .checked_add(Bytes32::LEN)
            .ok_or_else(|| Bug::new(BugId::ID011, BugVariant::CheckedRegisterOverflow))?;

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

        if a > VM_MAX_RAM - Bytes32::LEN as Word || c > MEM_MAX_ACCESS_SIZE || b > VM_MAX_RAM - c {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b, c) = (a as usize, b as usize, c as usize);

        let bc = b
            .checked_add(c)
            .ok_or_else(|| Bug::new(BugId::ID012, BugVariant::CheckedRegisterOverflow))?;

        let mut h = Keccak256::new();

        h.update(&self.memory[b..bc]);

        self.try_mem_write(a, h.finalize().as_slice())?;

        self.inc_pc()
    }

    pub(crate) fn sha256(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        if a > VM_MAX_RAM - Bytes32::LEN as Word || c > MEM_MAX_ACCESS_SIZE || b > VM_MAX_RAM - c {
            return Err(PanicReason::MemoryOverflow.into());
        }

        let (a, b, c) = (a as usize, b as usize, c as usize);
        let bc = b
            .checked_add(c)
            .ok_or_else(|| Bug::new(BugId::ID013, BugVariant::CheckedRegisterOverflow))?;

        self.try_mem_write(a, Hasher::hash(&self.memory[b..bc]).as_ref())?;

        self.inc_pc()
    }
}
