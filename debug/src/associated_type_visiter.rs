use std::collections::HashMap;

use syn::{
    visit::{self, Visit},
    TypePath,
};

pub struct AssociatedTypeVisiter {
    pub generic_type_arr: Vec<String>,
    pub associated_type_map: HashMap<String, Vec<TypePath>>,
}

impl<'ast> Visit<'ast> for AssociatedTypeVisiter {
    fn visit_type_path(&mut self, i: &'ast TypePath) {
        if i.path.segments.len() >= 2 {
            let generic_type_name = i.path.segments[0].ident.to_string();
            if self.generic_type_arr.contains(&generic_type_name) {
                self.associated_type_map
                    .entry(generic_type_name)
                    .or_insert(vec![])
                    .push(i.clone());
            }
        }

        visit::visit_type_path(self, i);
    }
}
