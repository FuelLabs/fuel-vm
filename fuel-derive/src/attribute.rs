use proc_macro2::TokenTree;
use syn::{
    AttrStyle,
    Meta,
};
use synstructure::BindingInfo;

pub fn should_skip_field(binding: &BindingInfo<'_>) -> bool {
    for attr in &binding.ast().attrs {
        if attr.style != AttrStyle::Outer {
            continue
        }
        if let Meta::List(ml) = &attr.meta {
            if ml.path.segments.len() == 1 && ml.path.segments[0].ident == "canonical" {
                for token in ml.tokens.clone() {
                    if let TokenTree::Ident(ident) = &token {
                        if ident == "skip" {
                            return true
                        }
                    }
                }
            }
        }
    }

    false
}
