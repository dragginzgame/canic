//! Bounded local-record publication through the shared durable owner.

use super::*;
use crate::test_support::temp_dir;

#[test]
fn local_record_limit_admits_exact_json_and_preserves_old_receipt_on_overflow() {
    let directory = temp_dir("canic-streamed-local-record");
    let path = directory.join("receipt.json");
    let value = (u64::MAX, "bounded 🌍 record");
    let expected = serde_json::to_vec_pretty(&value).unwrap();

    publish_record(&path, &value, expected.len()).unwrap();
    assert_eq!(fs::read(&path).unwrap(), expected);

    assert!(matches!(
        publish_record(&path, &value, expected.len() - 1),
        Err(LocalFleetError::Capacity)
    ));
    assert_eq!(fs::read(&path).unwrap(), expected);
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
    fs::remove_dir_all(directory).unwrap();
}
