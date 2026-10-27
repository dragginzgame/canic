//! Full-range amounts remain exact through stored nesting and current-format refusal.

use crate::{
    cycles::Cycles,
    serialization::{deserialize, serialize},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Amount(u128);

#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Entry {
    Amount(u128),
    Pair(u128, Option<u128>),
    Record { amount: u128 },
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Record {
    amount: Amount,
    cycles: Cycles,
    sequence: Vec<u128>,
    array: [u128; 2],
    tuple: (u128, bool),
    map: BTreeMap<u128, Option<u128>>,
    entries: Vec<Entry>,
}

#[test]
fn unsigned_128_storage_covers_the_full_range_and_nested_records() {
    for amount in [
        0,
        1,
        u128::from(u64::MAX),
        u128::from(u64::MAX) + 1,
        u128::MAX,
    ] {
        let bytes = serialize(&amount).unwrap();
        assert_eq!(bytes, [&[0x50][..], &amount.to_be_bytes()].concat());
        assert_eq!(deserialize::<u128>(&bytes).unwrap(), amount);
        let record = Record {
            amount: Amount(amount),
            cycles: Cycles::new(amount),
            sequence: vec![amount],
            array: [amount; 2],
            tuple: (amount, true),
            map: BTreeMap::from([(amount, Some(amount)), (0, None)]),
            entries: vec![
                Entry::Amount(amount),
                Entry::Pair(amount, Some(amount)),
                Entry::Record { amount },
            ],
        };
        assert_eq!(
            deserialize::<Record>(&serialize(&record).unwrap()).unwrap(),
            record
        );
    }
}

#[test]
fn storage_refuses_wrong_amount_shapes_and_trailing_data() {
    for value in [
        serde_cbor::Value::Integer(1),
        serde_cbor::Value::Bytes(vec![0; 15]),
        serde_cbor::Value::Bytes(vec![0; 17]),
        serde_cbor::Value::Array(vec![serde_cbor::Value::Integer(0); 16]),
    ] {
        assert!(deserialize::<u128>(&serde_cbor::to_vec(&value).unwrap()).is_err());
    }
    let mut bytes = serialize(&u128::MAX).unwrap();
    bytes.push(0);
    assert!(deserialize::<u128>(&bytes).is_err());
}

#[test]
fn storage_retains_duplicate_fields_and_requires_complete_container_consumption() {
    #[derive(Debug, Deserialize)]
    struct Field {
        #[serde(rename = "x")]
        _value: u64,
    }
    // Two occurrences of x must reach the record visitor, not collapse in a map.
    assert!(deserialize::<Field>(&[0xa2, 0x61, b'x', 1, 0x61, b'x', 2]).is_err());
    assert!(deserialize::<(u64,)>(&[0x82, 1, 2]).is_err());
    let two_variants = serde_cbor::Value::Map(BTreeMap::from([
        (
            serde_cbor::Value::Text("Amount".into()),
            serde_cbor::Value::Null,
        ),
        (
            serde_cbor::Value::Text("Pair".into()),
            serde_cbor::Value::Null,
        ),
    ]));
    assert!(deserialize::<Entry>(&serde_cbor::to_vec(&two_variants).unwrap()).is_err());
}

#[test]
fn storage_parser_bounds_nesting_and_does_not_trust_declared_capacity() {
    let mut nested = vec![0x81; 130];
    nested.push(0);
    assert!(deserialize::<serde_cbor::Value>(&nested).is_err());
    // A truncated declared array cannot reserve its advertised u64::MAX entries.
    assert!(deserialize::<Vec<u64>>(&[0x9b, 255, 255, 255, 255, 255, 255, 255, 255]).is_err());
}

#[test]
fn storage_parser_borrows_definite_bytes_and_text() {
    use super::de::Value;
    use std::borrow::Cow;
    assert!(matches!(
        serde_cbor::from_slice::<Value<'_>>(&[0x42, 1, 2]).unwrap(),
        Value::Bytes(Cow::Borrowed(&[1, 2]))
    ));
    assert!(matches!(
        serde_cbor::from_slice::<Value<'_>>(&[0x61, b'x']).unwrap(),
        Value::Text(Cow::Borrowed("x"))
    ));
}
