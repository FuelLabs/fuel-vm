use proc_macro2::TokenStream as TokenStream2;
use quote::{
    format_ident,
    quote,
};

use regex::Regex;

const ATTR: &str = "da_compress";

/// Field attributes
pub enum FieldAttrs {
    /// Skipped when compacting, and must be reconstructed when decompacting.
    Skip,
    /// Compacted recursively.
    Normal,
    /// This value is compacted into a registry lookup.
    Registry(String),
}
impl FieldAttrs {
    pub fn parse(attrs: &[syn::Attribute]) -> Self {
        let re_registry = Regex::new(r#"^registry\s*=\s*"([a-zA-Z_]+)"$"#).unwrap();

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

                    let attr_contents = ml.tokens.to_string();
                    if attr_contents == "skip" {
                        result = Self::Skip;
                    } else if let Some(m) = re_registry.captures(&attr_contents) {
                        result = Self::Registry(m.get(1).unwrap().as_str().to_owned());
                    } else {
                        panic!("Invalid attribute: {}", ml.tokens);
                    }
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
                let reg_ident = format_ident!("{}", registry);
                let cty = quote! {
                    ::fuel_compression::Key<::fuel_compression::tables::#reg_ident>
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
                        let #cname = <#ty as Compactable>::compact(&#binding, ctx);
                    }
                }
                FieldAttrs::Registry(registry) => {
                    let reg_ident = format_ident!("{}", registry);
                    let cty = quote! {
                        Key<
                            tables::#reg_ident
                        >
                    };
                    quote! {
                        let #cname: #cty = ctx.to_key(
                            <tables::#reg_ident as Table>::Type::from(#binding.clone())
                        );
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
                        let #cname = <#ty as Compactable>::decompact(#binding, reg);
                    }
                }
                FieldAttrs::Registry(registry) => {
                    let reg_ident = format_ident!("{}", registry);
                    quote! {
                        let raw: <tables::#reg_ident as Table>::Type = reg.read(
                            #binding
                        );
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
                    let reg_ident = format_ident!("{}", registry);
                    quote! {
                        CountPerTable::#reg_ident(1)
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

    let name = &s.ast().ident;
    let compact_name = format_ident!("Compact{}", name);

    let g = s.ast().generics.clone();
    let w = g.where_clause.clone();
    let def = match &s.ast().data {
        syn::Data::Struct(v) => {
            let variant: &synstructure::VariantInfo = &s.variants()[0];
            let defs = field_defs(&variant.ast().fields);
            let semi = match v.fields {
                syn::Fields::Named(_) => quote! {},
                syn::Fields::Unnamed(_) => quote! {;},
                syn::Fields::Unit => quote! {;},
            };
            quote! {
                #[derive(Clone, serde::Serialize, serde::Deserialize)]
                #[doc = concat!("Compacted version of `", stringify!(#name), "`.")]
                pub struct #compact_name #g #w #defs #semi
            }
        }
        syn::Data::Enum(_) => {
            let variant_defs: TokenStream2 = s
                .variants()
                .iter()
                .map(|variant| {
                    let vname = variant.ast().ident.clone();
                    let defs = field_defs(&variant.ast().fields);
                    quote! {
                        #vname #defs,
                    }
                })
                .collect();

            quote! {
                #[derive(Clone, serde::Serialize, serde::Deserialize)]
                #[doc = concat!("Compacted version of `", stringify!(#name), "`.")]
                pub enum #compact_name #g #w { #variant_defs }
            }
        }
        syn::Data::Union(_) => panic!("unions are not supported"),
    };

    let count_per_variant = s.each_variant(|variant| sum_counts(variant));
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

        gen impl Compactable for @Self {
            type Compact = #compact_name #g;

            fn count(&self) -> CountPerTable {
                match self { #count_per_variant }
            }

            fn compact<R: RegistryDb>(&self, ctx: &mut CompactionContext<R>) -> Self::Compact {
                match self { #construct_per_variant }
            }

            fn decompact<R: RegistryDb>(compact: Self::Compact, reg: &R) -> Self {
                match compact { #decompact_per_variant }
            }
        }
    });
    let rs = quote! {
        #def
        #impls
    };

    let _ = std::fs::write(format!("/tmp/derive/{}.rs", name), &rs.to_string());

    rs
}
