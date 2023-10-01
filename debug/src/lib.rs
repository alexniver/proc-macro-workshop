use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

struct FieldInfo {
    ident: syn::Ident,
    debug_attr: Option<String>,
}

#[proc_macro_derive(CustomDebug, attributes(debug))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    match do_expand(ast) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn do_expand(ast: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &ast.ident;
    let field_info_arr = get_field_info_arr(&ast)?;

    let impl_debug = impl_debug(&ident, &field_info_arr)?;

    Ok(quote!(
        #impl_debug
    ))
}

fn impl_debug(
    ident: &syn::Ident,
    field_info_arr: &Vec<FieldInfo>,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident_str = &ident.to_string();

    let mut struct_ts = proc_macro2::TokenStream::new();
    struct_ts.extend(quote!(
        f.debug_struct(#ident_str)
    ));

    // .field("x", &self.x)
    // .field("y", &self.y)
    let mut field_ts = proc_macro2::TokenStream::new();
    for f in field_info_arr.iter() {
        let f_ident = &f.ident;
        let f_ident_str = &f.ident.to_string();
        if let Some(ref debug_attr) = f.debug_attr {
            field_ts.extend(quote!(
                .field(#f_ident_str, &format_args!(#debug_attr, &self.#f_ident))
            ));
        } else {
            field_ts.extend(quote!(
                .field(#f_ident_str, &self.#f_ident)
            ));
        }
    }
    Ok(quote!(
        impl std::fmt::Debug for #ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                #struct_ts
                #field_ts
                .finish()
            }
        }
    ))
}

fn get_field_info_arr(ast: &syn::DeriveInput) -> syn::Result<Vec<FieldInfo>> {
    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = &ast.data
    {
        named.iter().map(|f| get_field_info(f)).collect()
    } else {
        Err(syn::Error::new_spanned(ast, "not found fields named"))
    }
}

fn get_field_info(field: &syn::Field) -> syn::Result<FieldInfo> {
    if let Some(ref ident) = field.ident {
        let ident = ident.clone();
        let mut debug_attr = None;

        for attr in field.attrs.iter() {
            if attr.path().is_ident("debug") {
                if let Ok(syn::MetaNameValue {
                    value:
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(v),
                            ..
                        }),
                    ..
                }) = attr.meta.require_name_value()
                {
                    debug_attr = Some(v.value());
                }
            } else {
                return Err(syn::Error::new_spanned(attr, "except debug"));
            }
        }
        Ok(FieldInfo { ident, debug_attr })
    } else {
        Err(syn::Error::new_spanned(field, "fail to get ident"))
    }
}
