use std::{borrow::Cow, collections::BTreeMap};

use openapiv3::{AnySchema, NumberType, Schema, SchemaData, SchemaKind, Type};
use serde_json::{Map, Number, Value};

use super::forward_impl;
use crate::ToSchema;

impl ToSchema for Value {
    fn title() -> Cow<'static, str> {
        "Any".into()
    }

    fn schema(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Schema {
        Schema {
            schema_data: SchemaData {
                title: Some(Self::title().into()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Any(AnySchema::default()),
        }
    }
}

forward_impl!(Map<String, Value> => BTreeMap<String, Value>);

impl ToSchema for Number {
    fn title() -> Cow<'static, str> {
        "Number".into()
    }

    fn schema(_: &mut BTreeMap<String, Schema>, _: &mut Vec<String>) -> Schema {
        Schema {
            schema_data: SchemaData {
                title: Some(Self::title().into()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Number(NumberType::default())),
        }
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "raw_value")))]
#[cfg(feature = "raw_value")]
mod raw_value {
    use serde_json::{Value, value::RawValue};

    use super::forward_impl;
    forward_impl!(RawValue => Value);
}
