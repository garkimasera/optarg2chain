extern crate proc_macro;

use proc_macro::TokenStream;
// use proc_macro2;
use quote::quote;
use syn::parse::{Parse, ParseStream};

const ATTR_NAME_OPT_ARG: &str = "optarg";
const ATTR_NAME_DEFAULT_ARG: &str = "optarg_default";

const ERR_MSG_EMPTY_ARG: &str = "no arguments";

#[proc_macro_attribute]
pub fn optarg_func(attr: TokenStream, item: TokenStream) -> TokenStream {
    let FnAttr {
        builder_struct_name,
        finish_method_name,
    } = syn::parse_macro_input!(attr as FnAttr);
    let item: syn::ItemFn = syn::parse(item).unwrap();
    let return_type = &item.sig.output;
    let return_marker_type = return_marker_type(&return_type);
    let args: Vec<&syn::PatType> = item
        .sig
        .inputs
        .iter()
        .map(|a| match a {
            syn::FnArg::Typed(t) => t,
            syn::FnArg::Receiver(_) => panic!(),
        })
        .collect();
    let vis = &item.vis;

    let args = parse_typed_args(&args);
    let (impl_generics, ty_generics, where_clause) = item.sig.generics.split_for_impl();
    let (arg_name, req_ident, req_ty, opt_ident, opt_ty, opt_default_value) = separate_args(&args);
    let func_attrs = &item.attrs;

    let mut inner_func = item.clone();
    erase_optarg_attr(&mut inner_func.sig);
    inner_func.vis = syn::Visibility::Inherited;
    let func_name = &inner_func.sig.ident;

    let expanded = quote! {
        #vis struct #builder_struct_name #ty_generics {
            #(#req_ident: #req_ty,)*
            #(#opt_ident: core::option::Option<#opt_ty>,)*
            _result_marker: core::marker::PhantomData<fn() -> #return_marker_type>
        }

        impl #impl_generics #builder_struct_name #ty_generics {
            #(
                #vis fn #opt_ident(mut self, value: #opt_ty) -> Self {
                    self.#opt_ident = Some(value);
                    self
                }
            )*

            #vis fn #finish_method_name(self) #return_type #where_clause {
                #inner_func

                #(
                    let #req_ident: #req_ty = self.#req_ident;
                )*
                #(
                    let #opt_ident: #opt_ty = self.#opt_ident.unwrap_or_else(|| { #opt_default_value });
                )*
                #func_name (
                    #(
                        #arg_name,
                    )*
                )
            }
        }

        #(#func_attrs)*
        #vis fn #func_name #ty_generics (
            #(
                #req_ident: #req_ty,
            )*
        ) -> #builder_struct_name #ty_generics #where_clause {
            #builder_struct_name {
                #(
                    #req_ident,
                )*
                #(
                    #opt_ident: core::option::Option::None,
                )*
                _result_marker: core::marker::PhantomData,
            }
        }
    };

    TokenStream::from(expanded)
}

struct Arg<'a> {
    ident: &'a syn::Ident,
    ty: &'a syn::Type,
    default_value: Option<syn::Expr>,
}

struct FnAttr {
    builder_struct_name: syn::Ident,
    finish_method_name: syn::Ident,
}

impl Parse for FnAttr {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let builder_struct_name: syn::Ident = input.parse().unwrap();
        input.parse::<syn::Token![,]>().unwrap();
        let finish_method_name: syn::Ident = input.parse().unwrap();
        Ok(FnAttr {
            builder_struct_name,
            finish_method_name,
        })
    }
}

fn parse_typed_args<'a>(args: &[&'a syn::PatType]) -> Vec<Arg<'a>> {
    assert!(!args.is_empty(), ERR_MSG_EMPTY_ARG);
    args.iter()
        .map(|arg: &&syn::PatType| {
            let ident: &syn::Ident = match &*arg.pat {
                syn::Pat::Ident(ident) => &ident.ident,
                _ => panic!(),
            };
            let ty: &syn::Type = &*arg.ty;
            let default_value = parse_arg_attr(&arg.attrs, ty);
            Arg {
                ident,
                ty,
                default_value,
            }
        })
        .collect()
}

fn parse_arg_attr(attrs: &[syn::Attribute], ty: &syn::Type) -> Option<syn::Expr> {
    for attr in attrs {
        assert_eq!(attr.style, syn::AttrStyle::Outer);

        if attr.path.is_ident(ATTR_NAME_OPT_ARG) {
            return Some(attr.parse_args().unwrap());
        } else if attr.path.is_ident(ATTR_NAME_DEFAULT_ARG) {
            assert!(attr.tokens.is_empty());
            return Some(syn::parse_quote! {
                <#ty as core::default::Default>::default()
            });
        } else {
            continue;
        }
    }
    None
}

// separate args to (arg name, required ident, ty, optional ident, ty, defalut_value)
fn separate_args<'a>(
    args: &'a [Arg<'a>],
) -> (
    Vec<&'a syn::Ident>,
    Vec<&'a syn::Ident>,
    Vec<&'a syn::Type>,
    Vec<&'a syn::Ident>,
    Vec<&'a syn::Type>,
    Vec<&'a syn::Expr>,
) {
    let mut arg_name = vec![];
    let mut req_ident = vec![];
    let mut req_ty = vec![];
    let mut opt_ident = vec![];
    let mut opt_ty = vec![];
    let mut opt_default_value = vec![];
    for arg in args {
        if arg.default_value.is_none() {
            req_ident.push(arg.ident);
            req_ty.push(arg.ty);
        } else {
            opt_ident.push(arg.ident);
            opt_ty.push(arg.ty);
            opt_default_value.push(arg.default_value.as_ref().unwrap());
        }
        arg_name.push(arg.ident);
    }
    (
        arg_name,
        req_ident,
        req_ty,
        opt_ident,
        opt_ty,
        opt_default_value,
    )
}

fn erase_optarg_attr(sig: &mut syn::Signature) {
    for arg in sig.inputs.iter_mut() {
        match arg {
            syn::FnArg::Typed(pt) => {
                pt.attrs.retain(|attr| {
                    !attr.path.is_ident(ATTR_NAME_DEFAULT_ARG)
                        && !attr.path.is_ident(ATTR_NAME_OPT_ARG)
                });
            }
            _ => (),
        }
    }
}

fn return_marker_type(return_type: &syn::ReturnType) -> syn::Type {
    match return_type {
        syn::ReturnType::Default => {
            syn::parse_quote! { () }
        }
        syn::ReturnType::Type(_arrow, ty) => (**ty).clone(),
    }
}
