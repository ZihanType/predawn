use from_attr::{AttrsValue, FromAttr};
use proc_macro2::TokenStream;
use quote::quote;
use quote_use::quote_use;
use syn::{punctuated::Punctuated, Attribute, DeriveInput, Field, Generics, Ident, Token};

use crate::{
    schema_attr::SchemaAttr,
    serde_attr::SerdeAttr,
    types::{SchemaFields, SchemaProperties, SchemaVariant, UnitVariant},
    util,
};

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs,
        ident,
        generics,
        data,
        ..
    } = input;

    match util::extract_schema_properties(data)? {
        SchemaProperties::NamedStruct(fields) => {
            generate_named_struct(attrs, ident, generics, fields)
        }
        SchemaProperties::OnlyUnitEnum(variants) => generate_only_unit(attrs, ident, variants),
        SchemaProperties::NormalEnum(variants) => {
            generate_normal_enum(attrs, ident, generics, variants)
        }
    }
}

fn generate_named_struct(
    attrs: Vec<Attribute>,
    ident: Ident,
    generics: Generics,
    fields: Punctuated<Field, Token![,]>,
) -> syn::Result<TokenStream> {
    let mut errors = Vec::new();

    let add_properties = fields
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

                #(#add_properties)*

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

        {
            let schema = #generate_schema;

            obj.properties.insert(ToString::to_string(#ident), schema);

            if <#ty as ToSchema>::REQUIRED {
                obj.required.push(ToString::to_string(#ident));
            }
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

// {
//   "title": "SomeOne",
//   "type": "string",
//   "enum": [
//     "Alice",
//     "Bob"
//   ]
// }

fn generate_only_unit(
    attrs: Vec<Attribute>,
    ident: Ident,
    variants: Vec<UnitVariant>,
) -> syn::Result<TokenStream> {
    let title = ident.to_string();
    let title = util::generate_string_expr(&title);

    let description = util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let mut errors = Vec::new();

    let add_enumeration = variants
        .into_iter()
        .filter_map(|field| match generate_single_unit_variant(field) {
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

    let expand = quote_use! {
        # use std::collections::BTreeMap;
        # use predawn::ToSchema;
        # use predawn::openapi::{Schema, StringType, SchemaData, SchemaKind, Type};

        impl ToSchema for #ident {
            fn schema(schemas: &mut BTreeMap<String, Schema>) -> Schema {
                let mut data = SchemaData::default();

                data.title = Some(#title);

                #add_description

                let mut ty = StringType::default();

                #(#add_enumeration)*

                Schema {
                    schema_data: data,
                    schema_kind: SchemaKind::Type(Type::String(ty)),
                }
            }
        }
    };

    Ok(expand)
}

fn generate_single_unit_variant(variant: UnitVariant) -> syn::Result<TokenStream> {
    let UnitVariant { attrs, ident } = variant;

    let SerdeAttr {
        rename: serde_rename,
        flatten: _,
        default: _,
    } = SerdeAttr::new(&attrs);

    let SchemaAttr {
        rename: schema_rename,
        flatten: _,
        default: _,
    } = match SchemaAttr::from_attributes(&attrs) {
        Ok(Some(AttrsValue {
            value: field_attr, ..
        })) => field_attr,
        Ok(None) => Default::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let ident = schema_rename.unwrap_or_else(|| serde_rename.unwrap_or_else(|| ident.to_string()));

    Ok(quote! {
        ty.enumeration.push(Some(#ident.to_string()));
    })
}

fn generate_normal_enum(
    attrs: Vec<Attribute>,
    ident: Ident,
    generics: Generics,
    variants: Vec<SchemaVariant>,
) -> syn::Result<TokenStream> {
    let variants_len = variants.len();

    let mut errors = Vec::new();

    let one_of_schema = variants
        .into_iter()
        .filter_map(|variant| {
            let SchemaVariant {
                attrs,
                ident,
                fields,
            } = variant;

            let result = match fields {
                SchemaFields::Unit => generate_unit_variant(attrs, ident),
                SchemaFields::Unnamed(field) => generate_unnamed_variant(attrs, ident, field),
                SchemaFields::Named(fields) => generate_named_variant(attrs, ident, fields),
            };

            match result {
                Ok(o) => Some(o),
                Err(e) => {
                    errors.push(e);
                    None
                }
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
        # use std::collections::BTreeMap;
        # use predawn::ToSchema;
        # use predawn::openapi::{Schema, ObjectType, SchemaData, SchemaKind, Type};

        impl #impl_generics ToSchema for #ident #ty_generics #where_clause {
            fn schema(schemas: &mut BTreeMap<String, Schema>) -> Schema {
                let mut data = SchemaData::default();

                let title = #schema_title;
                data.title = Some(title);

                #add_description

                let mut one_of = Vec::with_capacity(#variants_len);

                #(one_of.push(#one_of_schema);)*

                Schema {
                    schema_data: data,
                    schema_kind: SchemaKind::OneOf {
                        one_of,
                    },
                }
            }
        }
    };

    Ok(expand)
}

fn generate_unit_variant(attrs: Vec<Attribute>, ident: Ident) -> syn::Result<TokenStream> {
    let SerdeAttr {
        rename: serde_rename,
        flatten: _,
        default: _,
    } = SerdeAttr::new(&attrs);

    let SchemaAttr {
        rename: schema_rename,
        flatten: _,
        default: _,
    } = match SchemaAttr::from_attributes(&attrs) {
        Ok(Some(AttrsValue {
            value: field_attr, ..
        })) => field_attr,
        Ok(None) => Default::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let ident = schema_rename.unwrap_or_else(|| serde_rename.unwrap_or_else(|| ident.to_string()));

    let description = util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let expand = quote_use! {
        # use predawn::openapi::{Schema, StringType, SchemaData, SchemaKind, Type, ReferenceOr};

        {
            let mut data = SchemaData::default();

            #add_description

            let mut ty = StringType::default();

            ty.enumeration.push(Some(#ident.to_string()));

            let schema = Schema {
                schema_data: data,
                schema_kind: SchemaKind::Type(Type::String(ty)),
            };

            ReferenceOr::Item(schema)
        }
    };

    Ok(expand)
}

fn generate_unnamed_variant(
    attrs: Vec<Attribute>,
    ident: Ident,
    field: Field,
) -> syn::Result<TokenStream> {
    let ty = field.ty;

    let SerdeAttr {
        rename: serde_rename,
        flatten: _,
        default: _,
    } = SerdeAttr::new(&attrs);

    let SchemaAttr {
        rename: schema_rename,
        flatten: _,
        default: _,
    } = match SchemaAttr::from_attributes(&attrs) {
        Ok(Some(AttrsValue {
            value: field_attr, ..
        })) => field_attr,
        Ok(None) => Default::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let ident = schema_rename.unwrap_or_else(|| serde_rename.unwrap_or_else(|| ident.to_string()));

    let description = util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let expand = quote_use! {
        # use std::string::ToString;
        # use predawn::ToSchema;
        # use predawn::openapi::{Schema, ObjectType, SchemaData, SchemaKind, Type, ReferenceOr};

        {
            let mut data = SchemaData::default();

            #add_description

            let mut obj = ObjectType::default();
            obj.required.push(ToString::to_string(#ident));
            obj.properties.insert(ToString::to_string(#ident), <#ty as ToSchema>::schema_ref_box(schemas));

            let schema = Schema {
                schema_data: data,
                schema_kind: SchemaKind::Type(Type::Object(obj)),
            };

            ReferenceOr::Item(schema)
        }
    };

    Ok(expand)
}

fn generate_named_variant(
    attrs: Vec<Attribute>,
    ident: Ident,
    fields: Punctuated<Field, Token![,]>,
) -> syn::Result<TokenStream> {
    let mut errors = Vec::new();

    let add_properties = fields
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

    let SerdeAttr {
        rename: serde_rename,
        flatten: _,
        default: _,
    } = SerdeAttr::new(&attrs);

    let SchemaAttr {
        rename: schema_rename,
        flatten: _,
        default: _,
    } = match SchemaAttr::from_attributes(&attrs) {
        Ok(Some(AttrsValue {
            value: field_attr, ..
        })) => field_attr,
        Ok(None) => Default::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    let ident = schema_rename.unwrap_or_else(|| serde_rename.unwrap_or_else(|| ident.to_string()));

    let description = util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let expand = quote_use! {
        # use std::string::ToString;
        # use predawn::ToSchema;
        # use predawn::openapi::{Schema, ObjectType, SchemaData, SchemaKind, Type, ReferenceOr};

        {
            let mut data = SchemaData::default();

            #add_description

            let mut obj = ObjectType::default();
            obj.required.push(ToString::to_string(#ident));
            obj.properties.insert(
                ToString::to_string(#ident),
                {
                    let mut obj = ObjectType::default();
                    #(#add_properties)*

                    let schema = Schema {
                        schema_data: SchemaData::default(),
                        schema_kind: SchemaKind::Type(Type::Object(obj)),
                    };

                    ReferenceOr::Item(Box::new(schema))
                },
            );

            let schema = Schema {
                schema_data: data,
                schema_kind: SchemaKind::Type(Type::Object(obj)),
            };

            ReferenceOr::Item(schema)
        }
    };

    Ok(expand)
}
