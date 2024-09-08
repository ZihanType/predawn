use from_attr::{AttrsValue, FromAttr};
use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{DeriveInput, Field};

use crate::{schema_attr::SchemaAttr, serde_attr::SerdeAttr, util};

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = input;

    let named = util::extract_named_struct_fields(data, "ToParameters")?;

    let fields_len = named.len();
    let mut push_params = Vec::new();
    let mut errors = Vec::new();

    named
        .into_iter()
        .for_each(|field| match generate_single_field(field) {
            Ok(push_param) => {
                push_params.push(push_param);
            }
            Err(e) => errors.push(e),
        });

    if let Some(e) = errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return Err(e);
    }

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expand = quote_use! {
        # use std::vec::Vec;
        # use std::collections::BTreeMap;
        # use predawn::openapi::{ParameterData, Schema};
        # use predawn::ToParameters;

        impl #impl_generics ToParameters for #ident #ty_generics #where_clause {
            fn parameters(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Vec<ParameterData> {
                let mut params = Vec::with_capacity(#fields_len);
                #(#push_params)*
                params
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
            # use predawn::ToParameters;

            params.extend(<#ty as ToParameters>::parameters(schemas));
        });
    }

    let ident = schema_rename.unwrap_or_else(|| {
        serde_rename.unwrap_or_else(|| {
            ident
                .expect("unreachable: named field must have an identifier")
                .to_string()
        })
    });

    let default_expr = util::generate_default_expr(&ty, serde_default, schema_default)?;
    let add_default = util::generate_add_default_to_schema(&ty, default_expr);

    let description = util::extract_description(&attrs);
    let description = if description.is_empty() {
        quote! { None }
    } else {
        let description = util::generate_string_expr(&description);
        quote! { Some(#description) }
    };

    let generate_schema = if add_default.is_empty() {
        quote_use! {
            # use predawn::ToSchema;

            <#ty as ToSchema>::schema_ref(schemas, schemas_in_progress)
        }
    } else {
        quote_use! {
            # use predawn::ToSchema;
            # use predawn::openapi::ReferenceOr;

            {
                let mut schema = <#ty as ToSchema>::schema(schemas, schemas_in_progress);
                #add_default
                ReferenceOr::Item(schema)
            }
        }
    };

    let expand = quote_use! {
        # use core::default::Default;
        # use std::string::ToString;
        # use predawn::ToSchema;
        # use predawn::openapi::{ParameterData, ParameterSchemaOrContent};

        let schema = #generate_schema;

        let param = ParameterData {
            name: ToString::to_string(#ident),
            description: #description,
            required: <#ty as ToSchema>::REQUIRED,
            deprecated: Default::default(),
            format: ParameterSchemaOrContent::Schema(schema),
            example: Default::default(),
            examples: Default::default(),
            explode: Default::default(),
            extensions: Default::default(),
        };
        params.push(param);
    };

    Ok(expand)
}
