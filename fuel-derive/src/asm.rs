//! fuel-asm types from macros

use proc_macro2::{
    Ident,
    Span,
    TokenStream,
};
use quote::quote;
use syn::parse::Parse;

const IMM_TYPES: &[&str] = &["Imm06", "Imm12", "Imm18", "Imm24"];

enum ArgType {
    Reg,
    Imm(usize),
}
impl ArgType {
    fn size_bits(&self) -> usize {
        match self {
            ArgType::Reg => 6,
            ArgType::Imm(bits) => *bits,
        }
    }

    fn smallest_containing_integer_type(&self) -> syn::Ident {
        match self {
            Self::Reg => syn::Ident::new("u8", Span::call_site()),
            Self::Imm(6) => syn::Ident::new("u8", Span::call_site()),
            Self::Imm(12) => syn::Ident::new("u16", Span::call_site()),
            Self::Imm(18) => syn::Ident::new("u32", Span::call_site()),
            Self::Imm(24) => syn::Ident::new("u32", Span::call_site()),
            _ => panic!("Invalid immediate size"),
        }
    }
}

struct InstructionArgument {
    name: syn::Ident,
    type_: syn::Ident,
}
impl Parse for InstructionArgument {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;
        let _: syn::Token![:] = input.parse()?;
        let type_: syn::Ident = input.parse()?;

        let tn = type_.to_string();
        if !(tn == "RegId" || IMM_TYPES.contains(&tn.as_str())) {
            return Err(syn::Error::new_spanned(
                &type_,
                format!("Invalid argument type: {}", tn),
            ));
        }

        Ok(Self { name, type_ })
    }
}
impl InstructionArgument {
    fn is_imm(&self) -> bool {
        self.type_.to_string().starts_with("Imm")
    }

    fn typeinfo(&self) -> ArgType {
        if self.is_imm() {
            let imm_size = self
                .type_
                .to_string()
                .trim_start_matches("Imm")
                .parse()
                .unwrap();
            ArgType::Imm(imm_size)
        } else {
            ArgType::Reg
        }
    }
}

struct Instruction {
    description: syn::LitStr,
    opcode_number: syn::LitInt,
    opcode_name: syn::Ident,
    opcode_fn_name: syn::Ident,
    args: Vec<InstructionArgument>,
}
impl Parse for Instruction {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let description: syn::LitStr = input.parse()?;
        let opcode_number: syn::LitInt = input.parse()?;
        let opcode_name: syn::Ident = input.parse()?;
        let opcode_fn_name: syn::Ident = input.parse()?;
        let mut args = Vec::new();

        let content;
        let _bracket_token = syn::bracketed!(content in input);

        while !content.is_empty() {
            let item: InstructionArgument = content.parse()?;
            args.push(item);
        }

        // Check argument format
        if args.len() > 4 {
            return Err(syn::Error::new_spanned(
                &opcode_name,
                format!("Too many arguments: {}", args.len()),
            ));
        }

        for arg in args.iter().rev().skip(1) {
            if arg.is_imm() {
                return Err(syn::Error::new_spanned(
                    &arg.type_,
                    "Immediate argument only allowed as last argument",
                ));
            }
        }

        if args.iter().map(|a| a.typeinfo().size_bits()).sum::<usize>() > 24 {
            return Err(syn::Error::new_spanned(
                &opcode_name,
                "Arguments exceed 24 bits",
            ));
        }

        Ok(Self {
            description,
            opcode_number,
            opcode_name,
            opcode_fn_name,
            args,
        })
    }
}
impl Instruction {
    fn has_imm(&self) -> bool {
        self.args.last().map(|arg| arg.is_imm()).unwrap_or(false)
    }

    #[allow(clippy::arithmetic_side_effects)] // Checked in opcode construction
    fn reserved_bits(&self) -> usize {
        if self.has_imm() {
            0
        } else {
            24 - self.args.len() * 6
        }
    }
}

struct InstructionList(Vec<Instruction>);
impl Parse for InstructionList {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut instructions = Vec::new();
        while !input.is_empty() {
            let item: Instruction = input.parse()?;
            instructions.push(item);
        }
        Ok(Self(instructions))
    }
}

/// Constructor functions and theirs shorthands
fn make_constructors(instructions: &InstructionList) -> TokenStream {
    instructions
        .0
        .iter()
        .map(
            |Instruction {
                 description,
                 opcode_name,
                 opcode_fn_name,
                 args,
                 ..
             }| {
                let strict_arguments: TokenStream = args
                    .iter()
                    .map(|arg| {
                        let name = &arg.name;
                        let type_ = &arg.type_;
                        quote! { #name: #type_, }
                    })
                    .collect();

                let pack_strict_arguments: TokenStream = args
                    .iter()
                    .enumerate()
                    .map(|(i, arg)| {
                        let name = &arg.name;
                        if arg.is_imm() {
                            quote! {
                                packed_integer |= (#name.to_smallest_int() as u32);
                            }
                        } else {
                            quote! {
                                packed_integer |= (#name.to_u8() as u32) << (6 * (3 - #i));
                            }
                        }
                    })
                    .collect();

                let pack_test_arguments: TokenStream = args
                    .iter()
                    .enumerate()
                    .map(|(i, arg)| {
                        let reg_name = Ident::new(&format!("reg{i}"), Span::call_site());
                        match arg.typeinfo() {
                            ArgType::Imm(bits) =>{
                                let bits: u32 = bits.try_into().expect("Type size is checked");
                                quote! {
                                    packed_integer |= imm & ((#bits << 1u32) -1);
                                }
                            },
                            ArgType::Reg => quote! {
                                packed_integer |= (#reg_name.to_u8() as u32) << (6 * (3 - #i));
                            }
                        }
                    })
                    .collect();

                let flexible_arguments: TokenStream = args
                    .iter()
                    .map(|arg| {
                        let name = &arg.name;
                        let type_ = &arg.type_;
                        if arg.is_imm() {
                            let int_type = arg.typeinfo().smallest_containing_integer_type();
                            quote! { #name: #int_type, }
                        } else {
                            let check_trait = Ident::new(
                                &format!("Check{type_}"),
                                Span::call_site(),
                            );
                            quote! { #name: impl crate::#check_trait, }
                        }
                    })
                    .collect();

                let check_flexible_arguments: TokenStream = args
                    .iter()
                    .map(|arg| if arg.is_imm() {
                        let name = &arg.name;
                        let type_ = &arg.type_;
                        quote! { #type_::new_checked(#name).expect("Immediate value overflows"), }
                    } else {
                        let name = &arg.name;
                        quote! { #name.check(), }
                    })
                    .collect();

                let pass_arguments: TokenStream = args
                    .iter()
                    .map(|InstructionArgument { name, .. }| quote! { #name, })
                    .collect();

                quote! {
                    #[doc = #description]
                    pub fn #opcode_fn_name(#flexible_arguments) -> Instruction {
                        #opcode_name::new(#check_flexible_arguments).into()
                    }

                    impl #opcode_name {
                        #[doc = "Construct the instruction from its parts."]
                        pub fn new(#strict_arguments) -> Self {
                            let mut packed_integer: u32 = 0;
                            #pack_strict_arguments
                            let packed = packed_integer.to_be_bytes();
                            Self([packed[1], packed[2], packed[3]])
                        }

                        #[doc = "Construct the instruction from all possible raw fields, ignoring inapplicable ones."]
                        pub fn test_construct(
                            reg0: RegId,
                            reg1: RegId,
                            reg2: RegId,
                            reg3: RegId,
                            imm: u32,
                        ) -> Self {
                            let mut packed_integer: u32 = 0;
                            #pack_test_arguments
                            let packed = packed_integer.to_be_bytes();
                            Self([packed[1], packed[2], packed[3]])
                        }
                    }


                    #[cfg(feature = "typescript")]
                    #[wasm_bindgen::prelude::wasm_bindgen]
                    impl #opcode_name {
                        #[wasm_bindgen(constructor)]
                        #[doc = "Construct the instruction from its parts."]
                        pub fn new_typescript(#strict_arguments) -> Self {
                            Self::new(#pass_arguments)
                        }
                    }
                }
            },
        )
        .collect()
}

fn make_op_unpacks(instructions: &InstructionList) -> TokenStream {
    instructions
        .0
        .iter()
        .map(
            |instr| {
               let Instruction {
                 opcode_name, args, ..
             } = instr;
                let arg_types: Vec<_> = args
                    .iter()
                    .map(|InstructionArgument { type_, .. }| type_)
                    .collect();
                let convert_reg_args: Vec<_> = args
                    .iter()
                    .enumerate()
                    .filter_map(
                        |(i, arg)| {
                            let type_ = &arg.type_;
                            if arg.is_imm() {
                                None
                            } else {
                                Some(quote! {
                                    #type_::new((integer >> (6 * (3 - #i))) as u8)
                                })
                            }
                        },
                    )
                    .collect();
                let reserved_bits = instr.reserved_bits();

                let mut ret_args = convert_reg_args;
                if let Some(convert_imm_arg) = args.last().and_then(|arg| {
                    let type_: &Ident = &arg.type_;
                    if arg.is_imm() {
                        Some(quote! { #type_::new(integer as _) })
                    } else {None}}
                ) {
                    ret_args.push(convert_imm_arg);
                }


                // Return value for unpack. If there is only one argument, doesn't wrap it in a tuple.
                let retval = if ret_args.len() == 1 {
                    let ra = &ret_args[0];
                    quote! { #ra }
                } else {
                    let ra: TokenStream = itertools::Itertools::intersperse(
                    ret_args.iter().cloned(),
                    quote!{,}
                    )
                    .collect();
                    quote! { ( #ra ) }
                };
                let arg_types = if arg_types.len() == 1 {
                    let at = arg_types[0];
                    quote! { #at }
                } else {
                    let ats: TokenStream = arg_types.iter().map(|at| quote! {#at,} ).collect();
                    quote! { (#ats) }
                };

                // Like above but always tuple-wraps
                let raw_regs = {
                    let ra: TokenStream =
                    ret_args.iter().map(|a| quote! {#a,})
                    .collect();
                    quote! { ( #ra ) }
                };

                let reg_ids: TokenStream = (0..4).map(|i| {
                    if let Some(arg) = args.get(i) {
                        let tuple_index = proc_macro2::Literal::usize_unsuffixed(i);
                        if !arg.is_imm() {
                            return quote! { Some(fields.#tuple_index), };
                        }
                    }
                    quote![ None, ]
                }).collect();

                quote! {
                    impl #opcode_name {
                        #[doc = "Convert the instruction into its parts, without checking for correctness."]
                        pub fn unpack(self) -> #arg_types {
                            let integer = u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]]);
                            #retval
                        }

                        #[doc = "Verify that the unused bits after the instruction are zero."]
                        pub(crate) fn reserved_part_is_zero(self) -> bool {
                            let integer = u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]]);
                            let mask = (1u32 << #reserved_bits) - 1;
                            (integer & mask) == 0
                        }

                        pub(crate) fn reg_ids(self) -> [Option<RegId>; 4] {
                            let integer = u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]]);
                            let fields = #raw_regs;
                            [ #reg_ids ]
                        }
                    }
                }
            },
        )
        .collect()
}

/// Make a struct for each opcode
fn make_op_structs(instructions: &InstructionList) -> TokenStream {
    instructions
        .0
        .iter()
        .map(
            |Instruction {
                 description,
                 opcode_name,
                 ..
             }| {
                quote! {
                    #[doc = #description]
                    #[derive(Clone, Copy, Eq, Hash, PartialEq)]
                    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
                    #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
                    pub struct #opcode_name(pub (super) [u8; 3]);

                    impl #opcode_name {
                        /// The opcode number for this instruction.
                        pub const OPCODE: Opcode = Opcode::#opcode_name;
                    }
                }
             })
        .collect()
}

fn make_op_debug_impl(instructions: &InstructionList) -> TokenStream {
    instructions
        .0
        .iter()
        .map(
            |Instruction {
                 opcode_name,
                 args,
                 ..
             }| {
                let values: TokenStream = itertools::Itertools::intersperse(args.iter().map(|arg| {
                    let name = &arg.name;
                    quote! {
                        #name
                    }
                }), quote!{,}).collect();
                let fields: TokenStream = args.iter().map(|arg| {
                    let name = &arg.name;
                    if arg.is_imm() {
                        quote! {
                            .field(stringify!(#name), &format_args!("{}", #name.to_smallest_int()))
                        }
                    } else {
                        quote! {
                            .field(stringify!(#name), &format_args!("{:#02x}", u8::from(#name)))
                        }
                    }
                }).collect();

                let unpack_if_needed = if args.is_empty() {
                    quote! {}
                } else {
                    quote! {
                        let (#values) = self.unpack();
                    }
                };

                quote! {
                    impl core::fmt::Debug for #opcode_name {
                        #[warn(clippy::unused_unit)] // Simplify code
                        fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                            #unpack_if_needed
                            f.debug_struct(stringify!(#opcode_name))
                                #fields
                                .finish()
                        }
                    }
                }
            },
        )
        .collect()
}

fn make_opcode_enum(instructions: &InstructionList) -> TokenStream {
    let variants: TokenStream = instructions
        .0
        .iter()
        .map(
            |Instruction {
                 description,
                 opcode_name,
                 opcode_number,
                 ..
             }| {
                quote! {
                    #[doc = #description]
                    #opcode_name = #opcode_number,
                }
            },
        )
        .collect();
    let variants_test_construct: TokenStream = instructions
        .0
        .iter()
        .map(
            |Instruction {
                 description,
                 opcode_name,
                 ..
             }| {
                quote! {
                    #[doc = #description]
                    Self::#opcode_name => Instruction::#opcode_name(
                        crate::_op::#opcode_name::test_construct(ra, rb, rc, rd, imm)
                    ),
                }
            },
        )
        .collect();
    quote! {
        #[doc = "The opcode numbers for each instruction."]
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        pub enum Opcode {
            #variants
        }

        impl Opcode {
            /// Construct the instruction from all possible raw fields, ignoring inapplicable ones.
            #[cfg(test)]
            pub fn test_construct(self, ra: RegId, rb: RegId, rc: RegId, rd: RegId, imm: u32) -> Instruction {
                match self {
                    #variants_test_construct
                }
            }
        }
    }
}

fn make_opcode_try_from(instructions: &InstructionList) -> TokenStream {
    let arms: TokenStream = instructions
        .0
        .iter()
        .map(
            |Instruction {
                 opcode_number,
                 opcode_name,
                 ..
             }| {
                quote! {
                    #opcode_number => Ok(Opcode::#opcode_name),
                }
            },
        )
        .collect();
    quote! {
        impl std::convert::TryFrom<u8> for Opcode {
            type Error = InvalidOpcode;

            fn try_from(value: u8) -> Result<Self, Self::Error> {
                match value {
                    #arms
                    _ => Err(InvalidOpcode),
                }
            }
        }
    }
}

fn make_from_op(instructions: &InstructionList) -> TokenStream {
    instructions
        .0
        .iter()
        .map(|Instruction { opcode_name, .. }| {
            quote! {
                impl From<#opcode_name> for [u8; 3] {
                    fn from(#opcode_name(arr): #opcode_name) -> Self {
                        arr
                    }
                }

                impl From<#opcode_name> for [u8; 4] {
                    fn from(#opcode_name([a, b, c]): #opcode_name) -> Self {
                        [#opcode_name::OPCODE as u8, a, b, c]
                    }
                }

                impl From<#opcode_name> for u32 {
                    fn from(op: #opcode_name) -> Self {
                        u32::from_be_bytes(op.into())
                    }
                }

                impl From<#opcode_name> for Instruction {
                    fn from(op: #opcode_name) -> Self {
                        Instruction::#opcode_name(op)
                    }
                }

                #[cfg(feature = "typescript")]
                impl From<#opcode_name> for typescript::Instruction {
                    fn from(opcode: #opcode_name) -> Self {
                        typescript::Instruction::new(opcode.into())
                    }
                }
            }
        })
        .collect()
}

fn make_instruction_enum(instructions: &InstructionList) -> TokenStream {
    let variants: TokenStream = instructions
        .0
        .iter()
        .map(
            |Instruction {
                 description,
                 opcode_name,
                 ..
             }| {
                quote! {
                    #[doc = #description]
                    #opcode_name(_op::#opcode_name),
                }
            },
        )
        .collect();
    let variant_opcodes: TokenStream = instructions
        .0
        .iter()
        .map(|Instruction { opcode_name, .. }| {
            quote! {
                Self::#opcode_name(_) => Opcode::#opcode_name,
            }
        })
        .collect();
    let variant_reg_ids: TokenStream = instructions
        .0
        .iter()
        .map(|Instruction { opcode_name, .. }| {
            quote! {
                Self::#opcode_name(op) => op.reg_ids(),
            }
        })
        .collect();

    let variant_debug: TokenStream = instructions
        .0
        .iter()
        .map(|Instruction { opcode_name, .. }| {
            quote! {
                Self::#opcode_name(op) => op.fmt(f),
            }
        })
        .collect();

    quote! {
        #[doc = r"
        Representation of a single instruction for the interpreter.
       
        The opcode is represented in the tag (variant), or may be retrieved in the form of an
        `Opcode` byte using the `opcode` method.
       
        The register and immediate data associated with the instruction is represented within
        an inner unit type wrapper around the 3 remaining bytes.
        "]
        #[derive(Clone, Copy, Eq, Hash, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub enum Instruction {
            #variants
        }

        impl Instruction {
            #[doc = "This instruction's opcode."]
            pub fn opcode(&self) -> Opcode {
                match self {
                    #variant_opcodes
                }
            }

            #[doc = "Unpacks all register IDs into a slice of options."]
            pub fn reg_ids(&self) -> [Option<RegId>; 4] {
                match self {
                    #variant_reg_ids
                }
            }
        }

        impl core::fmt::Debug for Instruction {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                match self {
                    #variant_debug
                }
            }
        }
    }
}

fn make_instruction_try_from_bytes(instructions: &InstructionList) -> TokenStream {
    let arms: TokenStream = instructions
        .0
        .iter()
        .map(|Instruction { opcode_name, .. }| {
            quote! {
                Opcode::#opcode_name => Ok(Self::#opcode_name({
                    let op = op::#opcode_name([a, b, c]);
                    if !op.reserved_part_is_zero() {
                        return Err(InvalidOpcode);
                    }
                    op
                })),
            }
        })
        .collect();
    quote! {
        impl std::convert::TryFrom<[u8; 4]> for Instruction {
            type Error = InvalidOpcode;

            fn try_from([op, a, b, c]: [u8; 4]) -> Result<Self, Self::Error> {
                match Opcode::try_from(op)? {
                    #arms
                    _ => Err(InvalidOpcode),
                }
            }
        }
    }
}

fn make_bytes_from_instruction(instructions: &InstructionList) -> TokenStream {
    let arms: TokenStream = instructions
        .0
        .iter()
        .map(|Instruction { opcode_name, .. }| {
            quote! {
                Instruction::#opcode_name(op) => op.into(),
            }
        })
        .collect();
    quote! {
        impl std::convert::From<Instruction> for [u8; 4] {
            fn from(instruction: Instruction) -> [u8; 4] {
                match instruction {
                    #arms
                }
            }
        }
    }
}

/// TODO: docs
pub fn impl_instructions(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let instructions: InstructionList = syn::parse_macro_input!(input as InstructionList);

    let op_structs = make_op_structs(&instructions);
    let op_debug_impl = make_op_debug_impl(&instructions);
    let from_op = make_from_op(&instructions);
    let constructors = make_constructors(&instructions);
    let op_unpacks = make_op_unpacks(&instructions);
    let opcode_enum = make_opcode_enum(&instructions);
    let opcode_try_from = make_opcode_try_from(&instructions);
    let instruction_enum = make_instruction_enum(&instructions);
    let instruction_try_from_bytes = make_instruction_try_from_bytes(&instructions);
    let bytes_from_instruction = make_bytes_from_instruction(&instructions);
    (quote! {
        /// Opcode-specific definitions and implementations.
        #[allow(clippy::unused_unit)] // Simplify codegen
        pub mod _op {
            use super::*;
            #op_structs
            #op_debug_impl
            #from_op
            #constructors
            #op_unpacks
        }
        #opcode_enum
        #opcode_try_from
        #instruction_enum
        #instruction_try_from_bytes
        #bytes_from_instruction

        #[cfg(feature = "typescript")]
        impl From<Instruction> for typescript::Instruction {
            fn from(inst: Instruction) -> Self {
                typescript::Instruction::new(inst)
            }
        }

    })
    .into()
}
