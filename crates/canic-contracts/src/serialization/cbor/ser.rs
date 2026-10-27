//! Apply the storage u128 representation through Serde's nested value callbacks.

use serde::{
    Serialize, Serializer,
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
        SerializeTupleStruct, SerializeTupleVariant,
    },
};

pub(super) struct Encoding<S>(pub(super) S);
struct Value<'a, T: ?Sized>(&'a T);
pub(super) struct Compound<S>(S);

impl<T: Serialize + ?Sized> Serialize for Value<'_, T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(Encoding(serializer))
    }
}

macro_rules! scalar {
    ($($method:ident: $ty:ty),* $(,)?) => {$ (
        #[inline]
        fn $method(self, value: $ty) -> Result<Self::Ok, Self::Error> {
            self.0.$method(value)
        }
    )*};
}

impl<S: Serializer> Serializer for Encoding<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    type SerializeSeq = Compound<S::SerializeSeq>;
    type SerializeTuple = Compound<S::SerializeTuple>;
    type SerializeTupleStruct = Compound<S::SerializeTupleStruct>;
    type SerializeTupleVariant = Compound<S::SerializeTupleVariant>;
    type SerializeMap = Compound<S::SerializeMap>;
    type SerializeStruct = Compound<S::SerializeStruct>;
    type SerializeStructVariant = Compound<S::SerializeStructVariant>;

    scalar! {
        serialize_bool: bool, serialize_i8: i8, serialize_i16: i16,
        serialize_i32: i32, serialize_i64: i64, serialize_i128: i128,
        serialize_u8: u8, serialize_u16: u16, serialize_u32: u32, serialize_u64: u64,
        serialize_f32: f32, serialize_f64: f64, serialize_char: char,
        serialize_str: &str, serialize_bytes: &[u8],
    }

    #[inline]
    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_bytes(&value.to_be_bytes())
    }
    #[inline]
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_none()
    }
    #[inline]
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_some(&Value(value))
    }
    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit()
    }
    #[inline]
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit_struct(name)
    }
    #[inline]
    fn serialize_unit_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_unit_variant(name, index, variant)
    }
    #[inline]
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.0.serialize_newtype_struct(name, &Value(value))
    }
    #[inline]
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        self.0
            .serialize_newtype_variant(name, index, variant, &Value(value))
    }
    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.0.serialize_seq(len).map(Compound)
    }
    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.0.serialize_tuple(len).map(Compound)
    }
    #[inline]
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.0.serialize_tuple_struct(name, len).map(Compound)
    }
    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.0
            .serialize_tuple_variant(name, index, variant, len)
            .map(Compound)
    }
    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.0.serialize_map(len).map(Compound)
    }
    #[inline]
    fn serialize_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.0.serialize_struct(name, len).map(Compound)
    }
    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.0
            .serialize_struct_variant(name, index, variant, len)
            .map(Compound)
    }
    #[inline]
    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }
}

macro_rules! sequence {
    ($trait:ident, $method:ident) => {
        impl<S: $trait> $trait for Compound<S> {
            type Ok = S::Ok;
            type Error = S::Error;
            #[inline]
            fn $method<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), S::Error> {
                self.0.$method(&Value(value))
            }
            #[inline]
            fn end(self) -> Result<S::Ok, S::Error> {
                self.0.end()
            }
        }
    };
}
sequence!(SerializeSeq, serialize_element);
sequence!(SerializeTuple, serialize_element);
sequence!(SerializeTupleStruct, serialize_field);
sequence!(SerializeTupleVariant, serialize_field);

impl<S: SerializeMap> SerializeMap for Compound<S> {
    type Ok = S::Ok;
    type Error = S::Error;
    #[inline]
    fn serialize_key<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), S::Error> {
        self.0.serialize_key(&Value(value))
    }
    #[inline]
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), S::Error> {
        self.0.serialize_value(&Value(value))
    }
    #[inline]
    fn end(self) -> Result<S::Ok, S::Error> {
        self.0.end()
    }
}

macro_rules! record {
    ($trait:ident) => {
        impl<S: $trait> $trait for Compound<S> {
            type Ok = S::Ok;
            type Error = S::Error;
            #[inline]
            fn serialize_field<T: Serialize + ?Sized>(
                &mut self,
                key: &'static str,
                value: &T,
            ) -> Result<(), S::Error> {
                self.0.serialize_field(key, &Value(value))
            }
            #[inline]
            fn skip_field(&mut self, key: &'static str) -> Result<(), S::Error> {
                self.0.skip_field(key)
            }
            #[inline]
            fn end(self) -> Result<S::Ok, S::Error> {
                self.0.end()
            }
        }
    };
}
record!(SerializeStruct);
record!(SerializeStructVariant);
