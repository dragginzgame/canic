//! Shared native schema-producer entrypoint for managed and generated consumers.

use super::{self as schema, CompiledMethod};
use ic_host_artifacts::wasm::{InspectionLimits, inspect};
use std::{
    fs::File,
    io::{self, Read, Write},
};

const MAX_WASM_BYTES: usize = 128 * 1024 * 1024;

/// Write native contract bytes with `--native`, or admit them against a Wasm path.
///
/// The executable must link its actual consumer library so its endpoint
/// registrations are present. No target layouts or supplier schemas are copied.
/// # Errors
/// Returns argument, bounded input, Wasm inspection or registry admission failures.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = clap::Command::new("wire_schema")
        .about("Produce the linked consumer's native contract or admit its final Wasm")
        .arg(
            clap::Arg::new("native")
                .long("native")
                .action(clap::ArgAction::SetTrue)
                .conflicts_with("wasm"),
        )
        .arg(
            clap::Arg::new("wasm")
                .required_unless_present("native")
                .value_parser(clap::value_parser!(std::path::PathBuf)),
        )
        .try_get_matches()?;
    if arguments.get_flag("native") {
        let document =
            schema::native_snapshot().map_err(|error| io::Error::other(format!("{error:?}")))?;
        io::stdout()
            .lock()
            .write_all(&serde_json::to_vec(&document)?)?;
        return Ok(());
    }
    let path = arguments
        .get_one::<std::path::PathBuf>("wasm")
        .ok_or("expected finalized Wasm path")?;
    let mut wasm = Vec::new();
    File::open(path)?
        .take((MAX_WASM_BYTES + 1) as u64)
        .read_to_end(&mut wasm)?;
    let facts = inspect(
        &wasm,
        InspectionLimits {
            module_bytes: MAX_WASM_BYTES,
            sections: 1024,
            exports: 4096,
            custom_sections: 1024,
        },
    )?;
    let mut declarations = Vec::new();
    for section in facts
        .custom_sections
        .iter()
        .filter(|section| section.name == "icp:private canic:wire")
    {
        let mut bytes = section.data;
        while !bytes.is_empty() {
            declarations.push(CompiledMethod::decode(&mut bytes)?);
        }
    }
    let bytes = serde_json::to_vec(
        &schema::snapshot(&declarations).map_err(|error| io::Error::other(format!("{error:?}")))?,
    )?;
    io::stdout().lock().write_all(&bytes)?;
    Ok(())
}
