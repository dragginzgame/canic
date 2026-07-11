//! Module: cdk::serialize
//!
//! Responsibility: serde-CBOR helpers for stable storage encoding.
//! Does not own: individual stable schema bounds or migration policy.
//! Boundary: maps serde encode/decode failures into typed Canic errors.

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error as ThisError;

///
/// SerializeError
///
/// Typed error returned by Canic stable serialization helpers.
///

#[derive(Debug, ThisError)]
pub enum SerializeError {
    #[error("serialize error: {0}")]
    Serialize(String),

    #[error("deserialize error: {0}")]
    Deserialize(String),
}

/// Serialize one value to CBOR bytes.
pub fn serialize<T>(value: &T) -> Result<Vec<u8>, SerializeError>
where
    T: Serialize,
{
    let mut bytes = Vec::new();
    ciborium::ser::into_writer(value, &mut bytes)
        .map_err(|err| SerializeError::Serialize(err.to_string()))?;
    Ok(bytes)
}

/// Deserialize one value from CBOR bytes.
pub fn deserialize<T>(bytes: &[u8]) -> Result<T, SerializeError>
where
    T: DeserializeOwned,
{
    ciborium::de::from_reader(bytes).map_err(|err| SerializeError::Deserialize(err.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{deserialize, serialize};
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    enum FixtureVariant {
        Unit,
        Tuple(u32, String),
        Struct { enabled: bool },
    }

    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    struct StableShapeFixture {
        flag: bool,
        count: u64,
        signed: i64,
        text: String,
        #[serde(with = "serde_bytes")]
        bytes: Vec<u8>,
        array: [u8; 3],
        values: Vec<u16>,
        present: Option<u32>,
        absent: Option<u32>,
        variants: Vec<FixtureVariant>,
        labels: BTreeMap<String, u64>,
    }

    #[test]
    fn stable_serde_shape_has_exact_cbor_bytes() {
        let fixture = StableShapeFixture {
            flag: true,
            count: 42,
            signed: -7,
            text: "canic".to_string(),
            bytes: vec![0, 1, 255],
            array: [2, 3, 4],
            values: vec![5, 256],
            present: Some(9),
            absent: None,
            variants: vec![
                FixtureVariant::Unit,
                FixtureVariant::Tuple(10, "ten".to_string()),
                FixtureVariant::Struct { enabled: false },
            ],
            labels: BTreeMap::from([("a".to_string(), 1), ("b".to_string(), 2)]),
        };
        let bytes = serialize(&fixture).expect("serialize stable shape fixture");

        assert_eq!(
            bytes,
            hex_fixture(
                "ab64666c6167f565636f756e74182a667369676e65642664746578746563616e6963656279746573430001ff656172726179830203046676616c75657382051901006770726573656e740966616273656e74f66876617269616e74738364556e6974a1655475706c65820a6374656ea166537472756374a167656e61626c6564f4666c6162656c73a2616101616202"
            )
        );
        assert_eq!(
            deserialize::<StableShapeFixture>(&bytes).expect("deserialize stable shape fixture"),
            fixture
        );
    }

    fn hex_fixture(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let pair = std::str::from_utf8(pair).expect("fixture hex is UTF-8");
                u8::from_str_radix(pair, 16).expect("fixture hex byte is valid")
            })
            .collect()
    }
}
