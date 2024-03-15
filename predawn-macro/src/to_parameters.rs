use proc_macro2::TokenStream;
use quote_use::quote_use;
use syn::{Data, DataEnum, DataStruct, DataUnion, DeriveInput, Field, Fields, FieldsNamed};

use crate::serde_attr::SerdeAttr;

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = input;

    let named = match data {
        Data::Struct(DataStruct { fields, .. }) => match fields {
            Fields::Named(FieldsNamed { named, .. }) => named,
            Fields::Unnamed(_) | Fields::Unit => {
                return Err(syn::Error::new_spanned(
                    fields,
                    "`ToParameters` can only be derived for structs with named fields",
                ));
            }
        },
        Data::Enum(DataEnum { enum_token, .. }) => {
            return Err(syn::Error::new(
                enum_token.span,
                "`ToParameters` can only be derived for structs",
            ));
        }
        Data::Union(DataUnion { union_token, .. }) => {
            return Err(syn::Error::new(
                union_token.span,
                "`ToParameters` can only be derived for structs",
            ));
        }
    };

    let mut parameter_impls = Vec::new();
    let mut errors = Vec::new();

    named
        .into_iter()
        .for_each(|field| match generate_single_field(field) {
            Ok(parameter) => {
                parameter_impls.push(parameter);
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
        # use predawn::openapi::{ParameterData, Components};
        # use predawn::ToParameters;

        impl #impl_generics ToParameters for #ident #ty_generics #where_clause {
            fn parameters(components: &mut Components) -> Vec<ParameterData> {
                [
                    #(#parameter_impls)*
                ]
                .to_vec()
            }
        }
    };

    Ok(expand)
}

fn generate_single_field(field: Field) -> syn::Result<TokenStream> {
    let Field {
        attrs, ident, ty, ..
    } = field;

    let SerdeAttr { rename } = SerdeAttr::new(&attrs)?;

    let ident = rename.unwrap_or_else(|| {
        ident
            .expect("unreachable: named field must have an identifier")
            .to_string()
    });

    let expand = quote_use! {
        # use core::default::Default;
        # use std::string::ToString;
        # use predawn::ToSchema;
        # use predawn::openapi::{ParameterData, ParameterSchemaOrContent};

        ParameterData {
            name: ToString::to_string(#ident),
            description: Default::default(),
            required: <#ty as ToSchema>::REQUIRED,
            deprecated: Default::default(),
            format: ParameterSchemaOrContent::Schema(<#ty as ToSchema>::schema_ref(components)),
            example: Default::default(),
            examples: Default::default(),
            explode: Default::default(),
            extensions: Default::default(),
        },
    };

    Ok(expand)
}
