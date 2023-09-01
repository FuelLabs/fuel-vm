use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use core::cmp::Ordering;

use crate::{
    attribute::{
        parse_enum_repr,
        should_skip_field,
        should_skip_field_binding,
        EnumAttrs,
        StructAttrs,
    },
    evaluate::evaluate_simple_expr,
};

fn serialize_struct(s: &synstructure::Structure) -> TokenStream2 {
    let attrs = StructAttrs::parse(s);

    assert_eq!(s.variants().len(), 1, "structs must have one variant");

    let variant: &synstructure::VariantInfo = &s.variants()[0];
    let encode_static = variant.each(|binding| {
        if should_skip_field_binding(binding) {
            quote! {}
        } else {
            quote! {
                ::fuel_types::canonical::Serialize::encode_static(#binding, buffer)?;
            }
        }
    });

    let encode_dynamic = variant.each(|binding| {
        if should_skip_field_binding(binding) {
            quote! {}
        } else {
            quote! {
                ::fuel_types::canonical::Serialize::encode_dynamic(#binding, buffer)?;
            }
        }
    });

    let size_dynamic_code = variant.each(|binding| {
        if should_skip_field_binding(binding) {
            quote! {}
        } else {
            quote! {
                size = ::fuel_types::canonical::add_sizes(size, #binding.size_dynamic());
            }
        }
    });
    let size_dynamic_code =
        quote! { let mut size = 0; match self { #size_dynamic_code}; size };

    let prefix = if let Some(prefix_type) = attrs.prefix.as_ref() {
        quote! {
            let prefix: u64 = #prefix_type.into();
            <u64 as ::fuel_types::canonical::Serialize>::encode(&prefix, buffer)?;
        }
    } else {
        quote! {}
    };

    let (mut size_static, size_no_dynamic) = constsize_fields(variant.ast().fields);
    if attrs.prefix.is_some() {
        size_static = quote! { ::fuel_types::canonical::add_sizes(#size_static, 8) };
    };

    s.gen_impl(quote! {
        gen impl ::fuel_types::canonical::Serialize for @Self {
            #[inline(always)]
            fn encode_static<O: ::fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), ::fuel_types::canonical::Error> {
                #prefix
                match self {
                    #encode_static
                };

                ::core::result::Result::Ok(())
            }

            fn encode_dynamic<O: ::fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), ::fuel_types::canonical::Error> {
                match self {
                    #encode_dynamic
                };

                ::core::result::Result::Ok(())
            }

            #[inline(always)]
            fn size_dynamic(&self) -> usize {
                #size_dynamic_code
            }

            const SIZE_NO_DYNAMIC: bool = #size_no_dynamic;
        }

        gen impl ::fuel_types::canonical::SerializedSizeFixed for @Self {
            const SIZE_STATIC: usize = #size_static;
        }
    })
}

fn serialize_enum(s: &synstructure::Structure) -> TokenStream2 {
    let repr = parse_enum_repr(&s.ast().attrs);
    let attrs = EnumAttrs::parse(s);

    assert!(!s.variants().is_empty(), "got invalid empty enum");
    let mut next_discriminant = 0u64;
    let encode_static = s.variants().iter().map(|v| {
        let pat = v.pat();
        let encode_static_iter = v.bindings().iter().map(|binding| {
            if should_skip_field_binding(binding) {
                quote! {}
            } else {
                quote! {
                    ::fuel_types::canonical::Serialize::encode_static(#binding, buffer)?;
                }
            }
        });

        // Handle #[canonical(inner_discriminant = Type)]
        let discr = if let Some(discr_type) = attrs.inner_discriminant.as_ref() {
            if v.ast().discriminant.is_some() {
                panic!("User-specified discriminants are not supported with #[canonical(discriminant = Type)]")
            }
            quote! { {
                #discr_type::from(self).into()
            } }
        } else {
            if let Some((_, d)) = v.ast().discriminant {
                next_discriminant = evaluate_simple_expr(d).expect("Unable to evaluate discriminant expression");
            };
            let v = next_discriminant;
            next_discriminant = next_discriminant.checked_add(1).expect("Discriminant overflow");
            quote! { #v }
        };

        let encode_discriminant = if attrs.inner_discriminant.is_some() {
            quote! {}
        } else {
            quote! {
                <::core::primitive::u64 as ::fuel_types::canonical::Serialize>::encode(&#discr, buffer)?;
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
                    ::fuel_types::canonical::Serialize::encode_dynamic(#binding, buffer)?;
                }
            }
        });
        quote! {
            #encode_dynamic_iter
        }
    });

    // Handle #[canonical(serialize_with = function)]
    let data_helper = attrs.serialize_with.as_ref();
    let size_helper_static = attrs.serialized_size_static_with.as_ref();
    let size_helper_dynamic = attrs.serialized_size_dynamic_with.as_ref();
    if let (Some(data_helper), Some(size_helper_static), Some(size_helper_dynamic)) =
        (data_helper, size_helper_static, size_helper_dynamic)
    {
        let size_no_dynamic = attrs
            .SIZE_NO_DYNAMIC
            .expect("serialize_with requires SIZE_NO_DYNAMIC key");

        return s.gen_impl(quote! {
            gen impl ::fuel_types::canonical::Serialize for @Self {
                const SIZE_NO_DYNAMIC: bool = #size_no_dynamic;

                #[inline(always)]
                fn encode_static<O: ::fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), ::fuel_types::canonical::Error> {
                    #data_helper(self, buffer)
                }

                fn encode_dynamic<O: ::fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), ::fuel_types::canonical::Error> {
                    ::core::result::Result::Ok(())
                }

                fn size_dynamic(&self) -> usize {
                    #size_helper_dynamic(self)
                }
            }

            gen impl ::fuel_types::canonical::SerializedSize for @Self {
                #[inline(always)]
                fn size_static(&self) -> usize {
                    #size_helper_static(self)
                }
            }
        })
    } else if ![
        data_helper.is_none(),
        size_helper_static.is_none(),
        size_helper_dynamic.is_none(),
    ]
    .into_iter()
    .all(|v| v)
    {
        panic!("serialize_with, serialized_size_static_with and serialized_size_dynamic_with must be used together");
    }

    let (variant_size_static, size_no_dynamic): (Vec<TokenStream2>, Vec<TokenStream2>) =
        s.variants()
            .iter()
            .map(|v| constsize_fields(v.ast().fields))
            .unzip();

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
            gen impl ::fuel_types::canonical::SerializedSizeFixed for @Self {
                const SIZE_STATIC: usize = #repr_size;
            }
        })
    } else {
        let match_size_static: TokenStream2 = s
            .variants()
            .iter()
            .zip(variant_size_static)
            .map(|(variant, static_size_code)| {
                let p = variant.pat();
                quote! { #p => #static_size_code, }
            })
            .collect();
        let match_size_static = quote! { match self { #match_size_static } };
        let discr_size = if attrs.inner_discriminant.is_some() {
            quote! { 0usize }
        } else {
            quote! { 8usize }
        };
        s.gen_impl(quote! {
            gen impl ::fuel_types::canonical::SerializedSize for @Self {
                fn size_static(&self) -> usize {
                    ::fuel_types::canonical::add_sizes(#discr_size, #match_size_static)
                }
            }
        })
    };

    let match_size_dynamic: TokenStream2 = s
        .variants()
        .iter()
        .map(|variant| {
            variant.each(|binding| {
                if should_skip_field_binding(binding) {
                    quote! {}
                } else {
                    quote! {
                        size = ::fuel_types::canonical::add_sizes(size, #binding.size_dynamic());
                    }
                }
            })
        })
        .collect();
    let match_size_dynamic =
        quote! {{ let mut size = 0; match self { #match_size_dynamic } size }};

    let impl_code = s.gen_impl(quote! {
        gen impl ::fuel_types::canonical::Serialize for @Self {
            #[inline(always)]
            fn encode_static<O: ::fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), ::fuel_types::canonical::Error> {
                match self {
                    #(
                        #encode_static
                    )*,
                    _ => return ::core::result::Result::Err(::fuel_types::canonical::Error::UnknownDiscriminant),
                };

                ::core::result::Result::Ok(())
            }

            fn encode_dynamic<O: ::fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), ::fuel_types::canonical::Error> {
                match self {
                    #(
                        #encode_dynamic
                    )*,
                    _ => return ::core::result::Result::Err(::fuel_types::canonical::Error::UnknownDiscriminant),
                };

                ::core::result::Result::Ok(())
            }

            #[inline(always)]
            fn size_dynamic(&self) -> usize {
                #match_size_dynamic
            }

            const SIZE_NO_DYNAMIC: bool = #size_no_dynamic;
        }
    });

    quote! {
        #impl_code
        #impl_size
    }
}

fn is_sized_primitive(type_name: &str) -> bool {
    // This list contains types that are known to be primitive types,
    // including common aliases of them. These types cannot be resolved
    // further, and their size is known at compile time.
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
        if let syn::Expr::Lit(lit) = expr {
            if let syn::Lit::Int(ref int) = lit.lit {
                if let Ok(value) = int.base10_parse::<usize>() {
                    return Self::Constant(value)
                }
            }
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
            (Self::Constant(a), Self::Constant(b)) => Self::Constant(
                a.checked_add(b)
                    .expect("Size would overflow on compile time"),
            ),
            (Self::Computed(a), Self::Computed(b)) => Self::Computed(quote! {
                ::fuel_types::canonical::add_sizes(#a, #b)
            }),
            (Self::Constant(a), Self::Computed(b)) => Self::Computed(quote! {
                ::fuel_types::canonical::add_sizes(#a, #b)
            }),
            (Self::Computed(a), Self::Constant(b)) => Self::Computed(quote! {
                ::fuel_types::canonical::add_sizes(#a, #b)
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
fn try_builtin_sized(ty: &syn::Type, align: bool) -> Option<TypeSize> {
    match ty {
        syn::Type::Group(group) => try_builtin_sized(&group.elem, align),
        syn::Type::Array(arr) => {
            let elem_size = try_builtin_sized(arr.elem.as_ref(), false)?;
            let elem_count = TypeSize::from_expr(&arr.len);
            let unpadded_size = elem_size * elem_count;
            Some(TypeSize::Computed(
                quote! { ::fuel_types::canonical::aligned_size(#unpadded_size) },
            ))
        }
        syn::Type::Tuple(tup) => tup
            .elems
            .iter()
            .map(|type_| try_builtin_sized(type_, true))
            .try_fold(TypeSize::Constant(0), |acc, item| Some(acc + item?)),
        syn::Type::Path(p) => {
            if p.qself.is_some() {
                return None
            }

            if !is_sized_primitive(p.path.get_ident()?.to_string().as_str()) {
                return None
            }

            Some(TypeSize::Computed(if align {
                quote! { <#p as ::fuel_types::canonical::SerializedSizeFixed>::SIZE_STATIC }
            } else {
                quote! {
                    if <#p as ::fuel_types::canonical::Serialize>::UNALIGNED_BYTES {
                        1
                    } else {
                        <#p as ::fuel_types::canonical::SerializedSizeFixed>::SIZE_STATIC
                    }
                }
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
            } else if let Some(size_code) = try_builtin_sized(type_, true) {
                Some(TypeSize::Computed(quote! { #size_code }))
            } else {
                Some(TypeSize::Computed(quote! { <#type_>::SIZE_STATIC }))
            }
        })
        .collect();
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let static_size: TokenStream2 = sizes.into_iter().fold(quote! { 0 }, |acc, item| {
        quote! {
            ::fuel_types::canonical::add_sizes(#acc, #item)
        }
    });

    let no_dynamic_size: TokenStream2 = fields
        .iter()
        .filter_map(|field| {
            let type_ = &field.ty;
            if should_skip_field(&field.attrs)
                || try_builtin_sized(type_, false).is_some()
            {
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

    match s.ast().data {
        syn::Data::Struct(_) => serialize_struct(&s),
        syn::Data::Enum(_) => serialize_enum(&s),
        _ => panic!("Can't derive `Serialize` for `union`s"),
    }
}
