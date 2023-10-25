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
    error::SimpleResult,
    prelude::MemoryRange,
};

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

impl<S, Ecal, Tx> Interpreter<S, Ecal, Tx>
where
    Tx: ExecutableTransaction,
{
    pub(crate) fn secp256k1_recover(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        secp256k1_recover(&mut self.memory, owner, err, pc, a, b, c)
    }

    pub(crate) fn secp256r1_recover(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        secp256r1_recover(&mut self.memory, owner, err, pc, a, b, c)
    }

    pub(crate) fn ed25519_verify(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> SimpleResult<()> {
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        ed25519_verify(&mut self.memory, err, pc, a, b, c)
    }

    pub(crate) fn keccak256(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        keccak256(&mut self.memory, owner, self.registers.pc_mut(), a, b, c)
    }

    pub(crate) fn sha256(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
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
) -> SimpleResult<()> {
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

    Ok(inc_pc(pc)?)
}

pub(crate) fn secp256r1_recover(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    err: RegMut<ERR>,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
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

    Ok(inc_pc(pc)?)
}

pub(crate) fn ed25519_verify(
    memory: &mut [u8; MEM_SIZE],
    err: RegMut<ERR>,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let pub_key = Bytes32::from(read_bytes(memory, a)?);
    let sig = Bytes64::from(read_bytes(memory, b)?);
    let msg = Bytes32::from(read_bytes(memory, c)?);
    let message = Message::from_bytes_ref(&msg);

    if fuel_crypto::ed25519::verify(&pub_key, &sig, message).is_ok() {
        clear_err(err);
    } else {
        set_err(err);
    }

    Ok(inc_pc(pc)?)
}

pub(crate) fn keccak256(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    use sha3::{
        Digest,
        Keccak256,
    };
    let src_range = MemoryRange::new(b, c)?;

    let mut h = Keccak256::new();
    h.update(&memory[src_range.usizes()]);

    try_mem_write(a, h.finalize().as_slice(), owner, memory)?;

    Ok(inc_pc(pc)?)
}

pub(crate) fn sha256(
    memory: &mut [u8; MEM_SIZE],
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let src_range = MemoryRange::new(b, c)?;

    try_mem_write(
        a,
        Hasher::hash(&memory[src_range.usizes()]).as_ref(),
        owner,
        memory,
    )?;

    Ok(inc_pc(pc)?)
}
