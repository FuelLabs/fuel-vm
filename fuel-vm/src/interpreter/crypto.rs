use super::{
    internal::{
        clear_err,
        inc_pc,
        set_err,
    },
    memory::OwnershipRegisters,
    ExecutableTransaction,
    Interpreter,
    Memory,
};
use crate::{
    constraints::reg_key::*,
    error::SimpleResult,
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

impl<S, Tx, Ecal> Interpreter<S, Tx, Ecal>
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
    memory: &mut Memory,
    owner: OwnershipRegisters,
    err: RegMut<ERR>,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let sig = Bytes64::from(memory.read_bytes(b)?);
    let msg = Bytes32::from(memory.read_bytes(c)?);

    let signature = Signature::from_bytes_ref(&sig);
    let message = Message::from_bytes_ref(&msg);

    match signature.recover(message) {
        Ok(pub_key) => {
            memory.write_bytes(owner, a, *pub_key)?;
            clear_err(err);
        }
        Err(_) => {
            memory.write_bytes(owner, a, [0; PublicKey::LEN])?;
            set_err(err);
        }
    }

    Ok(inc_pc(pc)?)
}

pub(crate) fn secp256r1_recover(
    memory: &mut Memory,
    owner: OwnershipRegisters,
    err: RegMut<ERR>,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let sig = Bytes64::from(memory.read_bytes(b)?);
    let msg = Bytes32::from(memory.read_bytes(c)?);
    let message = Message::from_bytes_ref(&msg);

    match fuel_crypto::secp256r1::recover(&sig, message) {
        Ok(pub_key) => {
            memory.write_bytes(owner, a, *pub_key)?;
            clear_err(err);
        }
        Err(_) => {
            memory.write_bytes(owner, a, [0; PublicKey::LEN])?;
            set_err(err);
        }
    }

    Ok(inc_pc(pc)?)
}

pub(crate) fn ed25519_verify(
    memory: &mut Memory,
    err: RegMut<ERR>,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    let pub_key = Bytes32::from(memory.read_bytes(a)?);
    let sig = Bytes64::from(memory.read_bytes(b)?);
    let msg = Bytes32::from(memory.read_bytes(c)?);
    let message = Message::from_bytes_ref(&msg);

    if fuel_crypto::ed25519::verify(&pub_key, &sig, message).is_ok() {
        clear_err(err);
    } else {
        set_err(err);
    }

    Ok(inc_pc(pc)?)
}

pub(crate) fn keccak256(
    memory: &mut Memory,
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
    let mut h = Keccak256::new();
    h.update(memory.read(b, c)?);

    memory.write_bytes(owner, a, *h.finalize().as_ref())?;

    Ok(inc_pc(pc)?)
}

pub(crate) fn sha256(
    memory: &mut Memory,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    memory.write_bytes(owner, a, *Hasher::hash(memory.read(b, c)?))?;
    Ok(inc_pc(pc)?)
}
