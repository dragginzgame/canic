//! Borrowed storage values keep CBOR parsing independent of concrete record types.
//! serde_cbor owns syntax and recursion bounds; Serde owns sequence/map traversal.

use serde::{
    Deserialize, Deserializer,
    de::{
        Error, Expected, IntoDeserializer, MapAccess, SeqAccess, Unexpected, Visitor,
        value::{MapAccessDeserializer, MapDeserializer, SeqDeserializer},
    },
};
use std::{borrow::Cow, fmt};

pub(super) enum Value<'de> {
    Null,
    Bool(bool),
    Integer(i128),
    Float(f64),
    Bytes(Cow<'de, [u8]>),
    Text(Cow<'de, str>),
    Array(Vec<Self>),
    Map(Vec<(Self, Self)>),
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value<'de>;

    #[inline]
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a current storage value")
    }
    #[inline]
    fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
        Ok(Value::Null)
    }
    #[inline]
    fn visit_bool<E: Error>(self, value: bool) -> Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }
    #[inline]
    fn visit_i64<E: Error>(self, value: i64) -> Result<Self::Value, E> {
        Ok(Value::Integer(value.into()))
    }
    #[inline]
    fn visit_u64<E: Error>(self, value: u64) -> Result<Self::Value, E> {
        Ok(Value::Integer(value.into()))
    }
    #[inline]
    fn visit_i128<E: Error>(self, value: i128) -> Result<Self::Value, E> {
        Ok(Value::Integer(value))
    }
    #[inline]
    fn visit_f64<E: Error>(self, value: f64) -> Result<Self::Value, E> {
        Ok(Value::Float(value))
    }
    #[inline]
    fn visit_str<E: Error>(self, value: &str) -> Result<Self::Value, E> {
        Ok(Value::Text(Cow::Owned(value.into())))
    }
    #[inline]
    fn visit_borrowed_str<E: Error>(self, value: &'de str) -> Result<Self::Value, E> {
        Ok(Value::Text(Cow::Borrowed(value)))
    }
    #[inline]
    fn visit_string<E: Error>(self, value: String) -> Result<Self::Value, E> {
        Ok(Value::Text(Cow::Owned(value)))
    }
    #[inline]
    fn visit_bytes<E: Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        Ok(Value::Bytes(Cow::Owned(value.into())))
    }
    #[inline]
    fn visit_borrowed_bytes<E: Error>(self, value: &'de [u8]) -> Result<Self::Value, E> {
        Ok(Value::Bytes(Cow::Borrowed(value)))
    }
    #[inline]
    fn visit_byte_buf<E: Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        Ok(Value::Bytes(Cow::Owned(value)))
    }
    #[inline]
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        // Do not allocate from an untrusted declared length before reading values.
        let mut values = Vec::new();
        while let Some(value) = access.next_element()? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    #[inline]
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
        let mut entries = Vec::new();
        while let Some(entry) = access.next_entry()? {
            entries.push(entry);
        }
        Ok(Value::Map(entries))
    }
}

impl<'de> Deserialize<'de> for Value<'de> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ValueVisitor)
    }
}

impl<'de> IntoDeserializer<'de, serde_cbor::Error> for Value<'de> {
    type Deserializer = Self;
    #[inline]
    fn into_deserializer(self) -> Self {
        self
    }
}

// Typed entrypoints avoid specializing every unrelated value branch for each
// record field. The shared error path retains Serde's expected/actual diagnostics.
impl Value<'_> {
    fn mismatch(&self, expected: &dyn Expected) -> serde_cbor::Error {
        let actual = match self {
            Self::Null => Unexpected::Unit,
            Self::Bool(value) => Unexpected::Bool(*value),
            Self::Integer(_) => Unexpected::Other("integer"),
            Self::Float(value) => Unexpected::Float(*value),
            Self::Bytes(value) => Unexpected::Bytes(value),
            Self::Text(value) => Unexpected::Str(value),
            Self::Array(_) => Unexpected::Seq,
            Self::Map(_) => Unexpected::Map,
        };
        serde_cbor::Error::invalid_type(actual, expected)
    }
}

macro_rules! delegate {
    ($target:ident: $($method:ident),* $(,)?) => {$ (
        #[inline]
        fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            self.$target(visitor)
        }
    )*};
}

impl<'de> Deserializer<'de> for Value<'de> {
    type Error = serde_cbor::Error;

    #[inline]
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Null => visitor.visit_unit(),
            Self::Bool(value) => visitor.visit_bool(value),
            Self::Integer(value) => {
                if let Ok(value) = u64::try_from(value) {
                    visitor.visit_u64(value)
                } else if let Ok(value) = i64::try_from(value) {
                    visitor.visit_i64(value)
                } else {
                    visitor.visit_i128(value)
                }
            }
            Self::Float(value) => visitor.visit_f64(value),
            Self::Bytes(Cow::Borrowed(value)) => visitor.visit_borrowed_bytes(value),
            Self::Bytes(Cow::Owned(value)) => visitor.visit_byte_buf(value),
            Self::Text(Cow::Borrowed(value)) => visitor.visit_borrowed_str(value),
            Self::Text(Cow::Owned(value)) => visitor.visit_string(value),
            Self::Array(values) => {
                SeqDeserializer::new(values.into_iter()).deserialize_any(visitor)
            }
            Self::Map(entries) => {
                MapDeserializer::new(entries.into_iter()).deserialize_any(visitor)
            }
        }
    }
    #[inline]
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let Self::Bytes(bytes) = self else {
            return Err(Self::Error::custom(
                "storage u128 requires 16 big-endian bytes",
            ));
        };
        let bytes = bytes.as_ref().try_into().map_err(|_| {
            Self::Error::invalid_length(bytes.len(), &"16 big-endian bytes for u128")
        })?;
        visitor.visit_u128(u128::from_be_bytes(bytes))
    }
    #[inline]
    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Null => visitor.visit_none(),
            value => visitor.visit_some(value),
        }
    }
    #[inline]
    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }
    #[inline]
    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self {
            Self::Text(value) => value
                .into_deserializer()
                .deserialize_enum(name, variants, visitor),
            Self::Map(entries) if entries.len() == 1 => {
                MapAccessDeserializer::new(MapDeserializer::new(entries.into_iter()))
                    .deserialize_enum(name, variants, visitor)
            }
            _ => Err(Self::Error::custom(
                "storage enum requires a name or one variant entry",
            )),
        }
    }
    #[inline]
    fn is_human_readable(&self) -> bool {
        false
    }
    #[inline]
    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Bool(value) => visitor.visit_bool(value),
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Integer(value) => {
                if let Ok(value) = u64::try_from(value) {
                    visitor.visit_u64(value)
                } else if let Ok(value) = i64::try_from(value) {
                    visitor.visit_i64(value)
                } else {
                    visitor.visit_i128(value)
                }
            }
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Float(value) => visitor.visit_f64(value),
            Self::Integer(value) => {
                if let Ok(value) = u64::try_from(value) {
                    visitor.visit_u64(value)
                } else if let Ok(value) = i64::try_from(value) {
                    visitor.visit_i64(value)
                } else {
                    visitor.visit_i128(value)
                }
            }
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Text(Cow::Borrowed(value)) => visitor.visit_borrowed_str(value),
            Self::Text(Cow::Owned(value)) => visitor.visit_string(value),
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Bytes(Cow::Borrowed(value)) => visitor.visit_borrowed_bytes(value),
            Self::Bytes(Cow::Owned(value)) => visitor.visit_byte_buf(value),
            Self::Array(values) => {
                SeqDeserializer::new(values.into_iter()).deserialize_any(visitor)
            }
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Null => visitor.visit_unit(),
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Array(values) => {
                SeqDeserializer::new(values.into_iter()).deserialize_any(visitor)
            }
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Map(entries) => {
                MapDeserializer::new(entries.into_iter()).deserialize_any(visitor)
            }
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self {
            Self::Array(values) => {
                SeqDeserializer::new(values.into_iter()).deserialize_any(visitor)
            }
            Self::Map(entries) => {
                MapDeserializer::new(entries.into_iter()).deserialize_any(visitor)
            }
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self {
            Self::Text(Cow::Borrowed(value)) => visitor.visit_borrowed_str(value),
            Self::Text(Cow::Owned(value)) => visitor.visit_string(value),
            Self::Bytes(Cow::Borrowed(value)) => visitor.visit_borrowed_bytes(value),
            Self::Bytes(Cow::Owned(value)) => visitor.visit_byte_buf(value),
            Self::Integer(value) => {
                if let Ok(value) = u64::try_from(value) {
                    visitor.visit_u64(value)
                } else {
                    Err(Self::Integer(value).mismatch(&visitor))
                }
            }
            value => Err(value.mismatch(&visitor)),
        }
    }
    #[inline]
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }
    #[inline]
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }
    #[inline]
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }
    #[inline]
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }
    delegate! { deserialize_i128: deserialize_i8, deserialize_i16, deserialize_i32, deserialize_i64, deserialize_u8, deserialize_u16, deserialize_u32, deserialize_u64 }
    delegate! { deserialize_f64: deserialize_f32 }
    delegate! { deserialize_string: deserialize_str, deserialize_char }
    delegate! { deserialize_byte_buf: deserialize_bytes }
}
