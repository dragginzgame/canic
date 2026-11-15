use super::*;

#[test]
fn maximal_report_preserves_quality_and_refuses_wrong_frames() {
    let counter = Counter {
        total: u128::MAX,
        overflowed: true,
    };
    let value = Snapshot {
        observed_at_ns: u64::MAX,
        timers: Some(Timers {
            started_at_version: u64::MAX,
            started_at_ns: u64::MAX,
            accepted: counter,
            completed: counter,
            abandoned: counter,
            expected_failures: counter,
            deferred_submissions: counter,
            provider_faults: counter,
        }),
    };
    let encoding = ic_wire::Encoding::Frame {
        method: METHOD,
        direction: ic_wire::Direction::Response,
    };
    let bytes = ic_wire::encode(&value, encoding, RESPONSE_LIMITS).unwrap();
    assert_eq!(
        ic_wire::decode::<Snapshot>(&bytes, encoding, RESPONSE_LIMITS).unwrap(),
        value
    );
    let wrong = ic_wire::Encoding::Frame {
        method: METHOD + 1,
        direction: ic_wire::Direction::Response,
    };
    assert!(ic_wire::decode::<Snapshot>(&bytes, wrong, RESPONSE_LIMITS).is_err());
    assert!(
        ic_wire::decode::<Snapshot>(
            &bytes,
            encoding,
            ic_wire::Limits {
                bytes: bytes.len() - 1,
                ..RESPONSE_LIMITS
            }
        )
        .is_err()
    );
    let json = serde_json::to_value(value).unwrap();
    assert_eq!(json["timers"]["accepted"]["total"], u128::MAX.to_string());
    assert_eq!(serde_json::from_value::<Snapshot>(json).unwrap(), value);
    assert_ne!(
        Counter {
            overflowed: false,
            ..counter
        },
        counter
    );
}
