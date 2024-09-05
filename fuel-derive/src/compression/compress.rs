use proc_macro2::{
    Span,
    TokenStream as TokenStream2,
};
use quote::{
    format_ident,
    quote,
};

use crate::helpers::where_clause_push;

use super::attribute::{
    FieldAttrs,
    StructureAttrs,
};

/// Map field definitions to compressed field definitions.
fn field_defs(fields: &syn::Fields) -> TokenStream2 {
    let mut defs = TokenStream2::new();

    for field in fields {
        let attrs = FieldAttrs::parse(&field.attrs);
        let field_content = match &attrs {
            FieldAttrs::Skip => continue,
            FieldAttrs::Normal => {
                let ty = &field.ty;
                quote! {
                    <#ty as ::fuel_compression::Compressible>::Compressed
                }
            }
        };
        defs.extend(if let Some(fname) = field.ident.as_ref() {
            quote! { #fname: #field_content, }
        } else {
            quote! { #field_content, }
        });
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
        .map(|binding| {
            let attrs = FieldAttrs::parse(&binding.ast().attrs);
            let ty = &binding.ast().ty;
            let cname = format_ident!("{}_c", binding.binding);

            match attrs {
                FieldAttrs::Skip => quote! {},
                FieldAttrs::Normal => {
                    quote! {
                        let #cname = <#ty as ::fuel_compression::CompressibleBy<_, _>>::compress(&#binding, ctx).await?;
                    }
                }
            }
        })
        .collect();

    let construct_fields: TokenStream2 = variant
        .bindings()
        .iter()
        .map(|binding| {
            let attrs = FieldAttrs::parse(&binding.ast().attrs);
            if matches!(attrs, FieldAttrs::Skip) {
                return quote! {};
            }
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

/// Generate a match arm for each variant of the compressed structure
/// using the given function to generate the pattern body.
pub fn each_variant_compressed<
    F: FnMut(&synstructure::VariantInfo<'_>) -> TokenStream2,
>(
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

/// Derives `Compress` trait for the given `struct` or `enum`.
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
    let mut w_structure = g.where_clause.take();
    let mut w_impl = w_structure.clone();
    for item in &s_attrs {
        match item {
            StructureAttrs::Bound(bound) => {
                for p in bound {
                    let id = syn::Ident::new(p, Span::call_site());
                    where_clause_push(
                        &mut w_structure,
                        syn::parse_quote! { #id: ::fuel_compression::Compressible },
                    );
                    where_clause_push(
                        &mut w_impl,
                        syn::parse_quote! { for<'de>  #id: ::fuel_compression::Compressible + serde::Serialize + serde::Deserialize<'de> + Clone },
                    );
                }
            }
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
                        syn::parse_quote! { #ty: ::fuel_compression::CompressibleBy<Ctx, E> },
                    );
                }
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
                        syn::parse_quote! { #ty: ::fuel_compression::DecompressibleBy<Ctx, E> },
                    );
                }
            }
        }
    }

    let def = match &s.ast().data {
        syn::Data::Struct(v) => {
            let variant: &synstructure::VariantInfo = &s.variants()[0];
            let defs = field_defs(variant.ast().fields);
            let semi = match v.fields {
                syn::Fields::Named(_) => quote! {},
                syn::Fields::Unnamed(_) => quote! {;},
                syn::Fields::Unit => quote! {;},
            };
            quote! {
                #[derive(Clone, serde::Serialize, serde::Deserialize)]
                #[doc = concat!("Compresseded version of `", stringify!(#name), "`.")]
                pub struct #compressed_name #g #w_structure #defs #semi
            }
        }
        syn::Data::Enum(_) => {
            let variant_defs: TokenStream2 = s
                .variants()
                .iter()
                .map(|variant| {
                    let vname = variant.ast().ident.clone();
                    let defs = field_defs(variant.ast().fields);
                    quote! {
                        #vname #defs,
                    }
                })
                .collect();

            quote! {
                #[derive(Clone, serde::Serialize, serde::Deserialize)]
                #[doc = concat!("Compresseded version of `", stringify!(#name), "`.")]
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

        gen impl<Ctx, E> ::fuel_compression::CompressibleBy<Ctx, E> for @Self #w_impl_field_bounds_compress {
            async fn compress(&self, ctx: &mut Ctx) -> Result<Self::Compressed, E> {
                Ok(match self { #compress_per_variant })
            }
        }
    });
    quote! {
        #def
        #impls
    }
}
