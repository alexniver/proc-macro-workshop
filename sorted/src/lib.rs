use proc_macro::TokenStream;
use quote::ToTokens;
use syn::parse_macro_input;

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
            for (f, s) in fields.iter().zip(sorted.iter()) {
                if f.0 != s.0 {
                    return syn::Result::Err(syn::Error::new(
                        s.1.ident.span(),
                        format!("{} should sort before {}", s.0, f.0),
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
