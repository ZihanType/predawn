use std::{any::type_name, sync::Arc};

use serde::{
    Deserializer,
    de::{self, DeserializeSeed, EnumAccess, Error, MapAccess, VariantAccess, Visitor},
    forward_to_deserialize_any,
};

use crate::response_error::{DeserializePathError, ParseErrorAtKeySnafu, UnsupportedTypeSnafu};

macro_rules! unsupported_type {
    ($trait_fn:ident) => {
        fn $trait_fn<V>(self, _: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            UnsupportedTypeSnafu {
                name: type_name::<V::Value>(),
            }
            .fail()
        }
    };
}

pub(crate) struct PathDeserializer<'de> {
    path_params: &'de [(Arc<str>, Arc<str>)],
}

impl<'de> PathDeserializer<'de> {
    #[inline]
    pub(crate) fn new(path_params: &'de [(Arc<str>, Arc<str>)]) -> Self {
        PathDeserializer { path_params }
    }
}

impl<'de> Deserializer<'de> for PathDeserializer<'de> {
    type Error = DeserializePathError;

    unsupported_type!(deserialize_any);

    unsupported_type!(deserialize_bool);

    unsupported_type!(deserialize_i8);

    unsupported_type!(deserialize_i16);

    unsupported_type!(deserialize_i32);

    unsupported_type!(deserialize_i64);

    unsupported_type!(deserialize_u8);

    unsupported_type!(deserialize_u16);

    unsupported_type!(deserialize_u32);

    unsupported_type!(deserialize_u64);

    unsupported_type!(deserialize_f32);

    unsupported_type!(deserialize_f64);

    unsupported_type!(deserialize_char);

    unsupported_type!(deserialize_str);

    unsupported_type!(deserialize_string);

    unsupported_type!(deserialize_bytes);

    unsupported_type!(deserialize_byte_buf);

    unsupported_type!(deserialize_option);

    unsupported_type!(deserialize_unit);

    unsupported_type!(deserialize_seq);

    unsupported_type!(deserialize_identifier);

    unsupported_type!(deserialize_ignored_any);

    fn deserialize_unit_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }

    fn deserialize_newtype_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }

    fn deserialize_tuple<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }

    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(MapDeserializer {
            params: self.path_params,
            pair: None,
        })
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }
}

struct MapDeserializer<'de> {
    params: &'de [(Arc<str>, Arc<str>)],
    pair: Option<(&'de str, &'de str)>,
}

impl<'de> MapAccess<'de> for MapDeserializer<'de> {
    type Error = DeserializePathError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.params.split_first() {
            Some(((key, value), tail)) => {
                self.params = tail;
                self.pair = Some((key, value));

                seed.deserialize(KeyDeserializer { key }).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.pair.take() {
            Some((key, value)) => seed.deserialize(ValueDeserializer { key, value }),
            None => Err(DeserializePathError::custom("value is missing")),
        }
    }
}

struct KeyDeserializer<'de> {
    key: &'de str,
}

macro_rules! parse_key {
    ($trait_fn:ident) => {
        fn $trait_fn<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            visitor.visit_str(&self.key)
        }
    };
}

impl<'de> Deserializer<'de> for KeyDeserializer<'de> {
    type Error = DeserializePathError;

    parse_key!(deserialize_identifier);

    parse_key!(deserialize_str);

    parse_key!(deserialize_string);

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes
        byte_buf option unit unit_struct seq tuple
        tuple_struct map newtype_struct struct enum ignored_any
    }

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(DeserializePathError::custom("Unexpected key type"))
    }
}

macro_rules! parse_value {
    ($trait_fn:ident, $visit_fn:ident, $ty:literal) => {
        fn $trait_fn<V>(self, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: Visitor<'de>,
        {
            let v = self.value.parse().map_err(|_| {
                ParseErrorAtKeySnafu {
                    key: self.key.clone(),
                    value: self.value.clone(),
                    expected_type: $ty,
                }
                .build()
            })?;
            visitor.$visit_fn(v)
        }
    };
}

#[derive(Debug)]
struct ValueDeserializer<'de> {
    key: &'de str,
    value: &'de str,
}

impl<'de> Deserializer<'de> for ValueDeserializer<'de> {
    type Error = DeserializePathError;

    unsupported_type!(deserialize_map);

    unsupported_type!(deserialize_identifier);

    unsupported_type!(deserialize_unit);

    unsupported_type!(deserialize_ignored_any);

    parse_value!(deserialize_bool, visit_bool, "bool");

    parse_value!(deserialize_i8, visit_i8, "i8");

    parse_value!(deserialize_i16, visit_i16, "i16");

    parse_value!(deserialize_i32, visit_i32, "i32");

    parse_value!(deserialize_i64, visit_i64, "i64");

    parse_value!(deserialize_i128, visit_i128, "i128");

    parse_value!(deserialize_u8, visit_u8, "u8");

    parse_value!(deserialize_u16, visit_u16, "u16");

    parse_value!(deserialize_u32, visit_u32, "u32");

    parse_value!(deserialize_u64, visit_u64, "u64");

    parse_value!(deserialize_u128, visit_u128, "u128");

    parse_value!(deserialize_f32, visit_f32, "f32");

    parse_value!(deserialize_f64, visit_f64, "f64");

    parse_value!(deserialize_string, visit_string, "String");

    parse_value!(deserialize_byte_buf, visit_string, "String");

    parse_value!(deserialize_char, visit_char, "char");

    fn deserialize_any<V>(self, v: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(v)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.value)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.value.as_bytes())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit_struct<V>(self, _: &'static str, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_tuple<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }

    fn deserialize_seq<V>(self, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }

    fn deserialize_tuple_struct<V>(
        self,
        _: &'static str,
        _: usize,
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }

    fn deserialize_struct<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        _: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: type_name::<V::Value>(),
        }
        .fail()
    }

    fn deserialize_enum<V>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(EnumDeserializer { value: self.value })
    }
}

struct EnumDeserializer<'de> {
    value: &'de str,
}

impl<'de> EnumAccess<'de> for EnumDeserializer<'de> {
    type Error = DeserializePathError;
    type Variant = UnitVariant;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        Ok((
            seed.deserialize(KeyDeserializer { key: self.value })?,
            UnitVariant,
        ))
    }
}

struct UnitVariant;

impl<'de> VariantAccess<'de> for UnitVariant {
    type Error = DeserializePathError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, _: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        UnsupportedTypeSnafu {
            name: "newtype enum variant",
        }
        .fail()
    }

    fn tuple_variant<V>(self, _: usize, _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: "tuple enum variant",
        }
        .fail()
    }

    fn struct_variant<V>(self, _: &'static [&'static str], _: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        UnsupportedTypeSnafu {
            name: "struct enum variant",
        }
        .fail()
    }
}
