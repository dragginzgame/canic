//! Module: artifact_io::wasm
//!
//! Responsibility: inspect final Canic Wasm structure and release-transform invariants.
//! Does not own: Wasm optimization, installation, or replica validation.
//! Boundary: parses top-level sections around artifact transforms and before publication.

use crate::canister_build::WasmArtifactMetrics;
use std::{collections::BTreeMap, fmt, fs, path::Path};

use canic_core::ids::BuildNetwork;
use ic_host_artifacts::wasm::{ExportKind, InspectionError, InspectionLimits};

// IC resource limits: https://docs.internetcomputer.org/references/resource-limits/
pub(super) const SUPPORTED_CODE_SECTION_LIMIT_BYTES: usize = 12 * 1024 * 1024;
pub(super) const SUPPORTED_DEFINED_FUNCTION_LIMIT: u32 = 50_000;
const IC_WASM_CODE_SECTION_WARNING_BYTES: usize = SUPPORTED_CODE_SECTION_LIMIT_BYTES - 768 * 1024;
#[cfg(test)]
const WASM_HEADER: &[u8; 8] = b"\0asm\x01\0\0\0";
const PUBLIC_CANDID_METADATA_SECTION: &str = "icp:public candid:service";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct WasmContractSnapshot {
    pub exports: BTreeMap<String, u8>,
    pub public_candid_metadata: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct WasmStructure {
    code_section_bytes: usize,
    data_section_bytes: usize,
    defined_functions: u32,
    contract: WasmContractSnapshot,
}

#[derive(Debug, Eq, PartialEq)]
enum WasmCodeSectionError {
    FunctionLimitExceeded { actual: u32, limit: u32 },
    LimitExceeded { actual: usize, limit: usize },
}

impl fmt::Display for WasmCodeSectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FunctionLimitExceeded { actual, limit } => write!(
                formatter,
                "Wasm defines {actual} functions, exceeding the supported limit of {limit}"
            ),
            Self::LimitExceeded { actual, limit } => write!(
                formatter,
                "Wasm code section is {actual} bytes, exceeding the supported limit of {limit} bytes by {} bytes",
                actual - limit
            ),
        }
    }
}

impl std::error::Error for WasmCodeSectionError {}

/// Enforce supported code-size and defined-function limits for every build network.
pub fn enforce_wasm_install_limits(
    build_network: BuildNetwork,
    wasm_path: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    let wasm = fs::read(wasm_path)?;
    let structure = inspect_wasm(&wasm)?;
    validate_wasm_code_section_size(structure.code_section_bytes)
        .and_then(|()| validate_defined_functions(structure.defined_functions))
        .map_err(|source| {
            format!(
                "{build_network:?} Wasm artifact {} cannot be installed: {source}",
                wasm_path.display()
            )
        })?;
    report_install_limit_distance(wasm_path, structure.code_section_bytes);
    Ok(structure.code_section_bytes)
}

const fn validate_defined_functions(count: u32) -> Result<(), WasmCodeSectionError> {
    if count > SUPPORTED_DEFINED_FUNCTION_LIMIT {
        return Err(WasmCodeSectionError::FunctionLimitExceeded {
            actual: count,
            limit: SUPPORTED_DEFINED_FUNCTION_LIMIT,
        });
    }
    Ok(())
}

fn report_install_limit_distance(wasm_path: &Path, code_section_bytes: usize) {
    let status = if code_section_bytes >= IC_WASM_CODE_SECTION_WARNING_BYTES {
        "WARN"
    } else {
        "SIZE"
    };
    eprintln!(
        "{} {}  code={}  headroom={}",
        wasm_progress_prefix(status),
        compact_wasm_artifact_name(wasm_path),
        format_byte_count(code_section_bytes),
        format_byte_count(SUPPORTED_CODE_SECTION_LIMIT_BYTES - code_section_bytes),
    );
}

fn wasm_progress_prefix(status: &str) -> String {
    const SCOPE_WIDTH: usize = 12;
    const STATUS_WIDTH: usize = 6;
    const SCOPE: &str = "[WASM]";

    format!("{SCOPE:<SCOPE_WIDTH$} {status:<STATUS_WIDTH$}")
}

fn compact_wasm_artifact_name(wasm_path: &Path) -> String {
    let file_name = wasm_path.file_name().map_or_else(
        || "artifact.wasm".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let Some(staging_directory) = wasm_path.parent() else {
        return file_name;
    };
    let is_staged_artifact = staging_directory
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with(".canic-artifact-stage-"));
    if !is_staged_artifact {
        return file_name;
    }
    match staging_directory.parent().and_then(Path::file_name) {
        Some(role) => format!("{}/{file_name}", role.to_string_lossy()),
        None => file_name,
    }
}

fn format_byte_count(bytes: usize) -> String {
    const MEBIBYTE: usize = 1024 * 1024;
    let rounded_fraction = ((bytes % MEBIBYTE) * 100 + MEBIBYTE / 2) / MEBIBYTE;
    let whole = bytes / MEBIBYTE + rounded_fraction / 100;
    let fraction = rounded_fraction % 100;
    format!("{whole}.{fraction:02} MiB ({bytes} B)")
}

pub fn wasm_artifact_metrics(
    wasm: &[u8],
    gzip_bytes: usize,
) -> Result<WasmArtifactMetrics, Box<dyn std::error::Error>> {
    let structure = inspect_wasm(wasm)?;
    Ok(WasmArtifactMetrics {
        raw_bytes: u64::try_from(wasm.len())?,
        gzip_bytes: u64::try_from(gzip_bytes)?,
        code_section_bytes: u64::try_from(structure.code_section_bytes)?,
        data_section_bytes: u64::try_from(structure.data_section_bytes)?,
        defined_functions: structure.defined_functions,
    })
}

pub(super) fn wasm_contract_snapshot(
    wasm: &[u8],
) -> Result<WasmContractSnapshot, Box<dyn std::error::Error>> {
    Ok(inspect_wasm(wasm)?.contract)
}

#[cfg(test)]
fn wasm_code_section_size(wasm: &[u8]) -> Result<usize, InspectionError> {
    Ok(inspect_wasm(wasm)?.code_section_bytes)
}

fn inspect_wasm(wasm: &[u8]) -> Result<WasmStructure, InspectionError> {
    // Every section/entry consumes source bytes. These bounds preserve the
    // existing artifact-size admission while bounding shared parser storage.
    let facts = ic_host_artifacts::wasm::inspect(
        wasm,
        InspectionLimits {
            module_bytes: wasm.len(),
            sections: wasm.len() / 2,
            exports: u32::try_from(wasm.len()).unwrap_or(u32::MAX),
            custom_sections: wasm.len() / 2,
        },
    )?;
    let exports = facts
        .exports
        .into_iter()
        .map(|(name, export)| {
            let kind = match export.kind {
                ExportKind::Function => 0,
                ExportKind::Table => 1,
                ExportKind::Memory => 2,
                ExportKind::Global => 3,
                ExportKind::Tag => 4,
            };
            (name.to_owned(), kind)
        })
        .collect();
    let public_candid_metadata = facts
        .custom_sections
        .into_iter()
        .filter(|section| section.name == PUBLIC_CANDID_METADATA_SECTION)
        .map(|section| section.data.to_vec())
        .collect();
    Ok(WasmStructure {
        code_section_bytes: facts.code_section_bytes,
        data_section_bytes: facts.data_section_bytes,
        defined_functions: facts.defined_functions,
        contract: WasmContractSnapshot {
            exports,
            public_candid_metadata,
        },
    })
}

const fn validate_wasm_code_section_size(size: usize) -> Result<(), WasmCodeSectionError> {
    if size > SUPPORTED_CODE_SECTION_LIMIT_BYTES {
        return Err(WasmCodeSectionError::LimitExceeded {
            actual: size,
            limit: SUPPORTED_CODE_SECTION_LIMIT_BYTES,
        });
    }
    Ok(())
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_the_exact_code_section_payload() {
        let wasm = [
            WASM_HEADER.as_slice(),
            &[3, 2, 1, 0],
            &[10, 4, 1, 2, 0, 0x0b],
        ]
        .concat();

        assert_eq!(wasm_code_section_size(&wasm).unwrap(), 4);
    }

    #[test]
    fn rejects_duplicate_code_sections() {
        let wasm = [WASM_HEADER.as_slice(), &[10, 1, 0, 10, 1, 0]].concat();

        assert!(matches!(
            wasm_code_section_size(&wasm),
            Err(InspectionError::Parse(_))
        ));
    }

    #[test]
    fn rejects_invalid_section_size_encoding() {
        let wasm = [WASM_HEADER.as_slice(), &[1, 0x80, 0x80, 0x80, 0x80, 0x10]].concat();

        assert!(matches!(
            wasm_code_section_size(&wasm),
            Err(InspectionError::Parse(_))
        ));
    }

    #[test]
    fn rejects_truncated_sections() {
        let wasm = [WASM_HEADER.as_slice(), &[10, 3, 1, 2]].concat();

        assert!(matches!(
            wasm_code_section_size(&wasm),
            Err(InspectionError::Parse(_))
        ));
    }

    #[test]
    fn accepts_the_exact_ic_code_section_limit() {
        assert_eq!(validate_wasm_code_section_size(12 * 1024 * 1024), Ok(()));
    }

    #[test]
    fn rejects_one_byte_over_the_ic_code_section_limit() {
        assert_eq!(
            validate_wasm_code_section_size(12 * 1024 * 1024 + 1),
            Err(WasmCodeSectionError::LimitExceeded {
                actual: 12 * 1024 * 1024 + 1,
                limit: 12 * 1024 * 1024,
            })
        );
    }

    #[test]
    fn enforces_the_defined_function_boundary() {
        assert_eq!(
            validate_defined_functions(SUPPORTED_DEFINED_FUNCTION_LIMIT),
            Ok(())
        );
        assert_eq!(
            validate_defined_functions(SUPPORTED_DEFINED_FUNCTION_LIMIT + 1),
            Err(WasmCodeSectionError::FunctionLimitExceeded {
                actual: SUPPORTED_DEFINED_FUNCTION_LIMIT + 1,
                limit: SUPPORTED_DEFINED_FUNCTION_LIMIT,
            })
        );
    }

    #[test]
    fn staged_wasm_diagnostics_hide_disposable_paths_but_retain_the_role() {
        let path =
            Path::new("/tmp/operator/artifacts/root/.canic-artifact-stage-19197-2/candidate.wasm");

        assert_eq!(compact_wasm_artifact_name(path), "root/candidate.wasm");
        assert_eq!(format_byte_count(8_713_911), "8.31 MiB (8713911 B)");
    }

    #[test]
    fn wasm_progress_prefix_matches_the_governed_output_columns() {
        assert_eq!(wasm_progress_prefix("SIZE"), "[WASM]       SIZE  ");
    }
}
