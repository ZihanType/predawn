use std::collections::BTreeMap;

use openapiv3::{AnySchema, NumberType, Schema, SchemaData, SchemaKind, Type};
use serde_json::{Map, Number, Value};

use super::forward_impl;
use crate::ToSchema;

impl ToSchema for Value {
    fn schema() -> Schema {
        Schema {
            schema_data: SchemaData {
                title: Some("Any".to_string()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Any(AnySchema::default()),
        }
    }
}

forward_impl!(Map<String, Value> => BTreeMap<String, Value>);

impl ToSchema for Number {
    fn schema() -> Schema {
        Schema {
            schema_data: SchemaData {
                title: Some("Number".to_string()),
                ..Default::default()
            },
            schema_kind: SchemaKind::Type(Type::Number(NumberType::default())),
        }
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "raw_value")))]
#[cfg(feature = "raw_value")]
mod raw_value {
    use serde_json::{value::RawValue, Value};

    use super::forward_impl;
    forward_impl!(RawValue => Value);
}
