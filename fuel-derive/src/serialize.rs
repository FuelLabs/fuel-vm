use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

fn serialize_struct(s: &synstructure::Structure) -> TokenStream2 {
    assert_eq!(s.variants().len(), 1, "structs must have one variant");

    let variant: &synstructure::VariantInfo = &s.variants()[0];
    let encode_static = variant.each(|binding| {
        quote! {
            if fuel_types::canonical::Serialize::size(#binding) % fuel_types::canonical::ALIGN > 0 {
                return ::core::result::Result::Err(fuel_types::canonical::Error::WrongAlign)
            }
            fuel_types::canonical::Serialize::encode_static(#binding, buffer)?;
        }
    });

    let encode_dynamic = variant.each(|binding| {
        quote! {
            fuel_types::canonical::Serialize::encode_dynamic(#binding, buffer)?;
        }
    });

    s.gen_impl(quote! {
        gen impl fuel_types::canonical::Serialize for @Self {
            #[inline(always)]
            fn encode_static<O: fuel_types::canonical::Output + ?Sized>(&self, buffer: &mut O) -> ::core::result::Result<(), fuel_types::canonical::Error> {
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
        }
    })
}

fn serialize_enum(s: &synstructure::Structure) -> TokenStream2 {
    assert!(!s.variants().is_empty(), "got invalid empty enum");
    let encode_static = s.variants().iter().enumerate().map(|(i, v)| {
        let pat = v.pat();
        let index = i as u64;
        let encode_static_iter = v.bindings().iter().map(|binding| {
            quote! {
                if fuel_types::canonical::Serialize::size(#binding) % fuel_types::canonical::ALIGN > 0 {
                    return ::core::result::Result::Err(fuel_types::canonical::Error::WrongAlign)
                }
                fuel_types::canonical::Serialize::encode_static(#binding, buffer)?;
            }
        });
        quote! {
            #pat => {
                { <::core::primitive::u64 as fuel_types::canonical::Serialize>::encode(&#index, buffer)?; }
                #(
                    { #encode_static_iter }
                )*
            }
        }
    });
    let encode_dynamic = s.variants().iter().map(|v| {
        let encode_dynamic_iter = v.each(|binding| {
            quote! {
                fuel_types::canonical::Serialize::encode_dynamic(#binding, buffer)?;
            }
        });
        quote! {
            #encode_dynamic_iter
        }
    });
    s.gen_impl(quote! {
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
        }
    })
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
