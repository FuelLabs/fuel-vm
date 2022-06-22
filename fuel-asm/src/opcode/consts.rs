/// Byte representation of an opcode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[repr(u8)]
#[non_exhaustive]
pub enum OpcodeRepr {
    /// RESERV00
    RESERV00 = 0x00,
    /// RESERV01
    RESERV01 = 0x01,
    /// RESERV02
    RESERV02 = 0x02,
    /// RESERV03
    RESERV03 = 0x03,
    /// RESERV04
    RESERV04 = 0x04,
    /// RESERV05
    RESERV05 = 0x05,
    /// RESERV06
    RESERV06 = 0x06,
    /// RESERV07
    RESERV07 = 0x07,
    /// RESERV08
    RESERV08 = 0x08,
    /// RESERV09
    RESERV09 = 0x09,
    /// RESERV0A
    RESERV0A = 0x0a,
    /// RESERV0B
    RESERV0B = 0x0b,
    /// RESERV0C
    RESERV0C = 0x0c,
    /// RESERV0D
    RESERV0D = 0x0d,
    /// RESERV0E
    RESERV0E = 0x0e,
    /// RESERV0F
    RESERV0F = 0x0f,

    // Classes 0x1_-0x4_ - No immediate value
    /// ADD
    ADD = 0x10,
    /// AND
    AND = 0x11,
    /// DIV
    DIV = 0x12,
    /// EQ
    EQ = 0x13,
    /// EXP
    EXP = 0x14,
    /// GT
    GT = 0x15,
    /// LT
    LT = 0x16,
    /// MLOG
    MLOG = 0x17,
    /// MROO
    MROO = 0x18,
    /// MOD
    MOD = 0x19,
    /// MOVE
    MOVE = 0x1a,
    /// MUL
    MUL = 0x1b,
    /// NOT
    NOT = 0x1c,
    /// OR
    OR = 0x1d,
    /// SLL
    SLL = 0x1e,
    /// SRL
    SRL = 0x1f,
    /// SUB
    SUB = 0x20,
    /// XOR
    XOR = 0x21,
    /// CIMV
    CIMV = 0x22,
    /// CTMV
    CTMV = 0x23,
    /// RET
    RET = 0x24,
    /// RETD
    RETD = 0x25,
    /// ALOC
    ALOC = 0x26,
    /// MCL
    MCL = 0x27,
    /// MCP
    MCP = 0x28,
    /// MEQ
    MEQ = 0x29,
    /// BHSH
    BHSH = 0x2a,
    /// BHEI
    BHEI = 0x2b,
    /// BURN
    BURN = 0x2c,
    /// CALL
    CALL = 0x2d,
    /// CCP
    CCP = 0x2e,
    /// CROO
    CROO = 0x2f,
    /// CSIZ
    CSIZ = 0x30,
    /// CB
    CB = 0x31,
    /// LDC
    LDC = 0x32,
    /// LOG
    LOG = 0x33,
    /// LOGD
    LOGD = 0x34,
    /// MINT
    MINT = 0x35,
    /// RVRT
    RVRT = 0x36,
    /// SLDC
    SLDC = 0x37,
    /// SRW
    SRW = 0x38,
    /// SRWQ
    SRWQ = 0x39,
    /// SWW
    SWW = 0x3a,
    /// SWWQ
    SWWQ = 0x3b,
    /// TR
    TR = 0x3c,
    /// TRO
    TRO = 0x3d,
    /// ECR
    ECR = 0x3e,
    /// K256
    K256 = 0x3f,
    /// S256
    S256 = 0x40,
    /// XIL
    XIL = 0x41,
    /// XIS
    XIS = 0x42,
    /// XOL
    XOL = 0x43,
    /// XOS
    XOS = 0x44,
    /// XWL
    XWL = 0x45,
    /// XWS
    XWS = 0x46,
    /// NOOP
    NOOP = 0x47,
    /// FLAG
    FLAG = 0x48,
    /// BAL
    BAL = 0x49,
    /// JMP
    JMP = 0x4a,
    /// JNE
    JNE = 0x4b,
    /// SMO
    SMO = 0x4c,
    /// RESERV4D
    RESERV4D = 0x4d,
    /// RESERV4E
    RESERV4E = 0x4e,
    /// RESERV4F
    RESERV4F = 0x4f,

    // Classes 0x5_-0x6_ - Immediate 12 bits
    /// ADDI
    ADDI = 0x50,
    /// ANDI
    ANDI = 0x51,
    /// DIVI
    DIVI = 0x52,
    /// EXPI
    EXPI = 0x53,
    /// MODI
    MODI = 0x54,
    /// MULI
    MULI = 0x55,
    /// ORI
    ORI = 0x56,
    /// SLLI
    SLLI = 0x57,
    /// SRLI
    SRLI = 0x58,
    /// SUBI
    SUBI = 0x59,
    /// XORI
    XORI = 0x5a,
    /// JNEI
    JNEI = 0x5b,
    /// LB
    LB = 0x5c,
    /// LW
    LW = 0x5d,
    /// SB
    SB = 0x5e,
    /// SW
    SW = 0x5f,
    /// MCPI
    MCPI = 0x60,
    /// RESERV61
    RESERV61 = 0x61,
    /// RESERV62
    RESERV62 = 0x62,
    /// RESERV63
    RESERV63 = 0x63,
    /// RESERV64
    RESERV64 = 0x64,
    /// RESERV65
    RESERV65 = 0x65,
    /// RESERV66
    RESERV66 = 0x66,
    /// RESERV67
    RESERV67 = 0x67,
    /// RESERV68
    RESERV68 = 0x68,
    /// RESERV69
    RESERV69 = 0x69,
    /// RESERV6A
    RESERV6A = 0x6a,
    /// RESERV6B
    RESERV6B = 0x6b,
    /// RESERV6C
    RESERV6C = 0x6c,
    /// RESERV6D
    RESERV6D = 0x6d,
    /// RESERV6E
    RESERV6E = 0x6e,
    /// RESERV6F
    RESERV6F = 0x6f,

    // Classes 0x7_-0x8_ - Immediate 18 bits
    /// MCLI
    MCLI = 0x70,
    /// GM
    GM = 0x71,
    /// MOVI
    MOVI = 0x72,
    /// JNZI
    JNZI = 0x73,
    /// RESERV74
    RESERV74 = 0x74,
    /// RESERV75
    RESERV75 = 0x75,
    /// RESERV76
    RESERV76 = 0x76,
    /// RESERV77
    RESERV77 = 0x77,
    /// RESERV78
    RESERV78 = 0x78,
    /// RESERV79
    RESERV79 = 0x79,
    /// RESERV7A
    RESERV7A = 0x7a,
    /// RESERV7B
    RESERV7B = 0x7b,
    /// RESERV7C
    RESERV7C = 0x7c,
    /// RESERV7D
    RESERV7D = 0x7d,
    /// RESERV7E
    RESERV7E = 0x7e,
    /// RESERV7F
    RESERV7F = 0x7f,
    /// RESERV80
    RESERV80 = 0x80,
    /// RESERV81
    RESERV81 = 0x81,
    /// RESERV82
    RESERV82 = 0x82,
    /// RESERV83
    RESERV83 = 0x83,
    /// RESERV84
    RESERV84 = 0x84,
    /// RESERV85
    RESERV85 = 0x85,
    /// RESERV86
    RESERV86 = 0x86,
    /// RESERV87
    RESERV87 = 0x87,
    /// RESERV88
    RESERV88 = 0x88,
    /// RESERV89
    RESERV89 = 0x89,
    /// RESERV8A
    RESERV8A = 0x8a,
    /// RESERV8B
    RESERV8B = 0x8b,
    /// RESERV8C
    RESERV8C = 0x8c,
    /// RESERV8D
    RESERV8D = 0x8d,
    /// RESERV8E
    RESERV8E = 0x8e,
    /// RESERV8F
    RESERV8F = 0x8f,

    // Classes 0x9_-0xa_ - Immediate 24 bits
    /// JI
    JI = 0x90,
    /// CFEI
    CFEI = 0x91,
    /// CFSI
    CFSI = 0x92,
    /// RESERV93
    RESERV93 = 0x93,
    /// RESERV94
    RESERV94 = 0x94,
    /// RESERV95
    RESERV95 = 0x95,
    /// RESERV96
    RESERV96 = 0x96,
    /// RESERV97
    RESERV97 = 0x97,
    /// RESERV98
    RESERV98 = 0x98,
    /// RESERV99
    RESERV99 = 0x99,
    /// RESERV9A
    RESERV9A = 0x9a,
    /// RESERV9B
    RESERV9B = 0x9b,
    /// RESERV9C
    RESERV9C = 0x9c,
    /// RESERV9D
    RESERV9D = 0x9d,
    /// RESERV9E
    RESERV9E = 0x9e,
    /// RESERV9F
    RESERV9F = 0x9f,
    /// RESERVA0
    RESERVA0 = 0xa0,
    /// RESERVA1
    RESERVA1 = 0xa1,
    /// RESERVA2
    RESERVA2 = 0xa2,
    /// RESERVA3
    RESERVA3 = 0xa3,
    /// RESERVA4
    RESERVA4 = 0xa4,
    /// RESERVA5
    RESERVA5 = 0xa5,
    /// RESERVA6
    RESERVA6 = 0xa6,
    /// RESERVA7
    RESERVA7 = 0xa7,
    /// RESERVA8
    RESERVA8 = 0xa8,
    /// RESERVA9
    RESERVA9 = 0xa9,
    /// RESERVAA
    RESERVAA = 0xaa,
    /// RESERVAB
    RESERVAB = 0xab,
    /// RESERVAC
    RESERVAC = 0xac,
    /// RESERVAD
    RESERVAD = 0xad,
    /// RESERVAE
    RESERVAE = 0xae,
    /// RESERVAF
    RESERVAF = 0xaf,

    /// RESERVB0
    RESERVB0 = 0xb0,
    /// RESERVB1
    RESERVB1 = 0xb1,
    /// RESERVB2
    RESERVB2 = 0xb2,
    /// RESERVB3
    RESERVB3 = 0xb3,
    /// RESERVB4
    RESERVB4 = 0xb4,
    /// RESERVB5
    RESERVB5 = 0xb5,
    /// RESERVB6
    RESERVB6 = 0xb6,
    /// RESERVB7
    RESERVB7 = 0xb7,
    /// RESERVB8
    RESERVB8 = 0xb8,
    /// RESERVB9
    RESERVB9 = 0xb9,
    /// RESERVBA
    RESERVBA = 0xba,
    /// RESERVBB
    RESERVBB = 0xbb,
    /// RESERVBC
    RESERVBC = 0xbc,
    /// RESERVBD
    RESERVBD = 0xbd,
    /// RESERVBE
    RESERVBE = 0xbe,
    /// RESERVBF
    RESERVBF = 0xbf,

    /// RESERVC0
    RESERVC0 = 0xc0,
    /// RESERVC1
    RESERVC1 = 0xc1,
    /// RESERVC2
    RESERVC2 = 0xc2,
    /// RESERVC3
    RESERVC3 = 0xc3,
    /// RESERVC4
    RESERVC4 = 0xc4,
    /// RESERVC5
    RESERVC5 = 0xc5,
    /// RESERVC6
    RESERVC6 = 0xc6,
    /// RESERVC7
    RESERVC7 = 0xc7,
    /// RESERVC8
    RESERVC8 = 0xc8,
    /// RESERVC9
    RESERVC9 = 0xc9,
    /// RESERVCA
    RESERVCA = 0xca,
    /// RESERVCB
    RESERVCB = 0xcb,
    /// RESERVCC
    RESERVCC = 0xcc,
    /// RESERVCD
    RESERVCD = 0xcd,
    /// RESERVCE
    RESERVCE = 0xce,
    /// RESERVCF
    RESERVCF = 0xcf,

    /// RESERVD0
    RESERVD0 = 0xd0,
    /// RESERVD1
    RESERVD1 = 0xd1,
    /// RESERVD2
    RESERVD2 = 0xd2,
    /// RESERVD3
    RESERVD3 = 0xd3,
    /// RESERVD4
    RESERVD4 = 0xd4,
    /// RESERVD5
    RESERVD5 = 0xd5,
    /// RESERVD6
    RESERVD6 = 0xd6,
    /// RESERVD7
    RESERVD7 = 0xd7,
    /// RESERVD8
    RESERVD8 = 0xd8,
    /// RESERVD9
    RESERVD9 = 0xd9,
    /// RESERVDA
    RESERVDA = 0xda,
    /// RESERVDB
    RESERVDB = 0xdb,
    /// RESERVDC
    RESERVDC = 0xdc,
    /// RESERVDD
    RESERVDD = 0xdd,
    /// RESERVDE
    RESERVDE = 0xde,
    /// RESERVDF
    RESERVDF = 0xdf,

    /// RESERVE0
    RESERVE0 = 0xe0,
    /// RESERVE1
    RESERVE1 = 0xe1,
    /// RESERVE2
    RESERVE2 = 0xe2,
    /// RESERVE3
    RESERVE3 = 0xe3,
    /// RESERVE4
    RESERVE4 = 0xe4,
    /// RESERVE5
    RESERVE5 = 0xe5,
    /// RESERVE6
    RESERVE6 = 0xe6,
    /// RESERVE7
    RESERVE7 = 0xe7,
    /// RESERVE8
    RESERVE8 = 0xe8,
    /// RESERVE9
    RESERVE9 = 0xe9,
    /// RESERVEA
    RESERVEA = 0xea,
    /// RESERVEB
    RESERVEB = 0xeb,
    /// RESERVEC
    RESERVEC = 0xec,
    /// RESERVED
    RESERVED = 0xed,
    /// RESERVEE
    RESERVEE = 0xee,
    /// RESERVEF
    RESERVEF = 0xef,

    /// RESERVF0
    RESERVF0 = 0xf0,
    /// RESERVF1
    RESERVF1 = 0xf1,
    /// RESERVF2
    RESERVF2 = 0xf2,
    /// RESERVF3
    RESERVF3 = 0xf3,
    /// RESERVF4
    RESERVF4 = 0xf4,
    /// RESERVF5
    RESERVF5 = 0xf5,
    /// RESERVF6
    RESERVF6 = 0xf6,
    /// RESERVF7
    RESERVF7 = 0xf7,
    /// RESERVF8
    RESERVF8 = 0xf8,
    /// RESERVF9
    RESERVF9 = 0xf9,
    /// RESERVFA
    RESERVFA = 0xfa,
    /// RESERVFB
    RESERVFB = 0xfb,
    /// RESERVFC
    RESERVFC = 0xfc,
    /// RESERVFD
    RESERVFD = 0xfd,
    /// RESERVFE
    RESERVFE = 0xfe,
    /// RESERVFF
    RESERVFF = 0xff,
}

impl OpcodeRepr {
    /// Check if the opcode representation is allowed for predicates
    ///
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/vm/main.md#predicate-verification>
    /// <https://github.com/FuelLabs/fuel-specs/blob/master/specs/vm/opcodes.md#contract-opcodes>
    pub const fn is_predicate_allowed(&self) -> bool {
        // TODO Update `OpcodeRepr` to separate contract opcodes
        // https://github.com/FuelLabs/fuel-asm/issues/68

        use OpcodeRepr::*;

        match self {
            ADD | AND | DIV | EQ | EXP | GT | LT | MLOG | MROO | MOD | MOVE | MUL | NOT | OR
            | SLL | SRL | SUB | XOR | CIMV | CTMV | RET | ALOC | MCL | MCP | MEQ | ECR | K256
            | S256 | XIL | XIS | XOL | XOS | XWL | XWS | NOOP | FLAG | ADDI | ANDI | DIVI
            | EXPI | MODI | MULI | ORI | SLLI | SRLI | SUBI | XORI | JNEI | LB | LW | SB | SW
            | MCPI | MCLI | GM | MOVI | JNZI | JI | JMP | JNE | CFEI | CFSI => true,

            _ => false,
        }
    }
}

#[test]
fn check_predicate_allowed() {
    use OpcodeRepr::*;

    // This exhaustive test will shield against changes in the opcodes structure
    for byte in 0..u8::MAX {
        let repr = OpcodeRepr::from_u8(byte);

        let should_allow = match repr {
            BAL | BHEI | BHSH | BURN | CALL | CB | CCP | CROO | CSIZ | LDC | LOG | LOGD | MINT
            | RETD | RVRT | SMO | SLDC | SRW | SRWQ | SWW | SWWQ | TR | TRO => false,

            RESERV00 | RESERV01 | RESERV02 | RESERV03 | RESERV04 | RESERV05 | RESERV06
            | RESERV07 | RESERV08 | RESERV09 | RESERV0A | RESERV0B | RESERV0C | RESERV0D
            | RESERV0E | RESERV0F | RESERV4D | RESERV4E | RESERV4F | RESERV61 | RESERV62
            | RESERV63 | RESERV64 | RESERV65 | RESERV66 | RESERV67 | RESERV68 | RESERV69
            | RESERV6A | RESERV6B | RESERV6C | RESERV6D | RESERV6E | RESERV6F | RESERV74
            | RESERV75 | RESERV76 | RESERV77 | RESERV78 | RESERV79 | RESERV7A | RESERV7B
            | RESERV7C | RESERV7D | RESERV7E | RESERV7F | RESERV80 | RESERV81 | RESERV82
            | RESERV83 | RESERV84 | RESERV85 | RESERV86 | RESERV87 | RESERV88 | RESERV89
            | RESERV8A | RESERV8B | RESERV8C | RESERV8D | RESERV8E | RESERV8F | RESERV93
            | RESERV94 | RESERV95 | RESERV96 | RESERV97 | RESERV98 | RESERV99 | RESERV9A
            | RESERV9B | RESERV9C | RESERV9D | RESERV9E | RESERV9F | RESERVA0 | RESERVA1
            | RESERVA2 | RESERVA3 | RESERVA4 | RESERVA5 | RESERVA6 | RESERVA7 | RESERVA8
            | RESERVA9 | RESERVAA | RESERVAB | RESERVAC | RESERVAD | RESERVAE | RESERVAF
            | RESERVB0 | RESERVB1 | RESERVB2 | RESERVB3 | RESERVB4 | RESERVB5 | RESERVB6
            | RESERVB7 | RESERVB8 | RESERVB9 | RESERVBA | RESERVBB | RESERVBC | RESERVBD
            | RESERVBE | RESERVBF | RESERVC0 | RESERVC1 | RESERVC2 | RESERVC3 | RESERVC4
            | RESERVC5 | RESERVC6 | RESERVC7 | RESERVC8 | RESERVC9 | RESERVCA | RESERVCB
            | RESERVCC | RESERVCD | RESERVCE | RESERVCF | RESERVD0 | RESERVD1 | RESERVD2
            | RESERVD3 | RESERVD4 | RESERVD5 | RESERVD6 | RESERVD7 | RESERVD8 | RESERVD9
            | RESERVDA | RESERVDB | RESERVDC | RESERVDD | RESERVDE | RESERVDF | RESERVE0
            | RESERVE1 | RESERVE2 | RESERVE3 | RESERVE4 | RESERVE5 | RESERVE6 | RESERVE7
            | RESERVE8 | RESERVE9 | RESERVEA | RESERVEB | RESERVEC | RESERVED | RESERVEE
            | RESERVEF | RESERVF0 | RESERVF1 | RESERVF2 | RESERVF3 | RESERVF4 | RESERVF5
            | RESERVF6 | RESERVF7 | RESERVF8 | RESERVF9 | RESERVFA | RESERVFB | RESERVFC
            | RESERVFD | RESERVFE | RESERVFF => false,

            _ => true,
        };

        assert_eq!(should_allow, repr.is_predicate_allowed());
    }
}
