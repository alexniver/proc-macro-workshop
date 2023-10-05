use quote::quote;
use syn::parse::Parse;

pub(crate) struct SeqParser {
    pub(crate) n_ident: syn::Ident,
    pub(crate) from: u32,
    pub(crate) to: u32,
    pub(crate) body: proc_macro2::TokenStream,
}

impl Parse for SeqParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let n_ident: syn::Ident = input.parse()?;

        input.parse::<syn::Token![in]>()?;

        let from: syn::LitInt = input.parse()?;

        input.parse::<syn::Token![..]>()?;

        let to: syn::LitInt = input.parse()?;

        let body_buf;
        syn::braced!(body_buf in input);
        let body: proc_macro2::TokenStream = body_buf.parse()?;

        Ok(Self {
            n_ident,
            from: from.base10_parse()?,
            to: to.base10_parse()?,
            body,
        })
    }
}

impl SeqParser {
    pub(crate) fn expand(&self, ts: &proc_macro2::TokenStream, n: u32) -> proc_macro2::TokenStream {
        let mut result = proc_macro2::TokenStream::new();

        let token_tree_arr = ts.clone().into_iter().collect::<Vec<_>>();
        let mut idx = 0;
        while idx < token_tree_arr.len() {
            let tree_node = &token_tree_arr[idx];
            match tree_node {
                proc_macro2::TokenTree::Group(g) => {
                    let inner_stream = self.expand(&g.stream(), n);

                    let mut wrap_group = proc_macro2::Group::new(g.delimiter(), inner_stream);
                    wrap_group.set_span(g.span());
                    result.extend(quote!(#wrap_group));
                    idx += 1;
                }
                proc_macro2::TokenTree::Ident(ident) => {
                    let mut is_find_target_n = false;
                    if idx + 1 < token_tree_arr.len() {
                        match token_tree_arr[idx + 1] {
                            proc_macro2::TokenTree::Punct(ref punct) => {
                                if punct.to_string() == "~" && idx + 2 < token_tree_arr.len() {
                                    match token_tree_arr[idx + 2] {
                                        proc_macro2::TokenTree::Ident(ref n_ident) => {
                                            if n_ident == &self.n_ident {
                                                let f_ident = syn::Ident::new(
                                                    &format!("{}{}", ident.to_string(), n),
                                                    ident.span(),
                                                );

                                                result.extend(quote!(#f_ident));

                                                idx += 3;
                                                is_find_target_n = true;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if !is_find_target_n {
                        if ident == &self.n_ident {
                            let mut n_ident = proc_macro2::Literal::u32_unsuffixed(n);
                            n_ident.set_span(ident.span());
                            result.extend(quote!(#n_ident));
                        } else {
                            result.extend(quote!(#tree_node));
                        }

                        idx += 1;
                    }
                }
                proc_macro2::TokenTree::Literal(_) | proc_macro2::TokenTree::Punct(_) => {
                    result.extend(quote!(#tree_node));
                    idx += 1;
                }
            }
        }

        result
    }
}
