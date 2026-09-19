//! Behavioral qualification of the fixture build-source cache boundary.

use super::*;
use ic_testkit::artifacts::workspace_root_for;
use std::time::SystemTime;

#[test]
pub fn journey_edits_reuse_artifacts_but_build_helper_edits_invalidate() {
    let workspace = workspace_root_for(env!("CARGO_MANIFEST_DIR"));
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
