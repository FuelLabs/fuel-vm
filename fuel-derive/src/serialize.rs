use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use core::cmp::Ordering;

use crate::attribute::{
    parse_enum_repr,
    should_skip_field,
    should_skip_field_binding,
    TypedefAttrs,
};

fn serialize_struct(s: &synstructure::Structure) -> TokenStream2 {
    let attrs = TypedefAttrs::parse(s);

    assert_eq!(s.variants().len(), 1, "structs must have one variant");

    let variant: &synstructure::VariantInfo = &s.variants()[0];
    let encode_static = variant.each(|binding| {
        if should_skip_field_binding(binding) {
            quote! {}
        } else {
            quote! {
                if fuel_types::canonical::Serialize::size(#binding) % fuel_types::canonical::ALIGN > 0 {
                    return ::core::result::Result::Err(fuel_types::canonical::Error::WrongAlign)
                }
                fuel_types::canonical::Serialize::encode_static(#binding, buffer)?;
            }
        }
    });

    let encode_dynamic = variant.each(|binding| {
        if should_skip_field_binding(binding) {
            quote! {}
        } else {
            quote! {
                fuel_types::canonical::Serialize::encode_dynamic(#binding, buffer)?;
            }
        }
    });

    let prefix = if let Some(prefix_type) = attrs.0.get("prefix") {
        quote! {
            let prefix: u64 = #prefix_type.into();
            <u64 as fuel_types::canonical::Serialize>::encode(&prefix, buffer)?;
        }
    } else {
        quote! {}
    };

    let (size_static, size_dynamic) = constsize_fields(variant.ast().fields);
    let size_prefix = if attrs.0.contains_key("prefix") {
        quote! {
            8 +
        }
    } else {
        quote! {}
    };

    s.gen_impl(quote! {
        gen impl fuel_types::canonical::Serialize for @Self {
            #[inline(always)]
            fn encode_static<O: fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), fuel_types::canonical::Error> {
                #prefix
                match self {
                    #encode_static
                };

                ::core::result::Result::Ok(())
            }

            fn encode_dynamic<O: fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), fuel_types::canonical::Error> {
                match self {
                    #encode_dynamic
                };

                ::core::result::Result::Ok(())
            }

            const SIZE_NO_DYNAMIC: bool = #size_dynamic;
        }

        gen impl fuel_types::canonical::SerializedSize for @Self {
            const SIZE_STATIC: usize = #size_prefix #size_static;
        }
    })
}

// TODO: somehow ensure that all enum variants have equal size, or zero-pad them
fn serialize_enum(s: &synstructure::Structure) -> TokenStream2 {
    let name = &s.ast().ident;
    let repr = parse_enum_repr(&s.ast().attrs);
    let attrs = TypedefAttrs::parse(s);

    assert!(!s.variants().is_empty(), "got invalid empty enum");
    let encode_static = s.variants().iter().enumerate().map(|(i, v)| {
        let pat = v.pat();
        let encode_static_iter = v.bindings().iter().map(|binding| {
            if should_skip_field_binding(binding) {
                quote! {}
            } else {
                quote! {
                    if fuel_types::canonical::Serialize::size(#binding) % fuel_types::canonical::ALIGN > 0 {
                        return ::core::result::Result::Err(fuel_types::canonical::Error::WrongAlign)
                    }
                    fuel_types::canonical::Serialize::encode_static(#binding, buffer)?;
                }
            }
        });

        // Handle #[canonical(discriminant = Type)] and  #[canonical(inner_discriminant)]
        let discr = if let Some(discr_type) = attrs.0.get("discriminant") {
            quote! { {
                #discr_type::from(self).into()
            } }
        } else if let Some(discr_type) = attrs.0.get("inner_discriminant") {
            quote! { {
                #discr_type::from(self).into()
            } }
        } else {
            let index = i as u64;
            quote! { #index }
        };

        let encode_discriminant = if attrs.0.contains_key("inner_discriminant") {
            quote! {}
        } else {
            quote! {
                <::core::primitive::u64 as fuel_types::canonical::Serialize>::encode(&#discr, buffer)?;
            }
        };

        quote! {
            #pat => {
                #encode_discriminant
                #(
                    { #encode_static_iter }
                )*
            }
        }
    });
    let encode_dynamic = s.variants().iter().map(|v| {
        let encode_dynamic_iter = v.each(|binding| {
            if should_skip_field_binding(binding) {
                quote! {}
            } else {
                quote! {
                    fuel_types::canonical::Serialize::encode_dynamic(#binding, buffer)?;
                }
            }
        });
        quote! {

            #encode_dynamic_iter
        }
    });

    // Handle #[canonical(serialize_with = function)]
    if let Some(helper) = attrs.0.get("serialize_with") {
        let size_no_dynamic = attrs
            .0
            .get("SIZE_NO_DYNAMIC")
            .expect("serialize_with requires SIZE_NO_DYNAMIC key");

        return s.gen_impl(quote! {
            gen impl fuel_types::canonical::Serialize for @Self {
                const SIZE_NO_DYNAMIC: bool = #size_no_dynamic;

                #[inline(always)]
                fn encode_static<O: fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), fuel_types::canonical::Error> {
                    #helper(self, buffer)
                }

                fn encode_dynamic<O: fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), fuel_types::canonical::Error> {
                    ::core::result::Result::Ok(())
                }
            }
        })
    }

    let (mut variant_size_static, size_no_dynamic): (
        Vec<TokenStream2>,
        Vec<TokenStream2>,
    ) = s
        .variants()
        .iter()
        .map(|v| constsize_fields(v.ast().fields))
        .unzip();

    variant_size_static.iter_mut().for_each(|aa| {
        if attrs.0.contains_key("inner_discriminant") {
            *aa = quote! { #aa - 8 } // TODO: panic handling
        }
    });

    let size_no_dynamic = size_no_dynamic.iter().fold(quote! { true }, |acc, item| {
        quote! {
            #acc && #item
        }
    });

    // #[repr(int)] types have a known static size
    let impl_size = if let Some(repr) = repr {
        // Repr size, already aligned up
        let repr_size: usize = match repr.as_str() {
            "u8" => 8,
            "u16" => 8,
            "u32" => 8,
            "u64" => 8,
            "u128" => 16,
            _ => panic!("Unknown repr: {}", repr),
        };
        s.gen_impl(quote! {
            gen impl fuel_types::canonical::SerializedSize for @Self {
                const SIZE_STATIC: usize = #repr_size;
            }
        })
    } else {
        quote! {}
    };

    let impl_code = s.gen_impl(quote! {
        gen impl fuel_types::canonical::Serialize for @Self {
            #[inline(always)]
            fn encode_static<O: fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), fuel_types::canonical::Error> {
                match self {
                    #(
                        #encode_static
                    )*,
                    _ => return ::core::result::Result::Err(fuel_types::canonical::Error::UnknownDiscriminant),
                };

                ::core::result::Result::Ok(())
            }

            fn encode_dynamic<O: fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), fuel_types::canonical::Error> {
                match self {
                    #(
                        #encode_dynamic
                    )*,
                    _ => return ::core::result::Result::Err(fuel_types::canonical::Error::UnknownDiscriminant),
                };

                ::core::result::Result::Ok(())
            }

            const SIZE_NO_DYNAMIC: bool = #size_no_dynamic;
        }
    });

    // Variant sizes are generated in under NameVariantSizes
    let variants_struct_name = syn::Ident::new(
        &format!("{}VariantSizes", name),
        proc_macro2::Span::call_site(),
    );

    let variants_impl_code: TokenStream2 = variant_size_static
        .into_iter()
        .zip(s.variants().iter())
        .map(|(value, variant)| {
            let variant_name = &variant.ast().ident;
            quote! {
                #[allow(non_upper_case_globals)]
                pub const #variant_name: usize = #value;
            }
        })
        .collect();

    quote! {
        pub struct #variants_struct_name;
        impl #variants_struct_name {
            #variants_impl_code
        }
        #impl_code
        #impl_size
    }
}

fn is_sized_primitive(type_name: &str) -> bool {
    [
        "u8",
        "u16",
        "u32",
        "u64",
        "u128",
        "usize",
        "i8",
        "i16",
        "i32",
        "i64",
        "i128",
        "isize",
        "Word",
        "RawInstruction",
    ]
    .contains(&type_name)
}

#[derive(Debug)]
enum TypeSize {
    /// The size of the type is known at parse time.
    Constant(usize),
    /// The size of the type is known at compile time, and
    /// the following token stream computes to it.
    Computed(TokenStream2),
}

impl TypeSize {
    pub fn from_expr(expr: &syn::Expr) -> Self {
        match expr {
            syn::Expr::Lit(lit) => match lit.lit {
                syn::Lit::Int(ref int) => {
                    if let Ok(value) = int.base10_parse::<usize>() {
                        return Self::Constant(value)
                    }
                }
                _ => {}
            },
            _ => {}
        }

        Self::Computed(quote! {
            #expr
        })
    }
}

impl quote::ToTokens for TypeSize {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::Constant(value) => {
                tokens.extend(quote! {
                    #value
                });
            }
            Self::Computed(expr) => {
                tokens.extend(expr.clone());
            }
        }
    }
}

// Used to sort constant first so that they can be folded left-to-right
impl PartialEq for TypeSize {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Constant(a), Self::Constant(b)) => a == b,
            _ => false,
        }
    }
}

// Used to sort constant first so that they can be folded left-to-right
impl PartialOrd for TypeSize {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(match (self, other) {
            (Self::Constant(a), Self::Constant(b)) => a.cmp(b),
            (Self::Computed(_), Self::Computed(_)) => Ordering::Equal,
            (Self::Constant(_), Self::Computed(_)) => Ordering::Less,
            (Self::Computed(_), Self::Constant(_)) => Ordering::Greater,
        })
    }
}

impl core::ops::Add for TypeSize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::Constant(a), Self::Constant(b)) => Self::Constant(a + b),
            (Self::Computed(a), Self::Computed(b)) => Self::Computed(quote! {
                #a + #b
            }),
            (Self::Constant(a), Self::Computed(b)) => Self::Computed(quote! {
                #a + #b
            }),
            (Self::Computed(a), Self::Constant(b)) => Self::Computed(quote! {
                #a + #b
            }),
        }
    }
}

impl core::ops::Mul for TypeSize {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::Constant(a), Self::Constant(b)) => Self::Constant(a * b),
            (Self::Computed(a), Self::Computed(b)) => Self::Computed(quote! {
                #a * #b
            }),
            (Self::Constant(a), Self::Computed(b)) => Self::Computed(quote! {
                #a * #b
            }),
            (Self::Computed(a), Self::Constant(b)) => Self::Computed(quote! {
                #a * #b
            }),
        }
    }
}

/// Determines serialized size of a type using `mem::size_of` if possible.
fn try_builtin_sized(ty: &syn::Type) -> Option<TypeSize> {
    match ty {
        syn::Type::Group(group) => try_builtin_sized(&group.elem),
        syn::Type::Array(arr) => {
            let elem_size = try_builtin_sized(arr.elem.as_ref())?;
            let elem_count = TypeSize::from_expr(&arr.len);
            Some(elem_size * elem_count)
        }
        syn::Type::Tuple(tup) => tup
            .elems
            .iter()
            .map(try_builtin_sized)
            .fold(Some(TypeSize::Constant(0)), |acc, item| Some(acc? + item?)),
        syn::Type::Path(p) => {
            if p.qself.is_some() {
                return None
            }

            if !is_sized_primitive(p.path.get_ident()?.to_string().as_str()) {
                return None
            }

            Some(TypeSize::Computed(quote! {
                fuel_types::canonical::aligned_size(::core::mem::size_of::<#p>())
            }))
        }
        _ => {
            panic!("Unsized type {:?}", ty);
        }
    }
}

fn constsize_fields(fields: &syn::Fields) -> (TokenStream2, TokenStream2) {
    let mut sizes: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let type_ = &field.ty;
            if should_skip_field(&field.attrs) {
                None
            } else if let Some(size_code) = try_builtin_sized(type_) {
                Some(size_code)
            } else {
                Some(TypeSize::Computed(quote! {
                    <#type_>::SIZE_STATIC
                }))
            }
        })
        .collect();
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let static_size: TokenStream2 = sizes.into_iter().fold(quote! { 0 }, |acc, item| {
        quote! {
            #acc + #item
        }
    });

    let no_dynamic_size: TokenStream2 = fields
        .iter()
        .filter_map(|field| {
            let type_ = &field.ty;
            if should_skip_field(&field.attrs) || try_builtin_sized(type_).is_some() {
                return None
            }
            Some(quote! {
                <#type_>::SIZE_NO_DYNAMIC
            })
        })
        .fold(quote! { true }, |acc, item| {
            quote! {
                #acc && #item
            }
        });

    (static_size, no_dynamic_size)
}

/// Derives `Serialize` trait for the given `struct` or `enum`.
pub fn serialize_derive(mut s: synstructure::Structure) -> TokenStream2 {
    s.add_bounds(synstructure::AddBounds::Fields)
        .underscore_const(true);

    let serialize = match s.ast().data {
        syn::Data::Struct(_) => serialize_struct(&s),
        syn::Data::Enum(_) => serialize_enum(&s),
        _ => panic!("Can't derive `Serialize` for `union`s"),
    };

    // crate::utils::write_and_fmt(
    //     format!("tts/{}.rs", s.ast().ident),
    //     quote::quote!(#serialize),
    // )
    // .expect("unable to save or format");

    quote! { #serialize }
}
