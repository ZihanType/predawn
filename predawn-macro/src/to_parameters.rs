use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{DeriveInput, Field};

use crate::{serde_attr::SerdeAttr, util};

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
            fn parameters(schemas: &mut BTreeMap<String, Schema>) -> Vec<ParameterData> {
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

    let SerdeAttr { rename, flatten } = SerdeAttr::new(&attrs)?;

    if flatten {
        return Ok(quote_use! {
            # use predawn::ToParameters;

            params.extend(<#ty as ToParameters>::parameters(schemas));
        });
    }

    let ident = rename.unwrap_or_else(|| {
        ident
            .expect("unreachable: named field must have an identifier")
            .to_string()
    });

    let description = util::extract_description(&attrs);
    let description = util::generate_optional_lit_str(&description)
        .unwrap_or_else(|| quote!(::core::option::Option::None));

    let expand = quote_use! {
        # use core::default::Default;
        # use std::string::ToString;
        # use predawn::ToSchema;
        # use predawn::openapi::{ParameterData, ParameterSchemaOrContent};

        let param = ParameterData {
            name: ToString::to_string(#ident),
            description: #description,
            required: <#ty as ToSchema>::REQUIRED,
            deprecated: Default::default(),
            format: ParameterSchemaOrContent::Schema(<#ty as ToSchema>::schema_ref(schemas)),
            example: Default::default(),
            examples: Default::default(),
            explode: Default::default(),
            extensions: Default::default(),
        };
        params.push(param);
    };

    Ok(expand)
}
