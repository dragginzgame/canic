use super::{BusinessRecord, ops};
use canic::__internal::core::cdk::structures::storable::{Bound, Storable};

#[test]
fn business_record_preserves_full_range_counters_and_explicit_empty_checkpoint() {
    let record = BusinessRecord {
        checkpoint: None,
        counter: u64::MAX,
        prepare_runs: u64::MAX,
        release_runs: u64::MAX,
    };
    let bytes = ops::encode(&record).unwrap();
    assert_eq!(ops::decode(&bytes).unwrap(), record);
    assert_eq!(
        BusinessRecord::from_bytes(record.to_bytes().expect("encode checkpoint record"))
            .expect("decode checkpoint record"),
        record,
    );
    assert!(
        matches!(BusinessRecord::BOUND, Bound::Bounded { max_size, is_fixed_size: false }
        if max_size as usize == ops::RECORD_LIMITS.bytes)
    );
}

#[test]
fn business_record_refuses_incomplete_trailing_and_oversized_wire() {
    let bytes = ops::encode(&BusinessRecord::default()).unwrap();
    for end in 0..bytes.len() {
        assert!(ops::decode(&bytes[..end]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(ops::decode(&trailing), Err(ic_wire::Error::Trailing));
    let mut oversized = bytes;
    oversized.resize(ops::RECORD_LIMITS.bytes + 1, 0);
    assert_eq!(ops::decode(&oversized), Err(ic_wire::Error::Limit));
}
