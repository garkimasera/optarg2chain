//! Functions for generics handling

use syn::fold::Fold;

pub fn merge_generics(
    impl_original_generics: &syn::Generics,
    func_generics: &syn::Generics,
) -> syn::Generics {
    let mut g = syn::Generics::default();

    for l in func_generics.lifetimes() {
        g.params.push(syn::GenericParam::Lifetime(l.clone()));
    }
    for l in impl_original_generics.lifetimes() {
        g.params.push(syn::GenericParam::Lifetime(l.clone()));
    }
    for t in func_generics.type_params() {
        g.params.push(syn::GenericParam::Type(t.clone()));
    }
    for t in impl_original_generics.type_params() {
        g.params.push(syn::GenericParam::Type(t.clone()));
    }
    let w: Vec<&syn::WherePredicate> = [
        &impl_original_generics.where_clause,
        &func_generics.where_clause,
    ]
    .iter()
    .flat_map(|opt| opt.iter())
    .flat_map(|w| w.predicates.iter())
    .collect();
    if !w.is_empty() {
        let where_clause: syn::WhereClause = syn::parse_quote! {
            where #(#w),*
        };
        g.where_clause = Some(where_clause);
    }

    g
}

pub fn erase_generics(ty: &syn::Type) -> syn::Type {
    let mut ty = ty.clone();
    match ty {
        syn::Type::Path(ref mut path) => {
            for s in &mut path.path.segments {
                s.arguments = syn::PathArguments::None;
            }
            ty
        }
        _ => ty,
    }
}

pub struct SelfReplace<'a>(pub &'a syn::Type);

impl<'a> Fold for SelfReplace<'a> {
    fn fold_type(&mut self, ty: syn::Type) -> syn::Type {
        match ty {
            syn::Type::Path(syn::TypePath {
                qself: None,
                ref path,
            }) => {
                if let Some(ident) = path.get_ident() {
                    if ident.to_string() == "Self" {
                        return self.0.clone();
                    }
                }
            }
            _ => (),
        }
        syn::fold::fold_type(self, ty)
    }
}
