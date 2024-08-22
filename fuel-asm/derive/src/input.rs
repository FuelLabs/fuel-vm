//! Input parsing

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;

#[derive(Debug, Clone)]
pub struct RegType {
    pub token: syn::Ident,
}

impl RegType {
    pub fn token(&self) -> &syn::Ident {
        &self.token
    }

    pub fn smallest_containing_integer_type(&self) -> syn::Ident {
        syn::Ident::new("u8", proc_macro2::Span::call_site())
    }

    pub fn size_bits(&self) -> usize {
        6
    }
}
#[derive(Debug, Clone)]
pub enum ImmType {
    Imm06 { token: syn::Ident },
    Imm12 { token: syn::Ident },
    Imm18 { token: syn::Ident },
    Imm24 { token: syn::Ident },
}
impl ImmType {
    pub fn token(&self) -> &syn::Ident {
        match self {
            Self::Imm06 { token } => token,
            Self::Imm12 { token } => token,
            Self::Imm18 { token } => token,
            Self::Imm24 { token } => token,
        }
    }

    pub fn smallest_containing_integer_type(&self) -> syn::Ident {
        match self {
            Self::Imm06 { .. } => syn::Ident::new("u8", proc_macro2::Span::call_site()),
            Self::Imm12 { .. } => syn::Ident::new("u16", proc_macro2::Span::call_site()),
            Self::Imm18 { .. } => syn::Ident::new("u32", proc_macro2::Span::call_site()),
            Self::Imm24 { .. } => syn::Ident::new("u32", proc_macro2::Span::call_site()),
        }
    }

    pub fn size_bits(&self) -> usize {
        match self {
            Self::Imm06 { .. } => 6,
            Self::Imm12 { .. } => 12,
            Self::Imm18 { .. } => 18,
            Self::Imm24 { .. } => 24,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AnyInstructionArgument {
    Reg(RegType),
    Imm(ImmType),
}
impl AnyInstructionArgument {
    pub fn token(&self) -> &syn::Ident {
        match self {
            Self::Reg(a) => a.token(),
            Self::Imm(a) => a.token(),
        }
    }

    pub fn smallest_containing_integer_type(&self) -> syn::Ident {
        match self {
            Self::Reg(a) => a.smallest_containing_integer_type(),
            Self::Imm(a) => a.smallest_containing_integer_type(),
        }
    }

    pub fn size_bits(&self) -> usize {
        match self {
            Self::Reg(a) => a.size_bits(),
            Self::Imm(a) => a.size_bits(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstructionArgument<T = AnyInstructionArgument> {
    pub name: syn::Ident,
    pub type_: T,
}
impl Parse for InstructionArgument<AnyInstructionArgument> {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;
        let _: syn::Token![:] = input.parse()?;
        let type_: syn::Ident = input.parse()?;

        let type_ = match type_.to_string().as_str() {
            "RegId" => AnyInstructionArgument::Reg(RegType { token: type_ }),
            "Imm06" => AnyInstructionArgument::Imm(ImmType::Imm06 { token: type_ }),
            "Imm12" => AnyInstructionArgument::Imm(ImmType::Imm12 { token: type_ }),
            "Imm18" => AnyInstructionArgument::Imm(ImmType::Imm18 { token: type_ }),
            "Imm24" => AnyInstructionArgument::Imm(ImmType::Imm24 { token: type_ }),
            _ => {
                return Err(syn::Error::new_spanned(
                    type_.clone(),
                    format!("Invalid argument type: {}", type_),
                ))
            }
        };

        Ok(Self { name, type_ })
    }
}
impl InstructionArgument {
    pub fn is_imm(&self) -> bool {
        matches!(self.type_, AnyInstructionArgument::Imm(_))
    }
}

#[derive(Debug, Clone)]
pub struct InstructionArguments(Vec<InstructionArgument>);
impl Parse for InstructionArguments {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = Vec::new();

        let content;
        let _ = syn::bracketed!(content in input);
        let full_span = content.span();

        while !content.is_empty() {
            let item: InstructionArgument = content.parse()?;
            args.push(item);
        }

        // Check argument format
        if args.len() > 4 {
            return Err(syn::Error::new(
                full_span,
                format!("Too many arguments: {}", args.len()),
            ));
        }

        for arg in args.iter().rev().skip(1) {
            if arg.is_imm() {
                return Err(syn::Error::new_spanned(
                    arg.type_.token(),
                    "Immediate argument only allowed as last argument",
                ));
            }
        }

        if args.iter().map(|a| a.type_.size_bits()).sum::<usize>() > 24 {
            return Err(syn::Error::new(full_span, "Arguments exceed 24 bits"));
        }

        Ok(Self(args))
    }
}

impl InstructionArguments {
    pub fn has_imm(&self) -> bool {
        self.0.last().map(|arg| arg.is_imm()).unwrap_or(false)
    }

    #[allow(clippy::arithmetic_side_effects)] // Checked in opcode construction
    pub fn reserved_bits(&self) -> usize {
        if self.has_imm() {
            0
        } else {
            24 - self.0.len() * 6
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Immedate argument, if any
    pub fn imm(&self) -> Option<InstructionArgument<ImmType>> {
        let last = self.0.last()?;
        if let AnyInstructionArgument::Imm(type_) = last.type_.clone() {
            Some(InstructionArgument {
                name: last.name.clone(),
                type_,
            })
        } else {
            None
        }
    }

    /// Register arguments
    pub fn regs(&self) -> impl Iterator<Item = InstructionArgument<RegType>> + '_ {
        self.iter().filter_map(|arg| {
            if let AnyInstructionArgument::Reg(type_) = arg.type_.clone() {
                Some(InstructionArgument {
                    name: arg.name.clone(),
                    type_,
                })
            } else {
                None
            }
        })
    }

    pub fn map<'a, F: FnMut(&InstructionArgument) -> T + 'a, T>(
        &'a self,
        f: F,
    ) -> impl Iterator<Item = T> + 'a {
        self.0.iter().map(f)
    }

    pub fn map_to_tokens<F: FnMut(&InstructionArgument) -> TokenStream>(
        &self,
        f: F,
    ) -> TokenStream {
        self.map(f).collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = &InstructionArgument> + '_ + Clone {
        self.0.iter()
    }

    /// `name: type` pairs like in a function signature
    pub fn singature_pairs(&self) -> impl Iterator<Item = TokenStream> + '_ + Clone {
        self.0.iter().map(|arg| {
            let name = &arg.name;
            let type_ = &arg.type_.token();
            quote! {
                #name: #type_
            }
        })
    }

    /// Just the names of the arguments as tokens
    pub fn names(&self) -> impl Iterator<Item = TokenStream> + '_ + Clone {
        self.0
            .iter()
            .map(|InstructionArgument { name, .. }| quote! { #name })
    }

    /// Just the types of the arguments as tokens
    pub fn types(&self) -> impl Iterator<Item = TokenStream> + '_ + Clone {
        self.0.iter().map(|InstructionArgument { type_, .. }| {
            let type_ = &type_.token();
            quote! { #type_ }
        })
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub description: syn::LitStr,
    pub opcode_number: syn::LitInt,
    pub opcode_name: syn::Ident,
    pub opcode_fn_name: syn::Ident,
    pub args: InstructionArguments,
}
impl Parse for Instruction {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let description: syn::LitStr = input.parse()?;
        let opcode_number: syn::LitInt = input.parse()?;
        let opcode_name: syn::Ident = input.parse()?;
        let opcode_fn_name: syn::Ident = input.parse()?;
        let args: InstructionArguments = input.parse()?;

        Ok(Self {
            description,
            opcode_number,
            opcode_name,
            opcode_fn_name,
            args,
        })
    }
}

#[derive(Debug, Clone)]
pub struct InstructionList(Vec<Instruction>);
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

impl InstructionList {
    pub fn map_to_tokens<F: FnMut(&Instruction) -> TokenStream>(
        &self,
        f: F,
    ) -> TokenStream {
        self.0.iter().map(f).collect()
    }
}
