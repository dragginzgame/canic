//! Module: artifact_io::wasm
//!
//! Responsibility: inspect final Canic Wasm structure and release-transform invariants.
//! Does not own: Wasm optimization, installation, or replica validation.
//! Boundary: parses top-level sections around artifact transforms and before publication.

use std::{collections::BTreeMap, fmt, fs, path::Path};

use crate::canister_build::WasmArtifactMetrics;

const IC_WASM_CODE_SECTION_LIMIT_BYTES: usize = 10 * 1024 * 1024;
const IC_WASM_CODE_SECTION_WARNING_BYTES: usize = 9 * 1024 * 1024 + 256 * 1024;
const WASM_HEADER: &[u8; 8] = b"\0asm\x01\0\0\0";
const WASM_CUSTOM_SECTION_ID: u8 = 0;
const WASM_FUNCTION_SECTION_ID: u8 = 3;
const WASM_EXPORT_SECTION_ID: u8 = 7;
const WASM_CODE_SECTION_ID: u8 = 10;
const WASM_DATA_SECTION_ID: u8 = 11;
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
    DuplicateCodeSection,

    DuplicateDataSection,

    DuplicateExport {
        name: String,
    },

    DuplicateExportSection,

    DuplicateFunctionSection,

    InvalidHeader,

    InvalidName {
        offset: usize,
    },

    InvalidSectionPayload {
        section_id: u8,
        offset: usize,
    },

    InvalidSectionSize {
        offset: usize,
    },

    LimitExceeded {
        actual: usize,
        limit: usize,
    },

    TruncatedSection {
        offset: usize,
        declared: usize,
        remaining: usize,
    },
}

impl fmt::Display for WasmCodeSectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCodeSection => write!(formatter, "duplicate Wasm code section"),
            Self::DuplicateDataSection => write!(formatter, "duplicate Wasm data section"),
            Self::DuplicateExport { name } => {
                write!(formatter, "duplicate Wasm export name {name}")
            }
            Self::DuplicateExportSection => write!(formatter, "duplicate Wasm export section"),
            Self::DuplicateFunctionSection => {
                write!(formatter, "duplicate Wasm function section")
            }
            Self::InvalidHeader => write!(formatter, "invalid Wasm module header"),
            Self::InvalidName { offset } => {
                write!(formatter, "invalid UTF-8 Wasm name at byte {offset}")
            }
            Self::InvalidSectionPayload { section_id, offset } => write!(
                formatter,
                "invalid Wasm section {section_id} payload at byte {offset}"
            ),
            Self::InvalidSectionSize { offset } => {
                write!(formatter, "invalid Wasm section size at byte {offset}")
            }
            Self::LimitExceeded { actual, limit } => write!(
                formatter,
                "Wasm code section is {actual} bytes, exceeding the IC limit of {limit} bytes by {} bytes",
                actual - limit
            ),
            Self::TruncatedSection {
                offset,
                declared,
                remaining,
            } => write!(
                formatter,
                "Wasm section at byte {offset} declares {declared} bytes with only {remaining} remaining"
            ),
        }
    }
}

impl std::error::Error for WasmCodeSectionError {}

/// Reject a final artifact whose code section cannot pass IC module validation.
pub fn enforce_wasm_code_section_limit(
    wasm_path: &Path,
) -> Result<usize, Box<dyn std::error::Error>> {
    let wasm = fs::read(wasm_path)?;
    let code_section_bytes = wasm_code_section_size(&wasm).map_err(|source| {
        format!(
            "failed to inspect Wasm code section for {}: {source}",
            wasm_path.display()
        )
    })?;
    validate_wasm_code_section_size(code_section_bytes).map_err(|source| {
        format!(
            "Wasm artifact {} cannot be installed: {source}",
            wasm_path.display()
        )
    })?;

    if code_section_bytes >= IC_WASM_CODE_SECTION_WARNING_BYTES {
        eprintln!(
            "warning: Wasm code section for {} is {} bytes; {} bytes remain before the IC limit",
            wasm_path.display(),
            code_section_bytes,
            IC_WASM_CODE_SECTION_LIMIT_BYTES - code_section_bytes
        );
    } else {
        eprintln!(
            "Wasm code section for {}: {} bytes; {} bytes remain before the IC limit",
            wasm_path.display(),
            code_section_bytes,
            IC_WASM_CODE_SECTION_LIMIT_BYTES - code_section_bytes
        );
    }

    Ok(code_section_bytes)
}

pub(super) fn wasm_artifact_metrics(
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

fn wasm_code_section_size(wasm: &[u8]) -> Result<usize, WasmCodeSectionError> {
    Ok(inspect_wasm(wasm)?.code_section_bytes)
}

fn inspect_wasm(wasm: &[u8]) -> Result<WasmStructure, WasmCodeSectionError> {
    if !wasm.starts_with(WASM_HEADER) {
        return Err(WasmCodeSectionError::InvalidHeader);
    }

    let mut cursor = WASM_HEADER.len();
    let mut code_section_bytes = None;
    let mut data_section_bytes = None;
    let mut defined_functions = None;
    let mut exports = None;
    let mut public_candid_metadata = Vec::new();
    while cursor < wasm.len() {
        let section_offset = cursor;
        let section_id = wasm[cursor];
        cursor += 1;
        let section_size = read_section_size(wasm, &mut cursor)?;
        let remaining = wasm.len() - cursor;
        if section_size > remaining {
            return Err(WasmCodeSectionError::TruncatedSection {
                offset: section_offset,
                declared: section_size,
                remaining,
            });
        }
        let payload = &wasm[cursor..cursor + section_size];
        match section_id {
            WASM_CUSTOM_SECTION_ID => {
                let (name, contents) = read_custom_section(payload, section_offset)?;
                if name == PUBLIC_CANDID_METADATA_SECTION {
                    public_candid_metadata.push(contents.to_vec());
                }
            }
            WASM_FUNCTION_SECTION_ID => {
                if defined_functions
                    .replace(read_vector_count(payload, section_id, section_offset)?)
                    .is_some()
                {
                    return Err(WasmCodeSectionError::DuplicateFunctionSection);
                }
            }
            WASM_EXPORT_SECTION_ID => {
                if exports
                    .replace(read_exports(payload, section_offset)?)
                    .is_some()
                {
                    return Err(WasmCodeSectionError::DuplicateExportSection);
                }
            }
            WASM_CODE_SECTION_ID => {
                if code_section_bytes.replace(section_size).is_some() {
                    return Err(WasmCodeSectionError::DuplicateCodeSection);
                }
                let code_functions = read_vector_count(payload, section_id, section_offset)?;
                if defined_functions.is_some_and(|functions| functions != code_functions) {
                    return Err(WasmCodeSectionError::InvalidSectionPayload {
                        section_id,
                        offset: section_offset,
                    });
                }
                defined_functions = Some(code_functions);
            }
            WASM_DATA_SECTION_ID if data_section_bytes.replace(section_size).is_some() => {
                return Err(WasmCodeSectionError::DuplicateDataSection);
            }
            _ => {}
        }
        cursor += section_size;
    }

    Ok(WasmStructure {
        code_section_bytes: code_section_bytes.unwrap_or(0),
        data_section_bytes: data_section_bytes.unwrap_or(0),
        defined_functions: defined_functions.unwrap_or(0),
        contract: WasmContractSnapshot {
            exports: exports.unwrap_or_default(),
            public_candid_metadata,
        },
    })
}

fn read_custom_section(
    payload: &[u8],
    section_offset: usize,
) -> Result<(&str, &[u8]), WasmCodeSectionError> {
    let mut cursor = 0;
    let name = read_name(payload, &mut cursor, section_offset)?;
    Ok((name, &payload[cursor..]))
}

fn read_exports(
    payload: &[u8],
    section_offset: usize,
) -> Result<BTreeMap<String, u8>, WasmCodeSectionError> {
    let mut cursor = 0;
    let count =
        read_u32(payload, &mut cursor).ok_or(WasmCodeSectionError::InvalidSectionPayload {
            section_id: WASM_EXPORT_SECTION_ID,
            offset: section_offset,
        })?;
    let mut exports = BTreeMap::new();
    for _ in 0..count {
        let name = read_name(payload, &mut cursor, section_offset)?.to_string();
        let kind =
            payload
                .get(cursor)
                .copied()
                .ok_or(WasmCodeSectionError::InvalidSectionPayload {
                    section_id: WASM_EXPORT_SECTION_ID,
                    offset: section_offset,
                })?;
        cursor += 1;
        read_u32(payload, &mut cursor).ok_or(WasmCodeSectionError::InvalidSectionPayload {
            section_id: WASM_EXPORT_SECTION_ID,
            offset: section_offset,
        })?;
        if exports.insert(name.clone(), kind).is_some() {
            return Err(WasmCodeSectionError::DuplicateExport { name });
        }
    }
    if cursor != payload.len() {
        return Err(WasmCodeSectionError::InvalidSectionPayload {
            section_id: WASM_EXPORT_SECTION_ID,
            offset: section_offset,
        });
    }
    Ok(exports)
}

fn read_name<'a>(
    bytes: &'a [u8],
    cursor: &mut usize,
    section_offset: usize,
) -> Result<&'a str, WasmCodeSectionError> {
    let name_offset = section_offset + *cursor;
    let length = read_u32(bytes, cursor).ok_or(WasmCodeSectionError::InvalidSectionPayload {
        section_id: WASM_CUSTOM_SECTION_ID,
        offset: section_offset,
    })? as usize;
    let end = cursor
        .checked_add(length)
        .filter(|end| *end <= bytes.len())
        .ok_or(WasmCodeSectionError::InvalidSectionPayload {
            section_id: WASM_CUSTOM_SECTION_ID,
            offset: section_offset,
        })?;
    let name = std::str::from_utf8(&bytes[*cursor..end]).map_err(|_| {
        WasmCodeSectionError::InvalidName {
            offset: name_offset,
        }
    })?;
    *cursor = end;
    Ok(name)
}

fn read_vector_count(
    payload: &[u8],
    section_id: u8,
    section_offset: usize,
) -> Result<u32, WasmCodeSectionError> {
    let mut cursor = 0;
    read_u32(payload, &mut cursor).ok_or(WasmCodeSectionError::InvalidSectionPayload {
        section_id,
        offset: section_offset,
    })
}

fn read_section_size(wasm: &[u8], cursor: &mut usize) -> Result<usize, WasmCodeSectionError> {
    let offset = *cursor;
    let mut value = 0_u32;
    for index in 0..5 {
        let Some(byte) = wasm.get(*cursor).copied() else {
            return Err(WasmCodeSectionError::InvalidSectionSize { offset });
        };
        *cursor += 1;

        if index == 4 && byte & 0xf0 != 0 {
            return Err(WasmCodeSectionError::InvalidSectionSize { offset });
        }
        value |= u32::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Ok(value as usize);
        }
    }

    Err(WasmCodeSectionError::InvalidSectionSize { offset })
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Option<u32> {
    let mut value = 0_u32;
    for index in 0..5 {
        let byte = bytes.get(*cursor).copied()?;
        *cursor += 1;
        if index == 4 && byte & 0xf0 != 0 {
            return None;
        }
        value |= u32::from(byte & 0x7f) << (index * 7);
        if byte & 0x80 == 0 {
            return Some(value);
        }
    }
    None
}

const fn validate_wasm_code_section_size(size: usize) -> Result<(), WasmCodeSectionError> {
    if size > IC_WASM_CODE_SECTION_LIMIT_BYTES {
        return Err(WasmCodeSectionError::LimitExceeded {
            actual: size,
            limit: IC_WASM_CODE_SECTION_LIMIT_BYTES,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_the_exact_code_section_payload() {
        let wasm = [WASM_HEADER.as_slice(), &[1, 1, 0], &[10, 3, 1, 2, 3]].concat();

        assert_eq!(wasm_code_section_size(&wasm), Ok(3));
    }

    #[test]
    fn rejects_duplicate_code_sections() {
        let wasm = [WASM_HEADER.as_slice(), &[10, 1, 0, 10, 1, 0]].concat();

        assert_eq!(
            wasm_code_section_size(&wasm),
            Err(WasmCodeSectionError::DuplicateCodeSection)
        );
    }

    #[test]
    fn rejects_invalid_section_size_encoding() {
        let wasm = [WASM_HEADER.as_slice(), &[1, 0x80, 0x80, 0x80, 0x80, 0x10]].concat();

        assert_eq!(
            wasm_code_section_size(&wasm),
            Err(WasmCodeSectionError::InvalidSectionSize { offset: 9 })
        );
    }

    #[test]
    fn rejects_truncated_sections() {
        let wasm = [WASM_HEADER.as_slice(), &[10, 3, 1, 2]].concat();

        assert_eq!(
            wasm_code_section_size(&wasm),
            Err(WasmCodeSectionError::TruncatedSection {
                offset: 8,
                declared: 3,
                remaining: 2,
            })
        );
    }

    #[test]
    fn accepts_the_exact_ic_code_section_limit() {
        assert_eq!(
            validate_wasm_code_section_size(IC_WASM_CODE_SECTION_LIMIT_BYTES),
            Ok(())
        );
    }

    #[test]
    fn rejects_one_byte_over_the_ic_code_section_limit() {
        assert_eq!(
            validate_wasm_code_section_size(IC_WASM_CODE_SECTION_LIMIT_BYTES + 1),
            Err(WasmCodeSectionError::LimitExceeded {
                actual: IC_WASM_CODE_SECTION_LIMIT_BYTES + 1,
                limit: IC_WASM_CODE_SECTION_LIMIT_BYTES,
            })
        );
    }
}
