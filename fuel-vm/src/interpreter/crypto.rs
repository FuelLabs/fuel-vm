use super::internal::{clear_err, inc_pc, set_err};
use super::memory::{try_mem_write, try_zeroize, OwnershipRegisters};
use super::{ExecutableTransaction, Interpreter};
use crate::constraints::reg_key::*;
use crate::consts::{MEM_MAX_ACCESS_SIZE, MEM_SIZE, MIN_VM_MAX_RAM_USIZE_MAX, VM_MAX_RAM};
use crate::error::RuntimeError;

use crate::arith::{checked_add_word, checked_sub_word};
use crate::prelude::MemoryRange;
use fuel_asm::PanicReason;
use fuel_crypto::coins_bip32::prelude::RecoveryId;
use fuel_crypto::{Hasher, Message, PublicKey, Signature};
use fuel_types::{Bytes32, Bytes64, Word};

#[cfg(test)]
mod tests;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn secp256k1_recover(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        secp256k1_recover(&mut self.memory, owner, err, pc, a, b, c)
    }

    pub(crate) fn secp256r1_recover(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        secp256r1_recover(&mut self.memory, owner, err, pc, a, b, c)
    }

    pub(crate) fn keccak256(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        keccak256(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }

    pub(crate) fn sha256(&mut self, a: Word, b: Word, c: Word) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        sha256(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }
}

pub(crate) fn secp256k1_recover(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    err: RegMut<ERR>,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    let bx = checked_add_word(b, Bytes64::LEN as Word)?;
    let cx = checked_add_word(c, Bytes32::LEN as Word)?;

    if a > checked_sub_word(VM_MAX_RAM, Bytes64::LEN as Word)?
        || bx > MIN_VM_MAX_RAM_USIZE_MAX
        || cx > MIN_VM_MAX_RAM_USIZE_MAX
    {
        return Err(PanicReason::MemoryOverflow.into());
    }

    // TODO: These casts may overflow/truncate on 32-bit?
    let (a, b, bx, c, cx) = (a as usize, b as usize, bx as usize, c as usize, cx as usize);

    let sig_bytes = <&_>::try_from(&memory[b..bx]).expect("memory bounds checked");
    let msg_bytes = <&_>::try_from(&memory[c..cx]).expect("memory bounds checked");
    let signature = Signature::from_bytes_ref(sig_bytes);
    let message = Message::from_bytes_ref(msg_bytes);

    match signature.recover(message) {
        Ok(pub_key) => {
            try_mem_write(a, pub_key.as_ref(), owner, memory)?;
            clear_err(err);
        }
        Err(_) => {
            try_zeroize(a, PublicKey::LEN, owner, memory)?;
            set_err(err);
        }
    }

    inc_pc(pc)
}

pub(crate) fn secp256r1_recover(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    err: RegMut<ERR>,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    use p256::ecdsa::{Signature, VerifyingKey};

    let sig = MemoryRange::new(b, Bytes64::LEN)?;
    let msg = MemoryRange::new(c, Bytes32::LEN)?;

    let Ok(signature) = Signature::from_slice(&memory[sig.usizes()]) else {
        // Invalid signature, so there is no valid key to recover
        try_zeroize(a as usize, PublicKey::LEN, owner, memory)?;
        set_err(err);
        return inc_pc(pc);
    };

    // Attempt the four possible recovery ids
    for recid in 0..RecoveryId::MAX {
        let recid = RecoveryId::from_byte(recid).unwrap();
        if let Ok(pub_key) = VerifyingKey::recover_from_msg(&memory[msg.usizes()], &signature, recid) {
            try_mem_write(a, &pub_key.to_sec1_bytes(), owner, memory)?;
            clear_err(err);
            return inc_pc(pc);
        }
    }

    // None of the keys matched
    try_zeroize(a as usize, PublicKey::LEN, owner, memory)?;
    set_err(err);
    inc_pc(pc)
}

pub(crate) fn keccak256(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    use sha3::{Digest, Keccak256};

    let bc = checked_add_word(b, c)?;

    if a > checked_sub_word(VM_MAX_RAM, Bytes32::LEN as Word)?
        || c > MEM_MAX_ACCESS_SIZE
        || bc > MIN_VM_MAX_RAM_USIZE_MAX
    {
        return Err(PanicReason::MemoryOverflow.into());
    }

    let (a, b, bc) = (a as usize, b as usize, bc as usize);

    let mut h = Keccak256::new();

    h.update(&memory[b..bc]);

    try_mem_write(a, h.finalize().as_slice(), owner, memory)?;

    inc_pc(pc)
}

pub(crate) fn sha256(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    let bc = checked_add_word(b, c)?;

    if a > checked_sub_word(VM_MAX_RAM, Bytes32::LEN as Word)?
        || c > MEM_MAX_ACCESS_SIZE
        || bc > MIN_VM_MAX_RAM_USIZE_MAX
    {
        return Err(PanicReason::MemoryOverflow.into());
    }

    let (a, b, bc) = (a as usize, b as usize, bc as usize);

    try_mem_write(a, Hasher::hash(&memory[b..bc]).as_ref(), owner, memory)?;

    inc_pc(pc)
}
