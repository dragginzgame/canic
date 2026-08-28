//! Module: artifact_io::wasm
//!
//! Responsibility: inspect final Canic Wasm artifacts against IC code-section limits.
//! Does not own: Wasm optimization, compression, installation, or replica validation.
//! Boundary: parses top-level sections after artifact transforms and before publication.

use std::{fmt, fs, path::Path};

const IC_WASM_CODE_SECTION_LIMIT_BYTES: usize = 10 * 1024 * 1024;
const IC_WASM_CODE_SECTION_WARNING_BYTES: usize = 9 * 1024 * 1024 + 256 * 1024;
const WASM_HEADER: &[u8; 8] = b"\0asm\x01\0\0\0";
const WASM_CODE_SECTION_ID: u8 = 10;

#[derive(Debug, Eq, PartialEq)]
enum WasmCodeSectionError {
    DuplicateCodeSection,

    InvalidHeader,

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
            Self::InvalidHeader => write!(formatter, "invalid Wasm module header"),
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

fn wasm_code_section_size(wasm: &[u8]) -> Result<usize, WasmCodeSectionError> {
    if !wasm.starts_with(WASM_HEADER) {
        return Err(WasmCodeSectionError::InvalidHeader);
    }

    let mut cursor = WASM_HEADER.len();
    let mut code_section_bytes = None;
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
        if section_id == WASM_CODE_SECTION_ID && code_section_bytes.replace(section_size).is_some()
        {
            return Err(WasmCodeSectionError::DuplicateCodeSection);
        }
        cursor += section_size;
    }

    Ok(code_section_bytes.unwrap_or(0))
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
        let wasm = [WASM_HEADER.as_slice(), &[10, 0, 10, 0]].concat();

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
