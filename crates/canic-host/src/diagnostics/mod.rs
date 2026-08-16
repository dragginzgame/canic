//! Module: canic_host::diagnostics
//!
//! Responsibility: validate permanent diagnostic allocations and expose rich host lookup.
//! Does not own: runtime construction, public wire mapping, or producer-coverage evidence.
//! Boundary: the checked-in host ledger is permanent identity authority; JSON is derived output.

use canic_core::diagnostics::DiagnosticCode;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt, sync::OnceLock};
use thiserror::Error as ThisError;

const ALLOCATION_LEDGER_TOML: &str = include_str!("../../diagnostics/allocations.toml");
const CURRENT_CODES_JSON: &str = include_str!("../../diagnostics/current-codes.json");
const CATALOG_OWNER: &str = "canic_host::diagnostics";

static CATALOG: OnceLock<Result<DiagnosticCatalog, DiagnosticCatalogError>> = OnceLock::new();

///
/// DiagnosticAllocationStatus
///
/// Permanent allocation status retained by the host ledger.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticAllocationStatus {
    Current,
    Retired,
}

impl DiagnosticAllocationStatus {
    /// Return the stable language-neutral status label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::Retired => "retired",
        }
    }
}

impl fmt::Display for DiagnosticAllocationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

///
/// DiagnosticClass
///
/// Broad host-only classification used for presentation and automation.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticClass {
    Conflict,
    Forbidden,
    Internal,
    InvalidInput,
    Invariant,
    NotFound,
    ResourceExhausted,
    Unauthorized,
    Unavailable,
}

impl DiagnosticClass {
    /// Return the stable language-neutral class label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Conflict => "conflict",
            Self::Forbidden => "forbidden",
            Self::Internal => "internal",
            Self::InvalidInput => "invalid_input",
            Self::Invariant => "invariant",
            Self::NotFound => "not_found",
            Self::ResourceExhausted => "resource_exhausted",
            Self::Unauthorized => "unauthorized",
            Self::Unavailable => "unavailable",
        }
    }
}

impl fmt::Display for DiagnosticClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

///
/// DiagnosticOrigin
///
/// Narrow host-only semantic domain that makes a diagnostic actionable.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticOrigin {
    Access,
    Artifact,
    Authentication,
    Blob,
    CanisterLifecycle,
    CanonicalState,
    Configuration,
    ControlPlaneState,
    Platform,
    RegistryDirectory,
    Runtime,
    Storage,
    TopologyAuthority,
}

impl DiagnosticOrigin {
    /// Return the stable language-neutral origin label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Access => "access",
            Self::Artifact => "artifact",
            Self::Authentication => "authentication",
            Self::Blob => "blob",
            Self::CanisterLifecycle => "canister_lifecycle",
            Self::CanonicalState => "canonical_state",
            Self::Configuration => "configuration",
            Self::ControlPlaneState => "control_plane_state",
            Self::Platform => "platform",
            Self::RegistryDirectory => "registry_directory",
            Self::Runtime => "runtime",
            Self::Storage => "storage",
            Self::TopologyAuthority => "topology_authority",
        }
    }
}

impl fmt::Display for DiagnosticOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

///
/// DiagnosticDisposition
///
/// Typed host-only retry or reconciliation behavior.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticDisposition {
    BoundedRetry,
    DoNotRetry,
    ExactRetry,
    Reconcile,
    RetryAfterStateChange,
}

impl DiagnosticDisposition {
    /// Return the stable language-neutral disposition label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BoundedRetry => "bounded_retry",
            Self::DoNotRetry => "do_not_retry",
            Self::ExactRetry => "exact_retry",
            Self::Reconcile => "reconcile",
            Self::RetryAfterStateChange => "retry_after_state_change",
        }
    }
}

impl fmt::Display for DiagnosticDisposition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

///
/// DiagnosticExposure
///
/// Reviewed relationship between an exact diagnostic and its public boundary.
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticExposure {
    Internal,
    Masked,
    Public,
}

impl DiagnosticExposure {
    /// Return the stable language-neutral exposure label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Internal => "internal",
            Self::Masked => "masked",
            Self::Public => "public",
        }
    }
}

impl fmt::Display for DiagnosticExposure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

///
/// DiagnosticEntry
///
/// Validated rich catalogue entry for one current registered diagnostic.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticEntry {
    pub code: DiagnosticCode,
    pub label: String,
    pub class: DiagnosticClass,
    pub origin: DiagnosticOrigin,
    pub disposition: DiagnosticDisposition,
    pub summary: String,
    pub condition: String,
    pub handling_key: String,
    pub producers: Vec<String>,
    pub split_rationale: String,
    pub exposure: DiagnosticExposure,
    pub public_code: Option<DiagnosticCode>,
    pub observability: String,
    pub remediation: String,
    pub action: String,
}

///
/// RetiredDiagnosticEntry
///
/// Permanent minimal ledger entry retained after the final producer is removed.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetiredDiagnosticEntry {
    pub code: DiagnosticCode,
    pub label: String,
    pub summary: String,
}

///
/// DiagnosticLookup
///
/// Lossless lookup outcome that distinguishes current, retired, and unknown identities.
///

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticLookup<'a> {
    Current(&'a DiagnosticEntry),
    Retired(&'a RetiredDiagnosticEntry),
    Unknown(DiagnosticCode),
}

///
/// DiagnosticCatalog
///
/// Validated host-only current and retired diagnostic allocation catalogue.
///

#[derive(Debug)]
pub struct DiagnosticCatalog {
    current: BTreeMap<u16, DiagnosticEntry>,
    retired: BTreeMap<u16, RetiredDiagnosticEntry>,
}

impl DiagnosticCatalog {
    /// Look up a raw identity without guessing metadata for unknown values.
    #[must_use]
    pub fn lookup(&self, code: DiagnosticCode) -> DiagnosticLookup<'_> {
        if let Some(entry) = self.current.get(&code.raw()) {
            return DiagnosticLookup::Current(entry);
        }
        if let Some(entry) = self.retired.get(&code.raw()) {
            return DiagnosticLookup::Retired(entry);
        }
        DiagnosticLookup::Unknown(code)
    }

    /// Iterate over current entries in numeric order.
    #[must_use]
    pub fn current_entries(&self) -> impl ExactSizeIterator<Item = &DiagnosticEntry> {
        self.current.values()
    }

    /// Iterate over retired entries in numeric order.
    #[must_use]
    pub fn retired_entries(&self) -> impl ExactSizeIterator<Item = &RetiredDiagnosticEntry> {
        self.retired.values()
    }
}

///
/// DiagnosticCatalogError
///
/// Invalid embedded ledger or deterministic current-registry rendering.
///

#[derive(Debug, ThisError)]
pub enum DiagnosticCatalogError {
    #[error("invalid diagnostic allocation ledger: {0}")]
    Invalid(String),

    #[error("failed to decode diagnostic allocation ledger: {0}")]
    Ledger(#[from] toml::de::Error),

    #[error("failed to render current diagnostic registry: {0}")]
    Registry(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct AllocationDocument {
    allocation: Vec<AllocationRow>,
}

#[derive(Debug, Deserialize)]
struct AllocationRow {
    code: u16,
    label: String,
    status: DiagnosticAllocationStatus,
    summary: String,
    catalog_owner: Option<String>,
    class: Option<DiagnosticClass>,
    origin: Option<DiagnosticOrigin>,
    disposition: Option<DiagnosticDisposition>,
    condition: Option<String>,
    handling_key: Option<String>,
    producers: Option<Vec<String>>,
    split_rationale: Option<String>,
    exposure: Option<DiagnosticExposure>,
    public_code: Option<u16>,
    observability: Option<String>,
    remediation: Option<String>,
    action: Option<String>,
}

#[cfg(test)]
#[derive(Serialize)]
struct CurrentCodeJsonRow<'a> {
    code: u16,
    label: &'a str,
    class: DiagnosticClass,
    origin: DiagnosticOrigin,
    disposition: DiagnosticDisposition,
    summary: &'a str,
    action: &'a str,
}

/// Return the validated embedded host catalogue.
pub fn diagnostic_catalog() -> Result<&'static DiagnosticCatalog, &'static DiagnosticCatalogError> {
    CATALOG
        .get_or_init(|| parse_catalog(ALLOCATION_LEDGER_TOML))
        .as_ref()
}

/// Look up one lossless raw diagnostic identity in the embedded host catalogue.
pub fn lookup_diagnostic(
    code: DiagnosticCode,
) -> Result<DiagnosticLookup<'static>, &'static DiagnosticCatalogError> {
    diagnostic_catalog().map(|catalog| catalog.lookup(code))
}

/// Return the checked-in language-neutral current-code registry.
#[must_use]
pub const fn current_codes_json() -> &'static str {
    CURRENT_CODES_JSON
}

#[cfg(test)]
fn render_current_codes_json_from(
    catalog: &DiagnosticCatalog,
) -> Result<String, DiagnosticCatalogError> {
    let rows = catalog
        .current_entries()
        .map(|entry| CurrentCodeJsonRow {
            code: entry.code.raw(),
            label: &entry.label,
            class: entry.class,
            origin: entry.origin,
            disposition: entry.disposition,
            summary: &entry.summary,
            action: &entry.action,
        })
        .collect::<Vec<_>>();
    let mut rendered = serde_json::to_string_pretty(&rows)?;
    rendered.push('\n');
    Ok(rendered)
}

fn parse_catalog(source: &str) -> Result<DiagnosticCatalog, DiagnosticCatalogError> {
    let document = toml::from_str::<AllocationDocument>(source)?;
    let mut current = BTreeMap::new();
    let mut labels = BTreeMap::new();
    let mut retired = BTreeMap::new();

    for row in document.allocation {
        if row.code == 0 {
            return Err(invalid("diagnostic code zero is not allocatable"));
        }
        if current.contains_key(&row.code) || retired.contains_key(&row.code) {
            return Err(invalid(format!(
                "diagnostic code {} is allocated more than once",
                row.code
            )));
        }
        if let Some(existing_code) = labels.insert(row.label.clone(), row.code) {
            return Err(invalid(format!(
                "diagnostic label {} is allocated by both E{existing_code} and E{}",
                row.label, row.code
            )));
        }
        match row.status {
            DiagnosticAllocationStatus::Current => {
                let entry = current_entry(row)?;
                current.insert(entry.code.raw(), entry);
            }
            DiagnosticAllocationStatus::Retired => {
                reject_retired_metadata(&row)?;
                retired.insert(
                    row.code,
                    RetiredDiagnosticEntry {
                        code: DiagnosticCode::from_raw(row.code),
                        label: row.label,
                        summary: row.summary,
                    },
                );
            }
        }
    }

    for entry in current.values() {
        if let Some(public_code) = entry.public_code
            && !current.contains_key(&public_code.raw())
        {
            return Err(invalid(format!(
                "diagnostic E{} projects to non-current E{}",
                entry.code.raw(),
                public_code.raw()
            )));
        }
    }

    Ok(DiagnosticCatalog { current, retired })
}

fn current_entry(row: AllocationRow) -> Result<DiagnosticEntry, DiagnosticCatalogError> {
    let code = row.code;
    let catalog_owner = required(row.catalog_owner, code, "catalog_owner")?;
    if catalog_owner != CATALOG_OWNER {
        return Err(invalid(format!(
            "current diagnostic E{code} has unexpected catalog owner {catalog_owner}"
        )));
    }
    let producers = required(row.producers, code, "producers")?;
    if producers.is_empty() {
        return Err(invalid(format!(
            "current diagnostic E{code} has no producer owner"
        )));
    }
    let exposure = required(row.exposure, code, "exposure")?;
    match (exposure, row.public_code) {
        (DiagnosticExposure::Internal, None) | (DiagnosticExposure::Masked, Some(_)) => {}
        (DiagnosticExposure::Public, Some(public_code)) if public_code == code => {}
        _ => {
            return Err(invalid(format!(
                "current diagnostic E{code} has inconsistent exposure and public projection"
            )));
        }
    }

    Ok(DiagnosticEntry {
        code: DiagnosticCode::from_raw(code),
        label: row.label,
        class: required(row.class, code, "class")?,
        origin: required(row.origin, code, "origin")?,
        disposition: required(row.disposition, code, "disposition")?,
        summary: row.summary,
        condition: required(row.condition, code, "condition")?,
        handling_key: required(row.handling_key, code, "handling_key")?,
        producers,
        split_rationale: required(row.split_rationale, code, "split_rationale")?,
        exposure,
        public_code: row.public_code.map(DiagnosticCode::from_raw),
        observability: required(row.observability, code, "observability")?,
        remediation: required(row.remediation, code, "remediation")?,
        action: required(row.action, code, "action")?,
    })
}

fn reject_retired_metadata(row: &AllocationRow) -> Result<(), DiagnosticCatalogError> {
    let has_active_metadata = row.catalog_owner.is_some()
        || row.class.is_some()
        || row.origin.is_some()
        || row.disposition.is_some()
        || row.condition.is_some()
        || row.handling_key.is_some()
        || row.producers.is_some()
        || row.split_rationale.is_some()
        || row.exposure.is_some()
        || row.public_code.is_some()
        || row.observability.is_some()
        || row.remediation.is_some()
        || row.action.is_some();
    if has_active_metadata {
        return Err(invalid(format!(
            "retired diagnostic E{} retains active catalog metadata",
            row.code
        )));
    }
    Ok(())
}

fn required<T>(value: Option<T>, code: u16, field: &str) -> Result<T, DiagnosticCatalogError> {
    value.ok_or_else(|| invalid(format!("current diagnostic E{code} is missing {field}")))
}

fn invalid(message: impl Into<String>) -> DiagnosticCatalogError {
    DiagnosticCatalogError::Invalid(message.into())
}

// -----------------------------------------------------------------------------
// Tests

#[cfg(test)]
mod tests {
    use super::*;
    use canic_core::diagnostics::codes::ALL_REGISTERED_DIAGNOSTIC_CODES;
    use std::collections::BTreeSet;

    #[test]
    fn current_ledger_catalog_runtime_inventory_and_json_are_bijective() {
        let catalog = diagnostic_catalog().expect("embedded diagnostic catalog");
        let catalog_codes = catalog
            .current_entries()
            .map(|entry| entry.code.raw())
            .collect::<BTreeSet<_>>();
        let runtime_codes = ALL_REGISTERED_DIAGNOSTIC_CODES
            .iter()
            .map(|code| code.raw_code().raw())
            .collect::<BTreeSet<_>>();

        assert_eq!(catalog_codes.len(), 991);
        assert_eq!(runtime_codes.len(), ALL_REGISTERED_DIAGNOSTIC_CODES.len());
        assert_eq!(catalog_codes, runtime_codes);
        assert_eq!(catalog.retired_entries().len(), 0);
        assert_eq!(
            render_current_codes_json_from(catalog).expect("render current diagnostic JSON"),
            current_codes_json()
        );
    }

    #[test]
    fn lookup_distinguishes_current_retired_and_unknown_codes() {
        let current = diagnostic_catalog()
            .expect("embedded diagnostic catalog")
            .lookup(DiagnosticCode::from_raw(1));
        assert!(
            matches!(current, DiagnosticLookup::Current(entry) if entry.label == "ACCESS_DEPENDENCY_UNAVAILABLE")
        );

        let fixture = r#"
[[allocation]]
code = 23
label = "FORMER_IDENTITY"
status = "retired"
summary = "Former diagnostic"
"#;
        let retired_catalog = parse_catalog(fixture).expect("retired ledger fixture");
        let retired = retired_catalog.lookup(DiagnosticCode::from_raw(23));
        assert!(
            matches!(retired, DiagnosticLookup::Retired(entry) if entry.label == "FORMER_IDENTITY")
        );

        let unknown = retired_catalog.lookup(DiagnosticCode::from_raw(65_000));
        assert_eq!(
            unknown,
            DiagnosticLookup::Unknown(DiagnosticCode::from_raw(65_000))
        );
    }

    #[test]
    fn invalid_ledger_rejects_reuse_and_retired_active_metadata() {
        let duplicate = r#"
[[allocation]]
code = 23
label = "FIRST"
status = "retired"
summary = "First"

[[allocation]]
code = 23
label = "SECOND"
status = "retired"
summary = "Second"
"#;
        assert!(parse_catalog(duplicate).is_err());

        let active_retired = r#"
[[allocation]]
code = 24
label = "FORMER"
status = "retired"
summary = "Former"
catalog_owner = "canic_host::diagnostics"
"#;
        assert!(parse_catalog(active_retired).is_err());
    }
}
