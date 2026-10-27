//! Module: canic_host::icp::diagnostic
//!
//! Responsibility: classify external ICP CLI wording into typed conditions.
//! Does not own: process execution, domain policy, command hints, or exit codes.
//! Boundary: the host ICP adapter is the only production owner of this wording.

use ic_platform::principal::ExternalPrincipal as Principal;

///
/// IcpDiagnostic
///
/// External ICP CLI conditions on which Canic has explicit behavior.
///

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IcpDiagnostic {
    AlreadyInstalled,
    CanisterOutOfCycles,
    CanisterNotFound { canister: String },
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
    if message.contains("IC0207") {
        return Some(IcpDiagnostic::CanisterOutOfCycles);
    }
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
    if let Some(canister) = replica_canister_not_found(message) {
        return Some(IcpDiagnostic::CanisterNotFound { canister });
    }
    if message.contains("has no query method")
        || message.contains("method not found")
        || message.contains("Canister has no query method")
    {
        return Some(IcpDiagnostic::MethodMissing);
    }
    if message.contains("Cannot find canister id")
        || message.contains("failed to lookup canister ID")
        || message.contains("could not find ID for canister")
        || message.contains("Canister ID is missing")
    {
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

fn replica_canister_not_found(message: &str) -> Option<String> {
    let message = message.trim_end();
    let rest = if let Some((_, rest)) =
        message.split_once("reject code DestinationInvalid, reject message Canister ")
    {
        rest.strip_suffix(", error code Some(\"IC0301\")")?
    } else {
        message
            .split_once("IC0301: Canister ")
            .or_else(|| message.split_once("code=3 message=Canister "))?
            .1
    };
    let canister = rest.strip_suffix(" not found")?;
    valid_canister_token(canister)
}

fn valid_canister_token(canister: &str) -> Option<String> {
    Principal::from_text(canister).ok().map(|pid| pid.to_text())
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
                "Replica error: IC0207: Canister does not have enough cycles for this operation",
                IcpDiagnostic::CanisterOutOfCycles,
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
                "Canister has no query method 'canic_coordinator_funding'",
                IcpDiagnostic::MethodMissing,
            ),
            (
                "Cannot find canister id for root",
                IcpDiagnostic::CanisterIdMissing,
            ),
            (
                "failed to lookup canister ID for canister 'root' in environment 'local'",
                IcpDiagnostic::CanisterIdMissing,
            ),
            (
                "could not find ID for canister 'root' in environment 'local'",
                IcpDiagnostic::CanisterIdMissing,
            ),
            (
                "Canister ID is missing for canister 'root'",
                IcpDiagnostic::CanisterIdMissing,
            ),
            (
                "local replica rejected query: code=3 message=Canister uxrrr-q7777-77774-qaaaq-cai not found",
                IcpDiagnostic::CanisterNotFound {
                    canister: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
                },
            ),
            (
                "replica rejected management status: IC0301: Canister uxrrr-q7777-77774-qaaaq-cai not found",
                IcpDiagnostic::CanisterNotFound {
                    canister: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
                },
            ),
            (
                "Error: The replica returned a rejection error: reject code DestinationInvalid, reject message Canister uxrrr-q7777-77774-qaaaq-cai not found, error code Some(\"IC0301\")",
                IcpDiagnostic::CanisterNotFound {
                    canister: "uxrrr-q7777-77774-qaaaq-cai".to_string(),
                },
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
        assert_eq!(
            classify_icp_diagnostic(
                "transport failure while checking whether Canister uxrrr-q7777-77774-qaaaq-cai was not found"
            ),
            None
        );
        assert_eq!(
            classify_icp_diagnostic("Canister uxrrr-q7777-77774-qaaaq-cai not found"),
            None
        );
        assert_eq!(
            classify_icp_diagnostic("Error: Canister uxrrr-q7777-77774-qaaaq-cai was not found."),
            None
        );
    }

    #[test]
    fn canister_absence_requires_a_complete_error_record_and_valid_principal() {
        let target = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let other = "r7inp-6aaaa-aaaaa-aaabq-cai";
        let message = format!(
            "referenced Canister {other} not found; replica error IC0301: Canister {target} not found"
        );
        assert_eq!(
            classify_icp_diagnostic(&message),
            Some(IcpDiagnostic::CanisterNotFound {
                canister: target.to_string()
            })
        );
        for message in [
            format!("IC0301 is referenced here; Canister {target} not found"),
            "IC0301: Canister invalid-principal not found".to_string(),
            format!("IC0301: Canister {target} not found in a nested diagnostic"),
            format!("IC0301: Canister {target} not found\nIC0301: Canister {other} not found"),
            format!(
                "reject code CanisterError, reject message Canister {target} not found, error code Some(\"IC0301\")"
            ),
            format!(
                "reject code DestinationInvalid, reject message Canister {target} not found, error code Some(\"IC0503\")"
            ),
        ] {
            assert_eq!(classify_icp_diagnostic(&message), None);
        }
    }
}
