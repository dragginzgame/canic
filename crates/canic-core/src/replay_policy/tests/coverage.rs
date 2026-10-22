//! Module: replay_policy::tests::coverage
//!
//! Responsibility: verify replay-policy manifest uniqueness and endpoint coverage.
//! Does not own: manifest data or endpoint declarations.
//! Boundary: test-only coverage checks against compiled source fixtures.

use super::*;
use std::collections::BTreeSet;

#[test]
fn endpoint_manifest_entries_are_unique() {
    let mut seen = BTreeSet::new();
    for entry in ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .chain(STORE_ENDPOINT_REPLAY_POLICY_MANIFEST)
    {
        assert!(
            seen.insert(entry.endpoint),
            "duplicate replay policy entry for {}",
            entry.endpoint
        );
    }
}

#[test]
fn emitted_canic_endpoints_have_replay_policy_entries() {
    for kind in [EndpointKind::Query, EndpointKind::Update] {
        let emitted = emitted_endpoint_names(kind);
        let manifest = ENDPOINT_REPLAY_POLICY_MANIFEST
            .iter()
            .chain(STORE_ENDPOINT_REPLAY_POLICY_MANIFEST)
            .filter(|entry| entry.endpoint_kind == kind)
            .map(|entry| entry.endpoint.to_owned())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            emitted, manifest,
            "endpoint inventory must exactly match replay policy"
        );
    }
}

#[test]
fn command_inventory_selects_the_exact_enum_before_extracting_variants() {
    let source = "pub enum CommandResponse {\n    Reply,\n}\npub enum Command {\n    Execute,\n}";
    assert_eq!(
        enum_variant_names_from_source(source, "pub enum Command"),
        BTreeSet::from(["Execute"]),
    );
}

#[test]
fn release_candidate_manifests_have_no_release_blockers() {
    let blockers = release_candidate_manifest_blockers();

    assert!(
        blockers.is_empty(),
        "release candidate manifests still contain replay blockers: {blockers:?}"
    );
}

#[test]
fn remaining_release_blockers_are_explicit_endpoint_slices() {
    let blockers = ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .chain(STORE_ENDPOINT_REPLAY_POLICY_MANIFEST)
        .filter(|entry| entry.implementation_status == ReplayImplementationStatus::ReleaseBlocker)
        .map(|entry| entry.endpoint)
        .collect::<BTreeSet<_>>();

    assert!(blockers.is_empty(), "unexpected blockers: {blockers:?}");
}

#[test]
fn intentionally_non_idempotent_entries_must_state_reason() {
    for entry in ENDPOINT_REPLAY_POLICY_MANIFEST
        .iter()
        .chain(STORE_ENDPOINT_REPLAY_POLICY_MANIFEST)
    {
        if let ReplayPolicy::IntentionallyNonIdempotent { reason, .. } = entry.replay_policy {
            assert!(
                !reason.trim().is_empty(),
                "non-idempotent entry {} must state a reason",
                entry.endpoint
            );
        }
    }
}
