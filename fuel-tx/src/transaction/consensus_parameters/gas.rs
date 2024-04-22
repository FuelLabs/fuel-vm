//! Tools for gas instrumentalization

use core::ops::Deref;

#[cfg(feature = "alloc")]
use alloc::sync::Arc;

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
}

#[allow(missing_docs)]
impl GasCostsValues {
    pub fn add(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.add,
        }
    }

    pub fn addi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.addi,
        }
    }

    pub fn aloc(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.aloc,
        }
    }

    pub fn and(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.and,
        }
    }

    pub fn andi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.andi,
        }
    }

    pub fn bal(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.bal,
        }
    }

    pub fn bhei(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.bhei,
        }
    }

    pub fn bhsh(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.bhsh,
        }
    }

    pub fn burn(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.burn,
        }
    }

    pub fn cb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.cb,
        }
    }

    pub fn cfei(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.cfei,
        }
    }

    pub fn cfsi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.cfsi,
        }
    }

    pub fn div(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.div,
        }
    }

    pub fn divi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.divi,
        }
    }

    pub fn eck1(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.eck1,
        }
    }

    pub fn ecr1(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.ecr1,
        }
    }

    pub fn ed19(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.ed19,
        }
    }

    pub fn eq_(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.eq,
        }
    }

    pub fn exp(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.exp,
        }
    }

    pub fn expi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.expi,
        }
    }

    pub fn flag(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.flag,
        }
    }

    pub fn gm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.gm,
        }
    }

    pub fn gt(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.gt,
        }
    }

    pub fn gtf(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.gtf,
        }
    }

    pub fn ji(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.ji,
        }
    }

    pub fn jmp(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jmp,
        }
    }

    pub fn jne(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jne,
        }
    }

    pub fn jnei(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnei,
        }
    }

    pub fn jnzi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnzi,
        }
    }

    pub fn jmpf(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jmpf,
        }
    }

    pub fn jmpb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jmpb,
        }
    }

    pub fn jnzf(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnzf,
        }
    }

    pub fn jnzb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnzb,
        }
    }

    pub fn jnef(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jnef,
        }
    }

    pub fn jneb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.jneb,
        }
    }

    pub fn lb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.lb,
        }
    }

    pub fn log(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.log,
        }
    }

    pub fn lt(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.lt,
        }
    }

    pub fn lw(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.lw,
        }
    }

    pub fn mint(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mint,
        }
    }

    pub fn mlog(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mlog,
        }
    }

    pub fn mod_op(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mod_op,
        }
    }

    pub fn modi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.modi,
        }
    }

    pub fn move_op(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.move_op,
        }
    }

    pub fn movi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.movi,
        }
    }

    pub fn mroo(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mroo,
        }
    }

    pub fn mul(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mul,
        }
    }

    pub fn muli(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.muli,
        }
    }

    pub fn mldv(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.mldv,
        }
    }

    pub fn noop(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.noop,
        }
    }

    pub fn not(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.not,
        }
    }

    pub fn or(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.or,
        }
    }

    pub fn ori(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.ori,
        }
    }

    pub fn poph(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.poph,
        }
    }

    pub fn popl(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.popl,
        }
    }

    pub fn pshh(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.pshh,
        }
    }

    pub fn pshl(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.pshl,
        }
    }

    pub fn ret(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.ret,
        }
    }

    pub fn rvrt(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.rvrt,
        }
    }

    pub fn sb(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sb,
        }
    }

    pub fn sll(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sll,
        }
    }

    pub fn slli(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.slli,
        }
    }

    pub fn srl(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.srl,
        }
    }

    pub fn srli(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.srli,
        }
    }

    pub fn srw(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.srw,
        }
    }

    pub fn sub(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sub,
        }
    }

    pub fn subi(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.subi,
        }
    }

    pub fn sw(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sw,
        }
    }

    pub fn sww(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.sww,
        }
    }

    pub fn time(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.time,
        }
    }

    pub fn tr(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.tr,
        }
    }

    pub fn tro(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.tro,
        }
    }

    pub fn wdcm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdcm,
        }
    }

    pub fn wqcm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqcm,
        }
    }

    pub fn wdop(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdop,
        }
    }

    pub fn wqop(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqop,
        }
    }

    pub fn wdml(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdml,
        }
    }

    pub fn wqml(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqml,
        }
    }

    pub fn wddv(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wddv,
        }
    }

    pub fn wqdv(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqdv,
        }
    }

    pub fn wdmd(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdmd,
        }
    }

    pub fn wqmd(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqmd,
        }
    }

    pub fn wdam(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdam,
        }
    }

    pub fn wqam(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqam,
        }
    }

    pub fn wdmm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wdmm,
        }
    }

    pub fn wqmm(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.wqmm,
        }
    }

    pub fn xor(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.xor,
        }
    }

    pub fn xori(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.xori,
        }
    }

    pub fn call(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.call,
        }
    }

    pub fn ccp(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.ccp,
        }
    }

    pub fn croo(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.croo,
        }
    }

    pub fn csiz(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.csiz,
        }
    }

    pub fn k256(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.k256,
        }
    }

    pub fn ldc(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.ldc,
        }
    }

    pub fn logd(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.logd,
        }
    }

    pub fn mcl(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.mcl,
        }
    }

    pub fn mcli(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.mcli,
        }
    }

    pub fn mcp(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.mcp,
        }
    }

    pub fn mcpi(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.mcpi,
        }
    }

    pub fn meq(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.meq,
        }
    }

    pub fn retd(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.retd,
        }
    }

    pub fn s256(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.s256,
        }
    }

    pub fn scwq(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.scwq,
        }
    }

    pub fn smo(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.smo,
        }
    }

    pub fn srwq(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.srwq,
        }
    }

    pub fn swwq(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.swwq,
        }
    }

    pub fn contract_root(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.contract_root,
        }
    }

    pub fn state_root(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.state_root,
        }
    }

    pub fn new_storage_per_byte(&self) -> Word {
        match self {
            GasCostsValues::V1(v1) => v1.new_storage_per_byte,
        }
    }

    pub fn vm_initialization(&self) -> DependentCost {
        match self {
            GasCostsValues::V1(v1) => v1.vm_initialization,
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
    #[cfg_attr(feature = "serde", serde(rename = "mod"))]
    pub mod_op: Word,
    pub modi: Word,
    #[cfg_attr(feature = "serde", serde(rename = "move"))]
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
    #[cfg_attr(feature = "serde", serde(rename = "ret_contract"))]
    pub ret: Word,
    #[cfg_attr(feature = "serde", serde(rename = "rvrt_contract"))]
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
    #[cfg_attr(feature = "serde", serde(rename = "retd_contract"))]
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
        GasCostsValuesV1::free().into()
    }

    /// Create costs that are all set to one.
    pub fn unit() -> Self {
        GasCostsValuesV1::unit().into()
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
