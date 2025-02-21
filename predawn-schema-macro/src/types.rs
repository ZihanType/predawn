use syn::{Attribute, Field, Ident, Token, punctuated::Punctuated};

pub(crate) struct UnitVariant {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) ident: Ident,
}

pub(crate) struct SchemaVariant {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) ident: Ident,
    pub(crate) fields: SchemaFields,
}

pub(crate) enum SchemaFields {
    Unit,
    Unnamed(Field),
    Named(Punctuated<Field, Token![,]>),
}

pub(crate) enum SchemaProperties {
    NamedStruct(Punctuated<Field, Token![,]>),
    OnlyUnitEnum(Vec<UnitVariant>),
    NormalEnum(Vec<SchemaVariant>),
}
