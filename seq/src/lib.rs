mod seq_parser;

use proc_macro::TokenStream;
use syn::parse_macro_input;

use crate::seq_parser::SeqParser;

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as SeqParser);

    let mut result = proc_macro2::TokenStream::new();

    if let Some(expand) = ast.expand_repeat(&ast.body, ast.from, ast.to) {
        result.extend(expand);
    } else {
        for i in ast.from..ast.to {
            result.extend(ast.expand_normal(&ast.body, i));
        }
    }

    result.into()
}
