use syn::Expr;

/// Evaluate a simple u64-valued expression, like enum discriminant value.
pub fn evaluate_simple_expr(expr: &Expr) -> Option<u64> {
    match expr {
        Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Int(int) => int.base10_parse().ok(),
            _ => None,
        },
        Expr::Paren(paren) => evaluate_simple_expr(&paren.expr),
        Expr::Group(group) => evaluate_simple_expr(&group.expr),
        _ => None,
    }
}
