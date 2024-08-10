use from_attr::{AttrsValue, FromAttr};
use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{DeriveInput, Field, Generics};

use crate::{schema_attr::SchemaAttr, serde_attr::SerdeAttr, util};

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs,
        ident,
        generics,
        data,
        ..
    } = input;

    let named = util::extract_named_struct_fields(data, "ToSchema")?;

    let mut errors = Vec::new();

    let properties = named
        .into_iter()
        .filter_map(|field| match generate_single_field(field) {
            Ok(o) => Some(o),
            Err(e) => {
                errors.push(e);
                None
            }
        })
        .collect::<Vec<_>>();

    if let Some(e) = errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return Err(e);
    }

    let schema_title = generate_schema_title(&ident.to_string(), &generics);

    let description = util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expand = quote_use! {
        # use core::default::Default;
        # use std::collections::BTreeMap;
        # use predawn::ToSchema;
        # use predawn::openapi::{Schema, ObjectType, SchemaData, SchemaKind, Type};

        impl #impl_generics ToSchema for #ident #ty_generics #where_clause {
            fn schema(schemas: &mut BTreeMap<String, Schema>) -> Schema {
                let mut data = SchemaData::default();

                let title = #schema_title;
                data.title = Some(title);

                #add_description

                let mut obj = ObjectType::default();

                #(#properties)*

                Schema {
                    schema_data: data,
                    schema_kind: SchemaKind::Type(Type::Object(obj)),
                }
            }
        }
    };

    Ok(expand)
}

fn generate_single_field(field: Field) -> syn::Result<TokenStream> {
    let Field {
        attrs, ident, ty, ..
    } = field;

    let SerdeAttr {
        rename: serde_rename,
        flatten: serde_flatten,
        default: serde_default,
    } = SerdeAttr::new(&attrs);

    let SchemaAttr {
        rename: schema_rename,
        flatten: schema_flatten,
        default: schema_default,
    } = match SchemaAttr::from_attributes(&attrs) {
        Ok(Some(AttrsValue {
            value: field_attr, ..
        })) => field_attr,
        Ok(None) => Default::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    if serde_flatten || schema_flatten {
        return Ok(quote_use! {
            # use predawn::ToSchema;
            # use predawn::openapi::{AnySchema, ObjectType, SchemaKind, Type};

            match <#ty as ToSchema>::schema(schemas).schema_kind {
                SchemaKind::Any(AnySchema {
                    properties,
                    required,
                    ..
                })
                | SchemaKind::Type(Type::Object(ObjectType {
                    properties,
                    required,
                    ..
                })) => {
                    obj.properties.extend(properties);
                    obj.required.extend(required);
                }
                _ => {},
            };
        });
    }

    let default_expr = util::generate_default_expr(&ty, serde_default, schema_default)?;
    let add_default = util::generate_add_default_to_schema(&ty, default_expr);

    let ident = schema_rename.unwrap_or_else(|| {
        serde_rename.unwrap_or_else(|| {
            ident
                .expect("unreachable: named field must have an identifier")
                .to_string()
        })
    });

    let description = util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = util::generate_string_expr(&description);
        quote! {
            schema.schema_data.description = Some(#description);
        }
    };

    let generate_schema = if add_description.is_empty() && add_default.is_empty() {
        quote_use! {
            # use predawn::ToSchema;

            <#ty as ToSchema>::schema_ref_box(schemas)
        }
    } else {
        quote_use! {
            # use std::boxed::Box;
            # use predawn::ToSchema;
            # use predawn::openapi::ReferenceOr;

            {
                // TODO: add example
                let mut schema = <#ty as ToSchema>::schema(schemas);

                #add_description
                #add_default

                ReferenceOr::Item(Box::new(schema))
            }
        }
    };

    let expand = quote_use! {
        # use std::string::ToString;
        # use predawn::ToSchema;

        let schema = #generate_schema;

        obj.properties
            .insert(ToString::to_string(#ident), schema);

        if <#ty as ToSchema>::REQUIRED {
            obj.required.push(ToString::to_string(#ident));
        }
    };

    Ok(expand)
}

fn generate_schema_title(name: &str, generics: &Generics) -> TokenStream {
    let mut have_first = false;

    let push_types = generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(ty) => {
                let ty = &ty.ident;

                let extract_title = quote_use! {
                    # use predawn::ToSchema;

                    let schema = <#ty as ToSchema>::schema(schemas);
                    let title = schema.schema_data.title.as_deref().unwrap_or("Unknown");
                };

                let push_comma = if !have_first {
                    have_first = true;

                    TokenStream::new()
                } else {
                    quote! {
                        name.push_str(", ");
                    }
                };

                let push_title = quote! {
                    name.push_str(title);
                };

                Some(quote! {
                    {
                        #extract_title
                        #push_comma
                        #push_title
                    }
                })
            }
            syn::GenericParam::Const(cns) => {
                let cns = &cns.ident;

                let push_comma = if !have_first {
                    have_first = true;

                    TokenStream::new()
                } else {
                    quote! {
                        name.push_str(", ");
                    }
                };

                let push_title = quote_use! {
                    # use std::string::ToString;

                    name.push_str(&<#cns as ToString>::to_string());
                };

                Some(quote! {
                    {
                        #push_comma
                        #push_title
                    }
                })
            }
            syn::GenericParam::Lifetime(_) => None,
        })
        .collect::<Vec<_>>();

    if push_types.is_empty() {
        quote_use! {
            # use std::string::ToString;

            {
                ToString::to_string(#name)
            }
        }
    } else {
        quote_use! {
            # use std::string::ToString;

            {
                let mut name = ToString::to_string(#name);

                name.push('<');
                #(#push_types)*
                name.push('>');

                name
            }
        }
    }
}
