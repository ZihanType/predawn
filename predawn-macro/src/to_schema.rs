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
                    "`ToSchema` can only be derived for structs with named fields",
                ));
            }
        },
        Data::Enum(DataEnum { enum_token, .. }) => {
            return Err(syn::Error::new(
                enum_token.span,
                "`ToSchema` can only be derived for structs",
            ));
        }
        Data::Union(DataUnion { union_token, .. }) => {
            return Err(syn::Error::new(
                union_token.span,
                "`ToSchema` can only be derived for structs",
            ));
        }
    };

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

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expand = quote_use! {
        # use core::default::Default;
        # use predawn::ToSchema;
        # use predawn::openapi::{Schema, ObjectType, SchemaKind, Type};

        impl #impl_generics ToSchema for #ident #ty_generics #where_clause {
            fn schema() -> Schema {
                let mut ty = ObjectType::default();

                #(#properties)*

                Schema {
                    schema_data: Default::default(),
                    schema_kind: SchemaKind::Type(Type::Object(ty)),
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

    let SerdeAttr { rename } = SerdeAttr::new(&attrs)?;

    let ident = rename.unwrap_or_else(|| {
        ident
            .expect("unreachable: named field must have an identifier")
            .to_string()
    });

    let expand = quote_use! {
        # use std::string::ToString;
        # use std::boxed::Box;
        # use predawn::ToSchema;
        # use predawn::openapi::ReferenceOr;

        #[allow(unused_mut)]
        let mut schema = <#ty as ToSchema>::schema();

        ty.properties
            .insert(ToString::to_string(#ident), ReferenceOr::Item(Box::new(schema)));

        if <#ty as ToSchema>::REQUIRED {
            ty.required.push(ToString::to_string(#ident));
        }
    };

    Ok(expand)
}
