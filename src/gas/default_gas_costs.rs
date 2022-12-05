use super::*;

pub const GIT: &'static str = "191107574449225ea7a176a6e6b7ffaeea627dc4";
pub fn default_gas_costs() -> GasCostsValues {
    GasCostsValues {
        add: 1,
        addi: 1,
        and: 1,
        andi: 1,
        div: 1,
        divi: 1,
        eq: 1,
        exp: 1,
        expi: 1,
        gt: 1,
        lt: 1,
        mlog: 1,
        mod_op: 1,
        modi: 1,
        move_op: 1,
        movi: 1,
        mroo: 2,
        mul: 1,
        muli: 1,
        noop: 1,
        not: 1,
        or: 1,
        ori: 1,
        sll: 1,
        slli: 1,
        srl: 1,
        srli: 1,
        sub: 1,
        subi: 1,
        xor: 1,
        xori: 1,
        ji: 1,
        jnei: 1,
        jnzi: 1,
        jmp: 1,
        jne: 1,
        ret: 12,
        retd: 1,
        rvrt: 1,
        smo: 1,
        aloc: 1,
        cfei: 1,
        cfsi: 1,
        lb: 1,
        lw: 1,
        sb: 1,
        sw: 1,
        bal: 178,
        bhei: 1,
        bhsh: 2,
        burn: 1,
        cb: 1,
        croo: 1,
        csiz: 1,
        ldc: 1,
        log: 1,
        logd: 1,
        mint: 1,
        scwq: 1,
        srw: 1,
        srwq: 1,
        sww: 94,
        swwq: 1,
        time: 1,
        ecr: 1845,
        k256: 19,
        s256: 4,
        flag: 1,
        gm: 1,
        gtf: 1,
        tr: 1,
        tro: 1,
        mcl: DependantCost {
            base: 1,
            dep_per_unit: 2122,
        },
        mcli: DependantCost {
            base: 1,
            dep_per_unit: 2122,
        },
        mcp: DependantCost {
            base: 1,
            dep_per_unit: 1114,
        },
        mcpi: DependantCost {
            base: 1,
            dep_per_unit: 0,
        },
        ccp: DependantCost {
            base: 1,
            dep_per_unit: 0,
        },
        meq: DependantCost {
            base: 1,
            dep_per_unit: 2071,
        },
        call: DependantCost {
            base: 363,
            dep_per_unit: 11,
        },
    }
}
