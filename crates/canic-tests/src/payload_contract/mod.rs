//! Decode compiled payload-limit evidence for the external-Candid admission fixture.
//! This module does not enumerate endpoints or infer limits from prose.

use canic_core::ingress::payload_contract::{
    CANDID_PAYLOAD_CONTRACT_PREFIX, PAYLOAD_CONTRACT_HEADER, PAYLOAD_CONTRACT_LIMITS,
    PayloadContract,
};
use std::{collections::BTreeSet, io};

/// Read one bounded, complete metadata record produced by the fixture declaration pass.
pub fn read(candid: &str) -> io::Result<PayloadContract> {
    let invalid = || io::Error::from(io::ErrorKind::InvalidData);
    let mut records = candid
        .lines()
        .filter_map(|line| line.strip_prefix(CANDID_PAYLOAD_CONTRACT_PREFIX));
    let encoded = records.next().ok_or_else(invalid)?;
    if records.next().is_some() || encoded.len() > 2 * PAYLOAD_CONTRACT_LIMITS.bytes {
        return Err(invalid());
    }
    let bytes = canic_core::cdk::utils::hash::decode_hex(encoded).map_err(|_| invalid())?;
    if !bytes.starts_with(&PAYLOAD_CONTRACT_HEADER) {
        return Err(invalid());
    }
    let (_, contract): ([u8; 4], PayloadContract) =
        ic_wire::native::decode(&bytes, PAYLOAD_CONTRACT_LIMITS).map_err(|_| invalid())?;
    if contract.schema_version != 1 {
        return Err(invalid());
    }
    let mut names = BTreeSet::new();
    if contract
        .update_overrides
        .iter()
        .any(|entry| !names.insert(entry.method.as_str()))
    {
        return Err(invalid());
    }
    Ok(contract)
}

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::ingress::payload_contract::UpdatePayloadDescriptor;

    fn record(contract: &PayloadContract) -> String {
        let bytes = ic_wire::native::encode(
            &(PAYLOAD_CONTRACT_HEADER, contract),
            PAYLOAD_CONTRACT_LIMITS,
        )
        .unwrap();
        format!(
            "{CANDID_PAYLOAD_CONTRACT_PREFIX}{}",
            canic_core::cdk::utils::hash::hex_bytes(&bytes)
        )
    }

    #[test]
    fn metadata_requires_one_complete_record() {
        let contract = PayloadContract {
            schema_version: 1,
            default_update_ingress_max_bytes: 1024,
            update_overrides: Vec::new(),
        };
        let record = record(&contract);
        assert_eq!(read(&record).unwrap(), contract);
        for invalid in [
            String::new(),
            format!("{record}\n{record}"),
            format!("{record}00"),
            record[..record.len() - 2].to_owned(),
            format!("{CANDID_PAYLOAD_CONTRACT_PREFIX}zz"),
            format!(
                "{CANDID_PAYLOAD_CONTRACT_PREFIX}{}",
                "00".repeat(PAYLOAD_CONTRACT_LIMITS.bytes + 1)
            ),
        ] {
            assert_eq!(
                read(&invalid).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn declaration_producer_round_trips_through_the_actual_reader() {
        let source = canic_core::ingress::payload_contract::annotate_candid(
            "service : { external_query : () -> () query; };".into(),
        );
        let contract = read(&source).unwrap();
        assert_eq!(contract.schema_version, 1);
        assert!(contract.default_update_ingress_max_bytes > 0);
    }

    #[test]
    fn metadata_preserves_schema_method_and_identity_admission() {
        let mut contract = PayloadContract {
            schema_version: 1,
            default_update_ingress_max_bytes: u64::MAX,
            update_overrides: vec![UpdatePayloadDescriptor {
                method: "bounded_update".into(),
                max_bytes: u64::MAX,
            }],
        };
        assert_eq!(read(&record(&contract)).unwrap(), contract);
        contract.schema_version = 0;
        assert_eq!(
            read(&record(&contract)).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        contract.schema_version = 1;
        contract
            .update_overrides
            .push(contract.update_overrides[0].clone());
        assert_eq!(
            read(&record(&contract)).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );

        contract.update_overrides.pop();
        let mut encoded = record(&contract);
        assert_eq!(read(&encoded).unwrap(), contract);
        let marker_start = CANDID_PAYLOAD_CONTRACT_PREFIX.len();
        encoded.replace_range(marker_start..marker_start + 2, "00");
        assert_eq!(
            read(&encoded).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
