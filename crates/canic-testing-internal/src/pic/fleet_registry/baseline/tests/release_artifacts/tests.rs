//! Behavioral qualification of the fixture build-source cache boundary.

use super::*;
use std::time::SystemTime;

#[test]
fn production_gzip_policy_binds_fixture_release_artifacts() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let config = workspace.join("apps/test/test-configs/generated-mixed-topology.toml");
    let snapshot = AppConfigSnapshot::load(&config).unwrap();
    let raw = b"fixture artifact with repeated data for production compression".repeat(128);
    let modules = snapshot
        .model()
        .roles
        .keys()
        .filter(|role| !role.is_root())
        .map(|role| (role.clone(), raw.clone()))
        .collect();
    let release = crate::pic::managed_test_init_identity().release_build_id;
    let (manifest, artifacts) = super::super::exact_root_store_fixture(&config, &modules, release);
    assert!(!manifest.entries.is_empty());

    let directory = std::env::temp_dir().join(format!(
        "canic-gzip-policy-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    let _cleanup = super::super::TestDirectoryCleanup(directory.clone());
    std::fs::create_dir(&directory).unwrap();
    let wasm = directory.join("fixture.wasm");
    let gzip = directory.join("fixture.wasm.gz");
    std::fs::write(&wasm, &raw).unwrap();
    canic_host::canister_build::write_gzip_artifact(&wasm, &gzip).unwrap();
    let production = std::fs::read(&gzip).unwrap();
    for entry in &manifest.entries {
        let artifact = &entry.artifact;
        let retained = &artifacts[&artifact.role];
        assert_eq!(retained, &production);
        assert_eq!(artifact.wasm_gz_size_bytes, retained.len() as u64);
        assert_eq!(
            artifact.wasm_gz_sha256_hex,
            canic_core::cdk::utils::hash::sha256_hex(retained)
        );
        assert_eq!(
            ic_host_artifacts::artifact::decode_gzip(retained, retained.len(), raw.len()).unwrap(),
            raw,
        );
    }
}

#[test]
#[ignore = "focused qualification resolves the real fixture Cargo graphs"]
pub fn batched_fixture_role_evidence_matches_isolated_validation() {
    let workspace =
        crate::workspace::discover().expect("discover Canic test workspace through Cargo");
    let path = workspace.join("apps/test/test-configs/generated-mixed-topology.toml");
    let snapshot = AppConfigSnapshot::load(&path).unwrap();
    let roles = snapshot
        .model()
        .roles
        .keys()
        .filter(|role| !role.is_root())
        .cloned()
        .collect::<Vec<_>>();
    assert!(!roles.is_empty());
    let isolated = || {
        roles
            .iter()
            .map(|role| {
                canic_host::role_contract::validate_declared_role_package(
                    &path,
                    snapshot.model(),
                    role,
                    PackageValidationMode::Passive,
                    &canic_host::role_contract::CargoFeatureSelection::default(),
                )
            })
            .collect::<Vec<_>>()
    };
    let batched = || {
        validate_declared_role_packages(
            &path,
            snapshot.model(),
            &roles,
            PackageValidationMode::Passive,
        )
    };
    // Alternate order to avoid attributing the first filesystem warm-up to batching.
    for round in 0..4 {
        let run = |batch: bool| {
            let started = Instant::now();
            let evidence = if batch { batched() } else { isolated() };
            let elapsed = started.elapsed();
            assert!(
                evidence
                    .iter()
                    .all(|item| matches!(item, RolePackageValidation::Supported(_)))
            );
            (evidence, elapsed)
        };
        let (first, second) = (run(round % 2 == 0), run(round % 2 != 0));
        assert_eq!(first.0, second.0);
        let (batch, serial) = if round % 2 == 0 {
            (first.1, second.1)
        } else {
            (second.1, first.1)
        };
        eprintln!(
            "fixture role evidence: round={round} roles={} isolated_ms={} batched_ms={}",
            roles.len(),
            serial.as_millis(),
            batch.as_millis(),
        );
    }
}

#[test]
pub fn journey_edits_reuse_artifacts_but_build_helper_edits_invalidate() {
    let workspace =
        crate::workspace::discover().expect("discover Canic test workspace through Cargo");
    let root = std::env::temp_dir().join(format!(
        "canic-build-helper-cache-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    let _cleanup = super::super::TestDirectoryCleanup(root.clone());
    let source = root.join("source");
    for path in BUILD_HELPERS {
        let destination = source.join(path);
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::copy(workspace.join(path), destination).unwrap();
    }
    let journey = source.join("crates/canic-testing-internal/src/pic/fleet_registry/baseline.rs");
    std::fs::write(&journey, b"original journey assertions").unwrap();
    let acquire = || {
        let cache = bind_build_helper_inputs(
            ArtifactCacheSpec::new(
                &root.join("cache"),
                "helper-boundary",
                "canic/helper-boundary/v1",
            )
            .with_output("artifact", &root.join("output")),
            &source,
        );
        match prepare_artifact_cache(&cache).unwrap() {
            ArtifactCachePreparation::Reused(record) => ArtifactCacheOutcome::Reused(record),
            ArtifactCachePreparation::Build(transaction) => {
                std::fs::write(
                    transaction.output_path("artifact").unwrap(),
                    b"fixture output",
                )
                .unwrap();
                transaction.commit().unwrap()
            }
        }
    };
    let initial = acquire();
    assert!(!initial.is_reused());
    std::fs::write(&journey, b"changed journey assertions and timing").unwrap();
    let replay = acquire();
    assert!(replay.is_reused());
    assert_eq!(initial.record().key(), replay.record().key());
    for path in BUILD_HELPERS {
        let helper = source.join(path);
        let original = std::fs::read(&helper).unwrap();
        let mut changed = original.clone();
        changed.extend_from_slice(b"\n// changed build helper\n");
        std::fs::write(&helper, changed).unwrap();
        let changed = acquire();
        assert!(!changed.is_reused());
        assert_ne!(initial.record().key(), changed.record().key());
        std::fs::write(&helper, original).unwrap();
        let restored = acquire();
        assert!(restored.is_reused());
        assert_eq!(initial.record().key(), restored.record().key());
    }
}
