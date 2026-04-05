// Category C - Artifact / deployment test (embedded config).
// These checks intentionally avoid the root hierarchy when one standalone
// canister is enough to exercise the behavior under test.

use canic::Error;
use canic_internal::canister::SCALE_HUB;
use canic_testing_internal::pic::install_standalone_canister;
use canic_testkit::artifacts::WasmBuildProfile;

#[test]
fn standalone_scale_hub_perf_probe_succeeds() {
    let fixture =
        install_standalone_canister("canister_scale_hub", SCALE_HUB, WasmBuildProfile::Fast);

    let response: Result<(bool, u64), Error> = fixture
        .pic
        .query_call(fixture.canister_id, "plan_create_worker_perf_test", ())
        .expect("plan_create_worker_perf_test transport query failed");
    let (_plan, perf) = response.expect("plan_create_worker_perf_test application query failed");

    assert!(perf > 0, "expected positive local instruction count");
}
