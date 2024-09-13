use from_attr::{AttrsValue, FromAttr, Map};
use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use quote_use::quote_use;
use syn::{
    parse_quote, spanned::Spanned, Expr, FnArg, ImplItem, ImplItemFn, ItemImpl, Label, PatType,
    Path, Receiver, ReturnType, Type,
};

use crate::{
    method::{Method, ENUM_METHODS},
    util,
};

#[derive(FromAttr)]
#[attribute(idents = [controller])]
pub(crate) struct ControllerAttr {
    paths: Vec<Expr>,
    middleware: Option<Path>,
    tags: Vec<Type>,
    security: Vec<Map<Type, Vec<String>>>,
}

#[derive(FromAttr)]
#[attribute(idents = [handler])]
struct MethodAttr {
    paths: Vec<Expr>,
    methods: Vec<Method>,
    middleware: Option<Path>,
    tags: Vec<Type>,
    security: Vec<Map<Type, Vec<String>>>,
}

fn default_paths() -> Vec<Expr> {
    vec![parse_quote!("")]
}

pub(crate) fn generate(
    controller_attr: ControllerAttr,
    mut item_impl: ItemImpl,
) -> syn::Result<TokenStream> {
    if let Some((_, trait_name, _)) = item_impl.trait_ {
        return Err(syn::Error::new(
            trait_name.span(),
            "not supported trait impl",
        ));
    }

    let ControllerAttr {
        paths,
        middleware,
        tags,
        security,
    } = controller_attr;

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

        let method_attr = match MethodAttr::remove_attributes(&mut f.attrs) {
            Ok(Some(AttrsValue {
                value: method_attr, ..
            })) => method_attr,
            Ok(None) => return,
            Err(AttrsValue { value: e, .. }) => {
                errors.push(e);
                return;
            }
        };

        match generate_single_fn_impl(
            &paths,
            middleware.as_ref(),
            &tags,
            &security,
            self_ty,
            f,
            method_attr,
        ) {
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
        # use std::vec::Vec;
        # use predawn::controller::Controller;
        # use predawn::handler::DynHandler;
        # use predawn::normalized_path::NormalizedPath;
        # use predawn::__internal::indexmap::IndexMap;
        # use predawn::__internal::http::Method;
        # use predawn::__internal::rudi::Context;
        # use predawn::openapi::{SecurityScheme, Operation, Tag, Schema};

        impl Controller for #self_ty {
            fn insert_routes(
                self: Arc<Self>,
                cx: &mut Context,
                route_table: &mut IndexMap<NormalizedPath, Vec<(Method, DynHandler)>>,
                paths: &mut IndexMap<NormalizedPath, Vec<(Method, Operation)>>,
                schemas: & mut BTreeMap<String, Schema>,
                schemas_in_progress: &mut Vec<String>,
                security_schemes: &mut BTreeMap<&'static str, (&'static str, SecurityScheme)>,
                tags: &mut BTreeMap<&'static str, (&'static str, Tag)>,
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
    controller_tags: &'a [Type],
    controller_security: &'a [Map<Type, Vec<String>>],
    self_ty: &'a Type,
    f: &'a ImplItemFn,
    method_attr: MethodAttr,
) -> syn::Result<TokenStream> {
    if f.sig.asyncness.is_none() {
        return Err(syn::Error::new(f.sig.span(), "the method must be async"));
    }

    let MethodAttr {
        paths,
        methods,
        middleware: method_middleware,
        tags: method_tags,
        security: method_security,
    } = method_attr;

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

    let security = if !method_security.is_empty() {
        &method_security
    } else {
        controller_security
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
        let arg_ident = format_ident!("a{}", idx);

        let from_request_head = quote_use! {
            # use predawn::from_request::FromRequestHead;

            let #arg_ident = <#ty as FromRequestHead>::from_request_head(&mut head).await?;
        };

        arg_idents.push(arg_ident);

        let parameters = quote_use! {
            # use predawn::api_request::ApiRequestHead;
            # use predawn::openapi::transform_parameters;

            if let Some(parameters) = <#ty as ApiRequestHead>::parameters(schemas, schemas_in_progress) {
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
                <<#ty as FromRequestHead>::Error as ResponseError>::responses(schemas, schemas_in_progress),
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

            let last = <#ty as FromRequest<_>>::from_request(&mut head, body).await?;
        };

        last_parameters = quote_use! {
            # use predawn::api_request::ApiRequest;
            # use predawn::openapi::transform_parameters;

            if let Some(parameters) = <#ty as ApiRequest<_>>::parameters(schemas, schemas_in_progress) {
                operation
                    .parameters
                    .extend(transform_parameters(parameters));
            }
        };

        last_request_body = quote_use! {
            # use predawn::api_request::ApiRequest;
            # use predawn::openapi::transform_request_body;

            operation.request_body = transform_request_body(<#ty as ApiRequest<_>>::request_body(schemas, schemas_in_progress));
        };

        last_error_responses = quote_use! {
            # use predawn::from_request::FromRequest;
            # use predawn::response_error::ResponseError;
            # use predawn::openapi::merge_responses;

            merge_responses(
                &mut responses,
                <<#ty as FromRequest<_>>::Error as ResponseError>::responses(schemas, schemas_in_progress),
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
            <<#return_ty as IntoResponse>::Error as ResponseError>::responses(schemas, schemas_in_progress),
        );
    };

    let return_responses = quote_use! {
        # use predawn::api_response::ApiResponse;
        # use predawn::openapi::merge_responses;

        if let Some(new) = <#return_ty as ApiResponse>::responses(schemas, schemas_in_progress) {
            merge_responses(&mut responses, new);
        }
    };

    let extract_from_request = quote! {
        #(#heads_from_request_head)*
        #last_from_request
    };

    let add_controller_middleware = controller_middeleware.map(|middleware| {
        quote_use! {
            # use predawn::handler::assert_handler;

            let handler = #middleware(cx, handler);
            assert_handler(&handler);
        }
    });

    let add_method_middleware = method_middleware.map(|middleware| {
        quote_use! {
            # use predawn::handler::assert_handler;

            let handler = #middleware(cx, handler);
            assert_handler(&handler);
        }
    });

    let (summary, description) = util::extract_summary_and_description(&f.attrs);

    let add_summary = if summary.is_empty() {
        TokenStream::new()
    } else {
        let summary = util::generate_string_expr(&summary);
        quote! {
            operation.summary = Some(#summary);
        }
    };

    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = util::generate_string_expr(&description);
        quote! {
            operation.description = Some(#description);
        }
    };

    let add_tags = if controller_tags.is_empty() && method_tags.is_empty() {
        TokenStream::new()
    } else {
        let insert_tags = controller_tags.iter().chain(method_tags.iter()).map(|ty| {
            quote_use! {
                # use core::any::type_name;
                # use std::string::ToString;
                # use predawn::Tag;

                let tag_type_name = type_name::<#ty>();
                let tag_name = <#ty as Tag>::NAME;

                if !tags.contains_key(tag_type_name) {
                    tags.insert(
                        tag_type_name,
                        (tag_name, <#ty as Tag>::create())
                    );
                }

                op_tags.insert(ToString::to_string(tag_name));
            }
        });

        quote_use! {
            # use std::collections::BTreeSet;
            # use std::vec::Vec;

            let mut op_tags = BTreeSet::new();
            #(#insert_tags)*
            operation.tags.extend(op_tags);
        }
    };

    let add_security = if security.is_empty() {
        TokenStream::new()
    } else {
        let push_security = security.iter().map(|Map(map)| {
            let insert_security_requirement = map.iter().map(|(ty, scopes)| {
                quote_use! {
                    # use core::any::type_name;
                    # use std::string::ToString;
                    # use predawn::SecurityScheme;

                    let scheme_type_name = type_name::<#ty>();
                    let scheme_name = <#ty as SecurityScheme>::NAME;

                    if !security_schemes.contains_key(scheme_type_name) {
                        security_schemes.insert(
                            scheme_type_name,
                            (scheme_name, <#ty as SecurityScheme>::create())
                        );
                    }

                    security_requirement.insert(
                        ToString::to_string(scheme_name),
                        vec![
                            #(ToString::to_string(#scopes)),*
                        ]
                    );
                }
            });

            quote_use! {
                # use predawn::openapi::SecurityRequirement;

                let mut security_requirement = SecurityRequirement::default();
                #(#insert_security_requirement)*
                security.push(security_requirement);
            }
        });

        quote_use! {
            # use std::vec::Vec;

            let mut security = Vec::new();
            #({#push_security})*
            operation.security = Some(security);
        }
    };

    let create_handler = quote_use! {
        # use std::sync::Arc;
        # use predawn::handler::{DynHandler, handler_fn};

        let #fn_name = {
            let this = this.clone();

            let handler = handler_fn(move |req| {
                let this = this.clone();

                async move {
                    #[allow(unused_variables, unused_mut)]
                    let (mut head, body) = req.split();

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
        # use std::format;
        # use std::any::type_name;
        # use std::collections::BTreeMap;
        # use predawn::openapi::Operation;
        # use predawn::openapi::transform_responses;

        let mut operation = Operation::default();

        #[doc = "add summary"]
        {
            #add_summary
        }

        #[doc = "add description"]
        {
            #add_description
        }

        #[doc = "add tags"]
        {
            #add_tags
        }

        #[doc = "add security"]
        {
            #add_security
        }

        #[doc = "add operation_id"]
        {
            operation.operation_id = Some(format!("{}::{}", type_name::<#self_ty>(), stringify!(#fn_name)));
        }

        #[doc = "add request_body"]
        {
            #last_request_body
        }

        #[doc = "add request parameters"]
        {
            #(#heads_parameters)*
            #last_parameters
        }

        let mut responses = BTreeMap::new();

        #[doc = "add response from read request error"]
        {
            #(#heads_error_responses)*
            #last_error_responses
        }

        #[doc = "add response from write response error"]
        {
            #return_error_responses
        }

        #[doc = "add response itself"]
        {
            #return_responses
        }

        operation.responses.responses.extend(transform_responses(responses));
    };

    let mut insert_fn_into_multi_path = Vec::new();

    controller_paths.iter().for_each(|controller_path| {
        method_paths.iter().for_each(|method_path| {
            let extract_single_path_map = quote_use! {
                # use core::convert::AsRef;
                # use std::clone::Clone;
                # use predawn::normalized_path::NormalizedPath;

                let path = NormalizedPath::join(
                    NormalizedPath::new(AsRef::<str>::as_ref(#controller_path)),
                    NormalizedPath::new(AsRef::<str>::as_ref(#method_path)),
                );

                let handlers = route_table.entry(Clone::clone(&path)).or_default();
                let operations = paths.entry(Clone::clone(&path)).or_default();
            };

            let insert_fn_into_multi_method = methods.iter().map(|method| {
                let uppercase_method = method.as_uppercase_ident();

                quote_use! {
                    # use predawn::__internal::http::Method;

                    handlers.push((Method::#uppercase_method, #fn_name.clone()));
                    operations.push((Method::#uppercase_method, operation.clone()));
                }
            });

            let insert_fn_into_single_path = quote! {
                #extract_single_path_map
                #(#insert_fn_into_multi_method)*
            };

            insert_fn_into_multi_path.push(insert_fn_into_single_path);
        });
    });

    let label: Label = syn::parse_str(&format!("'{}:", fn_name))?;

    let expand = quote! {
        #[allow(unused_labels)]
        #label {
            #create_handler
            #create_operation

            #[doc = "insert handler and operation"]
            {
                #(#insert_fn_into_multi_path)*
            }
        }
    };

    Ok(expand)
}
