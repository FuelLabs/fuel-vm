use proc_macro2::{
    Span,
    TokenStream as TokenStream2,
    TokenTree as TokenTree2,
};
use quote::{
    format_ident,
    quote,
};

use syn::parse::{
    Parse,
    ParseStream,
};

const ATTR: &str = "da_compress";

/// Structure (struct or enum) attributes
#[derive(Debug)]
pub enum StructureAttrs {
    /// Insert bounds for a generic type
    Bound(Vec<String>),
    /// Discard generic parameter
    Discard(Vec<String>),
}
impl Parse for StructureAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if let Ok(ml) = input.parse::<syn::MetaList>() {
            if ml.path.segments.len() == 1 {
                match ml.path.segments[0].ident.to_string().as_str() {
                    "bound" => {
                        let mut bound = Vec::new();
                        for item in ml.tokens {
                            match item {
                                TokenTree2::Ident(ident) => {
                                    bound.push(ident.to_string());
                                }
                                other => {
                                    return Err(syn::Error::new_spanned(
                                        other,
                                        "Expected generic (type) name",
                                    ))
                                }
                            }
                        }
                        return Ok(Self::Bound(bound));
                    }
                    "discard" => {
                        let mut discard = Vec::new();
                        for item in ml.tokens {
                            match item {
                                TokenTree2::Ident(ident) => {
                                    discard.push(ident.to_string());
                                }
                                other => {
                                    return Err(syn::Error::new_spanned(
                                        other,
                                        "Expected generic (type) name",
                                    ))
                                }
                            }
                        }
                        return Ok(Self::Discard(discard));
                    }
                    _ => {}
                }
            }
        }
        Err(syn::Error::new_spanned(
            input.parse::<syn::Ident>()?,
            "Expected `bound` or `discard`",
        ))
    }
}
impl StructureAttrs {
    pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Vec<Self>> {
        let mut result = Vec::new();
        for attr in attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if let syn::Meta::List(ml) = &attr.meta {
                if ml.path.segments.len() == 1 && ml.path.segments[0].ident == ATTR {
                    result.push(syn::parse2::<StructureAttrs>(ml.tokens.clone())?);
                }
            }
        }

        Ok(result)
    }
}

/// Field attributes
pub enum FieldAttrs {
    /// Skipped when compacting, and must be reconstructed when decompacting.
    Skip,
    /// Compresseded recursively.
    Normal,
    /// This value is compacted into a registry lookup.
    Registry,
}
impl FieldAttrs {
    pub fn parse(attrs: &[syn::Attribute]) -> Self {
        let mut result = Self::Normal;
        for attr in attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if let syn::Meta::List(ml) = &attr.meta {
                if ml.path.segments.len() == 1 && ml.path.segments[0].ident == ATTR {
                    if !matches!(result, Self::Normal) {
                        panic!("Duplicate attribute: {}", ml.tokens);
                    }

                    if let Ok(ident) = syn::parse2::<syn::Ident>(ml.tokens.clone()) {
                        if ident == "skip" {
                            result = Self::Skip;
                            continue;
                        } else if ident == "registry" {
                            result = Self::Registry;
                            continue;
                        }
                    }
                    panic!("Invalid attribute: {}", ml.tokens);
                }
            }
        }

        result
    }
}

/// Map field definitions to compacted field definitions.
fn field_defs(fields: &syn::Fields) -> TokenStream2 {
    let mut defs = TokenStream2::new();

    for field in fields {
        let attrs = FieldAttrs::parse(&field.attrs);
        defs.extend(match &attrs {
            FieldAttrs::Skip => quote! {},
            FieldAttrs::Normal => {
                let ty = &field.ty;
                let cty = quote! {
                    <#ty as ::fuel_compression::Compressible>::Compressed
                };
                if let Some(fname) = field.ident.as_ref() {
                    quote! { #fname: #cty, }
                } else {
                    quote! { #cty, }
                }
            }
            FieldAttrs::Registry => {
                if let Some(fname) = field.ident.as_ref() {
                    quote! { #fname: ::fuel_compression::RawKey, }
                } else {
                    quote! { ::fuel_compression::RawKey, }
                }
            }
        });
    }

    match fields {
        syn::Fields::Named(_) => quote! {{ #defs }},
        syn::Fields::Unnamed(_) => quote! {(#defs)},
        syn::Fields::Unit => quote! {},
    }
}

/// Construct compact version of the struct from the original one
fn construct_compact(
    // The structure to construct, i.e. struct name or enum variant path
    compact: &TokenStream2,
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
                        let #cname = <#ty as ::fuel_compression::CompressibleBy<_>>::compress(&#binding, ctx)?;
                    }
                }
                FieldAttrs::Registry => {
                    quote! {
                        let #cname = <#ty as ::fuel_compression::CompressibleBy<_>>::compress(&#binding, ctx)?;
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
        #compact #construct_fields
    }
}
/// Construct original version of the struct from the compacted one
fn construct_decompact(
    // The original structure to construct, i.e. struct name or enum variant path
    original: &TokenStream2,
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
                FieldAttrs::Skip => quote! {
                    let #cname = Default::default();
                },
                FieldAttrs::Normal => {
                    quote! {
                        let #cname = <#ty as ::fuel_compression::DecompressibleBy<_>>::decompress(#binding, ctx)?;
                    }
                }
                FieldAttrs::Registry => {
                    quote! {
                        let #cname = <#ty as ::fuel_compression::DecompressibleBy<_>>::decompress(#binding, ctx)?;
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

/// Generate a match arm for each variant of the compacted structure
/// using the given function to generate the pattern body.
fn each_variant_compact<F: FnMut(&synstructure::VariantInfo<'_>) -> TokenStream2>(
    s: &synstructure::Structure,
    compact_name: &TokenStream2,
    mut f: F,
) -> TokenStream2 {
    s.variants()
        .iter()
        .map(|variant| {
            // Modify the binding pattern to match the compact variant
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
            let p = quote! { #compact_name #(#p)* };

            let decompacted = f(variant);
            quote! {
                #p => { #decompacted }
            }
        })
        .collect()
}

/// Derives `Compressed` trait for the given `struct` or `enum`.
pub fn compact_derive(mut s: synstructure::Structure) -> TokenStream2 {
    s.add_bounds(synstructure::AddBounds::None)
        .underscore_const(true);

    let s_attrs = match StructureAttrs::parse(&s.ast().attrs) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let name = &s.ast().ident;
    let compact_name = format_ident!("Compressed{}", name);

    let mut g = s.ast().generics.clone();
    let mut w_structure = g.where_clause.take();
    let mut w_impl = w_structure.clone();
    for item in &s_attrs {
        match item {
            StructureAttrs::Bound(bound) => {
                if w_structure.is_none() {
                    w_structure = Some(syn::WhereClause {
                        where_token: syn::Token![where](proc_macro2::Span::call_site()),
                        predicates: Default::default(),
                    });
                    w_impl = Some(syn::WhereClause {
                        where_token: syn::Token![where](proc_macro2::Span::call_site()),
                        predicates: Default::default(),
                    });
                }
                for p in bound {
                    let id = syn::Ident::new(p, Span::call_site());
                    w_structure.as_mut().unwrap().predicates.push(
                        syn::parse_quote! { #id: ::fuel_compression::Compressible },
                    );
                    w_impl.as_mut().unwrap().predicates.push(
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
            let attrs = FieldAttrs::parse(&field.attrs);
            if matches!(attrs, FieldAttrs::Skip) {
                continue;
            }

            if w_impl_field_bounds_compress.is_none() {
                w_impl_field_bounds_compress = Some(syn::WhereClause {
                    where_token: syn::Token![where](proc_macro2::Span::call_site()),
                    predicates: Default::default(),
                });
            }

            w_impl_field_bounds_compress
                .as_mut()
                .unwrap()
                .predicates
                .push(syn::parse_quote! { #ty: ::fuel_compression::CompressibleBy<Ctx> });
        }
    }

    let mut w_impl_field_bounds_decompress = w_impl.clone();
    for variant in s.variants() {
        for field in variant.ast().fields.iter() {
            let ty = &field.ty;
            let attrs = FieldAttrs::parse(&field.attrs);
            if matches!(attrs, FieldAttrs::Skip) {
                continue;
            }

            if w_impl_field_bounds_decompress.is_none() {
                w_impl_field_bounds_decompress = Some(syn::WhereClause {
                    where_token: syn::Token![where](proc_macro2::Span::call_site()),
                    predicates: Default::default(),
                });
            }

            w_impl_field_bounds_decompress
                .as_mut()
                .unwrap()
                .predicates
                .push(
                    syn::parse_quote! { #ty: ::fuel_compression::DecompressibleBy<Ctx> },
                );
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
                pub struct #compact_name #g #w_structure #defs #semi
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
                pub enum #compact_name #g #w_structure { #variant_defs }
            }
        }
        syn::Data::Union(_) => panic!("unions are not supported"),
    };

    let construct_per_variant = s.each_variant(|variant| {
        let vname = variant.ast().ident.clone();
        let construct = match &s.ast().data {
            syn::Data::Struct(_) => quote! { #compact_name },
            syn::Data::Enum(_) => quote! {#compact_name :: #vname },
            syn::Data::Union(_) => unreachable!(),
        };
        construct_compact(&construct, variant)
    });

    let decompact_per_variant =
        each_variant_compact(&s, &quote! {#compact_name}, |variant| {
            let vname = variant.ast().ident.clone();
            let construct = match &s.ast().data {
                syn::Data::Struct(_) => quote! { #name },
                syn::Data::Enum(_) => quote! {#name :: #vname },
                syn::Data::Union(_) => unreachable!(),
            };
            construct_decompact(&construct, variant)
        });

    let impls = s.gen_impl(quote! {
        gen impl ::fuel_compression::Compressible for @Self #w_impl {
            type Compressed = #compact_name #g;
        }

        gen impl<Ctx> ::fuel_compression::CompressibleBy<Ctx> for @Self #w_impl_field_bounds_compress {
            fn compress(&self, ctx: &mut Ctx) -> anyhow::Result<Self::Compressed> {
                Ok(match self { #construct_per_variant })
            }
        }
        gen impl<Ctx> ::fuel_compression::DecompressibleBy<Ctx> for @Self #w_impl_field_bounds_decompress {
            fn decompress(compressed: &Self::Compressed, ctx: &Ctx) -> anyhow::Result<Self> {
                Ok(match compressed { #decompact_per_variant })
            }
        }
    });
    let rs = quote! {
        #def
        #impls
    };

    let _ = std::fs::write(format!("/tmp/derive/{}.rs", name), rs.to_string());

    rs
}
