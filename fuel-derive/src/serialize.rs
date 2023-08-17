use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::attribute::{
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
            let f = &binding.ast().ident;
            quote! {
                println!("Serializing field: {}", stringify!(#f));
                if fuel_types::canonical::Serialize::size(#binding) % fuel_types::canonical::ALIGN > 0 {
                    return ::core::result::Result::Err(fuel_types::canonical::Error::WrongAlign)
                }
                fuel_types::canonical::Serialize::encode_static(#binding, buffer)?;
                let mut tmp = Vec::new();
                fuel_types::canonical::Serialize::encode_static(#binding, &mut tmp).unwrap();
                println!("Serialized  sta => {:?}", tmp);
            }
        }
    });

    let encode_dynamic = variant.each(|binding| {
        if should_skip_field_binding(binding) {
            quote! {}
        } else {
            let f = &binding.ast().ident;
            quote! {
                println!("Serializing field: {}", stringify!(#f));
                fuel_types::canonical::Serialize::encode_dynamic(#binding, buffer)?;
                let mut tmp = Vec::new();
                fuel_types::canonical::Serialize::encode_dynamic(#binding, &mut tmp).unwrap();
                println!("Serialized  dyn => {:?}", tmp);
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
        }
    })
}

// TODO: somehow ensure that all enum variants have equal size, or zero-pad them
fn serialize_enum(s: &synstructure::Structure) -> TokenStream2 {
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
        return s.gen_impl(quote! {
            gen impl fuel_types::canonical::Serialize for @Self {
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
    let x = match s.ast().data {
        syn::Data::Struct(_) => serialize_struct(&s),
        syn::Data::Enum(_) => serialize_enum(&s),
        _ => panic!("Can't derive `Serialize` for `union`s"),
    };

    // crate::utils::write_and_fmt(format!("tts/{}.rs", s.ast().ident), quote::quote!(#x))
    //     .expect("unable to save or format");

    x
}
