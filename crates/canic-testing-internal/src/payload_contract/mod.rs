//! Decode compiled payload-limit evidence for the external-Candid admission fixture.
//! This module does not enumerate endpoints or infer limits from prose.

use canic_core::ingress::payload_contract::{CANDID_PAYLOAD_CONTRACT_PREFIX, PayloadContract};
use std::{collections::BTreeSet, io};

/// Read one bounded, complete metadata record produced by the fixture declaration pass.
pub fn read(candid: &str) -> io::Result<PayloadContract> {
    let invalid = || io::Error::from(io::ErrorKind::InvalidData);
    let mut records = candid
        .lines()
        .filter_map(|line| line.strip_prefix(CANDID_PAYLOAD_CONTRACT_PREFIX));
    let encoded = records.next().ok_or_else(invalid)?;
    if records.next().is_some() || encoded.len() > 1024 * 1024 {
        return Err(invalid());
    }
    let bytes = canic_core::cdk::utils::hash::decode_hex(encoded).map_err(|_| invalid())?;
    let mut remaining = bytes.as_slice();
    let contract: PayloadContract =
        serde_cbor::from_reader(&mut remaining).map_err(|_| invalid())?;
    if contract.schema_version != 1 || !remaining.is_empty() {
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
    #[test]
    fn metadata_requires_one_complete_record() {
        let contract = PayloadContract {
            schema_version: 1,
            default_update_ingress_max_bytes: 1024,
            update_overrides: Vec::new(),
        };
        let mut bytes = Vec::new();
        serde_cbor::to_writer(&mut bytes, &contract).unwrap();
        let record = format!(
            "{CANDID_PAYLOAD_CONTRACT_PREFIX}{}",
            canic_core::cdk::utils::hash::hex_bytes(&bytes)
        );
        assert_eq!(read(&record).unwrap(), contract);
        for invalid in [
            String::new(),
            format!("{record}\n{record}"),
            format!("{record}00"),
        ] {
            assert_eq!(
                read(&invalid).unwrap_err().kind(),
                io::ErrorKind::InvalidData
            );
        }
    }
}
