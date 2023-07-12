use super::*;
/// File generated by fuel-core: benches/src/bin/collect.rs:440. With the following git
/// hash
pub const GIT: &str = "9f1cecfbf8bd316e86f2359bb09813304d9e0986";
pub fn default_gas_costs() -> GasCostsValues {
    GasCostsValues {
        add: 1,
        addi: 1,
        aloc: 1,
        and: 1,
        andi: 1,
        bal: 22,
        bhei: 1,
        bhsh: 1,
        burn: 126,
        cb: 1,
        cfei: 1,
        cfsi: 1,
        croo: 20,
        div: 1,
        divi: 1,
        eck1: 1592,
        ecr1: 3000,
        ed19: 3000,
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
        k256: 16,
        lb: 1,
        log: 43,
        lt: 1,
        lw: 1,
        mcpi: 3,
        mint: 127,
        mlog: 1,
        srwq: DependentCost {
            base: 44,
            dep_per_unit: 5,
        },
        modi: 1,
        mod_op: 1,
        movi: 1,
        mroo: 2,
        mul: 1,
        muli: 1,
        mldv: 1,
        noop: 1,
        not: 1,
        or: 1,
        ori: 1,
        poph: 2,
        popl: 2,
        pshh: 2,
        pshl: 2,
        move_op: 1,
        ret: 63,
        s256: 4,
        sb: 1,
        scwq: 23,
        sll: 1,
        slli: 1,
        srl: 1,
        srli: 1,
        srw: 21,
        sub: 1,
        subi: 1,
        sw: 1,
        sww: 67,
        swwq: 68,
        time: 1,
        tr: 255,
        tro: 196,
        wdcm: 1,
        wqcm: 1,
        wdop: 1,
        wqop: 1,
        wdml: 1,
        wqml: 2,
        wddv: 2,
        wqdv: 3,
        wdmd: 4,
        wqmd: 7,
        wdam: 3,
        wqam: 4,
        wdmm: 4,
        wqmm: 4,
        xor: 1,
        xori: 1,
        call: DependentCost {
            base: 173,
            dep_per_unit: 180,
        },
        ccp: DependentCost {
            base: 22,
            dep_per_unit: 152,
        },
        csiz: DependentCost {
            base: 16,
            dep_per_unit: 868,
        },
        ldc: DependentCost {
            base: 22,
            dep_per_unit: 150,
        },
        logd: DependentCost {
            base: 48,
            dep_per_unit: 18,
        },
        mcl: DependentCost {
            base: 1,
            dep_per_unit: 2503,
        },
        mcli: DependentCost {
            base: 1,
            dep_per_unit: 2559,
        },
        mcp: DependentCost {
            base: 1,
            dep_per_unit: 1301,
        },
        meq: DependentCost {
            base: 1,
            dep_per_unit: 1747,
        },
        rvrt: 65,
        smo: DependentCost {
            base: 207,
            dep_per_unit: 18,
        },
        retd: DependentCost {
            base: 71,
            dep_per_unit: 18,
        },
    }
}
