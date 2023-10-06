use quote::ToTokens;

pub(crate) struct MatchVisitor {
    pub(crate) err: Option<syn::Error>,
}

impl syn::visit_mut::VisitMut for MatchVisitor {
    fn visit_expr_match_mut(&mut self, i: &mut syn::ExprMatch) {
        let mut sort_attr_idx = None;

        for (idx, attr) in i.attrs.iter().enumerate() {
            if get_path_string(attr.path()) == "sorted" {
                sort_attr_idx = Some(idx);
                break;
            }
        }

        if let Some(sort_attr_idx) = sort_attr_idx {
            i.attrs.remove(sort_attr_idx);

            let mut name_arr: Vec<(String, &dyn ToTokens)> = vec![];

            for arm in &i.arms {
                match &arm.pat {
                    syn::Pat::Ident(ident) => {
                        name_arr.push((ident.ident.to_string(), &ident.ident));
                    }
                    syn::Pat::Path(path) => {
                        name_arr.push((get_path_string(&path.path), &path.path));
                    }
                    syn::Pat::TupleStruct(s) => {
                        name_arr.push((get_path_string(&s.path), &s.path));
                    }
                    syn::Pat::Struct(s) => {
                        name_arr.push((get_path_string(&s.path), &s.path));
                    }
                    syn::Pat::Wild(w) => {
                        name_arr.push(("_".to_string(), &w.underscore_token));
                    }
                    _ => {
                        self.err = Some(syn::Error::new_spanned(
                            &arm.pat,
                            "unsupported by #[sorted]",
                        ));
                        return;
                    }
                }
            }

            let mut sorted_name_arr = name_arr.clone();
            sorted_name_arr.sort_by(|a, b| a.0.cmp(&b.0));

            for (name, sorted_name) in name_arr.iter().zip(sorted_name_arr.iter()) {
                if name.0 != sorted_name.0 {
                    self.err = Some(syn::Error::new_spanned(
                        sorted_name.1,
                        format!("{} should sort before {}", sorted_name.0, name.0),
                    ));
                    break;
                }
            }
        }

        syn::visit_mut::visit_expr_match_mut(self, i);
    }
}

fn get_path_string(path: &syn::Path) -> String {
    let mut result = vec![];

    for s in &path.segments {
        result.push(s.ident.to_string());
    }

    result.join("::")
}
