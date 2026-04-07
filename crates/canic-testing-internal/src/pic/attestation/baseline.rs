use candid::{Principal, encode_one};
use canic_core::dto::subnet::SubnetIdentity;
use canic_testkit::pic::{
    CachedPicBaseline, CachedPicBaselineGuard, restore_or_rebuild_cached_pic_baseline,
    role_pid as lookup_role_pid, wait_until_ready as wait_for_ready_canister,
};
use std::sync::{Mutex, OnceLock};

use super::{
    build::{build_normal_root_wasm, build_pic, build_test_root_wasm},
    capability::create_verifier_canister,
    fixture::{BaselinePicGuard, CachedInstalledRoot, progress},
};

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
static ROOT_SIGNER_BASELINE: OnceLock<
    Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
> = OnceLock::new();
static ROOT_SIGNER_VERIFIER_BASELINE: OnceLock<
    Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
> = OnceLock::new();
static ROOT_SIGNER_NO_TEST_HOOK_BASELINE: OnceLock<
    Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
> = OnceLock::new();

pub(super) struct AttestationBaselineMetadata {
    root_id: Principal,
    wasm_store_id: Principal,
    signer_id: Principal,
    verifier_id: Option<Principal>,
}

#[derive(Clone, Copy)]
enum AttestationCacheKind {
    SignerOnly,
    SignerAndVerifier,
    SignerOnlyWithoutTestMaterial,
}

struct InstalledRoot {
    pic: super::build::SerialPic,
    root_id: Principal,
}

// Restore or create the cached `root + signer` baseline.
#[must_use]
pub(super) fn install_signer_only_cached_root_fixture() -> CachedInstalledRoot {
    install_cached_root_fixture(AttestationCacheKind::SignerOnly)
}

// Restore or create the cached `root + signer + verifier` baseline.
#[must_use]
pub(super) fn install_signer_and_verifier_cached_root_fixture() -> CachedInstalledRoot {
    install_cached_root_fixture(AttestationCacheKind::SignerAndVerifier)
}

// Restore or create the cached normal-build `root + signer` baseline.
#[must_use]
pub(super) fn install_signer_only_without_test_material_cached_root_fixture() -> CachedInstalledRoot
{
    install_cached_root_fixture(AttestationCacheKind::SignerOnlyWithoutTestMaterial)
}

// Resolve the signer canister from the root-managed subnet registry.
#[must_use]
pub(super) fn signer_pid(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    lookup_role_pid(pic, root_id, "signer", 120)
}

// Resolve the managed wasm_store canister from the root-managed subnet registry.
#[must_use]
pub(super) fn wasm_store_pid(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    lookup_role_pid(pic, root_id, "wasm_store", 120)
}

// Restore or create the requested cached baseline and keep it alive until test drop.
fn install_cached_root_fixture(cache_kind: AttestationCacheKind) -> CachedInstalledRoot {
    progress(match cache_kind {
        AttestationCacheKind::SignerOnly => "request cached root+signer baseline",
        AttestationCacheKind::SignerAndVerifier => "request cached root+signer+verifier baseline",
        AttestationCacheKind::SignerOnlyWithoutTestMaterial => {
            "request cached root+signer normal-build baseline"
        }
    });
    let baseline_slot = baseline_slot(cache_kind).get_or_init(|| Mutex::new(None));
    let (baseline, cache_hit) = restore_or_rebuild_cached_baseline(baseline_slot, cache_kind);
    if cache_hit {
        progress("cache hit");
    }
    progress("cached fixture restore complete");

    CachedInstalledRoot {
        root_id: baseline.metadata().root_id,
        signer_id: baseline.metadata().signer_id,
        verifier_id: baseline.metadata().verifier_id,
        pic: BaselinePicGuard::new(baseline),
    }
}

// Restore a cached baseline when possible, or rebuild it if the underlying
// PocketIC instance has gone away between tests.
fn restore_or_rebuild_cached_baseline(
    baseline_slot: &'static Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
    cache_kind: AttestationCacheKind,
) -> (
    CachedPicBaselineGuard<'static, AttestationBaselineMetadata>,
    bool,
) {
    restore_or_rebuild_cached_pic_baseline(
        baseline_slot,
        || build_cached_baseline(cache_kind),
        restore_cached_baseline,
    )
}

// Build one reusable baseline and capture immutable snapshot IDs inside it.
fn build_cached_baseline(
    cache_kind: AttestationCacheKind,
) -> CachedPicBaseline<AttestationBaselineMetadata> {
    progress("cache miss, building fresh baseline");
    let InstalledRoot { pic, root_id } = match cache_kind {
        AttestationCacheKind::SignerOnly | AttestationCacheKind::SignerAndVerifier => {
            install_test_root()
        }
        AttestationCacheKind::SignerOnlyWithoutTestMaterial => {
            install_test_root_without_test_material()
        }
    };
    progress("waiting for signer registration");
    let signer_id = signer_pid(&pic, root_id);
    wait_for_ready_canister(&pic, signer_id, 240);
    let wasm_store_id = wasm_store_pid(&pic, root_id);
    wait_for_ready_canister(&pic, wasm_store_id, 240);
    progress("signer ready");
    let verifier_id = matches!(cache_kind, AttestationCacheKind::SignerAndVerifier).then(|| {
        progress("creating verifier baseline canister");
        let verifier_id = create_verifier_canister(&pic, root_id);
        progress("verifier baseline canister ready");
        verifier_id
    });

    progress("waiting for root readiness before snapshot capture");
    wait_for_ready_canister(&pic, root_id, 240);
    progress("capturing baseline snapshots");
    let controller_ids = std::iter::once(root_id)
        .chain(std::iter::once(wasm_store_id))
        .chain(std::iter::once(signer_id))
        .chain(verifier_id)
        .collect::<Vec<_>>();
    let baseline = CachedPicBaseline::capture(
        pic.into_pic(),
        root_id,
        controller_ids,
        AttestationBaselineMetadata {
            root_id,
            wasm_store_id,
            signer_id,
            verifier_id,
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

    progress("waiting for restored root and signer readiness");
    wait_for_ready_canister(baseline.pic(), baseline.metadata().wasm_store_id, 240);
    wait_for_ready_canister(baseline.pic(), baseline.metadata().signer_id, 240);
    if let Some(verifier_id) = baseline.metadata().verifier_id {
        progress("waiting for restored verifier readiness");
        wait_for_ready_canister(baseline.pic(), verifier_id, 240);
    }
    wait_for_ready_canister(baseline.pic(), baseline.metadata().root_id, 240);
}

// Install the test root with delegation-material hooks into a fresh PocketIC instance.
fn install_test_root() -> InstalledRoot {
    install_root_fixture(build_test_root_wasm())
}

// Install the test root without delegation-material hooks into a fresh PocketIC instance.
fn install_test_root_without_test_material() -> InstalledRoot {
    install_root_fixture(build_normal_root_wasm())
}

// Install one root wasm into a fresh serialized PocketIC instance.
fn install_root_fixture(root_wasm: Vec<u8>) -> InstalledRoot {
    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    InstalledRoot { pic, root_id }
}

// Return the immutable baseline slot for one cache kind.
const fn baseline_slot(
    cache_kind: AttestationCacheKind,
) -> &'static OnceLock<Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>> {
    match cache_kind {
        AttestationCacheKind::SignerOnly => &ROOT_SIGNER_BASELINE,
        AttestationCacheKind::SignerAndVerifier => &ROOT_SIGNER_VERIFIER_BASELINE,
        AttestationCacheKind::SignerOnlyWithoutTestMaterial => &ROOT_SIGNER_NO_TEST_HOOK_BASELINE,
    }
}

// Install the root canister under PocketIC with the manual subnet identity.
fn install_root_canister(pic: &pocket_ic::PocketIc, wasm: Vec<u8>) -> Principal {
    let root_id = pic.create_canister();
    pic.add_cycles(root_id, ROOT_INSTALL_CYCLES);
    pic.install_canister(
        root_id,
        wasm,
        encode_one(SubnetIdentity::Manual).expect("encode args"),
        None,
    );
    root_id
}
