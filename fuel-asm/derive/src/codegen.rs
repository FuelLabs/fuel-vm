use proc_macro2::{
    Ident,
    Span,
    TokenStream,
};
use quote::quote;

use crate::{
    input::{
        Instruction,
        InstructionList,
    },
    packing,
};

/// Helper function to generate a comma-separated list of tokens.
fn comma_separated(items: impl Iterator<Item = TokenStream>) -> TokenStream {
    itertools::Itertools::intersperse(items, quote! {,}).collect()
}

/// Wraps the items in a tuple, unless there is exactly one item.
fn tuple_or_single(items: impl IntoIterator<Item = TokenStream> + Clone) -> TokenStream {
    let items: Vec<_> = items.clone().into_iter().collect();
    if items.len() == 1 {
        items.into_iter().next().unwrap()
    } else {
        quote! { (#(#items),*) }
    }
}

/// `op::name(...)` shorthand
pub fn op_constructor_shorthand(instructions: &InstructionList) -> TokenStream {
    instructions.map_to_tokens(
        |Instruction {
             description,
             opcode_name,
             opcode_fn_name,
             args,
             ..
         }| {
            // Arguments that can be easily converted to form user input types, but might
            // be incorrect so these need to be checked.
            let arguments: TokenStream = args.map_to_tokens(|arg| {
                let name = &arg.name;
                if arg.is_imm() {
                    let int_type = arg.type_.smallest_containing_integer_type();
                    quote! { #name: #int_type, }
                } else {
                    let check_trait = Ident::new("CheckRegId", Span::call_site());
                    quote! { #name: impl crate::#check_trait, }
                }
            });

            let check_arguments: TokenStream = comma_separated(args.map(|arg| if arg.is_imm() {
                let name = &arg.name;
                let type_ = &arg.type_.token();
                quote! { #type_::new_checked(#name).expect("Immediate value overflows") }
            } else {
                let name = &arg.name;
                quote! { #name.check() }
            }));

            quote! {
                #[doc = #description]
                pub fn #opcode_fn_name(#arguments) -> Instruction {
                    #opcode_name::new(#check_arguments).into()
                }
            }
        },
    )
}

pub fn op_fn_new(instructions: &InstructionList) -> TokenStream {
    instructions.map_to_tokens(
        |Instruction {
             opcode_name, args, ..
         }| {
            let arguments: TokenStream = comma_separated(args.singature_pairs());

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
                        let offset = packing::argument_offset(i);
                        quote! {
                            packed_integer |= (#name.to_u8() as u32) << #offset;
                        }
                    }
                })
                .collect();

            quote! {
                impl #opcode_name {
                    #[doc = "Construct the instruction from its parts."]
                    pub fn new(#arguments) -> Self {
                        let mut packed_integer: u32 = 0;
                        #pack_strict_arguments
                        let packed = packed_integer.to_be_bytes();
                        Self([packed[1], packed[2], packed[3]])
                    }
                }
            }
        },
    )
}

pub fn op_constructors_typescript(instructions: &InstructionList) -> TokenStream {
    instructions.map_to_tokens(|Instruction {
        description,
        opcode_name,
        opcode_fn_name,
        args,
        ..
    }| {
        let arguments: TokenStream = comma_separated(args.singature_pairs());
        let pass_arguments: TokenStream = comma_separated(args.names());
        let raw_int_arguments: TokenStream = comma_separated(args
            .map(|arg| {
                let name = &arg.name;
                let inttype = arg.type_.smallest_containing_integer_type();
                quote! { #name: #inttype }
            }));

        quote! {
            #[cfg(feature = "typescript")]
            const _: () = {
                use super::*;
                #[wasm_bindgen::prelude::wasm_bindgen]
                #[doc = #description]
                pub fn #opcode_fn_name(#raw_int_arguments) -> typescript::Instruction {
                    crate::op::#opcode_fn_name(#pass_arguments).into()
                }
            };

            #[cfg(feature = "typescript")]
            #[wasm_bindgen::prelude::wasm_bindgen]
            impl #opcode_name {
                #[wasm_bindgen(constructor)]
                #[doc = "Construct the instruction from its parts."]
                pub fn new_typescript(#arguments) -> Self {
                    Self::new(#pass_arguments)
                }
            }
        }
    })
}

pub fn op_fn_unpack(instructions: &InstructionList) -> TokenStream {
    instructions.map_to_tokens(|Instruction {
        opcode_name, args, ..
    }| {
        let mut ret_args: Vec<_> = args.regs().enumerate().map(|(i, arg)| {
            let type_ = &arg.type_.token();
            let offset = packing::argument_offset(i);
            quote! {
                #type_::new((integer >> #offset) as u8)
            }
        }).collect();
        if let Some(imm) = args.imm() {
            let type_: &Ident = imm.type_.token();
            ret_args.push(quote! { #type_::new(integer as _) });
        }
        let ret_val = tuple_or_single(ret_args);
        let arg_types = tuple_or_single(args.types());
        quote! {
            impl #opcode_name {
                #[doc = "Convert the instruction into its parts, without checking for correctness."]
                pub fn unpack(self) -> #arg_types {
                    let integer = u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]]);
                    #ret_val
                }
            }
        }
    })
}

pub fn op_fn_reserved_part_is_zero(instructions: &InstructionList) -> TokenStream {
    instructions.map_to_tokens(|Instruction {
        opcode_name, args, ..
    }| {
        let reserved_bits = args.reserved_bits();
        quote! {
            impl #opcode_name {
                #[doc = "Verify that the unused bits after the instruction are zero."]
                pub(crate) fn reserved_part_is_zero(self) -> bool {
                    let integer = u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]]);
                    let with_zeroed_reserved = (integer >> #reserved_bits) << #reserved_bits;
                    with_zeroed_reserved == integer
                }
            }
        }
    })
}

pub fn op_fn_reg_ids(instructions: &InstructionList) -> TokenStream {
    instructions.map_to_tokens(|Instruction {
        opcode_name, args, ..
    }| {
        let reg_ids: Vec<_> = args.regs().enumerate().map(|(i, arg)| {
            let type_ = &arg.type_.token();
            let offset = packing::argument_offset(i);
            quote! {
                #type_::new((integer >> #offset) as u8)
            }
        }).collect();

        let reg_id_opts = comma_separated((0..4).map(|i| match reg_ids.get(i) {
            Some(reg_id) => quote! { Some(#reg_id) },
            None => quote! { None },
        }));

        quote! {
            impl #opcode_name {
                pub(crate) fn reg_ids(self) -> [Option<RegId>; 4] {
                    let integer = u32::from_be_bytes([0, self.0[0], self.0[1], self.0[2]]);
                    [ #reg_id_opts ]
                }
            }
        }
    })
}

pub fn op_structs(instructions: &InstructionList) -> TokenStream {
    instructions.map_to_tokens(|Instruction {
        description,
        opcode_name,
        ..
    }| quote! {
        #[doc = #description]
        #[derive(Clone, Copy, Eq, Hash, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        pub struct #opcode_name(pub (super) [u8; 3]);

        impl #opcode_name {
            /// The opcode number for this instruction.
            pub const OPCODE: Opcode = Opcode::#opcode_name;
        }
    })
}

pub fn op_debug_impl(instructions: &InstructionList) -> TokenStream {
    instructions.map_to_tokens(|Instruction {
        opcode_name,
        args,
        ..
    }| {
        let values: TokenStream = comma_separated(args.names());
        let fields: TokenStream = args.map_to_tokens(|arg| {
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
        });

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
    })
}

pub fn opcode_enum(instructions: &InstructionList) -> TokenStream {
    let variants: TokenStream = instructions.map_to_tokens(
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
    );
    quote! {
        #[doc = "The opcode numbers for each instruction."]
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "typescript", wasm_bindgen::prelude::wasm_bindgen)]
        pub enum Opcode {
            #variants
        }
    }
}

pub fn opcode_try_from(instructions: &InstructionList) -> TokenStream {
    let arms = instructions.map_to_tokens(
        |Instruction {
             opcode_number,
             opcode_name,
             ..
         }| {
            quote! {
                #opcode_number => Ok(Opcode::#opcode_name),
            }
        },
    );
    quote! {
        impl core::convert::TryFrom<u8> for Opcode {
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

pub fn op_conversions(instructions: &InstructionList) -> TokenStream {
    instructions.map_to_tokens(|Instruction { opcode_name, .. }| {
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
}

pub fn instruction_enum(instructions: &InstructionList) -> TokenStream {
    let variants = instructions.map_to_tokens(
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
    );
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
    }
}

pub fn instruction_enum_fn_opcode(instructions: &InstructionList) -> TokenStream {
    let arms = instructions.map_to_tokens(|Instruction { opcode_name, .. }| {
        quote! {
            Self::#opcode_name(_) => Opcode::#opcode_name,
        }
    });

    quote! {
        impl Instruction {
            #[doc = "This instruction's opcode."]
            pub fn opcode(&self) -> Opcode {
                match self {
                    #arms
                }
            }
        }
    }
}

pub fn instruction_enum_fn_reg_ids(instructions: &InstructionList) -> TokenStream {
    let variant_reg_ids =
        instructions.map_to_tokens(|Instruction { opcode_name, .. }| {
            quote! {
                Self::#opcode_name(op) => op.reg_ids(),
            }
        });

    quote! {
        impl Instruction {
            #[doc = "Unpacks all register IDs into a slice of options."]
            pub fn reg_ids(&self) -> [Option<RegId>; 4] {
                match self {
                    #variant_reg_ids
                }
            }
        }
    }
}

pub fn instruction_enum_debug(instructions: &InstructionList) -> TokenStream {
    let arms = instructions.map_to_tokens(|Instruction { opcode_name, .. }| {
        quote! {
            Self::#opcode_name(op) => op.fmt(f),
        }
    });

    quote! {
        impl core::fmt::Debug for Instruction {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                match self {
                    #arms
                }
            }
        }
    }
}

pub fn instruction_try_from_bytes(instructions: &InstructionList) -> TokenStream {
    let arms = instructions.map_to_tokens(|Instruction { opcode_name, .. }| {
        quote! {
            Opcode::#opcode_name => Ok(Self::#opcode_name({
                let op = op::#opcode_name([a, b, c]);
                if !op.reserved_part_is_zero() {
                    return Err(InvalidOpcode);
                }
                op
            })),
        }
    });
    quote! {
        impl core::convert::TryFrom<[u8; 4]> for Instruction {
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

pub fn bytes_from_instruction(instructions: &InstructionList) -> TokenStream {
    let arms = instructions.map_to_tokens(|Instruction { opcode_name, .. }| {
        quote! {
            Instruction::#opcode_name(op) => op.into(),
        }
    });
    quote! {
        impl core::convert::From<Instruction> for [u8; 4] {
            fn from(instruction: Instruction) -> [u8; 4] {
                match instruction {
                    #arms
                }
            }
        }
    }
}
