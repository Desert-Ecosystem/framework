use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, FnArg, ImplItem, ImplItemFn, ItemImpl, ItemStruct, Meta, Pat, PatType,
    Token, Type,
};

fn parse_controller_path(attr: TokenStream) -> String {
    if attr.is_empty() {
        return String::new();
    }
    let meta: Meta = syn::parse(attr).expect("expected `path = \"...\"`");
    match meta {
        Meta::NameValue(nv) if nv.path.is_ident("path") => match &nv.value {
            syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Str(s) => s.value(),
                _ => panic!("expected string literal for path"),
            },
            _ => panic!("expected string literal for path"),
        },
        _ => panic!("expected `path = \"...\"`"),
    }
}

fn route_path_from_attr(attr: TokenStream) -> String {
    let s: syn::LitStr = syn::parse(attr).expect("expected path string like `\"/path\"`");
    s.value()
}

fn method_code(http: &str) -> u8 {
    match http {
        "get" => 0,
        "post" => 1,
        "put" => 2,
        "delete" => 3,
        "patch" => 4,
        _ => 255,
    }
}

fn code_to_ident(code: u8) -> TokenStream2 {
    match code {
        0 => quote! { ::axum::routing::get },
        1 => quote! { ::axum::routing::post },
        2 => quote! { ::axum::routing::put },
        3 => quote! { ::axum::routing::delete },
        4 => quote! { ::axum::routing::patch },
        _ => unreachable!(),
    }
}

fn extract_route_info(attr: &syn::Attribute) -> (String, String) {
    let method_name = attr.path().segments.last().unwrap().ident.to_string();

    let path = match &attr.meta {
        Meta::List(meta_list) => {
            let lit: syn::LitStr =
                syn::parse2(meta_list.tokens.clone()).expect("expected path string");
            lit.value()
        }
        _ => panic!("expected #[method(\"path\")]"),
    };

    (method_name, path)
}

fn is_route_attr(attr: &syn::Attribute) -> bool {
    let ident = attr.path().segments.last().unwrap().ident.to_string();
    matches!(ident.as_str(), "get" | "post" | "put" | "delete" | "patch")
}

/// #[controller(path = "/api")] on struct
fn controller_on_struct(path: String, s: ItemStruct) -> TokenStream {
    let name = &s.ident;

    quote! {
        #s
        impl #name { pub const __CONTROLLER_PATH: &str = #path; }
        impl ::desert_framework::ControllerRoutes for #name {
            const CONTROLLER_PATH: &'static str = #path;
        }
    }
    .into()
}

/// #[controller] on impl block — discovers route methods automatically
fn controller_on_impl(impl_block: ItemImpl) -> TokenStream {
    if impl_block.trait_.is_some() {
        panic!("#[controller] on impl block is only supported for bare impls (not trait impls)");
    }

    let self_type = &impl_block.self_ty;

    let type_name = match self_type.as_ref() {
        Type::Path(type_path) => type_path.path.segments.last().unwrap().ident.clone(),
        _ => panic!("#[controller] on impl block requires a named type"),
    };

    let mut cleaned_methods: Vec<TokenStream2> = Vec::new();
    let mut factory_fns: Vec<TokenStream2> = Vec::new();
    let mut inventory_submits: Vec<TokenStream2> = Vec::new();

    for item in &impl_block.items {
        if let ImplItem::Fn(method) = item {
            let route_attr = method.attrs.iter().find(|a| is_route_attr(a));

            if let Some(attr) = route_attr {
                let (http_method, route_path) = extract_route_info(attr);
                let code = method_code(&http_method);
                let name = &method.sig.ident;
                let is_async = method.sig.asyncness.is_some();
                let router_fn = code_to_ident(code);

                let extra: Vec<&FnArg> = method
                    .sig
                    .inputs
                    .iter()
                    .filter(|a| !matches!(a, FnArg::Receiver(_)))
                    .collect();

                let pats: Vec<&Pat> = extra
                    .iter()
                    .map(|a| match a {
                        FnArg::Typed(PatType { pat, .. }) => pat.as_ref(),
                        _ => unreachable!(),
                    })
                    .collect();

                let tys: Vec<&Type> = extra
                    .iter()
                    .map(|a| match a {
                        FnArg::Typed(PatType { ty, .. }) => ty.as_ref(),
                        _ => unreachable!(),
                    })
                    .collect();

                let closure = if extra.is_empty() {
                    if is_async {
                        quote! { move || async move { state.#name().await } }
                    } else {
                        quote! { move || { state.#name() } }
                    }
                } else if is_async {
                    quote! {
                        move |#(#pats: #tys),*| async move {
                            state.#name(#(#pats),*).await
                        }
                    }
                } else {
                    quote! {
                        move |#(#pats: #tys),*| {
                            state.#name(#(#pats),*)
                        }
                    }
                };

                let factory_name =
                    syn::Ident::new(&format!("__make_route_{}", name), name.span());

                // Cleaned method (without route attribute)
                let non_route_attrs: Vec<_> = method
                    .attrs
                    .iter()
                    .filter(|a| !is_route_attr(a))
                    .collect();
                let vis = &method.vis;
                let sig = &method.sig;
                let block = &method.block;

                cleaned_methods.push(quote! {
                    #(#non_route_attrs)*
                    #vis #sig #block
                });

                // Factory function (type-erased)
                factory_fns.push(quote! {
                    fn #factory_name(
                        state: ::std::sync::Arc<dyn ::std::any::Any + Send + Sync>,
                    ) -> ::axum::routing::MethodRouter<()> {
                        let state = state.downcast::<#type_name>().unwrap();
                        #router_fn(#closure)
                    }
                });

                // inventory::submit!
                inventory_submits.push(quote! {
                    ::desert_framework::inventory::submit! {
                        ::desert_framework::RouteEntry {
                            controller_type_id: ::std::any::TypeId::of::<#type_name>(),
                            path: #route_path,
                            method: #code,
                            make_route: #factory_name,
                        }
                    }
                });
            } else {
                cleaned_methods.push(quote! { #method });
            }
        } else {
            cleaned_methods.push(quote! { #item });
        }
    }

    let defaultness = &impl_block.defaultness;
    let generics = &impl_block.generics;
    let self_ty = &impl_block.self_ty;
    let where_clause = &generics.where_clause;

    quote! {
        #defaultness impl #generics #self_ty #where_clause {
            #(#cleaned_methods)*
        }

        #(#factory_fns)*
        #(#inventory_submits)*
    }
    .into()
}

// ─── #[controller] dispatch ───

#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = item.clone();
    if let Ok(s) = syn::parse::<ItemStruct>(input) {
        let path = parse_controller_path(attr);
        return controller_on_struct(path, s);
    }

    let input = item.clone();
    if let Ok(impl_block) = syn::parse::<ItemImpl>(input) {
        return controller_on_impl(impl_block);
    }

    panic!("#[controller] can only be applied to structs or impl blocks");
}

// ─── Standalone route attributes (for backward compat) ───

fn process_route_method(http: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let method = parse_macro_input!(item as ImplItemFn);
    let name = &method.sig.ident;
    let is_async = method.sig.asyncness.is_some();
    let code = method_code(http);
    let path = route_path_from_attr(attr);

    let extra: Vec<&FnArg> = method
        .sig
        .inputs
        .iter()
        .filter(|a| !matches!(a, FnArg::Receiver(_)))
        .collect();

    let pats: Vec<&Pat> = extra
        .iter()
        .map(|a| match a {
            FnArg::Typed(PatType { pat, .. }) => pat.as_ref(),
            _ => unreachable!(),
        })
        .collect();

    let tys: Vec<&Type> = extra
        .iter()
        .map(|a| match a {
            FnArg::Typed(PatType { ty, .. }) => ty.as_ref(),
            _ => unreachable!(),
        })
        .collect();

    let router_fn = code_to_ident(code);

    let closure = if extra.is_empty() {
        if is_async {
            quote! { move || async move { state.#name().await } }
        } else {
            quote! { move || { state.#name() } }
        }
    } else if is_async {
        quote! {
            move |#(#pats: #tys),*| async move {
                state.#name(#(#pats),*).await
            }
        }
    } else {
        quote! {
            move |#(#pats: #tys),*| {
                state.#name(#(#pats),*)
            }
        }
    };

    let factory_name = syn::Ident::new(&format!("__make_route_{}", name), name.span());
    let method_const = syn::Ident::new(&format!("__ROUTE_METHOD_{}", name), name.span());
    let path_const = syn::Ident::new(&format!("__ROUTE_PATH_{}", name), name.span());

    quote! {
        #method

        #[allow(non_upper_case_globals)]
        pub const #method_const: u8 = #code;
        #[allow(non_upper_case_globals)]
        pub const #path_const: &str = #path;

        pub fn #factory_name(state: std::sync::Arc<Self>) -> ::axum::routing::MethodRouter<()> {
            #router_fn(#closure)
        }
    }
    .into()
}

#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    process_route_method("get", attr, item)
}

#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    process_route_method("post", attr, item)
}

#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    process_route_method("put", attr, item)
}

#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    process_route_method("delete", attr, item)
}

#[proc_macro_attribute]
pub fn patch(attr: TokenStream, item: TokenStream) -> TokenStream {
    process_route_method("patch", attr, item)
}

// ─── impl_routes! (backward compat) ───

struct ImplRoutesInput {
    type_: syn::Path,
    methods: Vec<syn::Ident>,
}

impl Parse for ImplRoutesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let type_: syn::Path = input.parse()?;
        let _: Option<Token![,]> = input.parse()?;
        let content;
        syn::bracketed!(content in input);
        let methods = content.parse_terminated(syn::Ident::parse, Token![,])?;
        Ok(ImplRoutesInput {
            type_,
            methods: methods.into_iter().collect(),
        })
    }
}

#[proc_macro]
pub fn impl_routes(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ImplRoutesInput);
    let ty = &input.type_;
    let methods = &input.methods;

    let entries: Vec<TokenStream2> = methods
        .iter()
        .map(|m| {
            let factory = syn::Ident::new(&format!("__make_route_{}", m), m.span());
            let path_const = syn::Ident::new(&format!("__ROUTE_PATH_{}", m), m.span());

            quote! {
                {
                    let __path_suffix = <#ty>::#path_const;
                    let __full_path = ::std::format!("{}{}", <#ty>::__CONTROLLER_PATH, __path_suffix);
                    let __mr = <#ty>::#factory(state.clone());
                    router = router.route(&__full_path, __mr);
                }
            }
        })
        .collect();

    quote! {
        impl #ty {
            pub fn get_router(self) -> ::axum::Router {
                let state = ::std::sync::Arc::new(self);
                let mut router = ::axum::Router::new();
                #(#entries)*
                router
            }
        }
    }
    .into()
}
