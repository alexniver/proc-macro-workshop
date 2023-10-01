use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Ident};

#[proc_macro_derive(Builder, attributes(builder))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    match do_extend(ast) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct FieldInfo {
    ident: syn::Ident,
    ty: syn::Type,
    field_path_seg: FieldPathSeg,
    each: Option<Ident>,
}

#[derive(PartialEq, Eq)]
enum FieldPathSeg {
    Normal, // no seg
    Option, // Option<>
    Vec,    // Vec<>
}

fn do_extend(ast: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let DeriveInput { ident, .. } = &ast;

    // eprintln!("{:#?}", &ast);

    let struct_fields = get_struct_fields(&ast)?;

    // let fields_ident = struct_fields.iter().map(|f| &f.ident).collect::<Vec<_>>();
    // let fields_ty = struct_fields.iter().map(|f| &f.ty).collect::<Vec<_>>();

    let builder_ident = Ident::new(&format!("{}Builder", ident), ident.span());

    let struct_builder = struct_builder(&builder_ident, &struct_fields)?;

    let struct_builder_impl = struct_builder_impl(&ident, &builder_ident, &struct_fields)?;

    let struct_impl = struct_impl(&ident, &builder_ident, &struct_fields)?;

    Ok(quote!(
        #struct_builder

        #struct_builder_impl

        #struct_impl
    ))
}

fn struct_builder(
    builder_ident: &Ident,
    struct_fields: &Vec<FieldInfo>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut inner = proc_macro2::TokenStream::new();
    for f in struct_fields.iter() {
        let FieldInfo {
            ident,
            ty,
            field_path_seg,
            ..
        } = f;
        if field_path_seg == &FieldPathSeg::Vec {
            inner.extend(quote!(
                #ident: std::option::Option<Vec<#ty>>,
            ));
        } else {
            inner.extend(quote!(
                #ident: std::option::Option<#ty>,
            ));
        }
    }

    Ok(quote!(
    pub struct #builder_ident {
        #inner
    }
    ))
}

fn struct_builder_impl(
    ident: &Ident,
    builder_ident: &Ident,
    struct_fields: &Vec<FieldInfo>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut builder_impls = proc_macro2::TokenStream::new();

    for f in struct_fields.iter() {
        let FieldInfo {
            ident,
            ty,
            each,
            field_path_seg,
            ..
        } = f;
        let mut is_same_name_gened = false;

        if field_path_seg == &FieldPathSeg::Vec {
            if let Some(each_name) = each {
                builder_impls.extend(quote!(
                    fn #each_name(&mut self, v: #ty) -> &mut Self{
                        if let Some(ref mut arr) = self.#ident {
                            arr.push(v);
                        } else {
                            self.#ident = Some(vec![v]);
                        }
                        self
                    }
                ));

                if each_name == &ident.to_string() {
                    is_same_name_gened = true;
                }
            }
        }

        if !is_same_name_gened {
            if field_path_seg == &FieldPathSeg::Vec {
                builder_impls.extend(quote!(
                    fn #ident(&mut self, #ident: Vec<#ty>) -> &mut Self{
                        self.#ident = std::option::Option::Some(#ident);
                        self
                    }
                ));
            } else {
                builder_impls.extend(quote!(
                    fn #ident(&mut self, #ident: #ty) -> &mut Self{
                        self.#ident = std::option::Option::Some(#ident);
                        self
                    }
                ));
            }
        }
    }

    let mut build_inner = proc_macro2::TokenStream::new();

    for f in struct_fields.iter() {
        let FieldInfo {
            ident,
            field_path_seg,
            ..
        } = f;
        match field_path_seg {
            FieldPathSeg::Normal | FieldPathSeg::Vec => {
                build_inner.extend(quote!(
                    let mut #ident;
                    if let Some(v) = self.#ident.take() {
                        #ident = v;
                    } else {
                        let err = format!("{} field is missing", stringify!(#ident));
                        return std::result::Result::Err(err.into());
                    }
                ));
            }
            FieldPathSeg::Option => {
                build_inner.extend(quote!(
                    let #ident = self.#ident.take();
                ));
            }
        }
    }

    let mut inner = proc_macro2::TokenStream::new();
    for f in struct_fields.iter() {
        let ident = &f.ident;
        inner.extend(quote!(#ident,));
    }

    build_inner.extend(quote!(
        return std::result::Result::Ok(#ident{
            #inner
        });
    ));

    builder_impls.extend(quote!(
    fn build(&mut self) -> std::result::Result<#ident, std::boxed::Box<dyn std::error::Error>> {
        #build_inner
    }
    ));

    Ok(quote!(
    impl #builder_ident {
        #builder_impls
    }
    ))
}

fn struct_impl(
    ident: &Ident,
    builder_ident: &Ident,
    struct_fields: &Vec<FieldInfo>,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut inner = proc_macro2::TokenStream::new();

    for f in struct_fields.iter() {
        let FieldInfo {
            ident,
            field_path_seg,
            ..
        } = f;

        if field_path_seg == &FieldPathSeg::Vec {
            inner.extend(quote!(
                #ident: Some(vec![]),
            ));
        } else {
            inner.extend(quote!(
                #ident: None,
            ));
        }
    }

    Ok(quote!(
    impl #ident {
        fn builder() -> #builder_ident{
            #builder_ident {
                #inner
            }
        }
    }))
}

fn get_struct_fields(ast: &syn::DeriveInput) -> syn::Result<Vec<FieldInfo>> {
    let data = &ast.data;
    if let syn::Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { named, .. }),
        ..
    }) = data
    {
        named
            .iter()
            .map(|f| get_real_field_info(f))
            .collect::<syn::Result<Vec<_>>>()
    } else {
        syn::Result::Err(syn::Error::new_spanned(
            &ast.ident,
            "Must define on a Struct with named fields",
        ))
    }
}

fn get_real_field_info(f: &Field) -> syn::Result<FieldInfo> {
    if let Some(ident) = &f.ident {
        let mut ty = f.ty.clone();
        let mut field_path_seg = FieldPathSeg::Normal;
        let mut each = None;

        if let syn::Type::Path(syn::TypePath { path, .. }) = ty.clone() {
            if let Some(seg) = path.segments.last() {
                if seg.ident == "Option" {
                    field_path_seg = FieldPathSeg::Option;
                } else if seg.ident == "Vec" {
                    field_path_seg = FieldPathSeg::Vec;
                }

                if field_path_seg != FieldPathSeg::Normal {
                    if let syn::PathArguments::AngleBracketed(
                        syn::AngleBracketedGenericArguments { ref args, .. },
                    ) = seg.arguments
                    {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.first() {
                            ty = inner_ty.clone();
                        }
                    }
                }
            }
        }

        // #[builder(each = "arg")]
        for attr in &f.attrs {
            if attr.path().is_ident("builder") {
                let res = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("each") {
                        // this parses the `=`
                        let value = meta.value()?;
                        // this parses `"EarlGrey"`
                        let s: syn::LitStr = value.parse()?;
                        each = Some(Ident::new(&s.value(), s.span()));
                        Ok(())
                    } else {
                        if let syn::Meta::List(ref list) = attr.meta {
                            Err(syn::Error::new_spanned(
                                list,
                                r#"expected `builder(each = "...")`"#,
                            ))
                        } else {
                            Err(syn::Error::new_spanned(
                                attr,
                                r#"expected `builder(each = "...")`"#,
                            ))
                        }
                        // Err(meta.error(r#"expected `builder(each = "...")`"#))
                    }
                });

                if let Err(err) = res {
                    return Err(err);
                }
            }
        }

        Ok(FieldInfo {
            ident: ident.clone(),
            ty,
            field_path_seg,
            each,
        })
    } else {
        Err(syn::Error::new_spanned(f, "no ident"))
    }
}
