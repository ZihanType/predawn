use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{DeriveInput, Field, Generics};

use crate::serde_attr::SerdeAttr;

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = input;

    let named = crate::util::extract_named_struct_fields(data, "ToSchema")?;

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

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expand = quote_use! {
        # use core::default::Default;
        # use predawn::ToSchema;
        # use predawn::openapi::{Schema, ObjectType, SchemaData, SchemaKind, Type};

        impl #impl_generics ToSchema for #ident #ty_generics #where_clause {
            fn schema() -> Schema {
                let mut ty = ObjectType::default();

                #(#properties)*

                let title = #schema_title;

                Schema {
                    schema_data: SchemaData {
                        title: Some(title),
                        ..Default::default()
                    },
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

fn generate_schema_title(name: &str, generics: &Generics) -> TokenStream {
    let mut have_first = false;

    let types = generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(ty) => {
                let ty = &ty.ident;

                let extract_title = quote_use! {
                    # use predawn::ToSchema;

                    let schema = <#ty as ToSchema>::schema();
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

                Some(quote! {{
                    #extract_title
                    #push_comma
                    #push_title
                }})
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

                Some(quote! {{
                    #push_comma
                    #push_title
                }})
            }
            syn::GenericParam::Lifetime(_) => None,
        })
        .collect::<Vec<_>>();

    if types.is_empty() {
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

                #(#types)*

                name.push('>');
            }
        }
    }
}
