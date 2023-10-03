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
    /// The struct is prefixed with the given word.
    /// The type must implement `Into<u64>`, which is used to serialize it.
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
