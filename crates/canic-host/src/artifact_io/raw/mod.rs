//! Parse the protocol-neutral endpoint declarations emitted by Canic macros.

use ic_host_artifacts::wasm::WasmFacts;
use ic_host_tools::interface::{ExportedMethod, MethodMode};
use std::collections::BTreeSet;

#[derive(Debug, Eq, thiserror::Error, PartialEq)]
pub(super) enum DeclarationError {
    #[error("truncated raw endpoint declaration")]
    Framing,
    #[error("invalid raw endpoint mode")]
    Mode,
    #[error("invalid raw endpoint name")]
    Name,
    #[error("duplicate raw endpoint name")]
    Duplicate,
    #[error("raw endpoint also appears in Candid")]
    CandidOverlap,
}

// Each record is mode:u8, name byte length:u16 LE, then exact UTF-8 bytes.
// Facts have already passed the artifact owner's byte/section admission.
pub(super) fn methods<'a>(
    facts: &'a WasmFacts<'a>,
    candid: &BTreeSet<ExportedMethod<'_>>,
) -> Result<BTreeSet<ExportedMethod<'a>>, DeclarationError> {
    let mut methods = BTreeSet::new();
    let mut names = BTreeSet::new();
    for section in &facts.custom_sections {
        if section.name != "icp:private canic:raw" {
            continue;
        }
        let mut bytes = section.data;
        while !bytes.is_empty() {
            let header = bytes.get(..3).ok_or(DeclarationError::Framing)?;
            let mode = match header[0] {
                1 => MethodMode::Query,
                2 => MethodMode::Update,
                3 => MethodMode::CompositeQuery,
                _ => return Err(DeclarationError::Mode),
            };
            let length = usize::from(u16::from_le_bytes([header[1], header[2]]));
            bytes = &bytes[3..];
            let name = bytes.get(..length).ok_or(DeclarationError::Framing)?;
            let name = std::str::from_utf8(name).map_err(|_| DeclarationError::Name)?;
            if name.is_empty() || name.chars().any(char::is_control) {
                return Err(DeclarationError::Name);
            }
            if !names.insert(name) {
                return Err(DeclarationError::Duplicate);
            }
            if candid.iter().any(|method| method.name == name) {
                return Err(DeclarationError::CandidOverlap);
            }
            methods.insert(ExportedMethod { name, mode });
            bytes = &bytes[length..];
        }
    }
    Ok(methods)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ic_host_artifacts::wasm::{CustomSection, InspectionLimits, inspect};

    fn parse(
        bytes: &[u8],
        candid: &BTreeSet<ExportedMethod<'_>>,
    ) -> Result<usize, DeclarationError> {
        let mut facts = inspect(
            b"\0asm\x01\0\0\0",
            InspectionLimits {
                module_bytes: 1024,
                sections: 10,
                exports: 10,
                custom_sections: 10,
            },
        )
        .unwrap();
        facts.custom_sections.push(CustomSection {
            name: "icp:private canic:raw",
            data: bytes,
        });
        methods(&facts, candid).map(|methods| methods.len())
    }

    #[test]
    fn exact_raw_declarations_refuse_bad_framing_modes_names_and_overlap() {
        let mut record = vec![1, 4, 0];
        record.extend_from_slice(b"read");
        let empty = BTreeSet::new();
        assert_eq!(parse(&record, &empty), Ok(1));
        for end in 1..record.len() {
            assert_eq!(
                parse(&record[..end], &empty),
                Err(DeclarationError::Framing)
            );
        }
        let mut duplicate = record.clone();
        duplicate.extend_from_slice(&record);
        assert_eq!(parse(&duplicate, &empty), Err(DeclarationError::Duplicate));
        assert_eq!(parse(&[9, 0, 0], &empty), Err(DeclarationError::Mode));
        assert_eq!(parse(&[1, 0, 0], &empty), Err(DeclarationError::Name));
        assert_eq!(parse(&[1, 1, 0, 255], &empty), Err(DeclarationError::Name));
        assert_eq!(
            parse(
                &record,
                &BTreeSet::from([ExportedMethod {
                    name: "read",
                    mode: MethodMode::Update
                }])
            ),
            Err(DeclarationError::CandidOverlap)
        );
    }
}
