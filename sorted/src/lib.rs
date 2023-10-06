mod match_visitor;

use proc_macro::TokenStream;
use quote::ToTokens;
use syn::{parse_macro_input, visit_mut::VisitMut};

#[proc_macro_attribute]
pub fn sorted(_args: TokenStream, input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input);

    match expand(&ast) {
        Ok(s) => s.into(),
        Err(e) => {
            let mut t = e.to_compile_error();
            t.extend(ast.to_token_stream());
            t.into()
        }
    }
}

fn expand(ast: &syn::Item) -> syn::Result<proc_macro2::TokenStream> {
    match ast {
        syn::Item::Enum(item_enum) => {
            let fields = item_enum
                .variants
                .iter()
                .map(|v| (v.ident.to_string(), v))
                .collect::<Vec<_>>();
            let mut sorted = fields.clone();
            sorted.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, sorted_name) in fields.iter().zip(sorted.iter()) {
                if name.0 != sorted_name.0 {
                    return syn::Result::Err(syn::Error::new(
                        sorted_name.1.ident.span(),
                        format!("{} should sort before {}", sorted_name.0, name.0),
                    ));
                }
            }
            Ok(ast.to_token_stream())
        }
        _ => syn::Result::Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected enum or match expression",
        )),
    }
}

#[proc_macro_attribute]
pub fn check(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as syn::ItemFn);

    match check_expand(&mut ast) {
        Ok(s) => s.into(),
        Err(e) => {
            let mut t = e.to_compile_error();
            t.extend(ast.to_token_stream());
            t.into()
        }
    }
}

fn check_expand(ast: &mut syn::ItemFn) -> syn::Result<proc_macro2::TokenStream> {
    let mut visitor = match_visitor::MatchVisitor { err: None };
    visitor.visit_item_fn_mut(ast);

    if let Some(e) = visitor.err {
        syn::Result::Err(e)
    } else {
        syn::Result::Ok(ast.to_token_stream())
    }
}
