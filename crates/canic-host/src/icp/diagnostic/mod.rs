//! Module: canic_host::icp::diagnostic
//!
//! Responsibility: classify external ICP CLI wording into typed conditions.
//! Does not own: process execution, domain policy, command hints, or exit codes.
//! Boundary: the host ICP adapter is the only production owner of this wording.

///
/// IcpDiagnostic
///
/// External ICP CLI conditions on which Canic has explicit behavior.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IcpDiagnostic {
    AlreadyInstalled,
    ProjectManifestMissing,
    LocalNetworkNotRunning,
    MethodMissing,
    CanisterIdMissing,
    CanisterWasmMissing,
    ForeignLocalReplicaOwner { network: String, project: String },
}

/// Classify the exact external ICP CLI diagnostics understood by Canic.
#[must_use]
pub fn classify_icp_diagnostic(message: &str) -> Option<IcpDiagnostic> {
    if let Some((network, project)) = foreign_local_replica_owner(message) {
        return Some(IcpDiagnostic::ForeignLocalReplicaOwner { network, project });
    }
    if message.contains("failed to locate project directory")
        || message.contains("project manifest not found")
    {
        return Some(IcpDiagnostic::ProjectManifestMissing);
    }
    if message.contains("network 'local' is not running")
        || message.contains("the local network for this project is not running")
    {
        return Some(IcpDiagnostic::LocalNetworkNotRunning);
    }
    if message.contains("has no query method")
        || message.contains("method not found")
        || message.contains("Canister has no query method")
    {
        return Some(IcpDiagnostic::MethodMissing);
    }
    if message.contains("Cannot find canister id") {
        return Some(IcpDiagnostic::CanisterIdMissing);
    }
    if message.contains("contains no Wasm module") || message.contains("wasm-module-not-found") {
        return Some(IcpDiagnostic::CanisterWasmMissing);
    }

    let lower = message.to_ascii_lowercase();
    if lower.contains("already")
        && (lower.contains("install") || lower.contains("installed") || lower.contains("canister"))
    {
        return Some(IcpDiagnostic::AlreadyInstalled);
    }

    None
}

fn foreign_local_replica_owner(message: &str) -> Option<(String, String)> {
    let marker = " network of the project at '";
    let marker_start = message.find(marker)?;
    let network = message[..marker_start]
        .split_whitespace()
        .last()?
        .to_string();
    let project_start = marker_start + marker.len();
    let rest = &message[project_start..];
    let project_end = rest.find('\'')?;
    Some((network, rest[..project_end].to_string()))
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_supported_icp_diagnostics() {
        let cases = [
            (
                "Error: canister is already installed",
                IcpDiagnostic::AlreadyInstalled,
            ),
            (
                "Error: failed to locate project directory\nproject manifest not found",
                IcpDiagnostic::ProjectManifestMissing,
            ),
            (
                "Error: the local network for this project is not running",
                IcpDiagnostic::LocalNetworkNotRunning,
            ),
            (
                "Canister has no query method 'canic_metrics'",
                IcpDiagnostic::MethodMissing,
            ),
            (
                "Cannot find canister id for root",
                IcpDiagnostic::CanisterIdMissing,
            ),
            (
                "canister contains no Wasm module",
                IcpDiagnostic::CanisterWasmMissing,
            ),
        ];

        for (message, expected) in cases {
            assert_eq!(classify_icp_diagnostic(message), Some(expected));
        }
    }

    #[test]
    fn classifies_foreign_local_replica_owner() {
        assert_eq!(
            classify_icp_diagnostic(
                "Error: port 8000 is in use by the demo network of the project at '/projects/toko'"
            ),
            Some(IcpDiagnostic::ForeignLocalReplicaOwner {
                network: "demo".to_string(),
                project: "/projects/toko".to_string(),
            })
        );
    }

    #[test]
    fn leaves_unknown_diagnostics_unclassified() {
        assert_eq!(
            classify_icp_diagnostic("unexpected transport failure"),
            None
        );
    }
}
