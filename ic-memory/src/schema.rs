use serde::{Deserialize, Serialize};

///
/// SchemaMetadata
///
/// Optional diagnostic metadata for an in-place store schema.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SchemaMetadata {
    /// Optional in-place schema version.
    pub schema_version: Option<u32>,
    /// Optional opaque schema fingerprint.
    pub schema_fingerprint: Option<String>,
}

impl SchemaMetadata {
    /// Construct schema metadata after validating the persisted encoding bounds.
    pub fn new(
        schema_version: Option<u32>,
        schema_fingerprint: Option<String>,
    ) -> Result<Self, SchemaMetadataError> {
        let metadata = Self {
            schema_version,
            schema_fingerprint,
        };
        metadata.validate()?;
        Ok(metadata)
    }

    /// Validate schema metadata encoding rules.
    pub fn validate(&self) -> Result<(), SchemaMetadataError> {
        if self.schema_version == Some(0) {
            return Err(SchemaMetadataError::InvalidVersion);
        }

        let Some(fingerprint) = &self.schema_fingerprint else {
            return Ok(());
        };

        if fingerprint.is_empty() {
            return Err(SchemaMetadataError::EmptyFingerprint);
        }
        if fingerprint.len() > 256 {
            return Err(SchemaMetadataError::FingerprintTooLong);
        }
        if !fingerprint.is_ascii() {
            return Err(SchemaMetadataError::NonAsciiFingerprint);
        }
        if fingerprint.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(SchemaMetadataError::ControlCharacterFingerprint);
        }

        Ok(())
    }
}

///
/// SchemaMetadataError
///
/// Schema metadata validation failure.
#[derive(Clone, Debug, Eq, thiserror::Error, PartialEq)]
pub enum SchemaMetadataError {
    /// Schema version zero is reserved for absence.
    #[error("schema_version must be greater than zero when present")]
    InvalidVersion,
    /// Present fingerprints must be non-empty.
    #[error("schema_fingerprint must not be empty when present")]
    EmptyFingerprint,
    /// Fingerprints must stay bounded for durable ledger storage.
    #[error("schema_fingerprint must be at most 256 bytes")]
    FingerprintTooLong,
    /// Fingerprints must not require Unicode normalization.
    #[error("schema_fingerprint must be ASCII")]
    NonAsciiFingerprint,
    /// Fingerprints must be printable metadata.
    #[error("schema_fingerprint must not contain ASCII control characters")]
    ControlCharacterFingerprint,
}
