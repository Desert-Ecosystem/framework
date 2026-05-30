use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, FnArg, ImplItemFn, ItemStruct, Meta, Pat, PatType, Token, Type,
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

/// #[controller(path = "/api")] on struct
#[proc_macro_attribute]
pub fn controller(attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_controller_path(attr);
    let s = parse_macro_input!(item as ItemStruct);
    let name = &s.ident;

    quote! {
        #s
        impl #name { pub const __CONTROLLER_PATH: &str = #path; }
    }
    .into()
}

/// Generate cleaned method + metadata consts + handler factory
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

/// impl_routes!(MyCtrl, [hello, login])
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
