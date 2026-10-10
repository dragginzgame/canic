//! Module: storage::stable::template::key
//!
//! Responsibility: encode shared template identities as bounded stable keys.
//! Does not own: identity declarations, template validation, or publication.
//! Boundary: transparent records preserve the canonical current CBOR key bytes.

use canic_contracts::ids::{TemplateChunkKey, TemplateReleaseKey};
use canic_core::impl_storable_bounded;
use serde::{Deserialize, Serialize};

/// Stable manifest/chunk-set key with the existing 256-byte storage ceiling.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(in crate::storage) struct TemplateReleaseKeyRecord(pub TemplateReleaseKey);

impl_storable_bounded!(TemplateReleaseKeyRecord, 256, false);

/// Stable chunk-reference key with the existing 320-byte storage ceiling.
#[cfg_attr(
    not(any(feature = "wasm-store-canister", test)),
    expect(
        dead_code,
        reason = "Root retains the registered chunk-reference map without constructing Store keys"
    )
)]
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(in crate::storage) struct TemplateChunkKeyRecord(pub TemplateChunkKey);

impl_storable_bounded!(TemplateChunkKeyRecord, 320, false);

#[cfg(test)]
mod tests {
    use super::*;
    use canic_contracts::{
        ids::{TemplateId, TemplateVersion},
        serialization::serialize,
    };
    use canic_core::cdk::structures::Storable;

    #[test]
    fn storage_records_preserve_shared_key_bytes() {
        let release = TemplateReleaseKey {
            template_id: TemplateId::new("worker"),
            version: TemplateVersion::new("0.110.54"),
        };
        let stored = TemplateReleaseKeyRecord(release.clone());
        assert_eq!(stored.to_bytes().as_ref(), serialize(&release).unwrap());
        assert_eq!(
            TemplateReleaseKeyRecord::from_bytes(stored.to_bytes()).0,
            release
        );
        let chunk = TemplateChunkKey::new(release, u32::MAX);
        let stored = TemplateChunkKeyRecord(chunk.clone());
        assert_eq!(stored.to_bytes().as_ref(), serialize(&chunk).unwrap());
        assert_eq!(
            TemplateChunkKeyRecord::from_bytes(stored.to_bytes()).0,
            chunk
        );
    }
}
