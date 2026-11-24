//! Qualification-only core Wasm measurements; installed execution remains the witness.

#[cfg(test)]
mod tests;

use wasmparser::{Encoding, Parser, Payload};

/// Complete measurements after bounded parsing and validation.
#[derive(Debug)]
pub struct Footprint {
    /// Complete uncompressed module size.
    pub raw_bytes: usize,
    /// Code section payload, including its function count.
    pub code_section_bytes: usize,
    /// Defined functions, excluding imports.
    pub defined_functions: u32,
}

/// Measure up to 1.9 MB, 64 exports, one signature section per export, and
/// 64 other sections including up to 32 other custom sections.
/// # Errors
/// Refuses oversized, malformed, invalid or non-core Wasm instead of returning partial facts.
pub fn measure(bytes: &[u8]) -> Result<Footprint, Box<dyn std::error::Error>> {
    let (bytes, identity) =
        ic_host_artifacts::read(bytes, ic_host_artifacts::Length::AtMost(1_900_000))?;
    let mut facts = Footprint {
        raw_bytes: usize::try_from(identity.bytes)?,
        code_section_bytes: 0,
        defined_functions: 0,
    };
    let mut sections = 0;
    let mut custom = 0;
    let mut signatures = 0;
    let mut exports = 0;
    for payload in Parser::new(0).parse_all(&bytes) {
        let payload = payload?;
        if payload.as_section().is_some() {
            sections += 1;
        }
        match payload {
            Payload::Version { encoding, .. } if encoding != Encoding::Module => {
                return Err("expected core Wasm".into());
            }
            Payload::FunctionSection(reader) => facts.defined_functions = reader.count(),
            Payload::CodeSectionStart {
                unchecked_range: range,
                ..
            } => facts.code_section_bytes = usize::try_from(range.end - range.start)?,
            Payload::ExportSection(reader) => {
                exports = reader.count();
                if exports > 64 {
                    return Err("fixture export budget exceeded".into());
                }
            }
            Payload::CustomSection(section)
                if section.name().starts_with("canic:signature:v1:") =>
            {
                signatures += 1;
            }
            Payload::CustomSection(_) => custom += 1,
            _ => {}
        }
        if sections - signatures > 64 || custom > 32 || signatures > 64 {
            return Err("fixture section budget exceeded".into());
        }
    }
    if signatures > exports {
        return Err("fixture has more signature sections than exports".into());
    }
    wasmparser::Validator::new().validate_all(&bytes)?;
    Ok(facts)
}
