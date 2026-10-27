//! Exact native tag and configuration label vectors.
use super::*;
const LIMITS: ic_wire::Limits = ic_wire::Limits {
    bytes: 64,
    allocation: 1024,
    nodes: 32,
    depth: 8,
};

#[test]
fn build_network_labels_and_tags_match_serde() {
    let values = [BuildNetwork::Ic, BuildNetwork::Local];
    let schema = ic_wire::typescript::native_schema::<BuildNetwork>().unwrap();
    for value in values {
        // Exhaustive match makes additions require a new schema/vector.
        let index = match value {
            BuildNetwork::Ic => 0usize,
            BuildNetwork::Local => 1usize,
        };
        let variant = &schema["variants"][index];
        assert_eq!(variant["name"], serde_json::to_value(value).unwrap());
        assert_eq!(variant["id"], index);
        let bytes = ic_wire::native::encode(&value, LIMITS).unwrap();
        assert_eq!(
            bytes,
            ic_wire::native::encode(&u32::try_from(index).unwrap(), LIMITS).unwrap()
        );
        assert_eq!(
            ic_wire::native::decode::<BuildNetwork>(&bytes, LIMITS).unwrap(),
            value
        );
    }
}
