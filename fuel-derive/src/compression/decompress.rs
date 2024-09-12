use proc_macro2::TokenStream as TokenStream2;
use quote::{
    format_ident,
    quote,
};

use crate::helpers::where_clause_push;

use super::attribute::{
    FieldAttrs,
    StructureAttrs,
};

/// Generate a match arm for each variant of the compressed structure
/// using the given function to generate the pattern body.
fn each_variant_compressed<F: FnMut(&synstructure::VariantInfo<'_>) -> TokenStream2>(
    s: &synstructure::Structure,
    compressed_name: &TokenStream2,
    mut f: F,
) -> TokenStream2 {
    s.variants()
        .iter()
        .map(|variant| {
            // Modify the binding pattern to match the compressed variant
            let mut v2 = variant.clone();
            v2.filter(|field| {
                let attrs = FieldAttrs::parse(&field.ast().attrs);
                !matches!(attrs, FieldAttrs::Skip)
            });
            v2.bindings_mut().iter_mut().for_each(|binding| {
                binding.style = synstructure::BindStyle::Move;
            });
            let mut p = v2.pat().into_iter();
            let _ = p.next().expect("pattern always begins with an identifier");
            let p = quote! { #compressed_name #(#p)* };

            let decompressed = f(variant);
            quote! {
                #p => { #decompressed }
            }
        })
        .collect()
}

/// Construct original version of the struct from the compressed one
fn construct_decompress(
    // The original structure to construct, i.e. struct name or enum variant path
    original: &TokenStream2,
    variant: &synstructure::VariantInfo<'_>,
) -> TokenStream2 {
    let bound_fields: TokenStream2 = variant
        .bindings()
        .iter()
        .map(|binding| {
            let ty = &binding.ast().ty;
            let cname = format_ident!("{}_c", binding.binding);

            match FieldAttrs::parse(&binding.ast().attrs) {
                FieldAttrs::Skip => quote! {
                    let #cname = ::core::default::Default::default();
                },
                FieldAttrs::Normal => {
                    quote! {
                        let #cname = <#ty as ::fuel_compression::DecompressibleBy<_>>::decompress_with(#binding, ctx).await?;
                    }
                }
            }
        })
        .collect();

    let construct_fields: TokenStream2 = variant
        .bindings()
        .iter()
        .map(|binding| {
            let cname = format_ident!("{}_c", binding.binding);
            if let Some(fname) = &binding.ast().ident {
                quote! { #fname: #cname, }
            } else {
                quote! { #cname, }
            }
        })
        .collect();

    let construct_fields = match variant.ast().fields {
        syn::Fields::Named(_) => quote! {{ #construct_fields }},
        syn::Fields::Unnamed(_) => quote! {(#construct_fields)},
        syn::Fields::Unit => quote! {},
    };

    quote! {
        #bound_fields
        #original #construct_fields
    }
}

/// Derives `DecompressibleBy` trait for the given `struct` or `enum`.
pub fn derive(mut s: synstructure::Structure) -> TokenStream2 {
    s.add_bounds(synstructure::AddBounds::None)
        .underscore_const(true);

    let s_attrs = match StructureAttrs::parse(&s.ast().attrs) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let name = &s.ast().ident;
    let compressed_name = format_ident!("Compressed{}", name);

    let mut g = s.ast().generics.clone();
    let w_structure = g.where_clause.take();
    let w_impl = w_structure.clone();
    for item in &s_attrs {
        match item {
            StructureAttrs::Discard(discard) => {
                g.params = g
                    .params
                    .into_pairs()
                    .filter(|pair| match pair.value() {
                        syn::GenericParam::Type(t) => {
                            !discard.contains(&t.ident.to_string())
                        }
                        _ => true,
                    })
                    .collect();
            }
        }
    }

    let mut w_impl_field_bounds_decompress = w_impl.clone();
    for variant in s.variants() {
        for field in variant.ast().fields.iter() {
            let ty = &field.ty;
            match FieldAttrs::parse(&field.attrs) {
                FieldAttrs::Skip => {}
                FieldAttrs::Normal => {
                    where_clause_push(
                        &mut w_impl_field_bounds_decompress,
                        syn::parse_quote! { #ty: ::fuel_compression::DecompressibleBy<Ctx> },
                    );
                }
            }
        }
    }
    where_clause_push(
        &mut w_impl_field_bounds_decompress,
        syn::parse_quote! { Ctx: ::fuel_compression::ContextError },
    );

    let decompress_per_variant =
        each_variant_compressed(&s, &quote! {#compressed_name}, |variant| {
            let vname = variant.ast().ident.clone();
            let construct = match &s.ast().data {
                syn::Data::Struct(_) => quote! { #name },
                syn::Data::Enum(_) => quote! {#name :: #vname },
                syn::Data::Union(_) => unreachable!(),
            };
            construct_decompress(&construct, variant)
        });

    let impls = s.gen_impl(quote! {
        gen impl<Ctx> ::fuel_compression::DecompressibleBy<Ctx> for @Self #w_impl_field_bounds_decompress {
            async fn decompress_with(compressed: Self::Compressed, ctx: &Ctx) -> Result<Self, Ctx::Error> {
                Ok(match compressed { #decompress_per_variant })
            }
        }
    });
    quote! {
        #impls
    }
}
