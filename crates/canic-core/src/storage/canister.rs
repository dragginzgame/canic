use crate::storage::prelude::*;

///
/// CanisterRecord
/// Full registry entry (authoritative)
///

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CanisterRecord {
    pub role: CanisterRole,
    pub parent_pid: Option<Principal>,
    pub module_hash: Option<Vec<u8>>,
    pub created_at: u64,
}

impl CanisterRecord {
    pub const STORABLE_MAX_SIZE: u32 = 256;
}

impl_storable_bounded!(CanisterRecord, CanisterRecord::STORABLE_MAX_SIZE, false);

///
/// TESTS
///

#[cfg(test)]
pub mod test {
    use super::*;
    use crate::cdk::structures::Storable;
    use std::str::FromStr;

    #[test]
    fn canister_record_fits_within_max() {
        let pid =
            Principal::from_str("pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae")
                .unwrap();

        let record = CanisterRecord {
            role: CanisterRole::new("really_long_canister_role"),
            parent_pid: Some(pid),
            module_hash: Some(vec![0u8; 32]),
            created_at: u64::MAX,
        };
        let bytes = record.to_bytes();

        assert!(
            bytes.len() <= CanisterRecord::STORABLE_MAX_SIZE as usize,
            "Size {} exceeded bound: {} bytes",
            CanisterRecord::STORABLE_MAX_SIZE,
            bytes.len()
        );
    }
}
