//! Functions for generics handling

use syn::fold::Fold;

pub fn merge_generics(
    impl_original_generics: &syn::Generics,
    method_sig: &syn::Signature,
    self_ty: &syn::Type,
) -> syn::Generics {
    let mut g = syn::Generics::default();
    let mut self_replace = SelfReplace(self_ty);
    let filter = TypeFilter::new(self_replace.fold_signature(method_sig.clone()));
    let method_generics: &syn::Generics = &method_sig.generics;

    for l in impl_original_generics.lifetimes() {
        if !filter.has_receiver && !filter.has_lifetime(&l.lifetime) {
            continue;
        }
        g.params.push(syn::GenericParam::Lifetime(l.clone()));
    }
    for l in method_generics.lifetimes() {
        g.params.push(syn::GenericParam::Lifetime(l.clone()));
    }
    for t in impl_original_generics.type_params() {
        if !filter.has_receiver && !filter.has_type(&t.ident) {
            continue;
        }
        g.params.push(syn::GenericParam::Type(t.clone()));
    }
    for t in method_generics.type_params() {
        g.params.push(syn::GenericParam::Type(t.clone()));
    }
    let w: Vec<&syn::WherePredicate> = [
        &impl_original_generics.where_clause,
        &method_generics.where_clause,
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
        if let Some(ident) = get_ident_from_type(&ty) {
            if ident.to_string() == "Self" {
                return self.0.clone();
            }
        }
        syn::fold::fold_type(self, ty)
    }
}

#[derive(Default, Debug)]
struct TypeFilter {
    types: Vec<syn::Ident>,
    lifetimes: Vec<syn::Lifetime>,
    has_receiver: bool,
}

#[derive(Default, Debug)]
struct TypeFilterBuilder(TypeFilter);

impl TypeFilter {
    fn new(sig: syn::Signature) -> TypeFilter {
        let mut builder = TypeFilterBuilder::default();
        builder.fold_signature(sig);
        builder.0
    }

    fn has_type(&self, ty: &syn::Ident) -> bool {
        for i in &self.types {
            if ty == i {
                return true;
            }
        }
        false
    }

    fn has_lifetime(&self, lifetime: &syn::Lifetime) -> bool {
        for l in &self.lifetimes {
            if lifetime == l {
                return true;
            }
        }
        false
    }
}

impl Fold for TypeFilterBuilder {
    fn fold_lifetime(&mut self, lifetime: syn::Lifetime) -> syn::Lifetime {
        self.0.lifetimes.push(lifetime.clone());
        lifetime
    }

    fn fold_type(&mut self, ty: syn::Type) -> syn::Type {
        if let Some(ident) = get_ident_from_type(&ty) {
            self.0.types.push(ident.clone());
            return ty;
        }
        syn::fold::fold_type(self, ty)
    }

    fn fold_receiver(&mut self, receiver: syn::Receiver) -> syn::Receiver {
        self.0.has_receiver = true;
        receiver
    }
}

fn get_ident_from_type(ty: &syn::Type) -> Option<&syn::Ident> {
    match ty {
        syn::Type::Path(syn::TypePath {
            qself: None,
            ref path,
        }) => {
            if let Some(ident) = path.get_ident() {
                return Some(ident);
            }
        }
        _ => (),
    }
    None
}
