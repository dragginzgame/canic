//! Module: manifest::validation::scalar
//!
//! Responsibility: validate scalar manifest fields and repeated scalar sets.
//! Does not own: section-level validation, topology binding, or persistence.
//! Boundary: returns typed manifest validation errors for primitive fields.

use crate::manifest::{ManifestValidationError, validation::SUPPORTED_MANIFEST_VERSION};

use std::{collections::BTreeSet, str::FromStr};

use candid::Principal;

pub(super) const fn validate_manifest_version(version: u16) -> Result<(), ManifestValidationError> {
    if version == SUPPORTED_MANIFEST_VERSION {
        Ok(())
    } else {
        Err(ManifestValidationError::UnsupportedManifestVersion(version))
    }
}

pub(super) fn validate_nonempty(
    field: &'static str,
    value: &str,
) -> Result<(), ManifestValidationError> {
    if value.trim().is_empty() {
        Err(ManifestValidationError::EmptyField(field))
    } else {
        Ok(())
    }
}

pub(super) fn validate_optional_nonempty(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ManifestValidationError> {
    if let Some(value) = value {
        validate_nonempty(field, value)?;
    }
    Ok(())
}

pub(super) fn validate_unique_values<F>(
    field: &'static str,
    values: &[String],
    error: F,
) -> Result<(), ManifestValidationError>
where
    F: Fn(&str) -> ManifestValidationError,
{
    let mut seen = BTreeSet::new();
    for value in values {
        validate_nonempty(field, value)?;
        if !seen.insert(value.as_str()) {
            return Err(error(value));
        }
    }

    Ok(())
}

pub(super) fn validate_principal(
    field: &'static str,
    value: &str,
) -> Result<(), ManifestValidationError> {
    validate_nonempty(field, value)?;
    Principal::from_str(value)
        .map(|_| ())
        .map_err(|_| ManifestValidationError::InvalidPrincipal {
            field,
            value: value.to_string(),
        })
}

pub(super) fn validate_optional_principal(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ManifestValidationError> {
    if let Some(value) = value {
        validate_principal(field, value)?;
    }
    Ok(())
}

pub(super) fn validate_hash(
    field: &'static str,
    value: &str,
) -> Result<(), ManifestValidationError> {
    const SHA256_HEX_LEN: usize = 64;
    validate_nonempty(field, value)?;
    if value.len() == SHA256_HEX_LEN && value.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(ManifestValidationError::InvalidHash(field))
    }
}

pub(super) fn validate_optional_hash(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), ManifestValidationError> {
    if let Some(value) = value {
        validate_hash(field, value)?;
    }
    Ok(())
}
