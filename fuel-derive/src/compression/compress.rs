use crate::helpers::where_clause_push;
use proc_macro2::TokenStream as TokenStream2;
use quote::{
    format_ident,
    quote,
};

use super::attribute::{
    FieldAttrs,
    StructureAttrs,
};

/// Iterator of field items for the compressed structure.
/// Gives either named or unnamed fields.
fn field_items(fields: &syn::Fields) -> impl Iterator<Item = TokenStream2> + '_ {
    fields.iter().filter_map(|field| {
        let attrs = FieldAttrs::parse(&field.attrs);
        match &attrs {
            FieldAttrs::Skip => None,
            FieldAttrs::Normal => {
                let ty = &field.ty;
                let ftype = quote! {
                    <#ty as ::fuel_compression::Compressible>::Compressed
                };
                Some(if let Some(fname) = field.ident.as_ref() {
                    quote! { #fname: #ftype }
                } else {
                    quote! { #ftype }
                })
            }
        }
    })
}

/// Map field definitions to compressed field definitions.
fn field_defs(fields: &syn::Fields, include_vis: bool) -> TokenStream2 {
    let mut defs = TokenStream2::new();
    for item in field_items(fields) {
        if include_vis {
            defs.extend(quote! { #item, });
        } else {
            defs.extend(quote! { pub #item, });
        }
    }
    match fields {
        syn::Fields::Named(_) => quote! {{ #defs }},
        syn::Fields::Unnamed(_) => quote! {(#defs)},
        syn::Fields::Unit => quote! {},
    }
}

/// Construct compressed version of the struct from the original one
fn construct_compressed(
    // The structure to construct, i.e. struct name or enum variant path
    compressed: &TokenStream2,
    variant: &synstructure::VariantInfo<'_>,
) -> TokenStream2 {
    let bound_fields: TokenStream2 = variant
        .bindings()
        .iter()
        .filter(|binding| !matches!(FieldAttrs::parse(&binding.ast().attrs), FieldAttrs::Skip))
        .map(|binding| {
            let ty = &binding.ast().ty;
            let cname = format_ident!("{}_c", binding.binding);
            quote! {
                let #cname = <#ty as ::fuel_compression::CompressibleBy<_>>::compress_with(&#binding, ctx).await?;
            }
        })
        .collect();

    let construct_fields: TokenStream2 = variant
        .bindings()
        .iter()
        .filter(|binding| {
            !matches!(FieldAttrs::parse(&binding.ast().attrs), FieldAttrs::Skip)
        })
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
        #compressed #construct_fields
    }
}

/// Derives `Compressible` and `CompressibleBy` traits for the given `struct` or `enum`.
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

    let mut w_impl_field_bounds_compress = w_impl.clone();
    for variant in s.variants() {
        for field in variant.ast().fields.iter() {
            let ty = &field.ty;
            match FieldAttrs::parse(&field.attrs) {
                FieldAttrs::Skip => {}
                FieldAttrs::Normal => {
                    where_clause_push(
                        &mut w_impl_field_bounds_compress,
                        syn::parse_quote! { #ty: ::fuel_compression::CompressibleBy<Ctx> },
                    );
                }
            }
        }
    }
    where_clause_push(
        &mut w_impl_field_bounds_compress,
        syn::parse_quote! { Ctx: ::fuel_compression::ContextError },
    );

    let def = match &s.ast().data {
        syn::Data::Struct(v) => {
            let variant: &synstructure::VariantInfo = &s.variants()[0];
            let defs = field_defs(variant.ast().fields, false);
            let semi = match v.fields {
                syn::Fields::Named(_) => quote! {},
                syn::Fields::Unnamed(_) => quote! {;},
                syn::Fields::Unit => quote! {;},
            };
            quote! {
                #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
                #[doc = concat!("Compressed version of `", stringify!(#name), "`.")]
                pub struct #compressed_name #g #w_structure #defs #semi
            }
        }
        syn::Data::Enum(_) => {
            let variant_defs: TokenStream2 = s
                .variants()
                .iter()
                .map(|variant| {
                    let vname = variant.ast().ident.clone();
                    let defs = field_defs(variant.ast().fields, true);
                    quote! {
                        #vname #defs,
                    }
                })
                .collect();

            quote! {
                #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
                #[doc = concat!("Compressed version of `", stringify!(#name), "`.")]
                pub enum #compressed_name #g #w_structure { #variant_defs }
            }
        }
        syn::Data::Union(_) => panic!("unions are not supported"),
    };

    let compress_per_variant = s.each_variant(|variant| {
        let vname = variant.ast().ident.clone();
        let construct = match &s.ast().data {
            syn::Data::Struct(_) => quote! { #compressed_name },
            syn::Data::Enum(_) => quote! {#compressed_name :: #vname },
            syn::Data::Union(_) => unreachable!(),
        };
        construct_compressed(&construct, variant)
    });

    let impls = s.gen_impl(quote! {
        gen impl ::fuel_compression::Compressible for @Self #w_impl {
            type Compressed = #compressed_name #g;
        }

        gen impl<Ctx> ::fuel_compression::CompressibleBy<Ctx> for @Self #w_impl_field_bounds_compress {
            async fn compress_with(&self, ctx: &mut Ctx) -> Result<Self::Compressed, Ctx::Error> {
                Ok(match self { #compress_per_variant })
            }
        }
    });
    quote! {
        #def
        #impls
    }
}
