//! Frozen release-manifest evidence remains byte-exact and cannot become transition authority.

use super::*;
use canic_core::ids::BuildNetwork;

const BYTES: &[u8] = include_bytes!("manifest.json");

#[test]
fn completed_release_manifest_binds_current_reinstall_authority() {
    let raw: serde_json::Value = serde_json::from_slice(BYTES).unwrap();
    let release = raw["release_build_id"].as_str().unwrap().parse().unwrap();
    let evidence = manifest(BYTES, release).unwrap();
    assert_eq!(serde_json::to_vec(&evidence).unwrap(), BYTES);
    assert_eq!(evidence.build_network, BuildNetwork::Ic);
    assert_eq!(
        evidence.transition_mode,
        crate::release_set::ReleaseTransitionMode::ReinstallOnly
    );
}

#[test]
fn changed_release_schema_fields_and_noncanonical_bytes_reject() {
    let original: serde_json::Value = serde_json::from_slice(BYTES).unwrap();
    let release = original["release_build_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    for mutation in ["release", "schema", "network", "missing", "extra", "format"] {
        let mut raw = original.clone();
        match mutation {
            "release" => raw["release_build_id"] = "11".repeat(32).into(),
            "schema" => raw["schema_version"] = 9.into(),
            "network" => raw["build_network"] = "".into(),
            "missing" => {
                raw.as_object_mut()
                    .unwrap()
                    .remove("infrastructure_artifact_manifest_sha256");
            }
            "extra" => raw["unbound_authority"] = true.into(),
            "format" => {}
            _ => unreachable!(),
        }
        let mut bytes = serde_json::to_vec(&raw).unwrap();
        if mutation == "format" {
            bytes.push(b'\n');
        }
        assert!(
            matches!(
                manifest(&bytes, release),
                Err(CompletedSourceProtocolError::ManifestIdentity
                    | CompletedSourceProtocolError::Decode(_))
            ),
            "{mutation}"
        );
    }
}
