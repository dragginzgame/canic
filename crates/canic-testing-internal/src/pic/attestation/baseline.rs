use candid::Principal;
use ic_testkit::pic::{
    CachedPicBaseline, InstallSpec, Pic, restore_or_rebuild_cached_pic_baseline,
};
use std::sync::{Mutex, OnceLock};

use crate::pic::{
    canic::{activate_managed_fleet, install_root_args},
    role_pid as lookup_role_pid, wait_until_ready as wait_for_ready_canister,
};

use super::{
    build::{build_pic, build_test_root_wasm},
    fixture::{CachedInstalledRoot, progress},
};

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
static ROOT_ISSUER_BASELINE: OnceLock<
    Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
> = OnceLock::new();

pub struct AttestationBaselineMetadata {
    root_id: Principal,
    wasm_store_id: Principal,
    issuer_id: Principal,
}

struct InstalledRoot {
    pic: super::build::SerialPic,
    root_id: Principal,
}

// Restore or create the cached `root + issuer` baseline.
#[must_use]
pub(super) fn install_cached_root_fixture() -> CachedInstalledRoot {
    progress("request cached root+issuer baseline");
    let baseline_slot = ROOT_ISSUER_BASELINE.get_or_init(|| Mutex::new(None));
    let (baseline, cache_hit) = restore_or_rebuild_cached_pic_baseline(
        baseline_slot,
        build_cached_baseline,
        restore_cached_baseline,
    );
    if cache_hit {
        progress("cache hit");
    }
    progress("cached fixture restore complete");

    CachedInstalledRoot {
        root_id: baseline.metadata().root_id,
        issuer_id: baseline.metadata().issuer_id,
        pic: baseline,
    }
}

// Resolve the issuer canister from the root-managed subnet registry.
#[must_use]
fn issuer_pid(pic: &Pic, root_id: Principal) -> Principal {
    lookup_role_pid(pic, root_id, "issuer", 120)
}

// Resolve the managed wasm_store canister from the root-managed subnet registry.
#[must_use]
fn wasm_store_pid(pic: &Pic, root_id: Principal) -> Principal {
    lookup_role_pid(pic, root_id, "wasm_store", 120)
}

// Build one reusable baseline and capture immutable snapshot IDs inside it.
fn build_cached_baseline() -> CachedPicBaseline<AttestationBaselineMetadata> {
    progress("cache miss, building fresh baseline");
    let InstalledRoot { pic, root_id } = install_test_root();
    progress("waiting for issuer registration");
    let issuer_id = issuer_pid(&pic, root_id);
    wait_for_ready_canister(&pic, issuer_id, 240);
    let wasm_store_id = wasm_store_pid(&pic, root_id);
    wait_for_ready_canister(&pic, wasm_store_id, 240);
    progress("issuer ready");

    progress("waiting for root readiness before snapshot capture");
    wait_for_ready_canister(&pic, root_id, 240);
    progress("capturing baseline snapshots");
    let controller_ids = vec![root_id, wasm_store_id, issuer_id];
    let baseline = CachedPicBaseline::capture(
        pic.into_pic(),
        root_id,
        controller_ids,
        AttestationBaselineMetadata {
            root_id,
            wasm_store_id,
            issuer_id,
        },
    )
    .expect("downloaded baseline snapshots unavailable");
    progress("fresh baseline ready");
    baseline
}

// Restore the cached baseline snapshots into the same baseline PocketIC instance.
fn restore_cached_baseline(baseline: &CachedPicBaseline<AttestationBaselineMetadata>) {
    progress("restoring cached baseline snapshots");
    baseline.restore(baseline.metadata().root_id);

    baseline.pic().tick();

    progress("waiting for restored root and issuer readiness");
    wait_for_ready_canister(baseline.pic(), baseline.metadata().wasm_store_id, 240);
    wait_for_ready_canister(baseline.pic(), baseline.metadata().issuer_id, 240);
    wait_for_ready_canister(baseline.pic(), baseline.metadata().root_id, 240);
}

// Install the test root into a fresh PocketIC instance.
fn install_test_root() -> InstalledRoot {
    install_root_fixture(build_test_root_wasm())
}

// Install one root wasm into a fresh serialized PocketIC instance.
fn install_root_fixture(root_wasm: Vec<u8>) -> InstalledRoot {
    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);
    activate_test_fleet(&pic, root_id);

    InstalledRoot { pic, root_id }
}

fn activate_test_fleet(pic: &Pic, root_id: Principal) {
    progress("preparing Fleet activation");
    progress("activating prepared Fleet");
    activate_managed_fleet(pic, root_id);
}

// Install the root canister under PocketIC with the current exact Fleet identity.
fn install_root_canister(pic: &Pic, wasm: Vec<u8>) -> Principal {
    pic.create_and_install(
        InstallSpec::new(
            wasm,
            install_root_args().expect("encode root install identity"),
            ROOT_INSTALL_CYCLES,
        )
        .label("role_attestation_root"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use candid::encode_one;
    use canic::{
        Error,
        dto::fleet_activation::{FleetActivationPhase, FleetActivationStatusResponse},
        protocol::CANIC_FLEET_ACTIVATION_STATUS,
    };
    use std::time::Duration;

    #[test]
    fn prepared_root_upgrade_does_not_run_runtime_or_application_continuations() {
        let root_wasm = build_test_root_wasm();
        let pic = build_pic();
        let root_id = install_root_canister(&pic, root_wasm.clone());
        assert_prepared(&pic, root_id);

        pic.wait_out_install_code_rate_limit(Duration::from_mins(5));
        pic.upgrade_canister(
            root_id,
            root_wasm,
            encode_one(()).expect("encode root upgrade"),
            None,
        )
        .expect("upgrade Prepared root");
        pic.advance_time(Duration::from_secs(1));
        pic.tick();
        assert_prepared(&pic, root_id);

        activate_test_fleet(&pic, root_id);
        wait_for_ready_canister(&pic, root_id, 240);
        pic.tick();

        let executions: Result<u64, Error> = pic
            .query_call(root_id, "root_upgrade_hook_executions", ())
            .expect("query root upgrade hook count");
        assert_eq!(
            executions.expect("root upgrade hook count"),
            0,
            "Prepared root upgrade must not run the application upgrade hook"
        );
    }

    fn assert_prepared(pic: &Pic, root_id: Principal) {
        let status: Result<FleetActivationStatusResponse, Error> = pic
            .query_call(root_id, CANIC_FLEET_ACTIVATION_STATUS, ())
            .expect("query root activation status");
        assert_eq!(
            status.expect("root activation status").phase,
            FleetActivationPhase::Prepared
        );
    }
}
