//! Recorded evidence distinguishes unchanged inputs, fresh qualification and corrupt artifacts.

use super::*;

#[test]
fn changed_sources_need_qualification_but_wrong_artifacts_never_reuse_evidence() {
    let bytes = b"fixture artifact";
    let evidence = FixtureEvidence {
        schema_version: 1,
        package: PACKAGE.into(),
        target: TARGET.into(),
        profile: PROFILE.into(),
        source_input_digest: "a1".repeat(32),
        build_fingerprint: "b2".repeat(32),
        lock_sha256: "c3".repeat(32),
        artifact_sha256: sha256_hex(bytes),
        cargo: "cargo identity".into(),
        rustc: "rustc identity".into(),
    };
    assert!(recorded_match(&evidence, &"a1".repeat(32), bytes).unwrap());
    assert!(!recorded_match(&evidence, &"d4".repeat(32), bytes).unwrap());
    assert_eq!(
        recorded_match(&evidence, &"a1".repeat(32), b"changed artifact")
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    let changed = FixtureEvidence {
        target: "another-target".into(),
        ..evidence
    };
    assert_eq!(
        recorded_match(&changed, &"a1".repeat(32), bytes)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
}
