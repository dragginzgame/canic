//! Qualify the current stable identity, full-range amounts and refusal boundaries.

use super::{STORAGE_HEADER, STORAGE_LIMITS, SerializeError, deserialize, serialize};
use crate::cycles::Cycles;
use ic_wire::Error;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum Entry {
    Amount(u128),
    Pair(u128, Option<u128>),
    Record { amount: u128 },
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Record {
    cycles: Cycles,
    sequence: Vec<u128>,
    array: [u128; 2],
    tuple: (u128, bool),
    map: BTreeMap<u128, Option<u128>>,
    entries: Vec<Entry>,
}

#[test]
fn full_range_amounts_use_native_wire_in_nested_stable_records() {
    for amount in [
        0,
        1,
        u128::from(u64::MAX),
        u128::from(u64::MAX) + 1,
        u128::MAX,
    ] {
        let bytes = serialize(&amount).unwrap();
        let native = ic_wire::native::encode(&amount, STORAGE_LIMITS).unwrap();
        assert_eq!(
            bytes,
            [STORAGE_HEADER.as_slice(), native.as_slice()].concat()
        );
        assert_eq!(deserialize::<u128>(&bytes).unwrap(), amount);
        let record = Record {
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
    let mut maximum = STORAGE_HEADER.to_vec();
    maximum.extend([0xff; 18]);
    maximum.push(3);
    assert_eq!(serialize(&u128::MAX).unwrap(), maximum);
}

#[test]
fn storage_identity_precedes_record_visitors() {
    struct NeverDecode;
    impl<'de> Deserialize<'de> for NeverDecode {
        fn deserialize<D: serde::Deserializer<'de>>(_: D) -> Result<Self, D::Error> {
            panic!("identity refusal must precede record decoding");
        }
    }
    assert!(matches!(
        deserialize::<NeverDecode>(b"wrong"),
        Err(SerializeError::Deserialize(Error::Identity))
    ));
}

#[test]
fn complete_values_refuse_truncation_trailing_data_and_invalid_variants() {
    let record = Record {
        cycles: Cycles::new(u128::MAX),
        sequence: vec![1, 2],
        array: [3, 4],
        tuple: (5, true),
        map: BTreeMap::new(),
        entries: vec![Entry::Amount(6)],
    };
    let bytes = serialize(&record).unwrap();
    for length in 0..bytes.len() {
        assert!(matches!(
            deserialize::<Record>(&bytes[..length]),
            Err(SerializeError::Deserialize(_))
        ));
    }
    let mut trailing = bytes;
    trailing.push(0);
    assert!(matches!(
        deserialize::<Record>(&trailing),
        Err(SerializeError::Deserialize(Error::Trailing))
    ));
    let mut invalid = STORAGE_HEADER.to_vec();
    invalid.push(127);
    assert!(matches!(
        deserialize::<Entry>(&invalid),
        Err(SerializeError::Deserialize(Error::Malformed))
    ));
}

#[test]
fn explicit_optional_fields_survive_none_and_refuse_incomplete_records() {
    use crate::{
        dto::component_provisioning::{
            ProvisioningFailureOrigin, ProvisioningFailureStage, ProvisioningRetryCategory,
        },
        values::provisioning_failure::ProvisioningDependencyFailure,
    };
    for retry_at_ns in [Some(u64::MAX), None] {
        let origin = ProvisioningFailureOrigin {
            dependency: Some(ProvisioningDependencyFailure {
                target: ic_platform::principal::ExternalPrincipal::from_slice(&[8]),
                method: crate::protocol::store_publication::inspect::METHOD.into(),
                step: "inspect_template".into(),
                rejection_code: None,
                rejection_detail: None,
            }),
            failed_at_ns: 123,
            retry_at_ns,
            stage: ProvisioningFailureStage::ComponentMembership,
            target: ic_platform::principal::ExternalPrincipal::from_slice(&[8]),
            operation_id: [9; 32],
            diagnostic_code: 137,
            retry_category: ProvisioningRetryCategory::Backoff,
        };
        let bytes = serialize(&origin).unwrap();
        assert_eq!(
            deserialize::<ProvisioningFailureOrigin>(&bytes).unwrap(),
            origin
        );
        for length in 0..bytes.len() {
            assert!(matches!(
                deserialize::<ProvisioningFailureOrigin>(&bytes[..length]),
                Err(SerializeError::Deserialize(_))
            ));
        }
    }
}

#[test]
fn storage_admission_bounds_deep_records_and_declared_collections() {
    #[derive(Serialize)]
    enum Nested {
        End,
        Next(Box<Self>),
    }
    let mut value = Nested::End;
    for _ in 0..STORAGE_LIMITS.depth {
        value = Nested::Next(Box::new(value));
    }
    assert!(matches!(
        serialize(&value),
        Err(SerializeError::Serialize(Error::Limit))
    ));
    let excessive_count = u64::try_from(STORAGE_LIMITS.nodes).unwrap() + 1;
    let bytes = serialize(&excessive_count).unwrap();
    assert!(matches!(
        deserialize::<Vec<()>>(&bytes),
        Err(SerializeError::Deserialize(Error::Limit))
    ));
}
