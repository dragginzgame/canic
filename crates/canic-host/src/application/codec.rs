//! Optional executable codec boundary. It receives no identity, journal or transport handle.
use super::Request;
use crate::{Error, Result, supplier};
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Debug, thiserror::Error)]
#[error("application codec failed: {source}\n{diagnostic}")]
struct Failure {
    #[source]
    source: crate::process::CaptureFailure,
    diagnostic: String,
}

fn failure(source: crate::process::CaptureFailure) -> Error {
    let diagnostic = ic_host_text::escape(&String::from_utf8_lossy(&source.stderr)).into_owned();
    supplier(Failure { source, diagnostic })
}

/// Conventional native companion output for a declared Wasm build.
#[must_use]
pub fn output(wasm: &Path) -> PathBuf {
    wasm.with_extension("client")
}

/// Ask the app's locally built codec to encode a named command.
/// # Errors
/// Refuses unsuccessful, malformed or oversized codec output before any call is prepared.
pub fn encode(client: &Path, command: &[String]) -> Result<Request> {
    let bytes = crate::process::capture(
        Command::new(client).arg("encode").args(command),
        512 * 1024,
        4096,
        Duration::from_secs(10),
    )
    .map_err(failure)?;
    let request: Request = serde_json::from_slice(&bytes).map_err(supplier)?;
    if request.arguments.len() > super::MAX_REQUEST_BYTES {
        return Err(Error::Invalid(
            "application request exceeds transport budget".into(),
        ));
    }
    Ok(request)
}

/// Decode an already retained reply. Failure never changes its external outcome.
/// # Errors
/// Refuses failed, malformed or oversized app output; the original reply remains available.
pub fn decode(client: &Path, reply: &[u8]) -> Result<serde_json::Value> {
    let bytes = crate::process::capture_input(
        Command::new(client).arg("decode"),
        ic_host_process::Input::Bytes(reply.to_vec()),
        65_536,
        4096,
        Duration::from_secs(10),
    )
    .map_err(failure)?;
    serde_json::from_slice(&bytes).map_err(supplier)
}
