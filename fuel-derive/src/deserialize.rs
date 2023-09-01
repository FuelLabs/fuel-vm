use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::{
    attribute::{
        should_skip_field,
        should_skip_field_binding,
        EnumAttrs,
        StructAttrs,
    },
    evaluate::evaluate_simple_expr,
};

fn deserialize_struct(s: &mut synstructure::Structure) -> TokenStream2 {
    assert_eq!(s.variants().len(), 1, "structs must have one variant");

    let variant: &synstructure::VariantInfo = &s.variants()[0];
    let decode_main = variant.construct(|field, _| {
        let ty = &field.ty;
        if should_skip_field(&field.attrs) {
            quote! {
                ::core::default::Default::default()
            }
        } else {
            quote! {{
                <#ty as ::fuel_types::canonical::Deserialize>::decode_static(buffer)?
            }}
        }
    });

    let decode_dynamic = variant.each(|binding| {
        if should_skip_field_binding(binding) {
            quote! {
                *#binding = ::core::default::Default::default();
            }
        } else {
            quote! {{
                ::fuel_types::canonical::Deserialize::decode_dynamic(#binding, buffer)?;
            }}
        }
    });

    let remove_prefix = if let Some(expected_prefix) = StructAttrs::parse(s).prefix {
        quote! {{
            let prefix = <u64 as ::fuel_types::canonical::Deserialize>::decode_static(buffer)?;
            if prefix.try_into() != Ok(#expected_prefix) {
                return ::core::result::Result::Err(::fuel_types::canonical::Error::InvalidPrefix)
            }
        }}
    } else {
        quote! {}
    };

    s.gen_impl(quote! {
        gen impl ::fuel_types::canonical::Deserialize for @Self {
            fn decode_static<I: ::fuel_types::canonical::Input + ?Sized>(buffer: &mut I) -> ::core::result::Result<Self, ::fuel_types::canonical::Error> {
                #remove_prefix
                ::core::result::Result::Ok(#decode_main)
            }

            fn decode_dynamic<I: ::fuel_types::canonical::Input + ?Sized>(&mut self, buffer: &mut I) -> ::core::result::Result<(), ::fuel_types::canonical::Error> {
                match self {
                    #decode_dynamic,
                };
                ::core::result::Result::Ok(())
            }
        }
    })
}

fn deserialize_enum(s: &synstructure::Structure) -> TokenStream2 {
    let attrs = EnumAttrs::parse(s);
    let _name = &s.ast().ident;

    assert!(!s.variants().is_empty(), "got invalid empty enum");

    let mut next_discriminant = 0u64;
    let decode_static: TokenStream2 = s
        .variants()
        .iter()
        .map(|variant| {
            let decode_main = variant.construct(|field, _| {
                if should_skip_field(&field.attrs) {
                    quote! {
                        ::core::default::Default::default()
                    }
                } else {
                    let ty = &field.ty;
                    quote! {{
                        <#ty as ::fuel_types::canonical::Deserialize>::decode_static(buffer)?
                    }}
                }
            });


            let discr = if let Some(discr_type) = attrs.inner_discriminant.as_ref() {
                let vname = variant.ast().ident;
                quote! { #discr_type::#vname }
            } else {
                if let Some((_, d)) = variant.ast().discriminant {
                    next_discriminant = evaluate_simple_expr(d).expect("Unable to evaluate discriminant expression");
                };
                let v = next_discriminant;
                next_discriminant = next_discriminant.checked_add(1).expect("Discriminant overflow");
                quote! { #v }
            };

            quote! {
                #discr => {
                    ::core::result::Result::Ok(#decode_main)
                }
            }
        }).collect();

    let decode_dynamic = s.variants().iter().map(|variant| {
        let decode_dynamic = variant.each(|binding| {
            if should_skip_field_binding(binding) {
                quote! {
                    *#binding = ::core::default::Default::default();
                }
            } else {
                quote! {{
                    ::fuel_types::canonical::Deserialize::decode_dynamic(#binding, buffer)?;
                }}
            }
        });

        quote! {
            #decode_dynamic
        }
    });

    // Handle #[canonical(inner_discriminant = Type)]
    let decode_discriminant = if attrs.inner_discriminant.is_some() {
        quote! {
            let buf = buffer.clone();
            let raw_discr = <::core::primitive::u64 as ::fuel_types::canonical::Deserialize>::decode(buffer)?;
            *buffer = buf; // Restore buffer position
        }
    } else {
        quote! {
            let raw_discr = <::core::primitive::u64 as ::fuel_types::canonical::Deserialize>::decode(buffer)?;
        }
    };

    // Handle #[canonical(inner_discriminant = Type)]
    let mapped_discr = if let Some(discr_type) = attrs.inner_discriminant {
        quote! { {
            use ::num_enum::{TryFromPrimitive, IntoPrimitive};
            let Ok(discr) = #discr_type::try_from_primitive(raw_discr) else {
                return ::core::result::Result::Err(::fuel_types::canonical::Error::UnknownDiscriminant)
            };
            discr
        } }
    } else {
        quote! { raw_discr }
    };

    // Handle #[canonical(deserialize_with = function)]
    if let Some(helper) = attrs.deserialize_with {
        return s.gen_impl(quote! {
            gen impl ::fuel_types::canonical::Deserialize for @Self {
                fn decode_static<I: ::fuel_types::canonical::Input + ?Sized>(buffer: &mut I) -> ::core::result::Result<Self, ::fuel_types::canonical::Error> {
                    let raw_discr = <::core::primitive::u64 as ::fuel_types::canonical::Deserialize>::decode(buffer)?;
                    #helper(#mapped_discr, buffer)
                }

                fn decode_dynamic<I: ::fuel_types::canonical::Input + ?Sized>(&mut self, buffer: &mut I) -> ::core::result::Result<(), ::fuel_types::canonical::Error> {
                    ::core::result::Result::Ok(())
                }
            }
        })
    }

    s.gen_impl(quote! {
        gen impl ::fuel_types::canonical::Deserialize for @Self {
            fn decode_static<I: ::fuel_types::canonical::Input + ?Sized>(buffer: &mut I) -> ::core::result::Result<Self, ::fuel_types::canonical::Error> {
                #decode_discriminant
                match #mapped_discr {
                    #decode_static
                    _ => ::core::result::Result::Err(::fuel_types::canonical::Error::UnknownDiscriminant),
                }
            }

            fn decode_dynamic<I: ::fuel_types::canonical::Input + ?Sized>(&mut self, buffer: &mut I) -> ::core::result::Result<(), ::fuel_types::canonical::Error> {
                match self {
                    #(
                        #decode_dynamic
                    )*
                    _ => return ::core::result::Result::Err(::fuel_types::canonical::Error::UnknownDiscriminant),
                };

                ::core::result::Result::Ok(())
            }
        }
    })
}

/// Derives `Deserialize` trait for the given `struct` or `enum`.
pub fn deserialize_derive(mut s: synstructure::Structure) -> TokenStream2 {
    s.bind_with(|_| synstructure::BindStyle::RefMut)
        .add_bounds(synstructure::AddBounds::Fields)
        .underscore_const(true);

    match s.ast().data {
        syn::Data::Struct(_) => deserialize_struct(&mut s),
        syn::Data::Enum(_) => deserialize_enum(&s),
        _ => panic!("Can't derive `Deserialize` for `union`s"),
    }
}
