pub struct DocAttrs {
    pub doc_builder_struct: syn::Attribute,
    pub doc_setter: Vec<syn::Attribute>,
    pub doc_terminal_method: syn::Attribute,
}

/// Generates document attributes for struct and methods
pub fn generate_doc(func_name: &syn::Ident, opt_ident: &[&syn::Ident]) -> DocAttrs {
    let msg = format!("Argument builder struct for `{}`.", func_name);
    let doc_builder_struct = syn::parse_quote! { #[doc = #msg] };

    let doc_setter: Vec<syn::Attribute> = opt_ident
        .iter()
        .map(|i| {
            let msg = format!("Sets optional argument `{}`.", i);
            let a: syn::Attribute = syn::parse_quote! { #[doc = #msg] };
            a
        })
        .collect();

    let msg = format!("Executes `{}` and get the result.", func_name);
    let doc_terminal_method = syn::parse_quote! { #[doc = #msg] };

    DocAttrs {
        doc_builder_struct,
        doc_setter,
        doc_terminal_method,
    }
}
