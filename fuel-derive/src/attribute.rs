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
    /// Useful with`#[canonical(inner_discriminant)]`.
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

/// Pop-level `canonical` attributes for an enum
#[allow(non_snake_case)]
pub struct EnumAttrs {
    /// This is a wrapper enum where every variant can be recognized from it's first
    /// word. This means that the enum itself doesn't have to serialize the
    /// discriminant, but the field itself does so. This can be done using
    /// `#[canonical(prefix = ...)]` attribute. `TryFromPrimitive` traits are used to
    /// convert the raw bytes into the given type.
    pub inner_discriminant: Option<TokenStream>,
    /// Replaces calculation of the serialized static size with a custom function.
    pub serialized_size_static_with: Option<TokenStream>,
    /// Replaces calculation of the serialized dynamic size with a custom function.
    pub serialized_size_dynamic_with: Option<TokenStream>,
    /// Replaces serialization with a custom function.
    pub serialize_with: Option<TokenStream>,
    /// Replaces deserialization with a custom function.
    pub deserialize_with: Option<TokenStream>,
    /// Determines whether the enum has a dynamic size when `serialize_with` is used.
    pub SIZE_NO_DYNAMIC: Option<TokenStream>,
}

impl EnumAttrs {
    #[allow(non_snake_case)]
    pub fn parse(s: &synstructure::Structure) -> Self {
        let mut attrs = parse_attrs(s);

        let inner_discriminant = attrs.remove("inner_discriminant");
        let serialized_size_static_with = attrs.remove("serialized_size_static_with");
        let serialized_size_dynamic_with = attrs.remove("serialized_size_dynamic_with");
        let serialize_with = attrs.remove("serialize_with");
        let deserialize_with = attrs.remove("deserialize_with");
        let SIZE_NO_DYNAMIC = attrs.remove("SIZE_NO_DYNAMIC");

        if !attrs.is_empty() {
            panic!("unknown canonical attributes: {:?}", attrs.keys())
        }

        Self {
            inner_discriminant,
            serialized_size_static_with,
            serialized_size_dynamic_with,
            serialize_with,
            deserialize_with,
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
