//! Converts optional arguments to chaining style.
//!
//! Rust doesn't have optional or named arguments. This crate provides macros to convert optional arguments given by attributes to method chaining style instead.
//!
//! # Function with optional arguments
//!
//! Specify `optarg_fn` for a function with 2 arguments, the name of builder struct and terminal method. Use `#[optarg(expr)]` to give default value for an argument. `#[optarg_default]` gives default value by [`Default::default()`](https://doc.rust-lang.org/std/default/trait.Default.html).
//! ```
//! use optarg2chain::optarg_fn;
//!
//! // specify optarg_fn attribute with builder struct name and terminal method name
//! #[optarg_fn(JoinStringBuilder, exec)]
//! fn join_strings(
//!     mut a: String,                         // Required argument, no default value
//!     #[optarg_default] b: String,           // String::default() is the default value to b
//!     #[optarg("ccc".to_owned())] c: String, // "ccc".to_owned() is the default value to c
//! ) -> String {
//!     a.push_str(&b);
//!     a.push_str(&c);
//!     a
//! }
//! ```
//! You can use the generated function as below:
//! ```
//! # use optarg2chain::optarg_fn;
//! # #[optarg_fn(JoinStringBuilder, exec)]
//! # fn join_strings(
//! #     mut a: String,                         // Required argument, no default value
//! #     #[optarg_default] b: String,           // String::default() is the default value to b
//! #     #[optarg("ccc".to_owned())] c: String, // "ccc".to_owned() is the default value to c
//! # ) -> String {
//! #     a.push_str(&b);
//! #     a.push_str(&c);
//! #     a
//! # }
//! assert_eq!(join_strings("aaa".to_owned()).exec(), "aaaccc"); // Use default values
//! assert_eq!(
//!     join_strings("xxx".to_owned())
//!         .b("yyy".to_owned()) // Pass a value to `b` explicitly
//!         .c("zzz".to_owned()) // Pass a value to `c` explicitly
//!         .exec(),
//!     "xxxyyyzzz"
//! );
//! ```
//!
//! # Method with optional arguments
//! Use `#[optarg_impl]` and `#[optarg_method(BuilderStructName, terminal_method_name)]` for methods in `impl`
//! ```
//! use optarg2chain::optarg_impl;
//!
//! struct MyVec<T> {
//!     data: Vec<T>,
//! }
//!
//! #[optarg_impl]
//! impl<T: Default + Copy> MyVec<T> {
//!     #[optarg_method(MyVecGetOr, get)]
//!     fn get_or<'a>(&'a self, i: usize, #[optarg_default] other: T) -> T {
//!         self.data.get(i).copied().unwrap_or(other)
//!     }
//! }
//! ```
//! You can use this as below:
//! ```
//! # use optarg2chain::optarg_impl;
//! # struct MyVec<T> {
//! #     data: Vec<T>,
//! # }
//! #
//! # #[optarg_impl]
//! # impl<T: Default + Copy> MyVec<T> {
//! #     #[optarg_method(MyVecGetOr, get)]
//! #     fn get_or<'a>(&'a self, i: usize, #[optarg_default] other: T) -> T {
//! #         self.data.get(i).copied().unwrap_or(other)
//! #     }
//! # }
//! let myvec = MyVec { data: vec![2, 4, 6] };
//! assert_eq!(myvec.get_or(1).get(), 4);
//! assert_eq!(myvec.get_or(10).get(), 0);
//! assert_eq!(myvec.get_or(10).other(42).get(), 42);
//! ```

extern crate proc_macro;

mod doc;
mod generics;

use generics::*;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::fold::Fold;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Error, Result};

const ATTR_PREFIX: &str = "optarg";
const ATTR_NAME_OPT_ARG: &str = "optarg";
const ATTR_NAME_DEFAULT_ARG: &str = "optarg_default";
const ATTR_NAME_METHOD: &str = "optarg_method";

const INNER_SELF_VAR: &str = "_optarg_self";

const ERR_MSG_TRAIT_IMPL: &str = "(optarg2chain) impl for traits is not supported";
const ERR_MSG_IMPLICIT_LIFETIME: &str = "(optarg2chain) explicit lifetime is neeeded";
const ERR_MSG_UNDERSCORE_ARG: &str = "(optarg2chain) `_` cannot be used for this argument name";
const ERR_MSG_UNUSABLE_PAT: &str = "(optarg2chain) unusable pattern found";

/// Generates a builder struct and methods for the specified function.
#[proc_macro_attribute]
pub fn optarg_fn(attr: TokenStream, item: TokenStream) -> TokenStream {
    let FnAttr {
        builder_struct_name,
        terminal_method_name,
    } = syn::parse_macro_input!(attr as FnAttr);
    let item: syn::ItemFn = syn::parse_macro_input!(item);
    if let Err(e) = check_sig(&item.sig) {
        return TokenStream::from(e.to_compile_error());
    }
    let return_type = &item.sig.output;
    let struct_marker_type = generics::generate_type_holder(&item.sig.generics);
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
    let async_ = &item.sig.asyncness;
    let await_ = if async_.is_some() {
        Some(quote! { .await })
    } else {
        None
    };

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

    TokenStream::from(quote! {
        #doc_builder_struct
        #vis struct #builder_struct_name #ty_generics {
            #(#req_ident: #req_ty,)*
            #(#opt_ident: core::option::Option<#opt_ty>,)*
            _optarg_marker: #struct_marker_type
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
            #vis #async_ fn #terminal_method_name(self) #return_type #where_clause {
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
                #await_
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
                _optarg_marker: core::marker::PhantomData,
            }
        }
    })
}

/// This attribute is used with `optarg_method` attribute.
/// Specify `#[optarg_method(BuilderStructName, terminal_method_name)]` to target methods for code generation.
#[proc_macro_attribute]
pub fn optarg_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item: syn::ItemImpl = syn::parse_macro_input!(item);
    if let Some(trait_) = &item.trait_ {
        let err = Error::new(trait_.1.span(), ERR_MSG_TRAIT_IMPL);
        return TokenStream::from(err.to_compile_error());
    }
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
            syn::ImplItem::Method(method) => match optarg_method(method, generics, self_ty) {
                Ok((mut optarg_method, optarg_struct, optarg_struct_impl)) => {
                    optarg_methods.append(&mut optarg_method);
                    optarg_structs.push(optarg_struct);
                    optarg_struct_impls.push(optarg_struct_impl);
                }
                Err(e) => {
                    return TokenStream::from(e.to_compile_error());
                }
            },
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
) -> Result<(Vec<syn::ImplItem>, syn::ItemStruct, syn::ItemImpl)> {
    check_sig(&input.sig)?;
    let (optarg_attrs, other_attrs) = separate_attrs(&input.attrs);
    let FnAttr {
        builder_struct_name,
        terminal_method_name,
    } = optarg_attrs[0].parse_args().unwrap();
    let vis = input.vis;
    let mut self_replace = SelfReplace(self_ty);
    let return_type = self_replace.fold_return_type(input.sig.output.clone());
    let method_name = &input.sig.ident;
    let merged_generics = merge_generics(impl_original_generics, &input.sig, self_ty);
    let (impl_generics, ty_generics, where_clause) = merged_generics.split_for_impl();
    let (original_receiver, receiver_ident, receiver_ty, args) =
        separate_receiver(&input.sig, self_ty)?;
    let struct_marker_type = generics::generate_type_holder(&merged_generics);

    let replaced_args: Vec<syn::PatType> = args
        .iter()
        .map(|pt| self_replace.fold_pat_type((*pt).clone()))
        .collect();
    let args: Vec<&syn::PatType> = replaced_args.iter().map(|pt| pt).collect();
    let args = parse_typed_args(&args);
    let (arg_name, arg_ty, req_ident, req_ty, opt_ident, opt_ty, opt_default_value) =
        separate_args(&args);
    let async_ = &input.sig.asyncness;
    let await_ = if async_.is_some() {
        Some(quote! { .await })
    } else {
        None
    };

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

    let mut inner_method: syn::ImplItemMethod = syn::parse_quote! {
        #async_ fn #inner_method_ident (
            #(#original_receiver,)*
            #(#arg_name: #arg_ty,)*) #return_type #where_clause #inner_method_block
    };
    inner_method.sig.generics = input.sig.generics.clone();
    let inner_method: syn::ImplItem = inner_method.into();

    let item_struct: syn::ItemStruct = syn::parse_quote! {
        #doc_builder_struct
        #vis struct #builder_struct_name #ty_generics {
            #(#receiver_ident: #receiver_ty,)*
            #(#req_ident: #req_ty,)*
            #(#opt_ident: core::option::Option<#opt_ty>,)*
            _optarg_marker: #struct_marker_type,
        }
    };

    let mut new_method: syn::ImplItemMethod = syn::parse_quote! {
        #(#other_attrs)*
        #vis fn #method_name (
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
                _optarg_marker: core::marker::PhantomData,
            }
        }
    };
    new_method.sig.generics = input.sig.generics.clone();
    let new_method: syn::ImplItem = new_method.into();

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
            #vis #async_ fn #terminal_method_name(self) #return_type #where_clause {
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
                #await_
            }
        }
    };

    Ok((vec![new_method, inner_method], item_struct, struct_impl))
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
        let builder_struct_name: syn::Ident = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let terminal_method_name: syn::Ident = input.parse()?;
        Ok(FnAttr {
            builder_struct_name,
            terminal_method_name,
        })
    }
}

fn parse_typed_args<'a>(args: &[&'a syn::PatType]) -> Vec<Arg<'a>> {
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
) -> Result<(
    Vec<syn::FnArg>,
    Vec<syn::Ident>,
    Vec<syn::Type>,
    Vec<&'a syn::PatType>,
)> {
    let mut receiver = None;
    let mut typed_self: Option<&syn::PatType> = None;
    let mut args = vec![];
    for arg in &sig.inputs {
        match arg {
            syn::FnArg::Receiver(r) => {
                assert!(receiver.is_none());
                receiver = Some(r);
            }
            syn::FnArg::Typed(t) => {
                if let syn::Pat::Ident(syn::PatIdent { ref ident, .. }) = *t.pat {
                    if ident == "self" {
                        // Handles typed self like `self: Box<Self>`
                        assert!(typed_self.is_none());
                        typed_self = Some(t);
                        continue;
                    }
                }
                args.push(t);
            }
        }
    }
    let mut new_receiver: Vec<syn::FnArg> = vec![];
    let (receiver_ident, receiver_ty) = if let Some(receiver) = receiver {
        let self_ident = syn::Ident::new(INNER_SELF_VAR, Span::call_site());
        let receiver_ty: syn::Type = match (&receiver.reference, &receiver.mutability) {
            (Some((_, None)), _) => {
                return Err(Error::new(receiver.span(), ERR_MSG_IMPLICIT_LIFETIME));
            }
            (Some((_, Some(lifetime))), None) => {
                new_receiver = vec![syn::parse_quote! { &#lifetime self }];
                syn::parse_quote! { &#lifetime #self_ty }
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
    } else if let Some(pt) = typed_self {
        let self_ident = syn::Ident::new(INNER_SELF_VAR, Span::call_site());
        let mut self_replace = SelfReplace(self_ty);
        let receiver_ty = self_replace.fold_type((*pt.ty).clone());
        new_receiver.push(syn::FnArg::from(pt.clone()));
        (vec![self_ident], vec![receiver_ty])
    } else {
        (vec![], vec![])
    };
    Ok((new_receiver, receiver_ident, receiver_ty, args))
}

// Checks function signature and returns error if exists
fn check_sig(sig: &syn::Signature) -> Result<()> {
    for arg in &sig.inputs {
        match arg {
            syn::FnArg::Typed(t) => match *t.pat {
                syn::Pat::Ident(syn::PatIdent { ref ident, .. }) => {
                    if ident == INNER_SELF_VAR {
                        return Err(Error::new(
                            ident.span(),
                            format!("(optarg2chain) {} is reserved name", INNER_SELF_VAR),
                        ));
                    }
                }
                syn::Pat::Wild(ref w) => {
                    return Err(Error::new(w.span(), ERR_MSG_UNDERSCORE_ARG));
                }
                _ => {
                    return Err(Error::new(t.span(), ERR_MSG_UNUSABLE_PAT));
                }
            },
            _ => (),
        }
    }
    Ok(())
}
