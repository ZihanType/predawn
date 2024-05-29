use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{DeriveInput, Field, Generics};

use crate::{serde_attr::SerdeAttr, util};

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
    let add_description = util::generate_optional_lit_str(&description).map(|description| {
        quote! {
            data.description = #description;
        }
    });

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expand = quote_use! {
        # use core::default::Default;
        # use predawn::ToSchema;
        # use predawn::openapi::{ReferenceOr, Schema, ObjectType, SchemaData, SchemaKind, Type};
        # use predawn::__internal::indexmap::IndexMap;

        impl #impl_generics ToSchema for #ident #ty_generics #where_clause {
            fn schema(schemas: &mut IndexMap<String, ReferenceOr<Schema>>) -> Schema {
                let mut data = SchemaData::default();

                let title = #schema_title;
                data.title = Some(title);

                #add_description

                let mut ty = ObjectType::default();

                #(#properties)*

                Schema {
                    schema_data: data,
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

    let description = util::extract_description(&attrs);
    let add_description = util::generate_optional_lit_str(&description).map(|description| {
        quote! {
            schema.schema_data.description = #description;
        }
    });

    let generate_schema = if add_description.is_none() {
        quote_use! {
            # use predawn::ToSchema;

            <#ty as ToSchema>::schema_ref_box(schemas)
        }
    } else {
        let create = quote_use! {
            # use predawn::ToSchema;

            // TODO: add default, example, flatten etc.
            let mut schema = <#ty as ToSchema>::schema(schemas);
        };

        let finish = quote_use! {
            # use std::boxed::Box;
            # use predawn::openapi::ReferenceOr;

            ReferenceOr::Item(Box::new(schema))
        };

        quote! {{
            #create
            #add_description
            #finish
        }}
    };

    let expand = quote_use! {
        # use std::string::ToString;
        # use predawn::ToSchema;

        let schema = #generate_schema;

        ty.properties
            .insert(ToString::to_string(#ident), schema);

        if <#ty as ToSchema>::REQUIRED {
            ty.required.push(ToString::to_string(#ident));
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
