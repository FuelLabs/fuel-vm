use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::{
    attribute::{
        should_skip_field,
        should_skip_field_binding,
        TypedefAttrs,
    },
    evaluate::evaluate_simple_expr,
};

fn deserialize_struct(s: &mut synstructure::Structure) -> TokenStream2 {
    assert_eq!(s.variants().len(), 1, "structs must have one variant");
    let name = &s.ast().ident;

    let variant: &synstructure::VariantInfo = &s.variants()[0];
    let decode_main = variant.construct(|field, _| {
        let ty = &field.ty;
        if should_skip_field(&field.attrs) {
            quote! {
                ::core::default::Default::default()
            }
        } else {
            quote! {{
                println!("decode_static {:?}", stringify!(#ty));
                buffer.debug_peek();
                <#ty as fuel_types::canonical::Deserialize>::decode_static(buffer)?
            }}
        }
    });

    let decode_dynamic = variant.each(|binding| {
        if should_skip_field_binding(binding) {
            quote! {
                *#binding = ::core::default::Default::default();
            }
        } else {
            let ty = &binding.ast().ty;
            quote! {{
                println!("decode_dynamic {:?}", stringify!(#ty));
                buffer.debug_peek();
                fuel_types::canonical::Deserialize::decode_dynamic(#binding, buffer)?;
            }}
        }
    });

    let remove_prefix = if TypedefAttrs::parse(s).0.contains_key("prefix") {
        quote! {
            <u64 as fuel_types::canonical::Deserialize>::decode_static(buffer)?;
        }
    } else {
        quote! {}
    };

    s.gen_impl(quote! {
        gen impl fuel_types::canonical::Deserialize for @Self {
            fn decode_static<I: fuel_types::canonical::Input + ?Sized>(buffer: &mut I) -> ::core::result::Result<Self, fuel_types::canonical::Error> {
                println!("decode_static of struct {}", stringify!(#name));
                #remove_prefix
                ::core::result::Result::Ok(#decode_main)
            }

            fn decode_dynamic<I: fuel_types::canonical::Input + ?Sized>(&mut self, buffer: &mut I) -> ::core::result::Result<(), fuel_types::canonical::Error> {
                println!("decode_dynamic of struct {}", stringify!(#name));
                match self {
                    #decode_dynamic,
                };
                ::core::result::Result::Ok(())
            }
        }
    })
}

fn deserialize_enum(s: &synstructure::Structure) -> TokenStream2 {
    let attrs = TypedefAttrs::parse(s);
    let name = &s.ast().ident;

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
                        println!("decode_static {:?}", stringify!(#ty));
                        buffer.debug_peek();
                        <#ty as fuel_types::canonical::Deserialize>::decode_static(buffer)?
                    }}
                }
            });


            let discr = if let Some(discr_type) = attrs
                .0
                .get("discriminant")
                .or(attrs.0.get("inner_discriminant")) {
                let vname = variant.ast().ident;
                quote! { #discr_type::#vname }
            } else {
                if let Some((_, d)) = variant.ast().discriminant {
                    next_discriminant = evaluate_simple_expr(&d).expect("Unable to evaluate discriminant expression");
                };
                let v = next_discriminant;
                next_discriminant += 1;
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
                let ty = &binding.ast().ty;
                quote! {{
                    println!("decode_dynamic {:?}", stringify!(#ty));
                    buffer.debug_peek();
                    fuel_types::canonical::Deserialize::decode_dynamic(#binding, buffer)?;
                }}
            }
        });

        quote! {
            #decode_dynamic
        }
    });

    // Handle #[canonical(inner_discriminant = Type)]
    let decode_discriminant = if attrs.0.contains_key("inner_discriminant") {
        quote! {
            let rr = buffer.remaining();
            let buf = buffer.clone();
            let raw_discr = <::core::primitive::u64 as fuel_types::canonical::Deserialize>::decode(buffer)?;
            println!("Decoded inner discriminant {:?}", raw_discr);
            *buffer = buf; // Restore buffer position
            assert_eq!(rr, buffer.remaining(), "inner_discriminant must not consume any bytes");
            dbg!(buffer.remaining());
        }
    } else {
        quote! {
            let raw_discr = <::core::primitive::u64 as fuel_types::canonical::Deserialize>::decode(buffer)?;
        }
    };

    // Handle #[canonical(discriminant = Type)]
    let mapped_discr = if let Some(discr_type) = attrs
        .0
        .get("discriminant")
        .or(attrs.0.get("inner_discriminant"))
    {
        quote! { {
            use ::num_enum::{TryFromPrimitive, IntoPrimitive};
            let Ok(discr) = #discr_type::try_from_primitive(raw_discr) else {
                return ::core::result::Result::Err(fuel_types::canonical::Error::UnknownDiscriminant)
            };
            println!("Converted discr into {discr:?}");
            discr
        } }
    } else {
        quote! { raw_discr }
    };

    // Handle #[canonical(deserialize_with = function)]
    if let Some(helper) = attrs.0.get("deserialize_with") {
        return s.gen_impl(quote! {
            gen impl fuel_types::canonical::Deserialize for @Self {
                fn decode_static<I: fuel_types::canonical::Input + ?Sized>(buffer: &mut I) -> ::core::result::Result<Self, fuel_types::canonical::Error> {
                    let raw_discr = <::core::primitive::u64 as fuel_types::canonical::Deserialize>::decode(buffer)?;
                    #helper(#mapped_discr, buffer)
                }

                fn decode_dynamic<I: fuel_types::canonical::Input + ?Sized>(&mut self, buffer: &mut I) -> ::core::result::Result<(), fuel_types::canonical::Error> {
                    ::core::result::Result::Ok(())
                }
            }
        })
    }

    s.gen_impl(quote! {
        gen impl fuel_types::canonical::Deserialize for @Self {
            fn decode_static<I: fuel_types::canonical::Input + ?Sized>(buffer: &mut I) -> ::core::result::Result<Self, fuel_types::canonical::Error> {
                println!("decode_static of enum {}", stringify!(#name));
                #decode_discriminant
                match #mapped_discr {
                    #decode_static
                    _ => ::core::result::Result::Err(fuel_types::canonical::Error::UnknownDiscriminant),
                }
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

    let deser = match s.ast().data {
        syn::Data::Struct(_) => deserialize_struct(&mut s),
        syn::Data::Enum(_) => deserialize_enum(&s),
        _ => panic!("Can't derive `Deserialize` for `union`s"),
    };

    crate::utils::write_and_fmt(
        format!("tts/{}_deser.rs", s.ast().ident),
        quote::quote!(#deser),
    )
    .expect("Unable to write generated code to file");

    deser
}
