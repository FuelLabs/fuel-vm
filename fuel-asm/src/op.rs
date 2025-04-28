//! Definitions and implementations for each unique instruction type, one for each
//! unique `Opcode` variant.

use super::{
    CheckRegId,
    GMArgs,
    GTFArgs,
    Imm12,
    Imm18,
    Instruction,
    RegId,
    wideint,
};

// Here we re-export the generated instruction types and constructors, but extend them
// with `gm_args` and `gtf_args` short-hand constructors below to take their `GMArgs` and
// `GTFArgs` values respectively.
#[doc(inline)]
pub use super::_op::*;

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl GM {
    /// Construct a `GM` instruction from its arguments.
    pub fn from_args(ra: RegId, args: GMArgs) -> Self {
        Self::new(ra, Imm18::new(args as _))
    }
}

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl GTF {
    /// Construct a `GTF` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, args: GTFArgs) -> Self {
        Self::new(ra, rb, Imm12::new(args as _))
    }
}

/// Construct a `GM` instruction from its arguments.
pub fn gm_args<A: CheckRegId>(ra: A, args: GMArgs) -> Instruction {
    Instruction::GM(GM::from_args(ra.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `GM` instruction from its arguments.
    pub fn gm_args(ra: u8, args: GMArgs) -> typescript::Instruction {
        Instruction::GM(GM::from_args(ra.check(), args)).into()
    }
};

/// Construct a `GM` instruction from its arguments.
pub fn gtf_args<A: CheckRegId, B: CheckRegId>(
    ra: A,
    rb: B,
    args: GTFArgs,
) -> Instruction {
    Instruction::GTF(GTF::from_args(ra.check(), rb.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `GM` instruction from its arguments.
    pub fn gtf_args(ra: u8, rb: u8, args: GTFArgs) -> typescript::Instruction {
        Instruction::GTF(GTF::from_args(ra.check(), rb.check(), args)).into()
    }
};

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl WDCM {
    /// Construct a `WDCM` instruction from its arguments.
    pub fn from_args(
        ra: RegId,
        rb: RegId,
        rc: RegId,
        args: wideint::CompareArgs,
    ) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl WQCM {
    /// Construct a `WQCM` instruction from its arguments.
    pub fn from_args(
        ra: RegId,
        rb: RegId,
        rc: RegId,
        args: wideint::CompareArgs,
    ) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl WDOP {
    /// Construct a `WDOP` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: wideint::MathArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl WQOP {
    /// Construct a `WQOP` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: wideint::MathArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl WDML {
    /// Construct a `WDML` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: wideint::MulArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl WQML {
    /// Construct a `WQML` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: wideint::MulArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl WDDV {
    /// Construct a `WDDV` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: wideint::DivArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

#[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
impl WQDV {
    /// Construct a `WQDV` instruction from its arguments.
    pub fn from_args(ra: RegId, rb: RegId, rc: RegId, args: wideint::DivArgs) -> Self {
        Self::new(ra, rb, rc, args.to_imm())
    }
}

/// Construct a `WDCM` instruction from its arguments.
pub fn wdcm_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
    ra: A,
    rb: B,
    rc: C,
    args: wideint::CompareArgs,
) -> Instruction {
    Instruction::WDCM(WDCM::from_args(ra.check(), rb.check(), rc.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `WDCM` instruction from its arguments.
    pub fn wdcm_args(
        ra: u8,
        rb: u8,
        rc: u8,
        args: wideint::CompareArgs,
    ) -> typescript::Instruction {
        crate::op::wdcm_args(ra, rb, rc, args).into()
    }
};

/// Construct a `WQCM` instruction from its arguments.
pub fn wqcm_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
    ra: A,
    rb: B,
    rc: C,
    args: wideint::CompareArgs,
) -> Instruction {
    Instruction::WQCM(WQCM::from_args(ra.check(), rb.check(), rc.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `WQCM` instruction from its arguments.
    pub fn wqcm_args(
        ra: u8,
        rb: u8,
        rc: u8,
        args: wideint::CompareArgs,
    ) -> typescript::Instruction {
        crate::op::wqcm_args(ra, rb, rc, args).into()
    }
};

/// Construct a `WDOP` instruction from its arguments.
pub fn wdop_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
    ra: A,
    rb: B,
    rc: C,
    args: wideint::MathArgs,
) -> Instruction {
    Instruction::WDOP(WDOP::from_args(ra.check(), rb.check(), rc.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `WDOP` instruction from its arguments.
    pub fn wdop_args(
        ra: u8,
        rb: u8,
        rc: u8,
        args: wideint::MathArgs,
    ) -> typescript::Instruction {
        crate::op::wdop_args(ra, rb, rc, args).into()
    }
};

/// Construct a `WQOP` instruction from its arguments.
pub fn wqop_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
    ra: A,
    rb: B,
    rc: C,
    args: wideint::MathArgs,
) -> Instruction {
    Instruction::WQOP(WQOP::from_args(ra.check(), rb.check(), rc.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `WQOP` instruction from its arguments.
    pub fn wqop_args(
        ra: u8,
        rb: u8,
        rc: u8,
        args: wideint::MathArgs,
    ) -> typescript::Instruction {
        crate::op::wqop_args(ra, rb, rc, args).into()
    }
};

/// Construct a `WDML` instruction from its arguments.
pub fn wdml_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
    ra: A,
    rb: B,
    rc: C,
    args: wideint::MulArgs,
) -> Instruction {
    Instruction::WDML(WDML::from_args(ra.check(), rb.check(), rc.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `WDML` instruction from its arguments.
    pub fn wdml_args(
        ra: u8,
        rb: u8,
        rc: u8,
        args: wideint::MulArgs,
    ) -> typescript::Instruction {
        crate::op::wdml_args(ra, rb, rc, args).into()
    }
};

/// Construct a `WQML` instruction from its arguments.
pub fn wqml_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
    ra: A,
    rb: B,
    rc: C,
    args: wideint::MulArgs,
) -> Instruction {
    Instruction::WQML(WQML::from_args(ra.check(), rb.check(), rc.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `WQML` instruction from its arguments.
    pub fn wqml_args(
        ra: u8,
        rb: u8,
        rc: u8,
        args: wideint::MulArgs,
    ) -> typescript::Instruction {
        crate::op::wqml_args(ra, rb, rc, args).into()
    }
};

/// Construct a `WDDV` instruction from its arguments.
pub fn wddv_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
    ra: A,
    rb: B,
    rc: C,
    args: wideint::DivArgs,
) -> Instruction {
    Instruction::WDDV(WDDV::from_args(ra.check(), rb.check(), rc.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `WDDV` instruction from its arguments.
    pub fn wddv_args(
        ra: u8,
        rb: u8,
        rc: u8,
        args: wideint::DivArgs,
    ) -> typescript::Instruction {
        crate::op::wddv_args(ra, rb, rc, args).into()
    }
};

/// Construct a `WQDV` instruction from its arguments.
pub fn wqdv_args<A: CheckRegId, B: CheckRegId, C: CheckRegId>(
    ra: A,
    rb: B,
    rc: C,
    args: wideint::DivArgs,
) -> Instruction {
    Instruction::WQDV(WQDV::from_args(ra.check(), rb.check(), rc.check(), args))
}

#[cfg(feature = "typescript")]
const _: () = {
    use super::*;

    #[wasm_bindgen::prelude::wasm_bindgen]
    /// Construct a `WQDV` instruction from its arguments.
    pub fn wqdv_args(
        ra: u8,
        rb: u8,
        rc: u8,
        args: wideint::DivArgs,
    ) -> typescript::Instruction {
        crate::op::wqdv_args(ra, rb, rc, args).into()
    }
};
