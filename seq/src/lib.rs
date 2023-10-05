mod seq_parser;

use proc_macro::TokenStream;
use syn::parse_macro_input;

use crate::seq_parser::SeqParser;

#[proc_macro]
pub fn seq(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as SeqParser);

    let mut output = proc_macro2::TokenStream::new();

    for i in ast.from..ast.to {
        output.extend(ast.expand(&ast.body, i));
    }

    output.into()
}
