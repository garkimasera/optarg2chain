extern crate proc_macro;

mod doc;
mod generics;

use generics::*;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::fold::Fold;
use syn::parse::{Parse, ParseStream};

const ATTR_PREFIX: &str = "optarg";
const ATTR_NAME_OPT_ARG: &str = "optarg";
const ATTR_NAME_DEFAULT_ARG: &str = "optarg_default";
const ATTR_NAME_METHOD: &str = "optarg_method";

const ERR_MSG_EMPTY_ARG: &str = "no arguments";
const ERR_MSG_TRAIT_IMPL: &str = "impl for traits is not supported";
const ERR_IMPLICIT_LIFETIME: &str = "explicit lifetime is neeeded";

#[proc_macro_attribute]
pub fn optarg_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    let FnAttr {
        builder_struct_name,
        terminal_method_name,
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
    let (arg_name, _, req_ident, req_ty, opt_ident, opt_ty, opt_default_value) =
        separate_args(&args);
    let func_attrs = &item.attrs;

    let mut inner_func = item.clone();
    erase_optarg_attr(&mut inner_func.sig);
    inner_func.vis = syn::Visibility::Inherited;
    let func_name = inner_func.sig.ident.clone();
    let inner_func_name = syn::Ident::new("_optarg_inner_func", Span::call_site());
    inner_func.sig.ident = inner_func_name.clone();
    let doc::DocAttrs {
        doc_builder_struct,
        doc_setter,
        doc_terminal_method,
    } = doc::generate_doc(&func_name, &opt_ident);

    let expanded = quote! {
        #doc_builder_struct
        #vis struct #builder_struct_name #ty_generics {
            #(#req_ident: #req_ty,)*
            #(#opt_ident: core::option::Option<#opt_ty>,)*
            _result_marker: core::marker::PhantomData<fn() -> #return_marker_type>
        }

        impl #impl_generics #builder_struct_name #ty_generics {
            #(
                #doc_setter
                #vis fn #opt_ident<_OPTARG_VALUE: core::convert::Into<#opt_ty>>(
                    mut self, value: _OPTARG_VALUE) -> Self {
                    let value = <_OPTARG_VALUE as core::convert::Into<#opt_ty>>::into(value);
                    self.#opt_ident = Some(value);
                    self
                }
            )*

            #doc_terminal_method
            #vis fn #terminal_method_name(self) #return_type #where_clause {
                #inner_func

                #(
                    let #req_ident: #req_ty = self.#req_ident;
                )*
                #(
                    let #opt_ident: #opt_ty = self.#opt_ident.unwrap_or_else(|| { #opt_default_value });
                )*
                #inner_func_name (
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

#[proc_macro_attribute]
pub fn optarg_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item: syn::ItemImpl = syn::parse(item).unwrap();
    assert!(item.trait_.is_none(), ERR_MSG_TRAIT_IMPL);
    let generics = &item.generics;

    let self_ty = &item.self_ty;

    let (optarg_items, normal_items): (Vec<syn::ImplItem>, Vec<syn::ImplItem>) =
        item.items.iter().cloned().partition(|item| match item {
            syn::ImplItem::Method(method) => {
                for attr in &method.attrs {
                    if attr.path.is_ident(ATTR_NAME_METHOD) {
                        return true;
                    }
                }
                false
            }
            _ => false,
        });

    let mut optarg_methods = vec![];
    let mut optarg_structs = vec![];
    let mut optarg_struct_impls = vec![];

    for item in optarg_items {
        match item {
            syn::ImplItem::Method(method) => {
                let (mut optarg_method, optarg_struct, optarg_struct_impl) =
                    optarg_method(method, generics, self_ty);
                optarg_methods.append(&mut optarg_method);
                optarg_structs.push(optarg_struct);
                optarg_struct_impls.push(optarg_struct_impl);
            }
            _ => unreachable!(),
        }
    }

    item.items = normal_items;
    item.items.append(&mut optarg_methods);

    let expanded = quote! {
        #item
        #(#optarg_structs)*
        #(#optarg_struct_impls)*
    };
    TokenStream::from(expanded)
}

fn optarg_method(
    input: syn::ImplItemMethod,
    impl_original_generics: &syn::Generics,
    self_ty: &syn::Type,
) -> (Vec<syn::ImplItem>, syn::ItemStruct, syn::ItemImpl) {
    let (optarg_attrs, other_attrs) = separate_attrs(&input.attrs);
    let FnAttr {
        builder_struct_name,
        terminal_method_name,
    } = optarg_attrs[0].parse_args().unwrap();
    let vis = input.vis;
    let mut self_replace = SelfReplace(self_ty);
    let return_type = self_replace.fold_return_type(input.sig.output.clone());
    let return_marker_type = return_marker_type(&return_type);
    let method_name = &input.sig.ident;
    let (_, method_ty_generics, _) = input.sig.generics.split_for_impl();
    let merged_generics = merge_generics(impl_original_generics, &input.sig, self_ty);
    let (impl_generics, ty_generics, where_clause) = merged_generics.split_for_impl();
    let (original_receiver, receiver_ident, receiver_ty, args) =
        separate_receiver(&input.sig, self_ty);

    let replaced_args: Vec<syn::PatType> = args
        .iter()
        .map(|pt| self_replace.fold_pat_type((*pt).clone()))
        .collect();
    let args: Vec<&syn::PatType> = replaced_args.iter().map(|pt| pt).collect();
    let args = parse_typed_args(&args);
    let (arg_name, arg_ty, req_ident, req_ty, opt_ident, opt_ty, opt_default_value) =
        separate_args(&args);

    let insert_self = if receiver_ident.is_empty() {
        vec![]
    } else {
        vec![quote! { #(#receiver_ident: self)* }]
    };

    let inner_method_ident = syn::Ident::new(
        &format!("_optarg_inner_{}", method_name),
        method_name.span(),
    );
    let inner_method_block = &input.block;
    let doc::DocAttrs {
        doc_builder_struct,
        doc_setter,
        doc_terminal_method,
    } = doc::generate_doc(&method_name, &opt_ident);

    let inner_method: syn::ImplItem = syn::parse_quote! {
        fn #inner_method_ident #method_ty_generics (
            #(#original_receiver,)*
            #(#arg_name: #arg_ty,)*) #return_type #where_clause #inner_method_block
    };

    let item_struct: syn::ItemStruct = syn::parse_quote! {
        #doc_builder_struct
        #vis struct #builder_struct_name #ty_generics {
            #(#receiver_ident: #receiver_ty,)*
            #(#req_ident: #req_ty,)*
            #(#opt_ident: core::option::Option<#opt_ty>,)*
            _result_marker: core::marker::PhantomData<fn() -> #return_marker_type>
        }
    };

    let impl_item: syn::ImplItem = syn::parse_quote! {
        #(#other_attrs)*
        #vis fn #method_name #method_ty_generics (
            #(#original_receiver,)*
            #(#req_ident: #req_ty,)*
        ) -> #builder_struct_name #ty_generics {
            #builder_struct_name {
                #(#insert_self,)*
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

    let self_ty_no_generics = erase_generics(self_ty);

    let struct_impl: syn::ItemImpl = syn::parse_quote! {
        impl #impl_generics #builder_struct_name #ty_generics {
            #(
                #doc_setter
                #vis fn #opt_ident<_OPTARG_VALUE: core::convert::Into<#opt_ty>>(
                    mut self, value: _OPTARG_VALUE) -> Self {
                    let value = <_OPTARG_VALUE as core::convert::Into<#opt_ty>>::into(value);
                    self.#opt_ident = Some(value);
                    self
                }
            )*

            #doc_terminal_method
            #vis fn #terminal_method_name(self) #return_type #where_clause {
                #(
                    let #receiver_ident: #receiver_ty = self.#receiver_ident;
                )*
                #(
                    let #req_ident: #req_ty = self.#req_ident;
                )*
                #(
                    let #opt_ident: #opt_ty = self.#opt_ident.unwrap_or_else(|| { #opt_default_value });
                )*
                #self_ty_no_generics::#inner_method_ident( #(#receiver_ident,)* #(#arg_name, )* )
            }
        }
    };

    (vec![impl_item, inner_method], item_struct, struct_impl)
}

struct Arg<'a> {
    ident: &'a syn::Ident,
    ty: &'a syn::Type,
    default_value: Option<syn::Expr>,
}

struct FnAttr {
    builder_struct_name: syn::Ident,
    terminal_method_name: syn::Ident,
}

impl Parse for FnAttr {
    fn parse(input: ParseStream) -> syn::parse::Result<Self> {
        let builder_struct_name: syn::Ident = input.parse().unwrap();
        input.parse::<syn::Token![,]>().unwrap();
        let terminal_method_name: syn::Ident = input.parse().unwrap();
        Ok(FnAttr {
            builder_struct_name,
            terminal_method_name,
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
    Vec<&'a syn::Type>,
    Vec<&'a syn::Ident>,
    Vec<&'a syn::Type>,
    Vec<&'a syn::Ident>,
    Vec<&'a syn::Type>,
    Vec<&'a syn::Expr>,
) {
    let mut arg_name = vec![];
    let mut arg_ty = vec![];
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
        arg_ty.push(arg.ty);
    }
    (
        arg_name,
        arg_ty,
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

fn separate_attrs<'a>(
    attrs: &'a [syn::Attribute],
) -> (Vec<&'a syn::Attribute>, Vec<&'a syn::Attribute>) {
    let mut optarg_attrs = vec![];
    let mut other_attrs = vec![];

    for attr in attrs {
        if let Some(ident) = attr.path.get_ident().map(|ident| ident.to_string()) {
            if ident.starts_with(ATTR_PREFIX) {
                optarg_attrs.push(attr);
                continue;
            }
        }
        other_attrs.push(attr);
    }
    (optarg_attrs, other_attrs)
}
// Returns (receiver, reciever ident, receiver type, other args)
fn separate_receiver<'a>(
    sig: &'a syn::Signature,
    self_ty: &syn::Type,
) -> (
    Vec<syn::Receiver>,
    Vec<syn::Ident>,
    Vec<syn::Type>,
    Vec<&'a syn::PatType>,
) {
    let mut receiver = None;
    let mut args = vec![];
    for arg in &sig.inputs {
        match arg {
            syn::FnArg::Receiver(r) => {
                assert!(receiver.is_none());
                receiver = Some(r);
            }
            syn::FnArg::Typed(t) => {
                args.push(t);
            }
        }
    }
    let mut new_receiver: Vec<syn::Receiver> = vec![];
    let (receiver_ident, receiver_ty) = if let Some(receiver) = receiver {
        let self_ident = syn::Ident::new("_optarg_self", Span::call_site());
        let receiver_ty: syn::Type = match (&receiver.reference, &receiver.mutability) {
            (Some((_, None)), None) => {
                panic!(ERR_IMPLICIT_LIFETIME);
            }
            (Some((_, Some(lifetime))), None) => {
                new_receiver = vec![syn::parse_quote! { &#lifetime self }];
                syn::parse_quote! { &#lifetime #self_ty }
            }
            (Some((_, None)), Some(_)) => {
                panic!(ERR_IMPLICIT_LIFETIME);
            }
            (Some((_, Some(lifetime))), Some(_)) => {
                new_receiver = vec![syn::parse_quote! { &#lifetime mut self }];
                syn::parse_quote! { &#lifetime mut #self_ty }
            }
            (None, is_mut) => {
                if is_mut.is_some() {
                    new_receiver = vec![syn::parse_quote! { mut self }];
                } else {
                    new_receiver = vec![syn::parse_quote! { self }];
                }
                syn::parse_quote! { #self_ty }
            }
        };
        (vec![self_ident], vec![receiver_ty])
    } else {
        (vec![], vec![])
    };
    (new_receiver, receiver_ident, receiver_ty, args)
}
