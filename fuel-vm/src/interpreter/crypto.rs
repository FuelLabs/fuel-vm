use super::{
    ExecutableTransaction,
    Interpreter,
    Memory,
    MemoryInstance,
    internal::{
        clear_err,
        inc_pc,
        set_err,
    },
    memory::OwnershipRegisters,
};
use crate::{
    constraints::reg_key::*,
    error::SimpleResult,
};

use alloc::vec::Vec;
use bn::{
    AffineG1,
    AffineG2,
    Fq,
    Fq2,
    Fr,
    G1,
    G2,
    Group,
    Gt,
};
use fuel_asm::RegId;
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

impl<M, S, Tx, Ecal, V> Interpreter<M, S, Tx, Ecal, V>
where
    M: Memory,
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
        secp256k1_recover(self.memory.as_mut(), owner, err, pc, a, b, c)
    }

    pub(crate) fn secp256r1_recover(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
    ) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        secp256r1_recover(self.memory.as_mut(), owner, err, pc, a, b, c)
    }

    pub(crate) fn ed25519_verify(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
        len: Word,
    ) -> SimpleResult<()> {
        let (SystemRegisters { err, pc, .. }, _) = split_registers(&mut self.registers);
        ed25519_verify(self.memory.as_mut(), err, pc, a, b, c, len)
    }

    pub(crate) fn keccak256(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        keccak256(
            self.memory.as_mut(),
            owner,
            self.registers.pc_mut(),
            a,
            b,
            c,
        )
    }

    pub(crate) fn sha256(&mut self, a: Word, b: Word, c: Word) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        sha256(
            self.memory.as_mut(),
            owner,
            self.registers.pc_mut(),
            a,
            b,
            c,
        )
    }

    pub(crate) fn ec_operation(
        &mut self,
        a: Word,
        b: Word,
        c: Word,
        d: Word,
    ) -> SimpleResult<()> {
        let owner = self.ownership_registers();
        ec_operation(
            self.memory.as_mut(),
            owner,
            self.registers.pc_mut(),
            a,
            b,
            c,
            d,
        )
    }

    pub(crate) fn ec_pairing(
        &mut self,
        ra: RegId,
        b: Word,
        c: Word,
        d: Word,
    ) -> SimpleResult<()> {
        let (SystemRegisters { pc, .. }, mut w) = split_registers(&mut self.registers);
        let dest = &mut w[ra.try_into()?];
        ec_pairing(self.memory.as_mut(), pc, dest, b, c, d)
    }
}

pub(crate) fn secp256k1_recover(
    memory: &mut MemoryInstance,
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
    memory: &mut MemoryInstance,
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
    memory: &mut MemoryInstance,
    err: RegMut<ERR>,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
    len: Word,
) -> SimpleResult<()> {
    let pub_key = Bytes32::from(memory.read_bytes(a)?);
    let sig = Bytes64::from(memory.read_bytes(b)?);
    let msg = memory.read(c, len)?;

    if fuel_crypto::ed25519::verify(&pub_key, &sig, msg).is_ok() {
        clear_err(err);
    } else {
        set_err(err);
    }

    Ok(inc_pc(pc)?)
}

pub(crate) fn keccak256(
    memory: &mut MemoryInstance,
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
    memory: &mut MemoryInstance,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    a: Word,
    b: Word,
    c: Word,
) -> SimpleResult<()> {
    memory.write_bytes(owner, a, *Hasher::hash(memory.read(b, c)?))?;
    Ok(inc_pc(pc)?)
}

fn read_g1_point_alt_bn_128(
    memory: &MemoryInstance,
    point_ptr: Word,
) -> SimpleResult<G1> {
    // Big endian required by the library
    let arg_bytes: [u8; 2 * 32] = memory.read_bytes(point_ptr)?;

    let px = Fq::from_slice(&arg_bytes[..32])
        .map_err(|_| fuel_tx::PanicReason::InvalidEllipticCurvePoint)?;
    let py = Fq::from_slice(&arg_bytes[32..64])
        .map_err(|_| fuel_tx::PanicReason::InvalidEllipticCurvePoint)?;

    Ok(if px == Fq::zero() && py == Fq::zero() {
        G1::zero()
    } else {
        AffineG1::new(px, py)
            .map(Into::into)
            .map_err(|_| fuel_tx::PanicReason::InvalidEllipticCurvePoint)?
    })
}

fn read_g2_point_alt_bn_128(
    memory: &MemoryInstance,
    point_ptr: Word,
) -> SimpleResult<G2> {
    // Big endian required by the library
    let arg_bytes: [u8; 4 * 32] = memory.read_bytes(point_ptr)?;

    let ay = Fq::from_slice(&arg_bytes[..32])
        .map_err(|_| fuel_tx::PanicReason::InvalidEllipticCurvePoint)?;
    let ax = Fq::from_slice(&arg_bytes[32..64])
        .map_err(|_| fuel_tx::PanicReason::InvalidEllipticCurvePoint)?;
    let by = Fq::from_slice(&arg_bytes[64..96])
        .map_err(|_| fuel_tx::PanicReason::InvalidEllipticCurvePoint)?;
    let bx = Fq::from_slice(&arg_bytes[96..128])
        .map_err(|_| fuel_tx::PanicReason::InvalidEllipticCurvePoint)?;

    let a = Fq2::new(ax, ay);
    let b = Fq2::new(bx, by);
    Ok(if a.is_zero() && b.is_zero() {
        G2::zero()
    } else {
        G2::from(
            AffineG2::new(a, b)
                .map_err(|_| fuel_tx::PanicReason::InvalidEllipticCurvePoint)?,
        )
    })
}

pub(crate) fn ec_operation(
    memory: &mut MemoryInstance,
    owner: OwnershipRegisters,
    pc: RegMut<PC>,
    dst: Word,
    curve_id: Word,
    operation_type: Word,
    points_ptr: Word,
) -> SimpleResult<()> {
    match curve_id {
        0 => {
            match operation_type {
                // Two points addition
                0 => {
                    let point1 = read_g1_point_alt_bn_128(memory, points_ptr)?;
                    let point2 = read_g1_point_alt_bn_128(
                        memory,
                        points_ptr
                            .checked_add(64)
                            .ok_or(fuel_tx::PanicReason::MemoryOverflow)?,
                    )?;
                    let mut output = [0u8; 64];
                    // SAFETY: The library override the addition and is tested and
                    // audited. Here is the code of the addition :
                    // https://github.com/paritytech/bn/blob/63f8c587356a67b33c7396af98e065b66fca5dda/src/groups/mod.rs#L297
                    #[allow(clippy::arithmetic_side_effects)]
                    if let Some(sum) = AffineG1::from_jacobian(point1 + point2) {
                        sum.x().to_big_endian(&mut output[..32]).unwrap();
                        sum.y().to_big_endian(&mut output[32..]).unwrap();
                    }
                    memory.write_bytes(owner, dst, output)?;
                }
                // Scalar multiplication
                1 => {
                    let point = read_g1_point_alt_bn_128(memory, points_ptr)?;
                    let scalar = Fr::from_slice(
                        memory.read(
                            points_ptr
                                .checked_add(64)
                                .ok_or(fuel_tx::PanicReason::MemoryOverflow)?,
                            32u64,
                        )?,
                    )
                    .map_err(|_| fuel_tx::PanicReason::InvalidEllipticCurvePoint)?;
                    let mut output = [0u8; 64];
                    // SAFETY: The library override the multiplication and is tested and
                    // audited. Here is the code of the multiplication
                    // : https://github.com/paritytech/bn/blob/63f8c587356a67b33c7396af98e065b66fca5dda/src/groups/mod.rs#L275
                    #[allow(clippy::arithmetic_side_effects)]
                    if let Some(product) = AffineG1::from_jacobian(point * scalar) {
                        product.x().to_big_endian(&mut output[..32]).unwrap();
                        product.y().to_big_endian(&mut output[32..]).unwrap();
                    }
                    memory.write_bytes(owner, dst, output)?;
                }
                _ => return Err(fuel_tx::PanicReason::UnsupportedOperationType.into()),
            }
        }
        _ => return Err(fuel_tx::PanicReason::UnsupportedCurveId.into()),
    }
    Ok(inc_pc(pc)?)
}

pub(crate) fn ec_pairing(
    memory: &mut MemoryInstance,
    pc: RegMut<PC>,
    success: &mut u64,
    identifier: Word,
    number_elements: Word,
    elements_ptr: Word,
) -> SimpleResult<()> {
    match identifier {
        // Optimal ate pairing / alt_bn128
        0 => {
            // Each element consists of an uncompressed G1 point (64 bytes) and an
            // uncompressed G2 point (128 bytes).
            let element_size = 128 + 64;
            let mut elements = Vec::with_capacity(
                usize::try_from(number_elements)
                    .map_err(|_| fuel_tx::PanicReason::MemoryOverflow)?,
            );
            for idx in 0..number_elements {
                let start_offset = elements_ptr
                    .checked_add(
                        idx.checked_mul(element_size)
                            .ok_or(fuel_tx::PanicReason::MemoryOverflow)?,
                    )
                    .ok_or(fuel_tx::PanicReason::MemoryOverflow)?;
                let a = read_g1_point_alt_bn_128(memory, start_offset)?;
                let b = read_g2_point_alt_bn_128(
                    memory,
                    start_offset
                        .checked_add(64)
                        .ok_or(fuel_tx::PanicReason::MemoryOverflow)?,
                )?;
                elements.push((a, b));
            }
            *success = (bn::pairing_batch(&elements) == Gt::one()) as u64;
        }
        _ => return Err(fuel_tx::PanicReason::UnsupportedOperationType.into()),
    }
    Ok(inc_pc(pc)?)
}
