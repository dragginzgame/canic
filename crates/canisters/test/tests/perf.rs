use std::{collections::HashMap, env, fs, io, path::PathBuf};

use canic::{core::ops::perf::PerfSnapshot, types::PageRequest};
use canic_internal::canister;
use canic_testkit::pic::PicBuilder;

const TEST_WASM_ENV: &str = "CANIC_TEST_WASM";
const TEST_WASM_RELATIVE: &str = "../../../.dfx/local/canisters/test/test.wasm.gz";

fn load_test_wasm() -> Option<Vec<u8>> {
    // Skip on CI where the wasm is not built.
    if option_env!("GITHUB_ACTIONS") == Some("true") {
        return None;
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_path = manifest_dir.join(TEST_WASM_RELATIVE);

    let mut candidates = env::var(TEST_WASM_ENV)
        .ok()
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();
    candidates.push(default_path);

    for path in candidates {
        match fs::read(&path) {
            Ok(bytes) => return Some(bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => panic!("failed to read test wasm at {}: {}", path.display(), err),
        }
    }

    None
}

#[test]
fn perf_endpoints_record_counters() {
    let Some(test_wasm) = load_test_wasm() else {
        eprintln!(
            "skipping perf_endpoints_record_counters — run `make test` \
             to build canisters or set {TEST_WASM_ENV}"
        );
        return;
    };

    let pic = PicBuilder::new().with_application_subnet().build();
    let test_id = pic
        .create_and_install_canister(canister::TEST, test_wasm)
        .expect("install test canister");

    // Drive any timers registered during install.
    for _ in 0..20 {
        pic.tick();
    }

    // Run the perf workload and capture the snapshot returned by the update.
    let snapshot: PerfSnapshot = pic
        .update_call(test_id, "test_perf", ())
        .expect("call test_perf() endpoint");

    // Inspect the aggregated counters via the built-in perf query as well.
    let query_snapshot: PerfSnapshot = pic
        .query_call(test_id, "canic_perf", (PageRequest::DEFAULT,))
        .expect("query perf snapshot");

    let mut entries: HashMap<_, _> = snapshot
        .entries
        .into_iter()
        .map(|entry| (entry.label, entry.total_instructions))
        .collect();

    // Ensure the query path also exposes perf counters.
    assert!(
        !query_snapshot.entries.is_empty(),
        "canic_perf query should return perf entries"
    );

    for label in ["workload_one", "workload_two"] {
        let Some(total) = entries.remove(label) else {
            panic!("missing perf entry for {label}");
        };

        assert!(
            total > 0,
            "perf entry {label} should record instructions (got {total})"
        );
    }

    assert!(
        entries.contains_key("baseline"),
        "baseline perf checkpoint should be recorded"
    );
}
