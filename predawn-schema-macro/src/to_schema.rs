use from_attr::{AttrsValue, FromAttr};
use predawn_macro_core::{SchemaAttr, SerdeAttr};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use quote_use::quote_use;
use syn::{
    punctuated::Punctuated, Attribute, DeriveInput, Field, GenericParam, Generics, Ident, Token,
};

use crate::types::{SchemaFields, SchemaProperties, SchemaVariant, UnitVariant};

pub(crate) fn generate(input: DeriveInput) -> syn::Result<TokenStream> {
    let DeriveInput {
        attrs,
        ident,
        generics,
        data,
        ..
    } = input;

    let crate_name = crate::util::get_crate_name();

    match crate::util::extract_schema_properties(data)? {
        SchemaProperties::NamedStruct(fields) => {
            generate_named_struct(&crate_name, attrs, ident, generics, fields)
        }
        SchemaProperties::OnlyUnitEnum(variants) => {
            generate_only_unit(&crate_name, attrs, ident, variants)
        }
        SchemaProperties::NormalEnum(variants) => {
            generate_normal_enum(&crate_name, attrs, ident, generics, variants)
        }
    }
}

fn generate_named_struct(
    crate_name: &TokenStream,
    attrs: Vec<Attribute>,
    ident: Ident,
    generics: Generics,
    fields: Punctuated<Field, Token![,]>,
) -> syn::Result<TokenStream> {
    let mut errors = Vec::new();

    let add_properties = fields
        .into_iter()
        .filter_map(|field| match generate_single_field(crate_name, field) {
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

    let title_fn = generate_title_fn(crate_name, ident.to_string(), &generics);

    let description = predawn_macro_core::util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expand = quote_use! {
        # use std::collections::BTreeMap;
        # use #crate_name::ToSchema;
        # use #crate_name::openapi::{Schema, ObjectType, SchemaData, SchemaKind, Type};

        impl #impl_generics ToSchema for #ident #ty_generics #where_clause {
            #title_fn

            fn schema(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Schema {
                let mut data = SchemaData::default();
                data.title = Some(Self::title().into());

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

fn generate_single_field(crate_name: &TokenStream, field: Field) -> syn::Result<TokenStream> {
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
            # use #crate_name::ToSchema;
            # use #crate_name::openapi::{AnySchema, ObjectType, SchemaKind, Type};

            match <#ty as ToSchema>::schema(schemas, schemas_in_progress).schema_kind {
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

    let default_expr =
        predawn_macro_core::util::generate_default_expr(&ty, serde_default, schema_default)?;
    let add_default = predawn_macro_core::util::generate_add_default_to_schema(&ty, default_expr);

    let ident = schema_rename.unwrap_or_else(|| {
        serde_rename.unwrap_or_else(|| {
            ident
                .expect("unreachable: named field must have an identifier")
                .to_string()
        })
    });

    let description = predawn_macro_core::util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! {
            schema.schema_data.description = Some(#description);
        }
    };

    let generate_schema = if add_description.is_empty() && add_default.is_empty() {
        quote_use! {
            # use #crate_name::ToSchema;

            <#ty as ToSchema>::schema_ref_box(schemas, schemas_in_progress)
        }
    } else {
        quote_use! {
            # use std::boxed::Box;
            # use #crate_name::ToSchema;
            # use #crate_name::openapi::ReferenceOr;

            {
                // TODO: add example
                let mut schema = <#ty as ToSchema>::schema(schemas, schemas_in_progress);

                #add_description
                #add_default

                ReferenceOr::Item(Box::new(schema))
            }
        }
    };

    let push_required = if add_default.is_empty() {
        quote_use! {
            # use std::string::ToString;
            # use #crate_name::ToSchema;

            if <#ty as ToSchema>::REQUIRED {
                obj.required.push(ToString::to_string(#ident));
            }
        }
    } else {
        TokenStream::new()
    };

    let expand = quote_use! {
        # use std::string::ToString;
        # use #crate_name::ToSchema;

        {
            let schema = #generate_schema;

            obj.properties.insert(ToString::to_string(#ident), schema);

            #push_required
        }
    };

    Ok(expand)
}

fn generate_title_fn(crate_name: &TokenStream, ident: String, generics: &Generics) -> TokenStream {
    let mut have_first = false;
    let mut variable_definitions = Vec::new();
    let mut variable_idents = Vec::new();
    let mut generic_slots = String::new();

    generics.params.iter().enumerate().for_each(|(idx, param)| {
        let var_ident = format_ident!("var{}", idx);

        let variable_definition = match param {
            GenericParam::Lifetime(_) => return,
            GenericParam::Type(ty) => {
                let ty = &ty.ident;

                quote_use! {
                    # use #crate_name::ToSchema;

                    let #var_ident = <#ty as ToSchema>::title();
                }
            }
            GenericParam::Const(cns) => {
                let cns = &cns.ident;

                quote! {
                    let #var_ident = #cns;
                }
            }
        };

        variable_definitions.push(variable_definition);

        variable_idents.push(var_ident);

        let slot = if !have_first {
            have_first = true;
            "{}"
        } else {
            ", {}"
        };
        generic_slots.push_str(slot);
    });

    let body = if variable_definitions.is_empty() {
        quote! {
            ::std::borrow::Cow::Borrowed(#ident)
        }
    } else {
        let mut template = String::from(&ident);
        template.push('<');
        template.push_str(&generic_slots);
        template.push('>');

        quote! {
            ::std::format!(#template, #(#variable_idents),*).into()
        }
    };

    quote! {
        fn title() -> ::std::borrow::Cow<'static, str> {
            #(#variable_definitions)*

            #body
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
    crate_name: &TokenStream,
    attrs: Vec<Attribute>,
    ident: Ident,
    variants: Vec<UnitVariant>,
) -> syn::Result<TokenStream> {
    let title_literal = ident.to_string();

    let description = predawn_macro_core::util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
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
        # use std::borrow::Cow;
        # use #crate_name::ToSchema;
        # use #crate_name::openapi::{Schema, StringType, SchemaData, SchemaKind, Type};

        impl ToSchema for #ident {
            fn title() -> Cow<'static, str> {
                #title_literal.into()
            }

            fn schema(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Schema {
                let mut data = SchemaData::default();
                data.title = Some(Self::title().into());

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
    crate_name: &TokenStream,
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
                SchemaFields::Unit => generate_unit_variant(crate_name, attrs, ident),
                SchemaFields::Unnamed(field) => {
                    generate_unnamed_variant(crate_name, attrs, ident, field)
                }
                SchemaFields::Named(fields) => {
                    generate_named_variant(crate_name, attrs, ident, fields)
                }
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

    let title_fn = generate_title_fn(crate_name, ident.to_string(), &generics);

    let description = predawn_macro_core::util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expand = quote_use! {
        # use std::collections::BTreeMap;
        # use #crate_name::ToSchema;
        # use #crate_name::openapi::{Schema, ObjectType, SchemaData, SchemaKind, Type};

        impl #impl_generics ToSchema for #ident #ty_generics #where_clause {
            #title_fn

            fn schema(schemas: &mut BTreeMap<String, Schema>, schemas_in_progress: &mut Vec<String>) -> Schema {
                let mut data = SchemaData::default();
                data.title = Some(Self::title().into());

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

fn generate_unit_variant(
    crate_name: &TokenStream,
    attrs: Vec<Attribute>,
    ident: Ident,
) -> syn::Result<TokenStream> {
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

    let description = predawn_macro_core::util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let expand = quote_use! {
        # use #crate_name::openapi::{Schema, StringType, SchemaData, SchemaKind, Type, ReferenceOr};

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
    crate_name: &TokenStream,
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

    let description = predawn_macro_core::util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let expand = quote_use! {
        # use std::string::ToString;
        # use #crate_name::ToSchema;
        # use #crate_name::openapi::{Schema, ObjectType, SchemaData, SchemaKind, Type, ReferenceOr};

        {
            let mut data = SchemaData::default();

            #add_description

            let mut obj = ObjectType::default();
            obj.required.push(ToString::to_string(#ident));
            obj.properties.insert(ToString::to_string(#ident), <#ty as ToSchema>::schema_ref_box(schemas, schemas_in_progress));

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
    crate_name: &TokenStream,
    attrs: Vec<Attribute>,
    ident: Ident,
    fields: Punctuated<Field, Token![,]>,
) -> syn::Result<TokenStream> {
    let mut errors = Vec::new();

    let add_properties = fields
        .into_iter()
        .filter_map(|field| match generate_single_field(crate_name, field) {
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

    let description = predawn_macro_core::util::extract_description(&attrs);
    let add_description = if description.is_empty() {
        TokenStream::new()
    } else {
        let description = predawn_macro_core::util::generate_string_expr(&description);
        quote! {
            data.description = Some(#description);
        }
    };

    let expand = quote_use! {
        # use std::string::ToString;
        # use #crate_name::ToSchema;
        # use #crate_name::openapi::{Schema, ObjectType, SchemaData, SchemaKind, Type, ReferenceOr};

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
