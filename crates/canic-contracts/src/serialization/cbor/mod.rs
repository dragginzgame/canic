//! Current Canic storage policy over serde_cbor: unsigned 128-bit values are
//! exactly 16 big-endian bytes. Wire and external IC CBOR use their own codecs.
//!
//! Encoding streams directly. Decoding shares serde_cbor's bounded parser via a
//! private value tree, avoiding a copy of the parser for every record visitor.
//! Borrowed strings/bytes avoid copying definite-length input. Map entries stay
//! ordered and duplicated so record visitors still reject duplicate fields.
//! The temporary tree adds allocation; it does not change the stored bytes.

mod de;
mod ser;

use serde::{Serialize, de::DeserializeOwned};

pub(super) fn encode<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, serde_cbor::Error> {
    let mut bytes = Vec::new();
    value.serialize(ser::Encoding(&mut serde_cbor::Serializer::new(&mut bytes)))?;
    Ok(bytes)
}

pub(super) fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, serde_cbor::Error> {
    T::deserialize(serde_cbor::from_slice::<de::Value<'_>>(bytes)?)
}

#[cfg(test)]
mod tests;
