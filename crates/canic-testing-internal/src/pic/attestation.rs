use candid::{Principal, decode_one, encode_args, encode_one, utils::ArgumentEncoder};
use canic::Error;
use canic_core::api::rpc::RpcApi;
use canic_core::dto::{
    auth::SignedRoleAttestation,
    capability::{
        CAPABILITY_VERSION_V1, CapabilityProof, CapabilityService, PROOF_VERSION_V1,
        RoleAttestationProof, RootCapabilityEnvelopeV1, RootCapabilityResponseV1,
    },
    rpc::{CreateCanisterParent, CreateCanisterRequest, Request, Response},
    subnet::SubnetIdentity,
};
use canic_core::ids::CanisterRole;
use canic_testkit::artifacts::{
    WasmBuildProfile, build_internal_test_wasm_canisters,
    build_internal_test_wasm_canisters_with_env,
};
use canic_testkit::pic::{
    CachedPicBaseline, CachedPicBaselineGuard, Pic, PicSerialGuard, acquire_pic_serial_guard,
    pic as shared_pic, restore_or_rebuild_cached_pic_baseline, role_pid as lookup_role_pid,
    wait_until_ready as wait_for_ready_canister,
};
use serde::de::DeserializeOwned;
use std::{
    fs,
    io::Write,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::{Mutex, Once, OnceLock},
};

const ROOT_INSTALL_CYCLES: u128 = 80_000_000_000_000;
const CANISTER_PACKAGES: [&str; 1] = ["delegation_root_stub"];
static BUILD_ONCE: Once = Once::new();
static BUILD_WITHOUT_TEST_MATERIAL_ONCE: Once = Once::new();
static CANISTER_BUILD_SERIAL: Mutex<()> = Mutex::new(());
static ROOT_SIGNER_BASELINE: OnceLock<
    Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
> = OnceLock::new();
static ROOT_SIGNER_VERIFIER_BASELINE: OnceLock<
    Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
> = OnceLock::new();
static ROOT_SIGNER_NO_TEST_HOOK_BASELINE: OnceLock<
    Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
> = OnceLock::new();

///
/// CachedInstalledRoot
///

pub struct CachedInstalledRoot {
    pub pic: BaselinePicGuard<'static>,
    pub root_id: Principal,
    pub signer_id: Principal,
    pub verifier_id: Option<Principal>,
}

///
/// BaselinePicGuard
///

pub struct BaselinePicGuard<'a> {
    baseline: CachedPicBaselineGuard<'a, AttestationBaselineMetadata>,
}

impl Deref for BaselinePicGuard<'_> {
    type Target = pocket_ic::PocketIc;

    fn deref(&self) -> &Self::Target {
        &self.baseline.pic
    }
}

///
/// AttestationBaselineMetadata
///

struct AttestationBaselineMetadata {
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

///
/// SerialPic
///

struct SerialPic {
    pic: Pic,
    _serial_guard: PicSerialGuard,
}

impl Deref for SerialPic {
    type Target = pocket_ic::PocketIc;

    fn deref(&self) -> &Self::Target {
        &self.pic
    }
}

impl DerefMut for SerialPic {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pic
    }
}

///
/// InstalledRoot
///

struct InstalledRoot {
    pic: SerialPic,
    root_id: Principal,
}

// Emit one short progress marker for long grouped PocketIC scenario tests.
fn progress(phase: &str) {
    eprintln!("[pic_role_attestation] fixture: {phase}");
    let _ = std::io::stderr().flush();
}

/// Restore or create the cached `root + signer` baseline.
#[must_use]
pub fn install_test_root_cached() -> CachedInstalledRoot {
    install_cached_root_fixture(AttestationCacheKind::SignerOnly)
}

/// Restore or create the cached `root + signer + verifier` baseline.
#[must_use]
pub fn install_test_root_with_verifier_cached() -> CachedInstalledRoot {
    install_cached_root_fixture(AttestationCacheKind::SignerAndVerifier)
}

/// Restore or create the cached normal-build `root + signer` baseline.
#[must_use]
pub fn install_test_root_without_test_material_cached() -> CachedInstalledRoot {
    install_cached_root_fixture(AttestationCacheKind::SignerOnlyWithoutTestMaterial)
}

// Install the test root with delegation-material hooks into a fresh PocketIC instance.
fn install_test_root() -> InstalledRoot {
    let workspace_root = workspace_root();
    build_canisters_once(&workspace_root);
    let root_wasm = read_wasm(&workspace_root, "delegation_root_stub");
    install_root_fixture(root_wasm)
}

// Install the test root without delegation-material hooks into a fresh PocketIC instance.
fn install_test_root_without_test_material() -> InstalledRoot {
    let workspace_root = workspace_root();
    build_canisters_without_test_material_once(&workspace_root);
    let root_wasm = read_wasm_from_target(
        &test_target_dir_without_test_material(&workspace_root),
        "delegation_root_stub",
    );
    install_root_fixture(root_wasm)
}

// Install one root wasm into a fresh serialized PocketIC instance.
fn install_root_fixture(root_wasm: Vec<u8>) -> InstalledRoot {
    let pic = build_pic();
    let root_id = install_root_canister(&pic, root_wasm);

    InstalledRoot { pic, root_id }
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
        root_id: baseline.metadata.root_id,
        signer_id: baseline.metadata.signer_id,
        verifier_id: baseline.metadata.verifier_id,
        pic: BaselinePicGuard { baseline },
    }
}

// Restore a cached baseline when possible, or rebuild it if the underlying
// PocketIC instance has gone away between tests.
fn restore_or_rebuild_cached_baseline(
    baseline_slot: &'static Mutex<Option<CachedPicBaseline<AttestationBaselineMetadata>>>,
    cache_kind: AttestationCacheKind,
) -> (
    canic_testkit::pic::CachedPicBaselineGuard<'static, AttestationBaselineMetadata>,
    bool,
) {
    let (baseline, cache_hit) = restore_or_rebuild_cached_pic_baseline(
        baseline_slot,
        || build_cached_baseline(cache_kind),
        restore_cached_baseline,
    );
    (baseline, cache_hit)
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
    let SerialPic { pic, _serial_guard } = pic;
    let baseline = CachedPicBaseline::capture(
        pic,
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
    baseline.restore(baseline.metadata.root_id);

    baseline.pic.tick();

    progress("waiting for restored root and signer readiness");
    wait_for_ready_canister(&baseline.pic, baseline.metadata.wasm_store_id, 240);
    wait_for_ready_canister(&baseline.pic, baseline.metadata.signer_id, 240);
    if let Some(verifier_id) = baseline.metadata.verifier_id {
        progress("waiting for restored verifier readiness");
        wait_for_ready_canister(&baseline.pic, verifier_id, 240);
    }
    wait_for_ready_canister(&baseline.pic, baseline.metadata.root_id, 240);
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

// Create a non-root verifier canister through the root capability endpoint.
fn create_verifier_canister(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    let issued: Result<SignedRoleAttestation, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "root_issue_self_attestation_test",
        (60u64, Some(root_id), 0u64),
    );
    let issued = issued.expect("attestation issuance failed");
    let issued_at = issued.payload.issued_at;

    let request = Request::CreateCanister(CreateCanisterRequest {
        canister_role: CanisterRole::new("project_hub"),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
        metadata: None,
    });
    let envelope = RootCapabilityEnvelopeV1 {
        service: CapabilityService::Root,
        capability_version: CAPABILITY_VERSION_V1,
        capability: request.clone(),
        proof: encode_role_attestation_capability_proof(RoleAttestationProof {
            proof_version: PROOF_VERSION_V1,
            capability_hash: root_capability_hash(root_id, &request),
            attestation: issued,
        }),
        metadata: capability_metadata(issued_at, 41, 24, 60),
    };
    let response: Result<RootCapabilityResponseV1, Error> = update_call_as(
        pic,
        root_id,
        root_id,
        "canic_response_capability_v1",
        (envelope,),
    );
    let verifier_id = match response
        .expect("verifier canister creation capability call must succeed")
        .response
    {
        Response::CreateCanister(res) => res.new_canister_pid,
        other => panic!("expected create-canister response, got: {other:?}"),
    };
    wait_for_ready_canister(pic, verifier_id, 240);
    verifier_id
}

// Run one typed update call as the requested caller.
fn update_call_as<T, A>(
    pic: &pocket_ic::PocketIc,
    canister_id: Principal,
    caller: Principal,
    method: &str,
    args: A,
) -> T
where
    T: candid::CandidType + DeserializeOwned,
    A: ArgumentEncoder,
{
    let payload = encode_args(args).expect("encode args");
    let result = pic
        .update_call(canister_id, caller, method, payload)
        .expect("update_call failed");

    decode_one(&result).expect("decode response")
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

// Resolve the signer canister from the root-managed subnet registry.
#[must_use]
pub fn signer_pid(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    lookup_role_pid(pic, root_id, "signer", 120)
}

// Resolve the managed wasm_store canister from the root-managed subnet registry.
#[must_use]
pub fn wasm_store_pid(pic: &pocket_ic::PocketIc, root_id: Principal) -> Principal {
    lookup_role_pid(pic, root_id, "wasm_store", 120)
}

// Build the test canisters with delegation-material test cfg enabled.
fn build_canisters_once(workspace_root: &Path) {
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    BUILD_ONCE.call_once_force(|_| {
        let target_dir = test_target_dir(workspace_root);
        progress("building PIC wasm artifacts with test delegation material");
        build_internal_test_wasm_canisters_with_env(
            workspace_root,
            &target_dir,
            &CANISTER_PACKAGES,
            WasmBuildProfile::Fast,
            &[("CANIC_TEST_DELEGATION_MATERIAL", "1")],
        );
        progress("finished PIC wasm build with test delegation material");
    });
}

// Build the same test canisters without delegation-material test cfg enabled.
fn build_canisters_without_test_material_once(workspace_root: &Path) {
    let _serial_guard = CANISTER_BUILD_SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    BUILD_WITHOUT_TEST_MATERIAL_ONCE.call_once_force(|_| {
        let target_dir = test_target_dir_without_test_material(workspace_root);
        progress("building PIC wasm artifacts without test delegation material");
        build_internal_test_wasm_canisters(
            workspace_root,
            &target_dir,
            &CANISTER_PACKAGES,
            WasmBuildProfile::Fast,
        );
        progress("finished PIC wasm build without test delegation material");
    });
}

// Serialize full PocketIC usage to avoid concurrent server races across tests.
fn build_pic() -> SerialPic {
    progress("acquiring PocketIC serial guard");
    let serial_guard = acquire_pic_serial_guard();
    progress("starting serialized PocketIC instance");
    let pic = shared_pic();
    progress("serialized PocketIC instance ready");

    SerialPic {
        pic,
        _serial_guard: serial_guard,
    }
}

fn read_wasm(workspace_root: &Path, crate_name: &str) -> Vec<u8> {
    let wasm_path = wasm_path(workspace_root, crate_name);
    fs::read(&wasm_path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

fn read_wasm_from_target(target_dir: &Path, crate_name: &str) -> Vec<u8> {
    let wasm_path = wasm_path_from_target(target_dir, crate_name);
    fs::read(&wasm_path).unwrap_or_else(|err| panic!("failed to read {crate_name} wasm: {err}"))
}

fn wasm_path(workspace_root: &Path, crate_name: &str) -> PathBuf {
    let target_dir = test_target_dir(workspace_root);

    wasm_path_from_target(&target_dir, crate_name)
}

fn wasm_path_from_target(target_dir: &Path, crate_name: &str) -> PathBuf {
    target_dir
        .join("wasm32-unknown-unknown")
        .join("fast")
        .join(format!("{crate_name}.wasm"))
}

fn test_target_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join("target").join("pic-wasm")
}

fn test_target_dir_without_test_material(workspace_root: &Path) -> PathBuf {
    workspace_root
        .join("target")
        .join("pic-wasm-no-test-material")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(PathBuf::from)
        .expect("workspace root")
}

fn encode_role_attestation_capability_proof(proof: RoleAttestationProof) -> CapabilityProof {
    proof
        .try_into()
        .expect("role attestation proof should encode")
}

fn root_capability_hash(root_id: Principal, request: &Request) -> [u8; 32] {
    RpcApi::root_capability_hash(root_id, CAPABILITY_VERSION_V1, request)
        .expect("compute root capability hash")
}

const fn capability_metadata(
    issued_at: u64,
    request_id_seed: u8,
    nonce_seed: u8,
    ttl_seconds: u32,
) -> canic_core::dto::capability::CapabilityRequestMetadata {
    canic_core::dto::capability::CapabilityRequestMetadata {
        request_id: [request_id_seed; 16],
        nonce: [nonce_seed; 16],
        issued_at,
        ttl_seconds,
    }
}
