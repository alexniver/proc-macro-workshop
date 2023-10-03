mod associated_type_visiter;

use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, visit::Visit, DeriveInput};

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

    let none_phantom_generic_param_arr = get_none_phantom_generic_param_arr(&ast);

    let generic_type_arr: Vec<String> = ast
        .generics
        .params
        .iter()
        .filter_map(|f| {
            if let syn::GenericParam::Type(ty) = f {
                Some(ty.ident.to_string())
            } else {
                None
            }
        })
        .collect();

    let mut associated_type_visiter = associated_type_visiter::AssociatedTypeVisiter {
        generic_type_arr,
        associated_type_map: HashMap::new(),
    };

    associated_type_visiter.visit_derive_input(&ast);

    let impl_debug = impl_debug(
        &ast,
        &ident,
        &ast.generics,
        &field_info_arr,
        &none_phantom_generic_param_arr,
        &associated_type_visiter.associated_type_map,
    )?;

    Ok(quote!(
        #impl_debug
    ))
}

fn impl_debug(
    ast: &DeriveInput,
    ident: &syn::Ident,
    generics: &syn::Generics,
    field_info_arr: &Vec<FieldInfo>,
    none_phantom_generic_param_arr: &Vec<syn::Ident>,
    associated_type_map: &HashMap<String, Vec<syn::TypePath>>,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident_str = &ident.to_string();

    let mut generics = generics.clone();

    if let Some(ref s) = get_struct_escape_hatch(&ast) {
        generics.make_where_clause();
        if let Some(w) = generics.where_clause.as_mut() {
            if let Ok(s) = syn::parse_str(s) {
                w.predicates.push(s);
            }
        }
    } else {
        for g in generics.params.iter_mut() {
            if let syn::GenericParam::Type(t) = g {
                if none_phantom_generic_param_arr.contains(&t.ident)
                    && !associated_type_map.contains_key(&t.ident.to_string())
                {
                    t.bounds.push(parse_quote!(std::fmt::Debug));
                }
            }
        }

        generics.make_where_clause();
        for (_, associated_type_arr) in associated_type_map.iter() {
            for associated_type in associated_type_arr {
                if let Some(w) = generics.where_clause.as_mut() {
                    w.predicates
                        .push(parse_quote!(#associated_type: std::fmt::Debug));
                }
            }
        }
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

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
        impl #impl_generics std::fmt::Debug for #ident #ty_generics #where_clause {
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

fn get_none_phantom_generic_param_arr(ast: &syn::DeriveInput) -> Vec<syn::Ident> {
    let mut result = vec![];

    if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { named, .. }),
        ..
    }) = &ast.data
    {
        for field in named {
            if let syn::Type::Path(syn::TypePath {
                path: syn::Path { segments, .. },
                ..
            }) = &field.ty
            {
                if let Some(path_seg) = segments.last() {
                    if path_seg.ident != "PhantomData" {
                        let mut arguments = &path_seg.arguments;
                        let mut target_ident = path_seg.ident.clone();
                        loop {
                            if let syn::PathArguments::AngleBracketed(
                                syn::AngleBracketedGenericArguments { args, .. },
                            ) = arguments
                            {
                                if let Some(&syn::GenericArgument::Type(syn::Type::Path(
                                    syn::TypePath {
                                        path: syn::Path { ref segments, .. },
                                        ..
                                    },
                                ))) = args.first()
                                {
                                    if let Some(path_seg) = segments.last() {
                                        arguments = &path_seg.arguments;
                                        if arguments == &syn::PathArguments::None {
                                            target_ident = path_seg.ident.clone();
                                            break;
                                        }
                                    }
                                }
                            } else {
                                break;
                            }
                        }

                        result.push(target_ident);
                    }
                }
            }
        }
    }
    result
}

fn get_struct_escape_hatch(ast: &DeriveInput) -> Option<String> {
    let mut result = None;
    for attr in ast.attrs.iter() {
        if attr.path().is_ident("debug") {
            let res = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("bound") {
                    let value = meta.value()?;
                    // this parses `"EarlGrey"`
                    let s: syn::LitStr = value.parse()?;
                    result = Some(s.value());

                    Ok(())
                } else {
                    Err(syn::Error::new_spanned(attr, "ignore"))
                }
            });

            if let Ok(_) = res {
                break;
            }
        }
    }
    result
}
