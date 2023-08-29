// TODO: nice error messages instead of panics, see: https://stackoverflow.com/a/54394014/2867076

use std::collections::HashMap;

use proc_macro2::{
    TokenStream,
    TokenTree,
};
use syn::{
    AttrStyle,
    Attribute,
    Meta,
};
use synstructure::BindingInfo;

fn parse_attrs(s: &synstructure::Structure) -> HashMap<String, TokenStream> {
    let mut attrs = HashMap::new();

    for attr in &s.ast().attrs {
        if attr.style != AttrStyle::Outer {
            continue
        }
        if let Meta::List(ml) = &attr.meta {
            if ml.path.segments.len() == 1 && ml.path.segments[0].ident == "canonical" {
                let mut tt = ml.tokens.clone().into_iter().peekable();
                if let Some(key) = tt.next() {
                    let key = key.to_string();
                    if let Some(eq_sign) = tt.peek() {
                        if eq_sign.to_string() == "=" {
                            let _ = tt.next();
                        }
                    } else {
                        // Single token, no `=`, so it's a boolean flag.
                        let old = attrs.insert(key.clone(), TokenStream::new());
                        if old.is_some() {
                            panic!("duplicate canonical attribute: {}", key);
                        }
                        continue
                    }

                    // Key-value pair
                    let value = TokenStream::from_iter(tt);
                    let old = attrs.insert(key.clone(), value);
                    if old.is_some() {
                        panic!("duplicate canonical attribute: {}", key);
                    }
                    continue
                }
                panic!("enum-level canonical attribute must be a `key = value` pair");
            }
        }
    }

    attrs
}

/// Pop-level `canonical` attributes for a struct
pub struct StructAttrs {
    pub prefix: Option<TokenStream>,
}

impl StructAttrs {
    pub fn parse(s: &synstructure::Structure) -> Self {
        let mut attrs = parse_attrs(s);

        let prefix = attrs.remove("prefix");

        if !attrs.is_empty() {
            panic!("unknown canonical attributes: {:?}", attrs.keys())
        }

        Self { prefix }
    }
}

/// Pop-level `canonical` attributes for an enum
#[allow(non_snake_case)]
pub struct EnumAttrs {
    pub discriminant: Option<TokenStream>,
    pub inner_discriminant: Option<TokenStream>,
    pub serialized_size_with: Option<TokenStream>,
    pub serialize_with: Option<TokenStream>,
    pub deserialize_with: Option<TokenStream>,
    pub SIZE_STATIC: Option<TokenStream>,
    pub SIZE_NO_DYNAMIC: Option<TokenStream>,
}

impl EnumAttrs {
    #[allow(non_snake_case)]
    pub fn parse(s: &synstructure::Structure) -> Self {
        let mut attrs = parse_attrs(s);

        let discriminant = attrs.remove("discriminant");
        let inner_discriminant = attrs.remove("inner_discriminant");
        let serialized_size_with = attrs.remove("serialized_size_with");
        let serialize_with = attrs.remove("serialize_with");
        let deserialize_with = attrs.remove("deserialize_with");
        let SIZE_STATIC = attrs.remove("SIZE_STATIC");
        let SIZE_NO_DYNAMIC = attrs.remove("SIZE_NO_DYNAMIC");

        if !attrs.is_empty() {
            panic!("unknown canonical attributes: {:?}", attrs.keys())
        }

        Self {
            discriminant,
            inner_discriminant,
            serialized_size_with,
            serialize_with,
            deserialize_with,
            SIZE_STATIC,
            SIZE_NO_DYNAMIC,
        }
    }
}

/// Parse `#[repr(int)]` attribute for an enum.
pub fn parse_enum_repr(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.style != AttrStyle::Outer {
            continue
        }
        if let Meta::List(ml) = &attr.meta {
            if ml.path.segments.len() == 1 && ml.path.segments[0].ident == "repr" {
                if let Some(TokenTree::Ident(ident)) =
                    ml.tokens.clone().into_iter().next()
                {
                    return Some(ident.to_string())
                }
            }
        }
    }
    None
}

/// Parse `#[canonical(skip)]` attribute for a binding field.
pub fn should_skip_field_binding(binding: &BindingInfo<'_>) -> bool {
    should_skip_field(&binding.ast().attrs)
}

/// Parse `#[canonical(skip)]` attribute for a struct field.
pub fn should_skip_field(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.style != AttrStyle::Outer {
            continue
        }
        if let Meta::List(ml) = &attr.meta {
            if ml.path.segments.len() == 1 && ml.path.segments[0].ident == "canonical" {
                for token in ml.tokens.clone() {
                    if let TokenTree::Ident(ident) = &token {
                        if ident == "skip" {
                            return true
                        } else {
                            panic!("unknown canonical attribute: {}", ident)
                        }
                    }
                }
            }
        }
    }
    false
}
