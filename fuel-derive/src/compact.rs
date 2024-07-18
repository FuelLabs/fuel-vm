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
    /// Compacted recursively.
    Normal,
    /// This value is compacted into a registry lookup.
    Registry(syn::Path),
}
impl FieldAttrs {
    pub fn parse(attrs: &[syn::Attribute]) -> Self {
        let registry_path = syn::parse2::<syn::Path>(quote! {registry}).unwrap();

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
                        if ident.to_string() == "skip" {
                            result = Self::Skip;
                            continue;
                        }
                    } else if let Ok(kv) =
                        syn::parse2::<syn::MetaNameValue>(ml.tokens.clone())
                    {
                        if kv.path == registry_path {
                            if let syn::Expr::Path(p) = kv.value {
                                result = Self::Registry(p.path);
                                continue;
                            }
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
                    <#ty as ::fuel_compression::Compactable>::Compact
                };
                if let Some(fname) = field.ident.as_ref() {
                    quote! { #fname: #cty, }
                } else {
                    quote! { #cty, }
                }
            }
            FieldAttrs::Registry(registry) => {
                let cty = quote! {
                    ::fuel_compression::Key<#registry>
                };
                if let Some(fname) = field.ident.as_ref() {
                    quote! { #fname: #cty, }
                } else {
                    quote! { #cty, }
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
                        let #cname = <#ty as Compactable>::compact(&#binding, ctx)?;
                    }
                }
                FieldAttrs::Registry(registry) => {
                    let cty = quote! {
                        Key<
                            #registry
                        >
                    };
                    quote! {
                        let #cname: #cty = ctx.to_key(
                            <#registry as Table>::Type::from(#binding.clone())
                        )?;
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
                        let #cname = <#ty as Compactable>::decompact(#binding, reg)?;
                    }
                }
                FieldAttrs::Registry(registry) => {
                    quote! {
                        let raw: <#registry as Table>::Type = reg.read(
                            #binding
                        )?;
                        let #cname = raw.into();
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

// Sum of Compactable::count() of all fields.
fn sum_counts(variant: &synstructure::VariantInfo<'_>) -> TokenStream2 {
    variant
        .bindings()
        .iter()
        .map(|binding| {
            let attrs = FieldAttrs::parse(&binding.ast().attrs);
            let ty = &binding.ast().ty;

            match attrs {
                FieldAttrs::Skip => quote! { CountPerTable::default() },
                FieldAttrs::Normal => {
                    quote! { <#ty as Compactable>::count(&#binding) }
                }
                FieldAttrs::Registry(registry) => {
                    quote! {
                        #registry::count(1)
                    }
                }
            }
        })
        .fold(
            quote! { CountPerTable::default() },
            |acc, x| quote! { #acc + #x },
        )
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

/// Derives `Compact` trait for the given `struct` or `enum`.
pub fn compact_derive(mut s: synstructure::Structure) -> TokenStream2 {
    s.add_bounds(synstructure::AddBounds::None)
        .underscore_const(true);

    let s_attrs = match StructureAttrs::parse(&s.ast().attrs) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error(),
    };

    let name = &s.ast().ident;
    let compact_name = format_ident!("Compact{}", name);

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
                    w_structure
                        .as_mut()
                        .unwrap()
                        .predicates
                        .push(syn::parse_quote! { #id: ::fuel_compression::Compactable });
                    w_impl.as_mut().unwrap().predicates.push(
                        syn::parse_quote! { for<'de>  #id: ::fuel_compression::Compactable + serde::Serialize + serde::Deserialize<'de> + Clone },
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
                #[doc = concat!("Compacted version of `", stringify!(#name), "`.")]
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
                #[doc = concat!("Compacted version of `", stringify!(#name), "`.")]
                pub enum #compact_name #g #w_structure { #variant_defs }
            }
        }
        syn::Data::Union(_) => panic!("unions are not supported"),
    };

    let count_per_variant = s.each_variant(sum_counts);
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
        use ::fuel_compression::{RegistryDb, tables, Table, Key, Compactable, CountPerTable, CompactionContext};

        gen impl Compactable for @Self #w_impl {
            type Compact = #compact_name #g;

            fn count(&self) -> CountPerTable {
                match self { #count_per_variant }
            }

            fn compact<R: RegistryDb>(&self, ctx: &mut CompactionContext<R>) -> anyhow::Result<Self::Compact> {
                Ok(match self { #construct_per_variant })
            }

            fn decompact<R: RegistryDb>(compact: Self::Compact, reg: &R) -> anyhow::Result<Self> {
                Ok(match compact { #decompact_per_variant })
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
