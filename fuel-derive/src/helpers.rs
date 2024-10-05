use proc_macro2::TokenStream as TokenStream2;

pub fn where_clause_push(w: &mut Option<syn::WhereClause>, p: TokenStream2) {
    if w.is_none() {
        *w = Some(syn::WhereClause {
            where_token: syn::Token![where](proc_macro2::Span::call_site()),
            predicates: ::core::default::Default::default(),
        });
    }
    w.as_mut()
        .unwrap()
        .predicates
        .push(syn::parse_quote! { #p });
}
