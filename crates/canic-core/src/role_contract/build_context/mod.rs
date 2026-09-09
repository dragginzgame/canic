//! Module: role_contract::build_context
//!
//! Responsibility: encode exact package/role protocol inputs for one Cargo batch.
//! Does not own: artifact selection, compilation, or runtime protocol dispatch.
//! Boundary: the host and leaf build scripts share one bounded canonical contract.

use crate::role_contract::ProtocolProfileDigest;
use thiserror::Error;

/// Build-script input for a canonical package/role batch.
pub const PROTOCOL_BUILD_CONTEXT_ENV: &str = "CANIC_PROTOCOL_BUILD_CONTEXT";
const MAX_ENTRIES: usize = 128;
const MAX_BYTES: usize = 64 * 1024;

/// Exact protocol authority selected by one package's build script.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolBuildEntry {
    pub package: String,
    pub role: String,
    pub digest: String,
}

/// Invalid or ambiguous batch authority, rejected before compiling a runtime.
#[derive(Debug, Error)]
pub enum ProtocolBuildContextError {
    #[error("protocol build context exceeds its bounded size")]
    Bound,
    #[error("protocol build context is not canonical or has duplicate packages")]
    Canonical,
    #[error("protocol build context contains an invalid digest")]
    Digest,
    #[error("protocol build context has no exact package/role entry")]
    Missing,
}

/// Encode a sorted batch with exactly one authority per Cargo package.
pub fn encode_protocol_build_context(
    mut entries: Vec<ProtocolBuildEntry>,
) -> Result<String, ProtocolBuildContextError> {
    entries.sort_by(|left, right| left.package.cmp(&right.package));
    validate(&entries)?;
    let mut value = String::new();
    for entry in &entries {
        value.push_str(&entry.package);
        value.push('\t');
        value.push_str(&entry.role);
        value.push('\t');
        value.push_str(&entry.digest);
        value.push('\n');
    }
    if value.len() > MAX_BYTES {
        return Err(ProtocolBuildContextError::Bound);
    }
    Ok(value)
}

/// Select an exact entry; unrelated packages never inherit another role's digest.
pub fn select_protocol_build_context(
    value: &str,
    package: &str,
    role: &str,
) -> Result<ProtocolProfileDigest, ProtocolBuildContextError> {
    if value.len() > MAX_BYTES {
        return Err(ProtocolBuildContextError::Bound);
    }
    let entries = value
        .lines()
        .map(|line| {
            let fields = line.split('\t').collect::<Vec<_>>();
            let [package, role, digest] = fields.as_slice() else {
                return Err(ProtocolBuildContextError::Canonical);
            };
            Ok(ProtocolBuildEntry {
                package: (*package).to_string(),
                role: (*role).to_string(),
                digest: (*digest).to_string(),
            })
        })
        .collect::<Result<Vec<_>, ProtocolBuildContextError>>()?;
    validate(&entries)?;
    if encode_protocol_build_context(entries.clone())? != value {
        return Err(ProtocolBuildContextError::Canonical);
    }
    entries
        .iter()
        .find(|entry| entry.package == package && entry.role == role)
        .ok_or(ProtocolBuildContextError::Missing)?
        .digest
        .parse()
        .map_err(|_| ProtocolBuildContextError::Digest)
}

fn validate(entries: &[ProtocolBuildEntry]) -> Result<(), ProtocolBuildContextError> {
    if entries.is_empty() || entries.len() > MAX_ENTRIES {
        return Err(ProtocolBuildContextError::Bound);
    }
    if entries
        .iter()
        .any(|entry| !valid_name(&entry.package) || !valid_name(&entry.role))
        || entries
            .windows(2)
            .any(|pair| pair[0].package >= pair[1].package)
    {
        return Err(ProtocolBuildContextError::Canonical);
    }
    if entries
        .iter()
        .any(|entry| entry.digest.parse::<ProtocolProfileDigest>().is_err())
    {
        return Err(ProtocolBuildContextError::Digest);
    }
    Ok(())
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_selects_exact_package_and_role() {
        let entries = [1, 2].map(|id| ProtocolBuildEntry {
            package: format!("package-{id}"),
            role: format!("role-{id}"),
            digest: ProtocolProfileDigest::from_bytes([id; 32]).to_string(),
        });
        let value = encode_protocol_build_context(entries.to_vec()).unwrap();
        assert_eq!(
            select_protocol_build_context(&value, "package-2", "role-2").unwrap(),
            ProtocolProfileDigest::from_bytes([2; 32])
        );
        assert!(matches!(
            select_protocol_build_context(&value, "package-1", "role-2"),
            Err(ProtocolBuildContextError::Missing)
        ));
        assert!(matches!(
            encode_protocol_build_context(vec![entries[0].clone(); 2]),
            Err(ProtocolBuildContextError::Canonical)
        ));
        assert!(matches!(
            select_protocol_build_context(&format!(" {value}"), "package-1", "role-1"),
            Err(ProtocolBuildContextError::Canonical)
        ));
    }
}
