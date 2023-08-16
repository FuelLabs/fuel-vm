use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::attribute::{
    should_skip_field,
    should_skip_field_binding,
    EnumAttrs,
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
            quote! {
                <#ty as fuel_types::canonical::Deserialize>::decode_static(buffer)?
            }
        }
    });

    let decode_dynamic = variant.each(|binding| {
        if should_skip_field_binding(binding) {
            quote! {
                *#binding = ::core::default::Default::default();
            }
        } else {
            quote! {
                fuel_types::canonical::Deserialize::decode_dynamic(#binding, buffer)?;
            }
        }
    });

    s.gen_impl(quote! {
        gen impl fuel_types::canonical::Deserialize for @Self {
            fn decode_static<I: fuel_types::canonical::Input + ?Sized>(buffer: &mut I) -> ::core::result::Result<Self, fuel_types::canonical::Error> {
                ::core::result::Result::Ok(#decode_main)
            }

            fn decode_dynamic<I: fuel_types::canonical::Input + ?Sized>(&mut self, buffer: &mut I) -> ::core::result::Result<(), fuel_types::canonical::Error> {
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

    assert!(!s.variants().is_empty(), "got invalid empty enum");
    let decode_static = s
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
                    quote! {
                        <#ty as fuel_types::canonical::Deserialize>::decode_static(buffer)?
                    }
                }
            });

            quote! {
                {
                    ::core::result::Result::Ok(#decode_main)
                }
            }
        })
        .enumerate()
        .fold(quote! {}, |acc, (i, v)| {
            let index = i as u64;
            quote! {
                #acc
                #index => #v,
            }
        });

    let decode_dynamic = s.variants().iter().map(|variant| {
        let decode_dynamic = variant.each(|binding| {
            if should_skip_field_binding(binding) {
                quote! {
                    *#binding = ::core::default::Default::default();
                }
            } else {
                quote! {
                    fuel_types::canonical::Deserialize::decode_dynamic(#binding, buffer)?;
                }
            }
        });

        quote! {
            #decode_dynamic
        }
    });

    // Handle #[canonical(discriminant = Type)]
    let mapped_discr = if let Some(discr_type) = attrs.0.get("discriminant") {
        quote! { {
            use ::num_enum::{TryFromPrimitive, IntoPrimitive};
            let Ok(discr) = #discr_type::try_from_primitive(raw_discr) else {
                return ::core::result::Result::Err(fuel_types::canonical::Error::UnknownDiscriminant)
            };
            discr.into()
        } }
    } else {
        quote! { raw_discr }
    };

    // Handle #[canonical(deserialize_with = function)]
    let match_static = if let Some(helper) = attrs.0.get("deserialize_with") {
        quote! { {
            buffer.skip(8)?;
            #helper(discr, buffer)
        } }
    } else {
        quote! {
            match discr {
                #decode_static
                _ => ::core::result::Result::Err(fuel_types::canonical::Error::UnknownDiscriminant),
            }
        }
    };

    s.gen_impl(quote! {
        gen impl fuel_types::canonical::Deserialize for @Self {
            fn decode_static<I: fuel_types::canonical::Input + ?Sized>(buffer: &mut I) -> ::core::result::Result<Self, fuel_types::canonical::Error> {
                let raw_discr = <::core::primitive::u64 as fuel_types::canonical::Deserialize>::decode(buffer)?;
                let discr = #mapped_discr;
                #match_static
            }

            fn decode_dynamic<I: fuel_types::canonical::Input + ?Sized>(&mut self, buffer: &mut I) -> ::core::result::Result<(), fuel_types::canonical::Error> {
                match self {
                    #(
                        #decode_dynamic
                    )*
                    _ => return ::core::result::Result::Err(fuel_types::canonical::Error::UnknownDiscriminant),
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
