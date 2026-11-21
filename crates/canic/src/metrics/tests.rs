use super::*;
use crate::method::Contract;

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
        method: ReadMetrics::ID,
        direction: ic_wire::Direction::Response,
    };
    let bytes = ic_wire::encode(&value, encoding, ReadMetrics::REPLY).unwrap();
    assert_eq!(
        ic_wire::decode::<Snapshot>(&bytes, encoding, ReadMetrics::REPLY).unwrap(),
        value
    );
    let wrong = ic_wire::Encoding::Frame {
        method: ReadMetrics::ID + 1,
        direction: ic_wire::Direction::Response,
    };
    assert!(ic_wire::decode::<Snapshot>(&bytes, wrong, ReadMetrics::REPLY).is_err());
    assert!(
        ic_wire::decode::<Snapshot>(
            &bytes,
            encoding,
            ic_wire::Limits {
                bytes: bytes.len() - 1,
                ..ReadMetrics::REPLY
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

#[test]
fn complete_blob_report_fits_the_shared_reply_budget() {
    let value = Counter {
        total: u128::MAX,
        overflowed: true,
    };
    let report = Report::Blob {
        commands: [
            "authorize",
            "register",
            "retain",
            "release",
            "expire",
            "drain",
            "activate",
        ]
        .map(|name| BlobCommand {
            command: name.into(),
            succeeded: value,
            denied: value,
            capacity: value,
            expired: value,
            other_refusals: value,
        })
        .into(),
        sizes: BlobSizes {
            upper_bounds: [u64::MAX; 4],
            buckets: [value; 4],
            above: value,
        },
    };
    let bytes = ReadMetrics::encode_reply(&report).unwrap();
    ReadMetrics::decode_reply(&bytes).unwrap();
}

#[test]
fn binary_counter_matches_its_two_inspected_fields() {
    let encoding = ic_wire::Encoding::Frame {
        method: ReadMetrics::ID,
        direction: ic_wire::Direction::Response,
    };
    for (total, overflowed) in [(0, false), (u128::MAX, true)] {
        let bytes =
            ic_wire::encode(&Counter { total, overflowed }, encoding, ReadMetrics::REPLY).unwrap();
        let fields = ic_wire::decode::<(u128, bool)>(&bytes, encoding, ReadMetrics::REPLY).unwrap();
        assert_eq!(fields, (total, overflowed));
    }
}
