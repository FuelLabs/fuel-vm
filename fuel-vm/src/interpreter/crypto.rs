use super::{
    internal::{
        clear_err,
        inc_pc,
        set_err,
    },
    memory::{
        read_bytes,
        try_mem_write,
        try_zeroize,
        OwnershipRegisters,
    },
    ExecutableTransaction,
    Interpreter,
};
use crate::{
    constraints::reg_key::*,
    consts::*,
    error::RuntimeError,
};

use crate::arith::{
    checked_add_word,
    checked_sub_word,
};
use fuel_asm::PanicReason;
use fuel_crypto::{
    Hasher,
    Message,
    PublicKey,
    Signature,
};
use fuel_types::{
    Bytes32,
    Bytes64,
    Word,
};

#[cfg(test)]
mod tests;

impl<S, Tx> Interpreter<S, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn secp256k1_recover(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        secp256k1_recover(&mut self.memory, owner, err, pc, a, b, c)
    }

    pub(crate) fn secp256r1_recover(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        secp256r1_recover(&mut self.memory, owner, err, pc, a, b, c)
    }

    pub(crate) fn ed25519_verify(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> Result<(), RuntimeError> {
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        ed25519_verify(&mut self.memory, err, pc, a, b, c)
    }

    pub(crate) fn keccak256(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> Result<(), RuntimeError> {
        let owner = self.ownership_registers();
        keccak256(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }

    pub(crate) fn sha256(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> Result<(), RuntimeError> {
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
    let sig = Bytes64::from(read_bytes(memory, b)?);
    let msg = Bytes32::from(read_bytes(memory, c)?);

    let signature = Signature::from_bytes_ref(&sig);
    let message = Message::from_bytes_ref(&msg);

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
    let sig = Bytes64::from(read_bytes(memory, b)?);
    let msg = Bytes32::from(read_bytes(memory, c)?);
    let message = Message::from_bytes_ref(&msg);

    match fuel_crypto::secp256r1::recover(&sig, message) {
        Ok(pub_key) => {
            try_mem_write(a, &*pub_key, owner, memory)?;
            clear_err(err);
        }
        Err(_) => {
            try_zeroize(a, Bytes32::LEN, owner, memory)?;
            set_err(err);
        }
    }

    inc_pc(pc)
}

pub(crate) fn ed25519_verify(
    memory: &mut [u8; MEM_SIZE],
    err: RegMut<ERR>,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> Result<(), RuntimeError> {
    let pub_key = Bytes32::from(read_bytes(memory, a)?);
    let sig = Bytes64::from(read_bytes(memory, b)?);
    let msg = Bytes32::from(read_bytes(memory, c)?);
    let message = Message::from_bytes_ref(&msg);

    if fuel_crypto::ed25519::verify(&pub_key, &sig, message).is_ok() {
        clear_err(err);
    } else {
        set_err(err);
    }

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
    use sha3::{
        Digest,
        Keccak256,
    };

    let bc = checked_add_word(b, c)?;

    if a > checked_sub_word(VM_MAX_RAM, Bytes32::LEN as Word)?
        || c > MEM_MAX_ACCESS_SIZE
        || bc > MIN_VM_MAX_RAM_USIZE_MAX
    {
        return Err(PanicReason::MemoryOverflow.into())
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
        return Err(PanicReason::MemoryOverflow.into())
    }

    let (a, b, bc) = (a as usize, b as usize, bc as usize);

    try_mem_write(a, Hasher::hash(&memory[b..bc]).as_ref(), owner, memory)?;

    inc_pc(pc)
}
