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

#[derive(Debug)]
pub struct EnumAttrs(pub HashMap<String, TokenStream>);
impl EnumAttrs {
    pub fn parse(s: &synstructure::Structure) -> Self {
        let mut attrs = HashMap::new();

        for attr in &s.ast().attrs {
            if attr.style != AttrStyle::Outer {
                continue
            }
            if let Meta::List(ml) = &attr.meta {
                if ml.path.segments.len() == 1 && ml.path.segments[0].ident == "canonical"
                {
                    let mut tt = ml.tokens.clone().into_iter().peekable();
                    if let Some(key) = tt.next() {
                        let key = key.to_string();
                        if let Some(eq_sign) = tt.peek() {
                            if eq_sign.to_string() == "=" {
                                let _ = tt.next();
                            }
                        }

                        let value = TokenStream::from_iter(tt);
                        attrs.insert(key, value);
                        continue
                    }
                    panic!("enum-level canonical attribute must be a `key = value` pair");
                }
            }
        }

        Self(attrs)
    }
}

pub fn should_skip_field_binding(binding: &BindingInfo<'_>) -> bool {
    should_skip_field(&binding.ast().attrs)
}

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
