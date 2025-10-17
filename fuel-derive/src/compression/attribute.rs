use proc_macro2::TokenTree as TokenTree2;

use syn::parse::{
    Parse,
    ParseStream,
};

const ATTR: &str = "compress";

/// Structure (struct or enum) attributes
#[derive(Debug)]
pub enum StructureAttrs {
    /// Discard generic parameter
    /// `#[compress(discard(Type))]`
    Discard(Vec<String>),
}
impl Parse for StructureAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(ml) = input.parse::<syn::MetaList>()
            && ml.path.segments.len() == 1
            && ml.path.segments[0].ident.to_string().as_str() == "discard"
        {
            let mut discard = Vec::new();
            for item in ml.tokens {
                match item {
                    TokenTree2::Ident(ident) => {
                        discard.push(ident.to_string());
                    }
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "Expected generic (type) name",
                        ))
                    }
                }
            }
            return Ok(Self::Discard(discard));
        }
        Err(syn::Error::new_spanned(
            input.parse::<syn::Ident>()?,
            "Expected `discard`",
        ))
    }
}
impl StructureAttrs {
    pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Vec<Self>> {
        let mut result = Vec::new();
        for attr in attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if let syn::Meta::List(ml) = &attr.meta
                && ml.path.segments.len() == 1
                && ml.path.segments[0].ident == ATTR
            {
                result.push(syn::parse2::<StructureAttrs>(ml.tokens.clone())?);
            }
        }

        Ok(result)
    }
}

/// Field attributes
pub enum FieldAttrs {
    /// Skipped when compressing, and must be reconstructed when decompressing.
    /// `#[compress(skip)]`
    Skip,
    /// Compressed recursively.
    Normal,
}
impl FieldAttrs {
    pub fn parse(attrs: &[syn::Attribute]) -> Self {
        let mut result = Self::Normal;
        for attr in attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if let syn::Meta::List(ml) = &attr.meta
                && ml.path.segments.len() == 1
                && ml.path.segments[0].ident == ATTR
            {
                if !matches!(result, Self::Normal) {
                    panic!("Duplicate attribute: {}", ml.tokens);
                }

                if let Ok(ident) = syn::parse2::<syn::Ident>(ml.tokens.clone())
                    && ident == "skip"
                {
                    result = Self::Skip;
                    continue;
                }
                panic!("Invalid attribute: {}", ml.tokens);
            }
        }

        result
    }
}
