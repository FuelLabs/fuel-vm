//! Tools for gas instrumentalization

use core::ops::Deref;

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

use fuel_asm::PanicReason;
use fuel_types::Word;

/// Default gas costs are generated from the
/// `fuel-core` repo using the `collect` bin
/// in the `fuel-core-benches` crate.
/// The git sha is included in the file to
/// show what version of `fuel-core` was used
/// to generate the costs.
#[allow(dead_code)]
mod default_gas_costs;

/// Gas costings for every op.
/// The inner values are wrapped in an [`Arc`]
/// so this is cheap to clone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg(feature = "alloc")]
pub struct GasCosts(Arc<GasCostsValues>);

#[cfg(feature = "alloc")]
impl<'de> serde::Deserialize<'de> for GasCosts {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(GasCosts(Arc::new(serde::Deserialize::deserialize(
            deserializer,
        )?)))
    }
}

#[cfg(feature = "alloc")]
impl serde::Serialize for GasCosts {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde::Serialize::serialize(self.0.as_ref(), serializer)
    }
}

#[cfg(feature = "alloc")]
impl GasCosts {
    /// Create new cost values wrapped in an [`Arc`].
    pub fn new(costs: GasCostsValues) -> Self {
        Self(Arc::new(costs))
    }
}

#[cfg(feature = "alloc")]
impl Default for GasCosts {
    fn default() -> Self {
        Self(Arc::new(GasCostsValues::default()))
    }
}

impl Default for GasCostsValues {
    fn default() -> Self {
        // The default values for gas costs
        // are generated from fuel-core-benches.
        default_gas_costs::default_gas_costs()
    }
}

/// The versioned gas costs for every op.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GasCostsValues {
    /// Version 1 of the gas costs.
    V1(GasCostsValuesV1),
    /// Version 2 of the gas costs.
    V2(GasCostsValuesV2),
    /// Version 3 of the gas costs.
    V3(GasCostsValuesV3),
    /// Version 4 of the gas costs.
    V4(GasCostsValuesV4),
}

/// Gas cost for this instruction is not defined for this version.
pub struct GasCostNotDefined;

impl From<GasCostNotDefined> for PanicReason {
    fn from(_: GasCostNotDefined) -> PanicReason {
        PanicReason::GasCostNotDefined
    }
}

#[allow(missing_docs)]
impl GasCostsValues {
    pub fn add(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.add,
            GasCostsValues::V2(v2) => v2.add,
            GasCostsValues::V3(v3) => v3.add,
            GasCostsValues::V4(v4) => v4.add,
        }
    }

    pub fn addi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.addi,
            GasCostsValues::V2(v2) => v2.addi,
            GasCostsValues::V3(v3) => v3.addi,
            GasCostsValues::V4(v4) => v4.addi,
        }
    }

    pub fn and(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.and,
            GasCostsValues::V2(v2) => v2.and,
            GasCostsValues::V3(v3) => v3.and,
            GasCostsValues::V4(v4) => v4.and,
        }
    }

    pub fn andi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.andi,
            GasCostsValues::V2(v2) => v2.andi,
            GasCostsValues::V3(v3) => v3.andi,
            GasCostsValues::V4(v4) => v4.andi,
        }
    }

    pub fn bal(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.bal,
            GasCostsValues::V2(v2) => v2.bal,
            GasCostsValues::V3(v3) => v3.bal,
            GasCostsValues::V4(v4) => v4.bal,
        }
    }

    pub fn bhei(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.bhei,
            GasCostsValues::V2(v2) => v2.bhei,
            GasCostsValues::V3(v3) => v3.bhei,
            GasCostsValues::V4(v4) => v4.bhei,
        }
    }

    pub fn bhsh(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.bhsh,
            GasCostsValues::V2(v2) => v2.bhsh,
            GasCostsValues::V3(v3) => v3.bhsh,
            GasCostsValues::V4(v4) => v4.bhsh,
        }
    }

    pub fn burn(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.burn,
            GasCostsValues::V2(v2) => v2.burn,
            GasCostsValues::V3(v3) => v3.burn,
            GasCostsValues::V4(v4) => v4.burn,
        }
    }

    pub fn cb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.cb,
            GasCostsValues::V2(v2) => v2.cb,
            GasCostsValues::V3(v3) => v3.cb,
            GasCostsValues::V4(v4) => v4.cb,
        }
    }

    pub fn cfsi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.cfsi,
            GasCostsValues::V2(v2) => v2.cfsi,
            GasCostsValues::V3(v3) => v3.cfsi,
            GasCostsValues::V4(v4) => v4.cfsi,
        }
    }

    pub fn div(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.div,
            GasCostsValues::V2(v2) => v2.div,
            GasCostsValues::V3(v3) => v3.div,
            GasCostsValues::V4(v4) => v4.div,
        }
    }

    pub fn divi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.divi,
            GasCostsValues::V2(v2) => v2.divi,
            GasCostsValues::V3(v3) => v3.divi,
            GasCostsValues::V4(v4) => v4.divi,
        }
    }

    pub fn eck1(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.eck1,
            GasCostsValues::V2(v2) => v2.eck1,
            GasCostsValues::V3(v3) => v3.eck1,
            GasCostsValues::V4(v4) => v4.eck1,
        }
    }

    pub fn ecr1(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.ecr1,
            GasCostsValues::V2(v2) => v2.ecr1,
            GasCostsValues::V3(v3) => v3.ecr1,
            GasCostsValues::V4(v4) => v4.ecr1,
        }
    }

    pub fn eq_(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.eq,
            GasCostsValues::V2(v2) => v2.eq,
            GasCostsValues::V3(v3) => v3.eq,
            GasCostsValues::V4(v4) => v4.eq,
        }
    }

    pub fn exp(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.exp,
            GasCostsValues::V2(v2) => v2.exp,
            GasCostsValues::V3(v3) => v3.exp,
            GasCostsValues::V4(v4) => v4.exp,
        }
    }

    pub fn expi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.expi,
            GasCostsValues::V2(v2) => v2.expi,
            GasCostsValues::V3(v3) => v3.expi,
            GasCostsValues::V4(v4) => v4.expi,
        }
    }

    pub fn flag(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.flag,
            GasCostsValues::V2(v2) => v2.flag,
            GasCostsValues::V3(v3) => v3.flag,
            GasCostsValues::V4(v4) => v4.flag,
        }
    }

    pub fn gm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.gm,
            GasCostsValues::V2(v2) => v2.gm,
            GasCostsValues::V3(v3) => v3.gm,
            GasCostsValues::V4(v4) => v4.gm,
        }
    }

    pub fn gt(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.gt,
            GasCostsValues::V2(v2) => v2.gt,
            GasCostsValues::V3(v3) => v3.gt,
            GasCostsValues::V4(v4) => v4.gt,
        }
    }

    pub fn gtf(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.gtf,
            GasCostsValues::V2(v2) => v2.gtf,
            GasCostsValues::V3(v3) => v3.gtf,
            GasCostsValues::V4(v4) => v4.gtf,
        }
    }

    pub fn ji(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.ji,
            GasCostsValues::V2(v2) => v2.ji,
            GasCostsValues::V3(v3) => v3.ji,
            GasCostsValues::V4(v4) => v4.ji,
        }
    }

    pub fn jmp(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jmp,
            GasCostsValues::V2(v2) => v2.jmp,
            GasCostsValues::V3(v3) => v3.jmp,
            GasCostsValues::V4(v4) => v4.jmp,
        }
    }

    pub fn jne(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jne,
            GasCostsValues::V2(v2) => v2.jne,
            GasCostsValues::V3(v3) => v3.jne,
            GasCostsValues::V4(v4) => v4.jne,
        }
    }

    pub fn jnei(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnei,
            GasCostsValues::V2(v2) => v2.jnei,
            GasCostsValues::V3(v3) => v3.jnei,
            GasCostsValues::V4(v4) => v4.jnei,
        }
    }

    pub fn jnzi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnzi,
            GasCostsValues::V2(v2) => v2.jnzi,
            GasCostsValues::V3(v3) => v3.jnzi,
            GasCostsValues::V4(v4) => v4.jnzi,
        }
    }

    pub fn jmpf(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jmpf,
            GasCostsValues::V2(v2) => v2.jmpf,
            GasCostsValues::V3(v3) => v3.jmpf,
            GasCostsValues::V4(v4) => v4.jmpf,
        }
    }

    pub fn jmpb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jmpb,
            GasCostsValues::V2(v2) => v2.jmpb,
            GasCostsValues::V3(v3) => v3.jmpb,
            GasCostsValues::V4(v4) => v4.jmpb,
        }
    }

    pub fn jnzf(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnzf,
            GasCostsValues::V2(v2) => v2.jnzf,
            GasCostsValues::V3(v3) => v3.jnzf,
            GasCostsValues::V4(v4) => v4.jnzf,
        }
    }

    pub fn jnzb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnzb,
            GasCostsValues::V2(v2) => v2.jnzb,
            GasCostsValues::V3(v3) => v3.jnzb,
            GasCostsValues::V4(v4) => v4.jnzb,
        }
    }

    pub fn jnef(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnef,
            GasCostsValues::V2(v2) => v2.jnef,
            GasCostsValues::V3(v3) => v3.jnef,
            GasCostsValues::V4(v4) => v4.jnef,
        }
    }

    pub fn jneb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jneb,
            GasCostsValues::V2(v2) => v2.jneb,
            GasCostsValues::V3(v3) => v3.jneb,
            GasCostsValues::V4(v4) => v4.jneb,
        }
    }

    pub fn lb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.lb,
            GasCostsValues::V2(v2) => v2.lb,
            GasCostsValues::V3(v3) => v3.lb,
            GasCostsValues::V4(v4) => v4.lb,
        }
    }

    pub fn log(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.log,
            GasCostsValues::V2(v2) => v2.log,
            GasCostsValues::V3(v3) => v3.log,
            GasCostsValues::V4(v4) => v4.log,
        }
    }

    pub fn lt(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.lt,
            GasCostsValues::V2(v2) => v2.lt,
            GasCostsValues::V3(v3) => v3.lt,
            GasCostsValues::V4(v4) => v4.lt,
        }
    }

    pub fn lw(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.lw,
            GasCostsValues::V2(v2) => v2.lw,
            GasCostsValues::V3(v3) => v3.lw,
            GasCostsValues::V4(v4) => v4.lw,
        }
    }

    pub fn mint(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mint,
            GasCostsValues::V2(v2) => v2.mint,
            GasCostsValues::V3(v3) => v3.mint,
            GasCostsValues::V4(v4) => v4.mint,
        }
    }

    pub fn mlog(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mlog,
            GasCostsValues::V2(v2) => v2.mlog,
            GasCostsValues::V3(v3) => v3.mlog,
            GasCostsValues::V4(v4) => v4.mlog,
        }
    }

    pub fn mod_op(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mod_op,
            GasCostsValues::V2(v2) => v2.mod_op,
            GasCostsValues::V3(v3) => v3.mod_op,
            GasCostsValues::V4(v4) => v4.mod_op,
        }
    }

    pub fn modi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.modi,
            GasCostsValues::V2(v2) => v2.modi,
            GasCostsValues::V3(v3) => v3.modi,
            GasCostsValues::V4(v4) => v4.modi,
        }
    }

    pub fn move_op(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.move_op,
            GasCostsValues::V2(v2) => v2.move_op,
            GasCostsValues::V3(v3) => v3.move_op,
            GasCostsValues::V4(v4) => v4.move_op,
        }
    }

    pub fn movi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.movi,
            GasCostsValues::V2(v2) => v2.movi,
            GasCostsValues::V3(v3) => v3.movi,
            GasCostsValues::V4(v4) => v4.movi,
        }
    }

    pub fn mroo(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mroo,
            GasCostsValues::V2(v2) => v2.mroo,
            GasCostsValues::V3(v3) => v3.mroo,
            GasCostsValues::V4(v4) => v4.mroo,
        }
    }

    pub fn mul(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mul,
            GasCostsValues::V2(v2) => v2.mul,
            GasCostsValues::V3(v3) => v3.mul,
            GasCostsValues::V4(v4) => v4.mul,
        }
    }

    pub fn muli(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.muli,
            GasCostsValues::V2(v2) => v2.muli,
            GasCostsValues::V3(v3) => v3.muli,
            GasCostsValues::V4(v4) => v4.muli,
        }
    }

    pub fn mldv(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mldv,
            GasCostsValues::V2(v2) => v2.mldv,
            GasCostsValues::V3(v3) => v3.mldv,
            GasCostsValues::V4(v4) => v4.mldv,
        }
    }

    pub fn noop(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.noop,
            GasCostsValues::V2(v2) => v2.noop,
            GasCostsValues::V3(v3) => v3.noop,
            GasCostsValues::V4(v4) => v4.noop,
        }
    }

    pub fn not(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.not,
            GasCostsValues::V2(v2) => v2.not,
            GasCostsValues::V3(v3) => v3.not,
            GasCostsValues::V4(v4) => v4.not,
        }
    }

    pub fn or(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.or,
            GasCostsValues::V2(v2) => v2.or,
            GasCostsValues::V3(v3) => v3.or,
            GasCostsValues::V4(v4) => v4.or,
        }
    }

    pub fn ori(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.ori,
            GasCostsValues::V2(v2) => v2.ori,
            GasCostsValues::V3(v3) => v3.ori,
            GasCostsValues::V4(v4) => v4.ori,
        }
    }

    pub fn poph(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.poph,
            GasCostsValues::V2(v2) => v2.poph,
            GasCostsValues::V3(v3) => v3.poph,
            GasCostsValues::V4(v4) => v4.poph,
        }
    }

    pub fn popl(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.popl,
            GasCostsValues::V2(v2) => v2.popl,
            GasCostsValues::V3(v3) => v3.popl,
            GasCostsValues::V4(v4) => v4.popl,
        }
    }

    pub fn pshh(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.pshh,
            GasCostsValues::V2(v2) => v2.pshh,
            GasCostsValues::V3(v3) => v3.pshh,
            GasCostsValues::V4(v4) => v4.pshh,
        }
    }

    pub fn pshl(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.pshl,
            GasCostsValues::V2(v2) => v2.pshl,
            GasCostsValues::V3(v3) => v3.pshl,
            GasCostsValues::V4(v4) => v4.pshl,
        }
    }

    pub fn ret(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.ret,
            GasCostsValues::V2(v2) => v2.ret,
            GasCostsValues::V3(v3) => v3.ret,
            GasCostsValues::V4(v4) => v4.ret,
        }
    }

    pub fn rvrt(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.rvrt,
            GasCostsValues::V2(v2) => v2.rvrt,
            GasCostsValues::V3(v3) => v3.rvrt,
            GasCostsValues::V4(v4) => v4.rvrt,
        }
    }

    pub fn sb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sb,
            GasCostsValues::V2(v2) => v2.sb,
            GasCostsValues::V3(v3) => v3.sb,
            GasCostsValues::V4(v4) => v4.sb,
        }
    }

    pub fn sll(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sll,
            GasCostsValues::V2(v2) => v2.sll,
            GasCostsValues::V3(v3) => v3.sll,
            GasCostsValues::V4(v4) => v4.sll,
        }
    }

    pub fn slli(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.slli,
            GasCostsValues::V2(v2) => v2.slli,
            GasCostsValues::V3(v3) => v3.slli,
            GasCostsValues::V4(v4) => v4.slli,
        }
    }

    pub fn srl(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.srl,
            GasCostsValues::V2(v2) => v2.srl,
            GasCostsValues::V3(v3) => v3.srl,
            GasCostsValues::V4(v4) => v4.srl,
        }
    }

    pub fn srli(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.srli,
            GasCostsValues::V2(v2) => v2.srli,
            GasCostsValues::V3(v3) => v3.srli,
            GasCostsValues::V4(v4) => v4.srli,
        }
    }

    pub fn srw(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.srw,
            GasCostsValues::V2(v2) => v2.srw,
            GasCostsValues::V3(v3) => v3.srw,
            GasCostsValues::V4(v4) => v4.srw,
        }
    }

    pub fn sub(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sub,
            GasCostsValues::V2(v2) => v2.sub,
            GasCostsValues::V3(v3) => v3.sub,
            GasCostsValues::V4(v4) => v4.sub,
        }
    }

    pub fn subi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.subi,
            GasCostsValues::V2(v2) => v2.subi,
            GasCostsValues::V3(v3) => v3.subi,
            GasCostsValues::V4(v4) => v4.subi,
        }
    }

    pub fn sw(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sw,
            GasCostsValues::V2(v2) => v2.sw,
            GasCostsValues::V3(v3) => v3.sw,
            GasCostsValues::V4(v4) => v4.sw,
        }
    }

    pub fn sww(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sww,
            GasCostsValues::V2(v2) => v2.sww,
            GasCostsValues::V3(v3) => v3.sww,
            GasCostsValues::V4(v4) => v4.sww,
        }
    }

    pub fn time(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.time,
            GasCostsValues::V2(v2) => v2.time,
            GasCostsValues::V3(v3) => v3.time,
            GasCostsValues::V4(v4) => v4.time,
        }
    }

    pub fn tr(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.tr,
            GasCostsValues::V2(v2) => v2.tr,
            GasCostsValues::V3(v3) => v3.tr,
            GasCostsValues::V4(v4) => v4.tr,
        }
    }

    pub fn tro(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.tro,
            GasCostsValues::V2(v2) => v2.tro,
            GasCostsValues::V3(v3) => v3.tro,
            GasCostsValues::V4(v4) => v4.tro,
        }
    }

    pub fn wdcm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdcm,
            GasCostsValues::V2(v2) => v2.wdcm,
            GasCostsValues::V3(v3) => v3.wdcm,
            GasCostsValues::V4(v4) => v4.wdcm,
        }
    }

    pub fn wqcm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqcm,
            GasCostsValues::V2(v2) => v2.wqcm,
            GasCostsValues::V3(v3) => v3.wqcm,
            GasCostsValues::V4(v4) => v4.wqcm,
        }
    }

    pub fn wdop(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdop,
            GasCostsValues::V2(v2) => v2.wdop,
            GasCostsValues::V3(v3) => v3.wdop,
            GasCostsValues::V4(v4) => v4.wdop,
        }
    }

    pub fn wqop(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqop,
            GasCostsValues::V2(v2) => v2.wqop,
            GasCostsValues::V3(v3) => v3.wqop,
            GasCostsValues::V4(v4) => v4.wqop,
        }
    }

    pub fn wdml(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdml,
            GasCostsValues::V2(v2) => v2.wdml,
            GasCostsValues::V3(v3) => v3.wdml,
            GasCostsValues::V4(v4) => v4.wdml,
        }
    }

    pub fn wqml(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqml,
            GasCostsValues::V2(v2) => v2.wqml,
            GasCostsValues::V3(v3) => v3.wqml,
            GasCostsValues::V4(v4) => v4.wqml,
        }
    }

    pub fn wddv(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wddv,
            GasCostsValues::V2(v2) => v2.wddv,
            GasCostsValues::V3(v3) => v3.wddv,
            GasCostsValues::V4(v4) => v4.wddv,
        }
    }

    pub fn wqdv(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqdv,
            GasCostsValues::V2(v2) => v2.wqdv,
            GasCostsValues::V3(v3) => v3.wqdv,
            GasCostsValues::V4(v4) => v4.wqdv,
        }
    }

    pub fn wdmd(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdmd,
            GasCostsValues::V2(v2) => v2.wdmd,
            GasCostsValues::V3(v3) => v3.wdmd,
            GasCostsValues::V4(v4) => v4.wdmd,
        }
    }

    pub fn wqmd(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqmd,
            GasCostsValues::V2(v2) => v2.wqmd,
            GasCostsValues::V3(v3) => v3.wqmd,
            GasCostsValues::V4(v4) => v4.wqmd,
        }
    }

    pub fn wdam(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdam,
            GasCostsValues::V2(v2) => v2.wdam,
            GasCostsValues::V3(v3) => v3.wdam,
            GasCostsValues::V4(v4) => v4.wdam,
        }
    }

    pub fn wqam(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqam,
            GasCostsValues::V2(v2) => v2.wqam,
            GasCostsValues::V3(v3) => v3.wqam,
            GasCostsValues::V4(v4) => v4.wqam,
        }
    }

    pub fn wdmm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdmm,
            GasCostsValues::V2(v2) => v2.wdmm,
            GasCostsValues::V3(v3) => v3.wdmm,
            GasCostsValues::V4(v4) => v4.wdmm,
        }
    }

    pub fn wqmm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqmm,
            GasCostsValues::V2(v2) => v2.wqmm,
            GasCostsValues::V3(v3) => v3.wqmm,
            GasCostsValues::V4(v4) => v4.wqmm,
        }
    }

    pub fn xor(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.xor,
            GasCostsValues::V2(v2) => v2.xor,
            GasCostsValues::V3(v3) => v3.xor,
            GasCostsValues::V4(v4) => v4.xor,
        }
    }

    pub fn xori(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.xori,
            GasCostsValues::V2(v2) => v2.xori,
            GasCostsValues::V3(v3) => v3.xori,
            GasCostsValues::V4(v4) => v4.xori,
        }
    }

    pub fn eadd(&self) -> Result<Word, GasCostNotDefined> {
        match self {
            GasCostsValues::V1(_) => Err(GasCostNotDefined),
            GasCostsValues::V2(_) => Err(GasCostNotDefined),
            GasCostsValues::V3(_) => Err(GasCostNotDefined),
            GasCostsValues::V4(v4) => Ok(v4.eadd),
        }
    }

    pub fn emul(&self) -> Result<Word, GasCostNotDefined> {
        match self {
            GasCostsValues::V1(_) => Err(GasCostNotDefined),
            GasCostsValues::V2(_) => Err(GasCostNotDefined),
            GasCostsValues::V3(_) => Err(GasCostNotDefined),
            GasCostsValues::V4(v4) => Ok(v4.emul),
        }
    }

    pub fn aloc(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => DependentCost::HeavyOperation {
                base: v1.aloc,
                gas_per_unit: 0,
            },
            GasCostsValues::V2(v2) => v2.aloc,
            GasCostsValues::V3(v3) => v3.aloc,
            GasCostsValues::V4(v4) => v4.aloc,
        }
    }

    pub fn cfe(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => DependentCost::HeavyOperation {
                base: v1.cfei,
                gas_per_unit: 0,
            },
            GasCostsValues::V2(v2) => DependentCost::HeavyOperation {
                base: v2.cfei,
                gas_per_unit: 0,
            },
            GasCostsValues::V3(v3) => v3.cfe,
            GasCostsValues::V4(v4) => v4.cfe,
        }
    }

    pub fn cfei(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => DependentCost::HeavyOperation {
                base: v1.cfei,
                gas_per_unit: 0,
            },
            GasCostsValues::V2(v2) => DependentCost::HeavyOperation {
                base: v2.cfei,
                gas_per_unit: 0,
            },
            GasCostsValues::V3(v3) => v3.cfei,
            GasCostsValues::V4(v4) => v4.cfei,
        }
    }

    pub fn call(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.call,
            GasCostsValues::V2(v2) => v2.call,
            GasCostsValues::V3(v3) => v3.call,
            GasCostsValues::V4(v4) => v4.call,
        }
    }

    pub fn ccp(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.ccp,
            GasCostsValues::V2(v2) => v2.ccp,
            GasCostsValues::V3(v3) => v3.ccp,
            GasCostsValues::V4(v4) => v4.ccp,
        }
    }

    pub fn croo(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.croo,
            GasCostsValues::V2(v2) => v2.croo,
            GasCostsValues::V3(v3) => v3.croo,
            GasCostsValues::V4(v4) => v4.croo,
        }
    }

    pub fn csiz(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.csiz,
            GasCostsValues::V2(v2) => v2.csiz,
            GasCostsValues::V3(v3) => v3.csiz,
            GasCostsValues::V4(v4) => v4.csiz,
        }
    }

    pub fn ed19(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => DependentCost::HeavyOperation {
                base: v1.ed19,
                gas_per_unit: 0,
            },
            GasCostsValues::V2(v2) => DependentCost::HeavyOperation {
                base: v2.ed19,
                gas_per_unit: 0,
            },
            GasCostsValues::V3(v3) => DependentCost::HeavyOperation {
                base: v3.ed19,
                gas_per_unit: 0,
            },
            GasCostsValues::V4(v4) => v4.ed19,
        }
    }

    pub fn k256(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.k256,
            GasCostsValues::V2(v2) => v2.k256,
            GasCostsValues::V3(v3) => v3.k256,
            GasCostsValues::V4(v4) => v4.k256,
        }
    }

    pub fn ldc(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.ldc,
            GasCostsValues::V2(v2) => v2.ldc,
            GasCostsValues::V3(v3) => v3.ldc,
            GasCostsValues::V4(v4) => v4.ldc,
        }
    }

    pub fn logd(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.logd,
            GasCostsValues::V2(v2) => v2.logd,
            GasCostsValues::V3(v3) => v3.logd,
            GasCostsValues::V4(v4) => v4.logd,
        }
    }

    pub fn mcl(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.mcl,
            GasCostsValues::V2(v2) => v2.mcl,
            GasCostsValues::V3(v3) => v3.mcl,
            GasCostsValues::V4(v4) => v4.mcl,
        }
    }

    pub fn mcli(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.mcli,
            GasCostsValues::V2(v2) => v2.mcli,
            GasCostsValues::V3(v3) => v3.mcli,
            GasCostsValues::V4(v4) => v4.mcli,
        }
    }

    pub fn mcp(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.mcp,
            GasCostsValues::V2(v2) => v2.mcp,
            GasCostsValues::V3(v3) => v3.mcp,
            GasCostsValues::V4(v4) => v4.mcp,
        }
    }

    pub fn mcpi(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.mcpi,
            GasCostsValues::V2(v2) => v2.mcpi,
            GasCostsValues::V3(v3) => v3.mcpi,
            GasCostsValues::V4(v4) => v4.mcpi,
        }
    }

    pub fn meq(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.meq,
            GasCostsValues::V2(v2) => v2.meq,
            GasCostsValues::V3(v3) => v3.meq,
            GasCostsValues::V4(v4) => v4.meq,
        }
    }

    pub fn retd(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.retd,
            GasCostsValues::V2(v2) => v2.retd,
            GasCostsValues::V3(v3) => v3.retd,
            GasCostsValues::V4(v4) => v4.retd,
        }
    }

    pub fn s256(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.s256,
            GasCostsValues::V2(v2) => v2.s256,
            GasCostsValues::V3(v3) => v3.s256,
            GasCostsValues::V4(v4) => v4.s256,
        }
    }

    pub fn scwq(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.scwq,
            GasCostsValues::V2(v2) => v2.scwq,
            GasCostsValues::V3(v3) => v3.scwq,
            GasCostsValues::V4(v4) => v4.scwq,
        }
    }

    pub fn smo(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.smo,
            GasCostsValues::V2(v2) => v2.smo,
            GasCostsValues::V3(v3) => v3.smo,
            GasCostsValues::V4(v4) => v4.smo,
        }
    }

    pub fn srwq(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.srwq,
            GasCostsValues::V2(v2) => v2.srwq,
            GasCostsValues::V3(v3) => v3.srwq,
            GasCostsValues::V4(v4) => v4.srwq,
        }
    }

    pub fn swwq(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.swwq,
            GasCostsValues::V2(v2) => v2.swwq,
            GasCostsValues::V3(v3) => v3.swwq,
            GasCostsValues::V4(v4) => v4.swwq,
        }
    }

    pub fn bsiz(&self) -> Result<DependentCost, GasCostNotDefined> {
        match self {
            GasCostsValues::V1(_v1) => Err(GasCostNotDefined),
            GasCostsValues::V2(_v2) => Err(GasCostNotDefined),
            GasCostsValues::V3(_v3) => Err(GasCostNotDefined),
            GasCostsValues::V4(v4) => Ok(v4.bsiz),
        }
    }

    pub fn bldd(&self) -> Result<DependentCost, GasCostNotDefined> {
        match self {
            GasCostsValues::V1(_v1) => Err(GasCostNotDefined),
            GasCostsValues::V2(_v2) => Err(GasCostNotDefined),
            GasCostsValues::V3(_v3) => Err(GasCostNotDefined),
            GasCostsValues::V4(v4) => Ok(v4.bldd),
        }
    }

    pub fn epar(&self) -> Result<DependentCost, GasCostNotDefined> {
        match self {
            GasCostsValues::V1(_v1) => Err(GasCostNotDefined),
            GasCostsValues::V2(_v2) => Err(GasCostNotDefined),
            GasCostsValues::V3(_v3) => Err(GasCostNotDefined),
            GasCostsValues::V4(v4) => Ok(v4.epar),
        }
    }

    pub fn contract_root(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.contract_root,
            GasCostsValues::V2(v2) => v2.contract_root,
            GasCostsValues::V3(v3) => v3.contract_root,
            GasCostsValues::V4(v4) => v4.contract_root,
        }
    }

    pub fn state_root(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.state_root,
            GasCostsValues::V2(v2) => v2.state_root,
            GasCostsValues::V3(v3) => v3.state_root,
            GasCostsValues::V4(v4) => v4.state_root,
        }
    }

    pub fn new_storage_per_byte(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.new_storage_per_byte,
            GasCostsValues::V2(v2) => v2.new_storage_per_byte,
            GasCostsValues::V3(v3) => v3.new_storage_per_byte,
            GasCostsValues::V4(v4) => v4.new_storage_per_byte,
        }
    }

    pub fn vm_initialization(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.vm_initialization,
            GasCostsValues::V2(v2) => v2.vm_initialization,
            GasCostsValues::V3(v3) => v3.vm_initialization,
            GasCostsValues::V4(v4) => v4.vm_initialization,
        }
    }
}

/// Gas costs for every op.
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(default = "GasCostsValuesV1::unit")]
pub struct GasCostsValuesV1 {
    pub add: Word,
    pub addi: Word,
    pub aloc: Word,
    pub and: Word,
    pub andi: Word,
    pub bal: Word,
    pub bhei: Word,
    pub bhsh: Word,
    pub burn: Word,
    pub cb: Word,
    pub cfei: Word,
    pub cfsi: Word,
    pub div: Word,
    pub divi: Word,
    pub eck1: Word,
    pub ecr1: Word,
    pub ed19: Word,
    pub eq: Word,
    pub exp: Word,
    pub expi: Word,
    pub flag: Word,
    pub gm: Word,
    pub gt: Word,
    pub gtf: Word,
    pub ji: Word,
    pub jmp: Word,
    pub jne: Word,
    pub jnei: Word,
    pub jnzi: Word,
    pub jmpf: Word,
    pub jmpb: Word,
    pub jnzf: Word,
    pub jnzb: Word,
    pub jnef: Word,
    pub jneb: Word,
    pub lb: Word,
    pub log: Word,
    pub lt: Word,
    pub lw: Word,
    pub mint: Word,
    pub mlog: Word,
    #[serde(rename = "mod")]
    pub mod_op: Word,
    pub modi: Word,
    #[serde(rename = "move")]
    pub move_op: Word,
    pub movi: Word,
    pub mroo: Word,
    pub mul: Word,
    pub muli: Word,
    pub mldv: Word,
    pub noop: Word,
    pub not: Word,
    pub or: Word,
    pub ori: Word,
    pub poph: Word,
    pub popl: Word,
    pub pshh: Word,
    pub pshl: Word,
    #[serde(rename = "ret_contract")]
    pub ret: Word,
    #[serde(rename = "rvrt_contract")]
    pub rvrt: Word,
    pub sb: Word,
    pub sll: Word,
    pub slli: Word,
    pub srl: Word,
    pub srli: Word,
    pub srw: Word,
    pub sub: Word,
    pub subi: Word,
    pub sw: Word,
    pub sww: Word,
    pub time: Word,
    pub tr: Word,
    pub tro: Word,
    pub wdcm: Word,
    pub wqcm: Word,
    pub wdop: Word,
    pub wqop: Word,
    pub wdml: Word,
    pub wqml: Word,
    pub wddv: Word,
    pub wqdv: Word,
    pub wdmd: Word,
    pub wqmd: Word,
    pub wdam: Word,
    pub wqam: Word,
    pub wdmm: Word,
    pub wqmm: Word,
    pub xor: Word,
    pub xori: Word,

    // Dependent
    pub call: DependentCost,
    pub ccp: DependentCost,
    pub croo: DependentCost,
    pub csiz: DependentCost,
    pub k256: DependentCost,
    pub ldc: DependentCost,
    pub logd: DependentCost,
    pub mcl: DependentCost,
    pub mcli: DependentCost,
    pub mcp: DependentCost,
    pub mcpi: DependentCost,
    pub meq: DependentCost,
    #[serde(rename = "retd_contract")]
    pub retd: DependentCost,
    pub s256: DependentCost,
    pub scwq: DependentCost,
    pub smo: DependentCost,
    pub srwq: DependentCost,
    pub swwq: DependentCost,

    // Non-opcode costs
    pub contract_root: DependentCost,
    pub state_root: DependentCost,
    pub new_storage_per_byte: Word,
    pub vm_initialization: DependentCost,
}

/// Gas costs for every op.
/// The difference with [`GasCostsValuesV1`]:
/// - `aloc` is a [`DependentCost`] instead of a [`Word`]
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(default = "GasCostsValuesV2::unit")]
pub struct GasCostsValuesV2 {
    pub add: Word,
    pub addi: Word,
    pub and: Word,
    pub andi: Word,
    pub bal: Word,
    pub bhei: Word,
    pub bhsh: Word,
    pub burn: Word,
    pub cb: Word,
    pub cfei: Word,
    pub cfsi: Word,
    pub div: Word,
    pub divi: Word,
    pub eck1: Word,
    pub ecr1: Word,
    pub ed19: Word,
    pub eq: Word,
    pub exp: Word,
    pub expi: Word,
    pub flag: Word,
    pub gm: Word,
    pub gt: Word,
    pub gtf: Word,
    pub ji: Word,
    pub jmp: Word,
    pub jne: Word,
    pub jnei: Word,
    pub jnzi: Word,
    pub jmpf: Word,
    pub jmpb: Word,
    pub jnzf: Word,
    pub jnzb: Word,
    pub jnef: Word,
    pub jneb: Word,
    pub lb: Word,
    pub log: Word,
    pub lt: Word,
    pub lw: Word,
    pub mint: Word,
    pub mlog: Word,
    #[serde(rename = "mod")]
    pub mod_op: Word,
    pub modi: Word,
    #[serde(rename = "move")]
    pub move_op: Word,
    pub movi: Word,
    pub mroo: Word,
    pub mul: Word,
    pub muli: Word,
    pub mldv: Word,
    pub noop: Word,
    pub not: Word,
    pub or: Word,
    pub ori: Word,
    pub poph: Word,
    pub popl: Word,
    pub pshh: Word,
    pub pshl: Word,
    #[serde(rename = "ret_contract")]
    pub ret: Word,
    #[serde(rename = "rvrt_contract")]
    pub rvrt: Word,
    pub sb: Word,
    pub sll: Word,
    pub slli: Word,
    pub srl: Word,
    pub srli: Word,
    pub srw: Word,
    pub sub: Word,
    pub subi: Word,
    pub sw: Word,
    pub sww: Word,
    pub time: Word,
    pub tr: Word,
    pub tro: Word,
    pub wdcm: Word,
    pub wqcm: Word,
    pub wdop: Word,
    pub wqop: Word,
    pub wdml: Word,
    pub wqml: Word,
    pub wddv: Word,
    pub wqdv: Word,
    pub wdmd: Word,
    pub wqmd: Word,
    pub wdam: Word,
    pub wqam: Word,
    pub wdmm: Word,
    pub wqmm: Word,
    pub xor: Word,
    pub xori: Word,

    // Dependent
    pub aloc: DependentCost,
    pub call: DependentCost,
    pub ccp: DependentCost,
    pub croo: DependentCost,
    pub csiz: DependentCost,
    pub k256: DependentCost,
    pub ldc: DependentCost,
    pub logd: DependentCost,
    pub mcl: DependentCost,
    pub mcli: DependentCost,
    pub mcp: DependentCost,
    pub mcpi: DependentCost,
    pub meq: DependentCost,
    #[serde(rename = "retd_contract")]
    pub retd: DependentCost,
    pub s256: DependentCost,
    pub scwq: DependentCost,
    pub smo: DependentCost,
    pub srwq: DependentCost,
    pub swwq: DependentCost,

    // Non-opcode costs
    pub contract_root: DependentCost,
    pub state_root: DependentCost,
    pub new_storage_per_byte: Word,
    pub vm_initialization: DependentCost,
}

/// Gas costs for every op.
/// The difference with [`GasCostsValuesV2`]:
/// - Added `cfe` as a [`DependentCost`]
/// - `cfei` is a [`DependentCost`] instead of a [`Word`]
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(default = "GasCostsValuesV3::unit")]
pub struct GasCostsValuesV3 {
    pub add: Word,
    pub addi: Word,
    pub and: Word,
    pub andi: Word,
    pub bal: Word,
    pub bhei: Word,
    pub bhsh: Word,
    pub burn: Word,
    pub cb: Word,
    pub cfsi: Word,
    pub div: Word,
    pub divi: Word,
    pub eck1: Word,
    pub ecr1: Word,
    pub ed19: Word,
    pub eq: Word,
    pub exp: Word,
    pub expi: Word,
    pub flag: Word,
    pub gm: Word,
    pub gt: Word,
    pub gtf: Word,
    pub ji: Word,
    pub jmp: Word,
    pub jne: Word,
    pub jnei: Word,
    pub jnzi: Word,
    pub jmpf: Word,
    pub jmpb: Word,
    pub jnzf: Word,
    pub jnzb: Word,
    pub jnef: Word,
    pub jneb: Word,
    pub lb: Word,
    pub log: Word,
    pub lt: Word,
    pub lw: Word,
    pub mint: Word,
    pub mlog: Word,
    #[serde(rename = "mod")]
    pub mod_op: Word,
    pub modi: Word,
    #[serde(rename = "move")]
    pub move_op: Word,
    pub movi: Word,
    pub mroo: Word,
    pub mul: Word,
    pub muli: Word,
    pub mldv: Word,
    pub noop: Word,
    pub not: Word,
    pub or: Word,
    pub ori: Word,
    pub poph: Word,
    pub popl: Word,
    pub pshh: Word,
    pub pshl: Word,
    #[serde(rename = "ret_contract")]
    pub ret: Word,
    #[serde(rename = "rvrt_contract")]
    pub rvrt: Word,
    pub sb: Word,
    pub sll: Word,
    pub slli: Word,
    pub srl: Word,
    pub srli: Word,
    pub srw: Word,
    pub sub: Word,
    pub subi: Word,
    pub sw: Word,
    pub sww: Word,
    pub time: Word,
    pub tr: Word,
    pub tro: Word,
    pub wdcm: Word,
    pub wqcm: Word,
    pub wdop: Word,
    pub wqop: Word,
    pub wdml: Word,
    pub wqml: Word,
    pub wddv: Word,
    pub wqdv: Word,
    pub wdmd: Word,
    pub wqmd: Word,
    pub wdam: Word,
    pub wqam: Word,
    pub wdmm: Word,
    pub wqmm: Word,
    pub xor: Word,
    pub xori: Word,

    // Dependent
    pub aloc: DependentCost,
    pub cfe: DependentCost,
    pub cfei: DependentCost,
    pub call: DependentCost,
    pub ccp: DependentCost,
    pub croo: DependentCost,
    pub csiz: DependentCost,
    pub k256: DependentCost,
    pub ldc: DependentCost,
    pub logd: DependentCost,
    pub mcl: DependentCost,
    pub mcli: DependentCost,
    pub mcp: DependentCost,
    pub mcpi: DependentCost,
    pub meq: DependentCost,
    #[serde(rename = "retd_contract")]
    pub retd: DependentCost,
    pub s256: DependentCost,
    pub scwq: DependentCost,
    pub smo: DependentCost,
    pub srwq: DependentCost,
    pub swwq: DependentCost,

    // Non-opcode costs
    pub contract_root: DependentCost,
    pub state_root: DependentCost,
    pub new_storage_per_byte: Word,
    pub vm_initialization: DependentCost,
}

/// Gas costs for every op.
/// The difference with [`GasCostsValuesV3`]:
/// - Added `bsiz`, `bldd` instructions
/// - Changed `ed19` to be `DependentCost`
#[allow(missing_docs)]
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(default = "GasCostsValuesV4::unit")]
pub struct GasCostsValuesV4 {
    pub add: Word,
    pub addi: Word,
    pub and: Word,
    pub andi: Word,
    pub bal: Word,
    pub bhei: Word,
    pub bhsh: Word,
    pub burn: Word,
    pub cb: Word,
    pub cfsi: Word,
    pub div: Word,
    pub divi: Word,
    pub eck1: Word,
    pub ecr1: Word,
    pub eq: Word,
    pub exp: Word,
    pub expi: Word,
    pub flag: Word,
    pub gm: Word,
    pub gt: Word,
    pub gtf: Word,
    pub ji: Word,
    pub jmp: Word,
    pub jne: Word,
    pub jnei: Word,
    pub jnzi: Word,
    pub jmpf: Word,
    pub jmpb: Word,
    pub jnzf: Word,
    pub jnzb: Word,
    pub jnef: Word,
    pub jneb: Word,
    pub lb: Word,
    pub log: Word,
    pub lt: Word,
    pub lw: Word,
    pub mint: Word,
    pub mlog: Word,
    #[serde(rename = "mod")]
    pub mod_op: Word,
    pub modi: Word,
    #[serde(rename = "move")]
    pub move_op: Word,
    pub movi: Word,
    pub mroo: Word,
    pub mul: Word,
    pub muli: Word,
    pub mldv: Word,
    pub noop: Word,
    pub not: Word,
    pub or: Word,
    pub ori: Word,
    pub poph: Word,
    pub popl: Word,
    pub pshh: Word,
    pub pshl: Word,
    #[serde(rename = "ret_contract")]
    pub ret: Word,
    #[serde(rename = "rvrt_contract")]
    pub rvrt: Word,
    pub sb: Word,
    pub sll: Word,
    pub slli: Word,
    pub srl: Word,
    pub srli: Word,
    pub srw: Word,
    pub sub: Word,
    pub subi: Word,
    pub sw: Word,
    pub sww: Word,
    pub time: Word,
    pub tr: Word,
    pub tro: Word,
    pub wdcm: Word,
    pub wqcm: Word,
    pub wdop: Word,
    pub wqop: Word,
    pub wdml: Word,
    pub wqml: Word,
    pub wddv: Word,
    pub wqdv: Word,
    pub wdmd: Word,
    pub wqmd: Word,
    pub wdam: Word,
    pub wqam: Word,
    pub wdmm: Word,
    pub wqmm: Word,
    pub xor: Word,
    pub xori: Word,
    pub eadd: Word,
    pub emul: Word,

    // Dependent
    pub aloc: DependentCost,
    pub bsiz: DependentCost,
    pub bldd: DependentCost,
    pub cfe: DependentCost,
    pub cfei: DependentCost,
    pub call: DependentCost,
    pub ccp: DependentCost,
    pub croo: DependentCost,
    pub csiz: DependentCost,
    pub ed19: DependentCost,
    pub k256: DependentCost,
    pub ldc: DependentCost,
    pub logd: DependentCost,
    pub mcl: DependentCost,
    pub mcli: DependentCost,
    pub mcp: DependentCost,
    pub mcpi: DependentCost,
    pub meq: DependentCost,
    #[serde(rename = "retd_contract")]
    pub retd: DependentCost,
    pub s256: DependentCost,
    pub scwq: DependentCost,
    pub smo: DependentCost,
    pub srwq: DependentCost,
    pub swwq: DependentCost,
    pub epar: DependentCost,

    // Non-opcode costs
    pub contract_root: DependentCost,
    pub state_root: DependentCost,
    pub new_storage_per_byte: Word,
    pub vm_initialization: DependentCost,
}

/// Dependent cost is a cost that depends on the number of units.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum DependentCost {
    /// When an operation is dependent on the magnitude of its inputs, and the
    /// time per unit of input is less than a single no-op operation
    LightOperation {
        /// The minimum that this operation can cost.
        base: Word,
        /// How many elements can be processed with a single gas. The
        /// higher the `units_per_gas`, the less additional cost you will incur
        /// for a given number of units, because you need more units to increase
        /// the total cost.
        /// This must be nonzero.
        units_per_gas: Word,
    },

    /// When an operation is dependent on the magnitude of its inputs, and the
    /// time per unit of input is greater than a single no-op operation
    HeavyOperation {
        /// The minimum that this operation can cost.
        base: Word,
        /// How much gas is required to process a single unit.
        gas_per_unit: Word,
    },
}

#[cfg(feature = "alloc")]
impl GasCosts {
    /// Create costs that are all set to zero.
    pub fn free() -> Self {
        Self(Arc::new(GasCostsValues::free()))
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        Self(Arc::new(GasCostsValues::unit()))
    }
}

impl GasCostsValues {
    /// Create costs that are all set to zero.
    pub fn free() -> Self {
        GasCostsValuesV4::free().into()
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        GasCostsValuesV4::unit().into()
    }
}

impl GasCostsValuesV1 {
    /// Create costs that are all set to zero.
    pub fn free() -> Self {
        Self {
            add: 0,
            addi: 0,
            aloc: 0,
            and: 0,
            andi: 0,
            bal: 0,
            bhei: 0,
            bhsh: 0,
            burn: 0,
            cb: 0,
            cfei: 0,
            cfsi: 0,
            div: 0,
            divi: 0,
            eck1: 0,
            ecr1: 0,
            ed19: 0,
            eq: 0,
            exp: 0,
            expi: 0,
            flag: 0,
            gm: 0,
            gt: 0,
            gtf: 0,
            ji: 0,
            jmp: 0,
            jne: 0,
            jnei: 0,
            jnzi: 0,
            jmpf: 0,
            jmpb: 0,
            jnzf: 0,
            jnzb: 0,
            jnef: 0,
            jneb: 0,
            lb: 0,
            log: 0,
            lt: 0,
            lw: 0,
            mint: 0,
            mlog: 0,
            mod_op: 0,
            modi: 0,
            move_op: 0,
            movi: 0,
            mroo: 0,
            mul: 0,
            muli: 0,
            mldv: 0,
            noop: 0,
            not: 0,
            or: 0,
            ori: 0,
            poph: 0,
            popl: 0,
            pshh: 0,
            pshl: 0,
            ret: 0,
            rvrt: 0,
            sb: 0,
            sll: 0,
            slli: 0,
            srl: 0,
            srli: 0,
            srw: 0,
            sub: 0,
            subi: 0,
            sw: 0,
            sww: 0,
            time: 0,
            tr: 0,
            tro: 0,
            wdcm: 0,
            wqcm: 0,
            wdop: 0,
            wqop: 0,
            wdml: 0,
            wqml: 0,
            wddv: 0,
            wqdv: 0,
            wdmd: 0,
            wqmd: 0,
            wdam: 0,
            wqam: 0,
            wdmm: 0,
            wqmm: 0,
            xor: 0,
            xori: 0,
            call: DependentCost::free(),
            ccp: DependentCost::free(),
            croo: DependentCost::free(),
            csiz: DependentCost::free(),
            k256: DependentCost::free(),
            ldc: DependentCost::free(),
            logd: DependentCost::free(),
            mcl: DependentCost::free(),
            mcli: DependentCost::free(),
            mcp: DependentCost::free(),
            mcpi: DependentCost::free(),
            meq: DependentCost::free(),
            retd: DependentCost::free(),
            s256: DependentCost::free(),
            scwq: DependentCost::free(),
            smo: DependentCost::free(),
            srwq: DependentCost::free(),
            swwq: DependentCost::free(),

            // Non-opcode costs
            contract_root: DependentCost::free(),
            state_root: DependentCost::free(),
            new_storage_per_byte: 0,
            vm_initialization: DependentCost::free(),
        }
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        Self {
            add: 1,
            addi: 1,
            aloc: 1,
            and: 1,
            andi: 1,
            bal: 1,
            bhei: 1,
            bhsh: 1,
            burn: 1,
            cb: 1,
            cfei: 1,
            cfsi: 1,
            div: 1,
            divi: 1,
            eck1: 1,
            ecr1: 1,
            ed19: 1,
            eq: 1,
            exp: 1,
            expi: 1,
            flag: 1,
            gm: 1,
            gt: 1,
            gtf: 1,
            ji: 1,
            jmp: 1,
            jne: 1,
            jnei: 1,
            jnzi: 1,
            jmpf: 1,
            jmpb: 1,
            jnzf: 1,
            jnzb: 1,
            jnef: 1,
            jneb: 1,
            lb: 1,
            log: 1,
            lt: 1,
            lw: 1,
            mint: 1,
            mlog: 1,
            mod_op: 1,
            modi: 1,
            move_op: 1,
            movi: 1,
            mroo: 1,
            mul: 1,
            muli: 1,
            mldv: 1,
            noop: 1,
            not: 1,
            or: 1,
            ori: 1,
            ret: 1,
            poph: 1,
            popl: 1,
            pshh: 1,
            pshl: 1,
            rvrt: 1,
            sb: 1,
            sll: 1,
            slli: 1,
            srl: 1,
            srli: 1,
            srw: 1,
            sub: 1,
            subi: 1,
            sw: 1,
            sww: 1,
            time: 1,
            tr: 1,
            tro: 1,
            wdcm: 1,
            wqcm: 1,
            wdop: 1,
            wqop: 1,
            wdml: 1,
            wqml: 1,
            wddv: 1,
            wqdv: 1,
            wdmd: 1,
            wqmd: 1,
            wdam: 1,
            wqam: 1,
            wdmm: 1,
            wqmm: 1,
            xor: 1,
            xori: 1,
            call: DependentCost::unit(),
            ccp: DependentCost::unit(),
            croo: DependentCost::unit(),
            csiz: DependentCost::unit(),
            k256: DependentCost::unit(),
            ldc: DependentCost::unit(),
            logd: DependentCost::unit(),
            mcl: DependentCost::unit(),
            mcli: DependentCost::unit(),
            mcp: DependentCost::unit(),
            mcpi: DependentCost::unit(),
            meq: DependentCost::unit(),
            retd: DependentCost::unit(),
            s256: DependentCost::unit(),
            scwq: DependentCost::unit(),
            smo: DependentCost::unit(),
            srwq: DependentCost::unit(),
            swwq: DependentCost::unit(),

            // Non-opcode costs
            contract_root: DependentCost::unit(),
            state_root: DependentCost::unit(),
            new_storage_per_byte: 1,
            vm_initialization: DependentCost::unit(),
        }
    }
}

impl GasCostsValuesV2 {
    /// Create costs that are all set to zero.
    pub fn free() -> Self {
        Self {
            add: 0,
            addi: 0,
            and: 0,
            andi: 0,
            bal: 0,
            bhei: 0,
            bhsh: 0,
            burn: 0,
            cb: 0,
            cfei: 0,
            cfsi: 0,
            div: 0,
            divi: 0,
            eck1: 0,
            ecr1: 0,
            ed19: 0,
            eq: 0,
            exp: 0,
            expi: 0,
            flag: 0,
            gm: 0,
            gt: 0,
            gtf: 0,
            ji: 0,
            jmp: 0,
            jne: 0,
            jnei: 0,
            jnzi: 0,
            jmpf: 0,
            jmpb: 0,
            jnzf: 0,
            jnzb: 0,
            jnef: 0,
            jneb: 0,
            lb: 0,
            log: 0,
            lt: 0,
            lw: 0,
            mint: 0,
            mlog: 0,
            mod_op: 0,
            modi: 0,
            move_op: 0,
            movi: 0,
            mroo: 0,
            mul: 0,
            muli: 0,
            mldv: 0,
            noop: 0,
            not: 0,
            or: 0,
            ori: 0,
            poph: 0,
            popl: 0,
            pshh: 0,
            pshl: 0,
            ret: 0,
            rvrt: 0,
            sb: 0,
            sll: 0,
            slli: 0,
            srl: 0,
            srli: 0,
            srw: 0,
            sub: 0,
            subi: 0,
            sw: 0,
            sww: 0,
            time: 0,
            tr: 0,
            tro: 0,
            wdcm: 0,
            wqcm: 0,
            wdop: 0,
            wqop: 0,
            wdml: 0,
            wqml: 0,
            wddv: 0,
            wqdv: 0,
            wdmd: 0,
            wqmd: 0,
            wdam: 0,
            wqam: 0,
            wdmm: 0,
            wqmm: 0,
            xor: 0,
            xori: 0,
            aloc: DependentCost::free(),
            call: DependentCost::free(),
            ccp: DependentCost::free(),
            croo: DependentCost::free(),
            csiz: DependentCost::free(),
            k256: DependentCost::free(),
            ldc: DependentCost::free(),
            logd: DependentCost::free(),
            mcl: DependentCost::free(),
            mcli: DependentCost::free(),
            mcp: DependentCost::free(),
            mcpi: DependentCost::free(),
            meq: DependentCost::free(),
            retd: DependentCost::free(),
            s256: DependentCost::free(),
            scwq: DependentCost::free(),
            smo: DependentCost::free(),
            srwq: DependentCost::free(),
            swwq: DependentCost::free(),

            // Non-opcode costs
            contract_root: DependentCost::free(),
            state_root: DependentCost::free(),
            new_storage_per_byte: 0,
            vm_initialization: DependentCost::free(),
        }
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        Self {
            add: 1,
            addi: 1,
            and: 1,
            andi: 1,
            bal: 1,
            bhei: 1,
            bhsh: 1,
            burn: 1,
            cb: 1,
            cfei: 1,
            cfsi: 1,
            div: 1,
            divi: 1,
            eck1: 1,
            ecr1: 1,
            ed19: 1,
            eq: 1,
            exp: 1,
            expi: 1,
            flag: 1,
            gm: 1,
            gt: 1,
            gtf: 1,
            ji: 1,
            jmp: 1,
            jne: 1,
            jnei: 1,
            jnzi: 1,
            jmpf: 1,
            jmpb: 1,
            jnzf: 1,
            jnzb: 1,
            jnef: 1,
            jneb: 1,
            lb: 1,
            log: 1,
            lt: 1,
            lw: 1,
            mint: 1,
            mlog: 1,
            mod_op: 1,
            modi: 1,
            move_op: 1,
            movi: 1,
            mroo: 1,
            mul: 1,
            muli: 1,
            mldv: 1,
            noop: 1,
            not: 1,
            or: 1,
            ori: 1,
            ret: 1,
            poph: 1,
            popl: 1,
            pshh: 1,
            pshl: 1,
            rvrt: 1,
            sb: 1,
            sll: 1,
            slli: 1,
            srl: 1,
            srli: 1,
            srw: 1,
            sub: 1,
            subi: 1,
            sw: 1,
            sww: 1,
            time: 1,
            tr: 1,
            tro: 1,
            wdcm: 1,
            wqcm: 1,
            wdop: 1,
            wqop: 1,
            wdml: 1,
            wqml: 1,
            wddv: 1,
            wqdv: 1,
            wdmd: 1,
            wqmd: 1,
            wdam: 1,
            wqam: 1,
            wdmm: 1,
            wqmm: 1,
            xor: 1,
            xori: 1,
            aloc: DependentCost::unit(),
            call: DependentCost::unit(),
            ccp: DependentCost::unit(),
            croo: DependentCost::unit(),
            csiz: DependentCost::unit(),
            k256: DependentCost::unit(),
            ldc: DependentCost::unit(),
            logd: DependentCost::unit(),
            mcl: DependentCost::unit(),
            mcli: DependentCost::unit(),
            mcp: DependentCost::unit(),
            mcpi: DependentCost::unit(),
            meq: DependentCost::unit(),
            retd: DependentCost::unit(),
            s256: DependentCost::unit(),
            scwq: DependentCost::unit(),
            smo: DependentCost::unit(),
            srwq: DependentCost::unit(),
            swwq: DependentCost::unit(),

            // Non-opcode costs
            contract_root: DependentCost::unit(),
            state_root: DependentCost::unit(),
            new_storage_per_byte: 1,
            vm_initialization: DependentCost::unit(),
        }
    }
}

impl GasCostsValuesV3 {
    /// Create costs that are all set to zero.
    pub fn free() -> Self {
        Self {
            add: 0,
            addi: 0,
            and: 0,
            andi: 0,
            bal: 0,
            bhei: 0,
            bhsh: 0,
            burn: 0,
            cb: 0,
            cfsi: 0,
            div: 0,
            divi: 0,
            eck1: 0,
            ecr1: 0,
            ed19: 0,
            eq: 0,
            exp: 0,
            expi: 0,
            flag: 0,
            gm: 0,
            gt: 0,
            gtf: 0,
            ji: 0,
            jmp: 0,
            jne: 0,
            jnei: 0,
            jnzi: 0,
            jmpf: 0,
            jmpb: 0,
            jnzf: 0,
            jnzb: 0,
            jnef: 0,
            jneb: 0,
            lb: 0,
            log: 0,
            lt: 0,
            lw: 0,
            mint: 0,
            mlog: 0,
            mod_op: 0,
            modi: 0,
            move_op: 0,
            movi: 0,
            mroo: 0,
            mul: 0,
            muli: 0,
            mldv: 0,
            noop: 0,
            not: 0,
            or: 0,
            ori: 0,
            poph: 0,
            popl: 0,
            pshh: 0,
            pshl: 0,
            ret: 0,
            rvrt: 0,
            sb: 0,
            sll: 0,
            slli: 0,
            srl: 0,
            srli: 0,
            srw: 0,
            sub: 0,
            subi: 0,
            sw: 0,
            sww: 0,
            time: 0,
            tr: 0,
            tro: 0,
            wdcm: 0,
            wqcm: 0,
            wdop: 0,
            wqop: 0,
            wdml: 0,
            wqml: 0,
            wddv: 0,
            wqdv: 0,
            wdmd: 0,
            wqmd: 0,
            wdam: 0,
            wqam: 0,
            wdmm: 0,
            wqmm: 0,
            xor: 0,
            xori: 0,
            aloc: DependentCost::free(),
            cfe: DependentCost::free(),
            cfei: DependentCost::free(),
            call: DependentCost::free(),
            ccp: DependentCost::free(),
            croo: DependentCost::free(),
            csiz: DependentCost::free(),
            k256: DependentCost::free(),
            ldc: DependentCost::free(),
            logd: DependentCost::free(),
            mcl: DependentCost::free(),
            mcli: DependentCost::free(),
            mcp: DependentCost::free(),
            mcpi: DependentCost::free(),
            meq: DependentCost::free(),
            retd: DependentCost::free(),
            s256: DependentCost::free(),
            scwq: DependentCost::free(),
            smo: DependentCost::free(),
            srwq: DependentCost::free(),
            swwq: DependentCost::free(),

            // Non-opcode costs
            contract_root: DependentCost::free(),
            state_root: DependentCost::free(),
            new_storage_per_byte: 0,
            vm_initialization: DependentCost::free(),
        }
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        Self {
            add: 1,
            addi: 1,
            and: 1,
            andi: 1,
            bal: 1,
            bhei: 1,
            bhsh: 1,
            burn: 1,
            cb: 1,
            cfsi: 1,
            div: 1,
            divi: 1,
            eck1: 1,
            ecr1: 1,
            ed19: 1,
            eq: 1,
            exp: 1,
            expi: 1,
            flag: 1,
            gm: 1,
            gt: 1,
            gtf: 1,
            ji: 1,
            jmp: 1,
            jne: 1,
            jnei: 1,
            jnzi: 1,
            jmpf: 1,
            jmpb: 1,
            jnzf: 1,
            jnzb: 1,
            jnef: 1,
            jneb: 1,
            lb: 1,
            log: 1,
            lt: 1,
            lw: 1,
            mint: 1,
            mlog: 1,
            mod_op: 1,
            modi: 1,
            move_op: 1,
            movi: 1,
            mroo: 1,
            mul: 1,
            muli: 1,
            mldv: 1,
            noop: 1,
            not: 1,
            or: 1,
            ori: 1,
            ret: 1,
            poph: 1,
            popl: 1,
            pshh: 1,
            pshl: 1,
            rvrt: 1,
            sb: 1,
            sll: 1,
            slli: 1,
            srl: 1,
            srli: 1,
            srw: 1,
            sub: 1,
            subi: 1,
            sw: 1,
            sww: 1,
            time: 1,
            tr: 1,
            tro: 1,
            wdcm: 1,
            wqcm: 1,
            wdop: 1,
            wqop: 1,
            wdml: 1,
            wqml: 1,
            wddv: 1,
            wqdv: 1,
            wdmd: 1,
            wqmd: 1,
            wdam: 1,
            wqam: 1,
            wdmm: 1,
            wqmm: 1,
            xor: 1,
            xori: 1,
            aloc: DependentCost::unit(),
            cfe: DependentCost::unit(),
            cfei: DependentCost::unit(),
            call: DependentCost::unit(),
            ccp: DependentCost::unit(),
            croo: DependentCost::unit(),
            csiz: DependentCost::unit(),
            k256: DependentCost::unit(),
            ldc: DependentCost::unit(),
            logd: DependentCost::unit(),
            mcl: DependentCost::unit(),
            mcli: DependentCost::unit(),
            mcp: DependentCost::unit(),
            mcpi: DependentCost::unit(),
            meq: DependentCost::unit(),
            retd: DependentCost::unit(),
            s256: DependentCost::unit(),
            scwq: DependentCost::unit(),
            smo: DependentCost::unit(),
            srwq: DependentCost::unit(),
            swwq: DependentCost::unit(),

            // Non-opcode costs
            contract_root: DependentCost::unit(),
            state_root: DependentCost::unit(),
            new_storage_per_byte: 1,
            vm_initialization: DependentCost::unit(),
        }
    }
}

impl GasCostsValuesV4 {
    /// Create costs that are all set to zero.
    pub fn free() -> Self {
        Self {
            add: 0,
            addi: 0,
            and: 0,
            andi: 0,
            bal: 0,
            bhei: 0,
            bhsh: 0,
            burn: 0,
            cb: 0,
            cfsi: 0,
            div: 0,
            divi: 0,
            eck1: 0,
            ecr1: 0,
            eq: 0,
            exp: 0,
            expi: 0,
            flag: 0,
            gm: 0,
            gt: 0,
            gtf: 0,
            ji: 0,
            jmp: 0,
            jne: 0,
            jnei: 0,
            jnzi: 0,
            jmpf: 0,
            jmpb: 0,
            jnzf: 0,
            jnzb: 0,
            jnef: 0,
            jneb: 0,
            lb: 0,
            log: 0,
            lt: 0,
            lw: 0,
            mint: 0,
            mlog: 0,
            mod_op: 0,
            modi: 0,
            move_op: 0,
            movi: 0,
            mroo: 0,
            mul: 0,
            muli: 0,
            mldv: 0,
            noop: 0,
            not: 0,
            or: 0,
            ori: 0,
            poph: 0,
            popl: 0,
            pshh: 0,
            pshl: 0,
            ret: 0,
            rvrt: 0,
            sb: 0,
            sll: 0,
            slli: 0,
            srl: 0,
            srli: 0,
            srw: 0,
            sub: 0,
            subi: 0,
            sw: 0,
            sww: 0,
            time: 0,
            tr: 0,
            tro: 0,
            wdcm: 0,
            wqcm: 0,
            wdop: 0,
            wqop: 0,
            wdml: 0,
            wqml: 0,
            wddv: 0,
            wqdv: 0,
            wdmd: 0,
            wqmd: 0,
            wdam: 0,
            wqam: 0,
            wdmm: 0,
            wqmm: 0,
            xor: 0,
            xori: 0,
            eadd: 0,
            emul: 0,
            aloc: DependentCost::free(),
            bsiz: DependentCost::free(),
            bldd: DependentCost::free(),
            cfe: DependentCost::free(),
            cfei: DependentCost::free(),
            call: DependentCost::free(),
            ccp: DependentCost::free(),
            croo: DependentCost::free(),
            csiz: DependentCost::free(),
            ed19: DependentCost::free(),
            k256: DependentCost::free(),
            ldc: DependentCost::free(),
            logd: DependentCost::free(),
            mcl: DependentCost::free(),
            mcli: DependentCost::free(),
            mcp: DependentCost::free(),
            mcpi: DependentCost::free(),
            meq: DependentCost::free(),
            retd: DependentCost::free(),
            s256: DependentCost::free(),
            scwq: DependentCost::free(),
            smo: DependentCost::free(),
            srwq: DependentCost::free(),
            swwq: DependentCost::free(),
            epar: DependentCost::free(),

            // Non-opcode costs
            contract_root: DependentCost::free(),
            state_root: DependentCost::free(),
            new_storage_per_byte: 0,
            vm_initialization: DependentCost::free(),
        }
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        Self {
            add: 1,
            addi: 1,
            and: 1,
            andi: 1,
            bal: 1,
            bhei: 1,
            bhsh: 1,
            burn: 1,
            cb: 1,
            cfsi: 1,
            div: 1,
            divi: 1,
            eck1: 1,
            ecr1: 1,
            eq: 1,
            exp: 1,
            expi: 1,
            flag: 1,
            gm: 1,
            gt: 1,
            gtf: 1,
            ji: 1,
            jmp: 1,
            jne: 1,
            jnei: 1,
            jnzi: 1,
            jmpf: 1,
            jmpb: 1,
            jnzf: 1,
            jnzb: 1,
            jnef: 1,
            jneb: 1,
            lb: 1,
            log: 1,
            lt: 1,
            lw: 1,
            mint: 1,
            mlog: 1,
            mod_op: 1,
            modi: 1,
            move_op: 1,
            movi: 1,
            mroo: 1,
            mul: 1,
            muli: 1,
            mldv: 1,
            noop: 1,
            not: 1,
            or: 1,
            ori: 1,
            ret: 1,
            poph: 1,
            popl: 1,
            pshh: 1,
            pshl: 1,
            rvrt: 1,
            sb: 1,
            sll: 1,
            slli: 1,
            srl: 1,
            srli: 1,
            srw: 1,
            sub: 1,
            subi: 1,
            sw: 1,
            sww: 1,
            time: 1,
            tr: 1,
            tro: 1,
            wdcm: 1,
            wqcm: 1,
            wdop: 1,
            wqop: 1,
            wdml: 1,
            wqml: 1,
            wddv: 1,
            wqdv: 1,
            wdmd: 1,
            wqmd: 1,
            wdam: 1,
            wqam: 1,
            wdmm: 1,
            wqmm: 1,
            xor: 1,
            xori: 1,
            eadd: 1,
            emul: 1,
            aloc: DependentCost::unit(),
            bsiz: DependentCost::unit(),
            bldd: DependentCost::unit(),
            cfe: DependentCost::unit(),
            cfei: DependentCost::unit(),
            call: DependentCost::unit(),
            ccp: DependentCost::unit(),
            croo: DependentCost::unit(),
            csiz: DependentCost::unit(),
            ed19: DependentCost::unit(),
            k256: DependentCost::unit(),
            ldc: DependentCost::unit(),
            logd: DependentCost::unit(),
            mcl: DependentCost::unit(),
            mcli: DependentCost::unit(),
            mcp: DependentCost::unit(),
            mcpi: DependentCost::unit(),
            meq: DependentCost::unit(),
            retd: DependentCost::unit(),
            s256: DependentCost::unit(),
            scwq: DependentCost::unit(),
            smo: DependentCost::unit(),
            srwq: DependentCost::unit(),
            swwq: DependentCost::unit(),
            epar: DependentCost::unit(),

            // Non-opcode costs
            contract_root: DependentCost::unit(),
            state_root: DependentCost::unit(),
            new_storage_per_byte: 1,
            vm_initialization: DependentCost::unit(),
        }
    }
}

impl DependentCost {
    /// Create costs that make operations free.
    pub fn free() -> Self {
        Self::HeavyOperation {
            base: 0,
            gas_per_unit: 0,
        }
    }

    /// Create costs that make operations cost `1`.
    pub fn unit() -> Self {
        Self::HeavyOperation {
            base: 1,
            gas_per_unit: 0,
        }
    }

    pub fn from_units_per_gas(base: Word, units_per_gas: Word) -> Self {
        debug_assert!(
            units_per_gas > 0,
            "Cannot create dependent gas cost with per-0-gas ratio"
        );
        DependentCost::LightOperation {
            base,
            units_per_gas,
        }
    }

    pub fn from_gas_per_unit(base: Word, gas_per_unit: Word) -> Self {
        DependentCost::HeavyOperation { base, gas_per_unit }
    }

    pub fn base(&self) -> Word {
        match self {
            DependentCost::LightOperation { base, .. } => *base,
            DependentCost::HeavyOperation { base, .. } => *base,
        }
    }

    pub fn set_base(&mut self, value: Word) {
        match self {
            DependentCost::LightOperation { base, .. } => *base = value,
            DependentCost::HeavyOperation { base, .. } => *base = value,
        };
    }

    pub fn resolve(&self, units: Word) -> Word {
        let base = self.base();
        let dependent_value = self.resolve_without_base(units);
        base.saturating_add(dependent_value)
    }

    pub fn resolve_without_base(&self, units: Word) -> Word {
        match self {
            DependentCost::LightOperation { units_per_gas, .. } => {
                // Apply the linear transformation:
                //   f(x) = 1/m * x = x/m
                // where:
                //   x is the number of units
                //   1/m is the gas_per_unit
                units
                    .checked_div(*units_per_gas)
                    .expect("units_per_gas cannot be zero")
            }
            DependentCost::HeavyOperation { gas_per_unit, .. } => {
                // Apply the linear transformation:
                //   f(x) = mx
                // where:
                //   x is the number of units
                //   m is the gas per unit
                units.saturating_mul(*gas_per_unit)
            }
        }
    }
}

#[cfg(feature = "alloc")]
impl Deref for GasCosts {
    type Target = GasCostsValues;

    fn deref(&self) -> &Self::Target {
        &(self.0)
    }
}

impl From<GasCostsValues> for GasCosts {
    fn from(i: GasCostsValues) -> Self {
        Self(Arc::new(i))
    }
}

impl From<GasCosts> for GasCostsValues {
    fn from(i: GasCosts) -> Self {
        (*i.0).clone()
    }
}

impl From<GasCostsValuesV1> for GasCostsValues {
    fn from(i: GasCostsValuesV1) -> Self {
        GasCostsValues::V1(i)
    }
}

impl From<GasCostsValuesV2> for GasCostsValues {
    fn from(i: GasCostsValuesV2) -> Self {
        GasCostsValues::V2(i)
    }
}

impl From<GasCostsValuesV3> for GasCostsValues {
    fn from(i: GasCostsValuesV3) -> Self {
        GasCostsValues::V3(i)
    }
}
impl From<GasCostsValuesV4> for GasCostsValues {
    fn from(i: GasCostsValuesV4) -> Self {
        GasCostsValues::V4(i)
    }
}

#[cfg(test)]
mod tests {
    use crate::DependentCost;

    #[test]
    fn light_operation_gas_cost_resolves_correctly() {
        // Create a linear gas cost function with a slope of 1/10
        let cost = DependentCost::from_units_per_gas(0, 10);
        let total = cost.resolve(0);
        assert_eq!(total, 0);

        let total = cost.resolve(5);
        assert_eq!(total, 0);

        let total = cost.resolve(10);
        assert_eq!(total, 1);

        let total = cost.resolve(100);
        assert_eq!(total, 10);

        let total = cost.resolve(721);
        assert_eq!(total, 72);
    }

    #[test]
    fn heavy_operation_gas_cost_resolves_correctly() {
        // Create a linear gas cost function with a slope of 10
        let cost = DependentCost::from_gas_per_unit(0, 10);
        let total = cost.resolve(0);
        assert_eq!(total, 0);

        let total = cost.resolve(5);
        assert_eq!(total, 50);

        let total = cost.resolve(10);
        assert_eq!(total, 100);

        let total = cost.resolve(100);
        assert_eq!(total, 1_000);

        let total = cost.resolve(721);
        assert_eq!(total, 7_210);
    }
}
