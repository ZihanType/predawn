use from_attr::{AttrsValue, FromAttr};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use quote_use::quote_use;
use syn::{
    parse_quote, spanned::Spanned, Expr, FnArg, ImplItem, ImplItemFn, ItemImpl, Label, PatType,
    Path, Receiver, ReturnType, Type,
};

use crate::method::{Method, ENUM_METHODS};

pub(crate) fn generate(impl_attr: ImplAttr, mut item_impl: ItemImpl) -> syn::Result<TokenStream> {
    if let Some((_, trait_name, _)) = item_impl.trait_ {
        return Err(syn::Error::new(
            trait_name.span(),
            "not supported trait impl",
        ));
    }

    let ImplAttr { paths, middleware } = impl_attr;

    let paths = if !paths.is_empty() {
        paths
    } else {
        default_paths()
    };

    let self_ty = &item_impl.self_ty;

    let mut errors = Vec::new();
    let mut insert_routes_impls = Vec::new();

    item_impl.items.iter_mut().for_each(|impl_item| {
        let f = match impl_item {
            ImplItem::Fn(f) => f,
            _ => return,
        };

        let fn_attr = match ImplFnAttr::remove_attributes(&mut f.attrs) {
            Ok(Some(AttrsValue { value: fn_attr, .. })) => fn_attr,
            Ok(None) => return,
            Err(AttrsValue { value: e, .. }) => {
                errors.push(e);
                return;
            }
        };

        match generate_single_fn_impl(&paths, middleware.as_ref(), self_ty, f, fn_attr) {
            Ok(insert_routes_impl) => insert_routes_impls.push(insert_routes_impl),
            Err(e) => errors.push(e),
        }
    });

    if let Some(e) = errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return Err(e);
    }

    #[cfg(not(feature = "auto-register"))]
    let auto_register = TokenStream::new();

    #[cfg(feature = "auto-register")]
    let auto_register = quote_use! {
        # use predawn::__internal::rudi::Singleton;
        # use predawn::__internal::paste::paste;
        # use std::sync::Arc;
        # use core::any::type_name;
        # use predawn::controller::Controller;

        paste! {
            #[Singleton(name = type_name::<#self_ty>())]
            fn [<#self_ty ToController>](c: #self_ty) -> Arc<dyn Controller> {
                Arc::new(c)
            }
        }
    };

    let expand = quote_use! {
        # use std::sync::Arc;
        # use std::collections::BTreeMap;
        # use std::collections::BTreeMap;
        # use predawn::controller::Controller;
        # use predawn::handler::{Handler, DynHandler};
        # use predawn::normalized_path::NormalizedPath;
        # use predawn::__internal::http::Method;
        # use predawn::__internal::indexmap::IndexMap;
        # use predawn::__internal::rudi::Context;
        # use predawn::openapi::{ReferenceOr, PathItem, Components};

        impl Controller for #self_ty {
            fn insert_routes(
                self: Arc<Self>,
                cx: &mut Context,
                route_table: &mut BTreeMap<NormalizedPath, IndexMap<Method, DynHandler>>,
                paths: &mut BTreeMap<NormalizedPath, ReferenceOr<PathItem>>,
                components: &mut Components,
            ) {
                let this = self;

                #(#insert_routes_impls)*
            }
        }

        #item_impl

        #auto_register
    };

    Ok(expand)
}

fn generate_single_fn_impl<'a>(
    controller_paths: &'a [Expr],
    controller_middeleware: Option<&'a Path>,
    self_ty: &'a Type,
    f: &'a ImplItemFn,
    fn_attr: ImplFnAttr,
) -> syn::Result<TokenStream> {
    if f.sig.asyncness.is_none() {
        return Err(syn::Error::new(f.sig.span(), "the method must be async"));
    }

    let ImplFnAttr {
        paths,
        methods,
        middleware,
    } = fn_attr;

    let method_paths = if !paths.is_empty() {
        paths
    } else {
        default_paths()
    };

    let methods = if !methods.is_empty() {
        methods
    } else {
        ENUM_METHODS.to_vec()
    };

    let fn_name = &f.sig.ident;

    let mut args = f.sig.inputs.iter();
    let first = args.next();
    let last = args.next_back();

    fn first_arg_err(span: Span) -> syn::Error {
        syn::Error::new(span, "the first argument of the method must be `&self`")
    }

    match first {
        Some(FnArg::Receiver(Receiver {
            reference: Some(_),
            mutability: None,
            colon_token: None,
            ..
        })) => {}
        Some(arg) => return Err(first_arg_err(arg.span())),
        None => return Err(first_arg_err(f.sig.paren_token.span.join())),
    }

    let mut arg_idents = Vec::new();

    let mut heads_from_request_head = Vec::new();
    let mut heads_parameters = Vec::new();
    let mut heads_error_responses = Vec::new();

    args.filter_map(|arg| match arg {
        FnArg::Typed(PatType { ty, .. }) => Some(ty),
        _ => None,
    })
    .enumerate()
    .for_each(|(idx, ty)| {
        let arg = format_ident!("a{}", idx);
        arg_idents.push(arg.clone());

        let from_request_head = quote_use! {
            # use predawn::from_request::FromRequestHead;

            let #arg = <#ty as FromRequestHead>::from_request_head(&head).await?;
        };

        let parameters = quote_use! {
            # use predawn::api_request::ApiRequestHead;
            # use predawn::openapi::transform_parameters;

            if let Some(parameters) = <#ty as ApiRequestHead>::parameters(components) {
                operation
                    .parameters
                    .extend(transform_parameters(parameters));
            }
        };

        let error_responses = quote_use! {
            # use predawn::from_request::FromRequestHead;
            # use predawn::response_error::ResponseError;
            # use predawn::openapi::merge_responses;

            merge_responses(
                &mut responses,
                <<#ty as FromRequestHead>::Error as ResponseError>::responses(components),
            );
        };

        heads_from_request_head.push(from_request_head);
        heads_parameters.push(parameters);
        heads_error_responses.push(error_responses);
    });

    let last_from_request;
    let last_parameters;
    let last_request_body;
    let last_error_responses;

    if let Some(FnArg::Typed(PatType { ty, .. })) = last {
        arg_idents.push(format_ident!("last"));

        last_from_request = quote_use! {
            # use predawn::from_request::FromRequest;

            let last = <#ty as FromRequest<_>>::from_request(&head, body).await?;
        };

        last_parameters = quote_use! {
            # use predawn::api_request::ApiRequest;
            # use predawn::openapi::transform_parameters;

            if let Some(parameters) = <#ty as ApiRequest<_>>::parameters(components) {
                operation
                    .parameters
                    .extend(transform_parameters(parameters));
            }
        };

        last_request_body = quote_use! {
            # use predawn::api_request::ApiRequest;
            # use predawn::openapi::transform_request_body;

            operation.request_body = transform_request_body(<#ty as ApiRequest<_>>::request_body(components));
        };

        last_error_responses = quote_use! {
            # use predawn::from_request::FromRequest;
            # use predawn::response_error::ResponseError;
            # use predawn::openapi::merge_responses;

            merge_responses(
                &mut responses,
                <<#ty as FromRequest<_>>::Error as ResponseError>::responses(components),
            );
        };
    } else {
        last_from_request = TokenStream::new();
        last_parameters = TokenStream::new();
        last_request_body = TokenStream::new();
        last_error_responses = TokenStream::new();
    }

    let return_ty = match &f.sig.output {
        ReturnType::Default => quote!(()),
        ReturnType::Type(_, ty) => quote!(#ty),
    };

    let return_error_responses = quote_use! {
        # use predawn::into_response::IntoResponse;
        # use predawn::response_error::ResponseError;
        # use predawn::openapi::merge_responses;

        merge_responses(
            &mut responses,
            <<#return_ty as IntoResponse>::Error as ResponseError>::responses(components),
        );
    };

    let return_responses = quote_use! {
        # use predawn::api_response::ApiResponse;
        # use predawn::openapi::merge_responses;

        if let Some(new) = <#return_ty as ApiResponse>::responses(components) {
            merge_responses(&mut responses, new);
        }
    };

    let extract_from_request = quote! {
        #(#heads_from_request_head)*
        #last_from_request
    };

    let add_controller_middleware = match controller_middeleware {
        None => TokenStream::new(),
        Some(middleware) => {
            quote_use! {
                # use predawn::handler::assert_handler;

                let handler = #middleware(cx, handler);
                assert_handler(&handler);
            }
        }
    };

    let add_method_middleware = match middleware {
        None => TokenStream::new(),
        Some(middleware) => {
            quote_use! {
                # use predawn::handler::assert_handler;

                let handler = #middleware(cx, handler);
                assert_handler(&handler);
            }
        }
    };

    let mut insert_fn_into_multi_path = Vec::new();

    controller_paths.iter().for_each(|controller_path| {
        method_paths.iter().for_each(|method_path| {
            let mut insert_handler_into_multi_method = Vec::new();
            let mut insert_operation_into_multi_method = Vec::new();

            methods.iter().for_each(|method| {
                let uppercase_method = method.as_uppercase_ident();
                let lowercase_method = method.as_lowercase_ident();

                let runtime_panic = quote_use! {
                    # use core::panic;
                    # use core::stringify;

                    panic!("path: `{}`, method: `{}` already exists", path, stringify!(#uppercase_method));
                };

                let insert_handler_into_single_method = quote_use! {
                    # use predawn::__internal::http::Method;
                    # use core::panic;
                    # use core::stringify;

                    map.insert(Method::#uppercase_method, #fn_name.clone())
                        .inspect(|_| { #runtime_panic });
                };

                let insert_operation_into_single_method = quote_use! {
                    # use core::panic;
                    # use core::stringify;

                    if path_item.#lowercase_method.is_some() {
                        #runtime_panic
                    }
                    path_item.#lowercase_method = Some(operation.clone());
                };

                insert_handler_into_multi_method.push(insert_handler_into_single_method);
                insert_operation_into_multi_method.push(insert_operation_into_single_method);
            });

            let convert_single_path = quote_use! {
                # use core::convert::AsRef;
                # use predawn::normalized_path::NormalizedPath;

                let path = NormalizedPath::join(
                    NormalizedPath::new(AsRef::<str>::as_ref(#controller_path)),
                    NormalizedPath::new(AsRef::<str>::as_ref(#method_path)),
                );
            };

            let insert_handler_into_single_path = quote_use! {
                # use std::clone::Clone;

                let map = route_table.entry(Clone::clone(&path)).or_default();
                #(#insert_handler_into_multi_method)*
            };

            let insert_operation_into_single_path = quote_use! {
                # use core::default::Default;
                # use std::clone::Clone;
                # use predawn::openapi::ReferenceOr;

                if let ReferenceOr::Item(path_item) = paths
                    .entry(Clone::clone(&path))
                    .or_insert_with(|| ReferenceOr::Item(Default::default()))
                {
                    #(#insert_operation_into_multi_method)*
                }
            };

            let insert_fn_into_single_path = quote! {
                #convert_single_path
                #insert_handler_into_single_path
                #insert_operation_into_single_path
            };

            insert_fn_into_multi_path.push(insert_fn_into_single_path);
        });
    });

    let create_handler = quote_use! {
        # use std::sync::Arc;
        # use predawn::handler::{DynHandler, handler_fn};

        let #fn_name = {
            let this = this.clone();

            let handler = handler_fn(move |req| {
                let this = this.clone();

                async move {
                    let (head, body) = req.split();

                    #extract_from_request

                    let response = this.#fn_name(#(#arg_idents,)*).await;

                    Ok(response)
                }
            });

            #add_method_middleware
            #add_controller_middleware

            DynHandler::new(handler)
        };
    };

    let create_operation = quote_use! {
        # use core::stringify;
        # use std::collections::BTreeMap;
        # use predawn::openapi::Operation;
        # use predawn::component_id;
        # use predawn::openapi::transform_responses;

        let mut operation = Operation::default();

        #last_request_body

        operation.operation_id = Some(format!("{}.{}", component_id::<#self_ty>(), stringify!(#fn_name)));

        #(#heads_parameters)*
        #last_parameters

        let mut responses = BTreeMap::new();

        #(#heads_error_responses)*
        #last_error_responses
        #return_error_responses
        #return_responses

        operation.responses.responses.extend(transform_responses(responses));
    };

    let label: Label = syn::parse_str(&format!("'{}:", fn_name))?;

    let expand = quote! {
        #[allow(unused_labels)]
        #label {
            #create_handler
            #create_operation
            #(#insert_fn_into_multi_path)*
        }
    };

    Ok(expand)
}

#[derive(FromAttr)]
#[attribute(idents = [controller])]
pub(crate) struct ImplAttr {
    paths: Vec<Expr>,

    middleware: Option<Path>,
}

#[derive(FromAttr)]
#[attribute(idents = [handler])]
struct ImplFnAttr {
    paths: Vec<Expr>,

    methods: Vec<Method>,

    middleware: Option<Path>,
}

fn default_paths() -> Vec<Expr> {
    vec![parse_quote!("")]
}
