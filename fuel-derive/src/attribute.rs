use proc_macro2::TokenTree;
use syn::{
    AttrStyle,
    Attribute,
    Meta,
};
use synstructure::BindingInfo;

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
                            // TODO: nice error messages: https://stackoverflow.com/a/54394014/2867076
                            panic!("unknown canonical attribute: {}", ident)
                        }
                    }
                }
            }
        }
    }
    false
}
