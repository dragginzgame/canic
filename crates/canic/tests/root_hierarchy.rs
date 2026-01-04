use canic::{
    PublicError,
    cdk::types::{Principal, TC},
    core::{
        dto::{
            canister::CanisterSummaryView,
            env::EnvView,
            page::{Page, PageRequest},
            rpc::{CreateCanisterParent, CreateCanisterRequest, Request, Response},
            topology::{DirectoryEntryView, SubnetRegistryEntryView, SubnetRegistryView},
        },
        ids::{CanisterRole, SubnetRole},
        protocol,
    },
};
use canic_internal::canister;
use canic_testkit::pic::{Pic, pic};
use std::{collections::HashMap, env, fs, io, path::PathBuf};

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

/// Environment variable override for providing a pre-built root canister wasm.
const ROOT_WASM_ENV: &str = "CANIC_ROOT_WASM";

/// Default location of the root wasm relative to this crate’s manifest dir.
/// Only used when not running under GitHub Actions and when `$CANIC_ROOT_WASM`
/// is not provided.
const ROOT_WASM_RELATIVE: &str = "../../.dfx/local/canisters/root/root.wasm.gz";

// -----------------------------------------------------------------------------
// WASM Loader
// -----------------------------------------------------------------------------

/// Attempts to load the compiled root canister wasm.
///
/// Priority order:
/// 1. `$CANIC_ROOT_WASM` if set.
/// 2. Default local path under `.dfx`.
/// 3. Return `None` when no wasm is available.
///
/// This allows running tests without `make test` while still supporting
/// end-to-end PIC tests on local machines.
fn load_root_wasm() -> Option<Vec<u8>> {
    // Construct the default `.dfx` candidate path.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_path = manifest_dir.join(ROOT_WASM_RELATIVE);

    // Build list of candidates: ENV override first, then default.
    let mut candidates = env::var(ROOT_WASM_ENV)
        .ok()
        .map(PathBuf::from)
        .into_iter()
        .collect::<Vec<_>>();
    candidates.push(default_path);

    // Try each path in order.
    for path in candidates {
        match fs::read(&path) {
            Ok(bytes) => return Some(bytes),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                // Not found → try next path.
            }
            Err(err) => panic!("failed to read root wasm at {}: {}", path.display(), err),
        }
    }

    None
}

fn fetch_registry(pic: &Pic, root_id: Principal) -> Vec<SubnetRegistryEntryView> {
    let SubnetRegistryView(registry) = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("query registry");

    registry
}

fn fetch_subnet_directory(pic: &Pic, root_id: Principal) -> HashMap<CanisterRole, Principal> {
    let subnet_directory_page: Page<DirectoryEntryView> = pic
        .query_call(
            root_id,
            protocol::CANIC_SUBNET_DIRECTORY,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("query subnet directory");

    subnet_directory_page
        .entries
        .into_iter()
        .map(|entry| (entry.role, entry.pid))
        .collect()
}

fn registry_has_role(registry: &[SubnetRegistryEntryView], role: &CanisterRole) -> bool {
    registry.iter().any(|entry| &entry.role == role)
}

fn root_response(pic: &Pic, root_id: Principal, request: Request) -> Response {
    let response: Result<Response, PublicError> = pic
        .update_call_as(root_id, root_id, protocol::CANIC_RESPONSE, (request,))
        .expect("canic_response transport");

    response.expect("canic_response failed")
}

fn ensure_root_canister(pic: &Pic, root_id: Principal, role: CanisterRole) {
    let registry = fetch_registry(pic, root_id);
    if registry_has_role(&registry, &role) {
        return;
    }

    let request = Request::CreateCanister(CreateCanisterRequest {
        canister_role: role.clone(),
        parent: CreateCanisterParent::Root,
        extra_arg: None,
    });

    match root_response(pic, root_id, request) {
        Response::CreateCanister(_) => {}
        other => panic!("unexpected response for {role}: {other:?}"),
    }
}

// -----------------------------------------------------------------------------
// TESTS
// -----------------------------------------------------------------------------

struct Setup {
    pic: Pic,
    root_id: Principal,
    subnet_directory: HashMap<CanisterRole, Principal>,
}

fn setup_root() -> Setup {
    let root_wasm = load_root_wasm().expect("load root wasm");

    let pic = pic(); // now returns a *fresh* Pic
    let root_id = pic
        .create_and_install_canister(CanisterRole::ROOT, root_wasm)
        .expect("install root canister");

    pic.add_cycles(root_id, 50 * TC);

    let required = [
        canister::APP,
        canister::AUTH,
        canister::SCALE_HUB,
        canister::SHARD_HUB,
    ];

    for role in required {
        ensure_root_canister(&pic, root_id, role);
    }

    pic.tick_n(5);

    let subnet_directory = fetch_subnet_directory(&pic, root_id);

    Setup {
        pic,
        root_id,
        subnet_directory,
    }
}

#[test]
fn root_builds_hierarchy_and_exposes_env() {
    // setup
    let setup = setup_root();
    let pic = setup.pic;
    let root_id = setup.root_id;
    let registry = fetch_registry(&pic, root_id);
    let subnet_directory = &setup.subnet_directory;

    let expected = [
        (CanisterRole::ROOT, None),
        (canister::APP, Some(root_id)),
        (canister::AUTH, Some(root_id)),
        (canister::SCALE_HUB, Some(root_id)),
        (canister::SHARD_HUB, Some(root_id)),
    ];

    for (role, parent) in expected {
        let entry = registry
            .iter()
            .find_map(|entry| (entry.role == role).then_some(&entry.entry))
            .unwrap_or_else(|| panic!("missing {role} entry in registry"));

        assert_eq!(entry.parent_pid, parent, "unexpected parent for {role}");
    }

    let children = [
        canister::APP,
        canister::AUTH,
        canister::SCALE_HUB,
        canister::SHARD_HUB,
    ];

    for child_role in children {
        let entry_pid = subnet_directory
            .get(&child_role)
            .copied()
            .unwrap_or_else(|| panic!("missing {child_role} entry in subnet directory"));

        let env: EnvView = pic
            .query_call(entry_pid, protocol::CANIC_ENV, ())
            .expect("query child env");

        assert_eq!(
            env.canister_role,
            Some(child_role.clone()),
            "env canister role for {child_role}",
        );
        assert_eq!(env.parent_pid, Some(root_id), "env parent for {child_role}",);
        assert_eq!(env.root_pid, Some(root_id), "env root for {child_role}",);
        assert_eq!(
            env.prime_root_pid,
            Some(root_id),
            "env prime root for {child_role}",
        );
        assert_eq!(
            env.subnet_role,
            Some(SubnetRole::PRIME),
            "env subnet role for {child_role}",
        );

        assert!(
            env.subnet_pid.is_some(),
            "env subnet pid should be set for {child_role}"
        );
    }
}

#[test]
fn directories_are_consistent_across_canisters() {
    // setup
    let setup = setup_root();
    let pic = setup.pic;
    let root_id = setup.root_id;
    let subnet_directory = &setup.subnet_directory;

    let print_counts = true;

    let root_app_dir: Page<DirectoryEntryView> = pic
        .query_call(
            root_id,
            protocol::CANIC_APP_DIRECTORY,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("root app directory");
    let root_subnet_dir: Page<DirectoryEntryView> = pic
        .query_call(
            root_id,
            protocol::CANIC_SUBNET_DIRECTORY,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("root subnet directory");

    if print_counts {
        eprintln!(
            "root app directory entries: {}, root subnet directory entries: {}",
            root_app_dir.entries.len(),
            root_subnet_dir.entries.len()
        );
    }

    for (role, entry_pid) in subnet_directory.iter().filter(|(role, _)| !role.is_root()) {
        let app_dir: Page<DirectoryEntryView> = pic
            .query_call(
                *entry_pid,
                protocol::CANIC_APP_DIRECTORY,
                (PageRequest {
                    limit: 100,
                    offset: 0,
                },),
            )
            .expect("child app directory");
        let subnet_dir: Page<DirectoryEntryView> = pic
            .query_call(
                *entry_pid,
                protocol::CANIC_SUBNET_DIRECTORY,
                (PageRequest {
                    limit: 100,
                    offset: 0,
                },),
            )
            .expect("child subnet directory");

        if print_counts {
            eprintln!(
                "{role} app directory entries: {}, subnet directory entries: {}",
                app_dir.entries.len(),
                subnet_dir.entries.len(),
            );
        }

        assert_eq!(
            app_dir.entries, root_app_dir.entries,
            "app directory mismatch for {role}"
        );
        assert_eq!(
            subnet_dir.entries, root_subnet_dir.entries,
            "subnet directory mismatch for {role}"
        );
    }
}

#[test]
fn subnet_children_matches_registry_on_root() {
    // setup
    let setup = setup_root();
    let pic = setup.pic;
    let root_id = setup.root_id;
    let registry = fetch_registry(&pic, root_id);

    let mut expected_children: Vec<CanisterSummaryView> = registry
        .iter()
        .filter(|entry| entry.entry.parent_pid == Some(root_id))
        .map(|entry| CanisterSummaryView {
            role: entry.role.clone(),
            parent_pid: entry.entry.parent_pid,
        })
        .collect();

    assert!(
        !expected_children.is_empty(),
        "registry should contain root children"
    );

    let mut page: Page<CanisterSummaryView> = pic
        .query_call(
            root_id,
            protocol::CANIC_CANISTER_CHILDREN,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("query root subnet children");

    expected_children.sort_by(|a, b| a.role.cmp(&b.role));
    page.entries.sort_by(|a, b| a.role.cmp(&b.role));

    assert_eq!(
        page.total,
        expected_children.len() as u64,
        "reported total mismatch"
    );
    assert_eq!(
        page.entries, expected_children,
        "child list from endpoint must match registry"
    );
}

#[test]
fn worker_topology_cascades_through_parent() {
    // setup
    let setup = setup_root();
    let pic = setup.pic;
    let root_id = setup.root_id;
    let registry = fetch_registry(&pic, root_id);
    let subnet_directory = &setup.subnet_directory;

    let scale_hub_pid = subnet_directory
        .get(&canister::SCALE_HUB)
        .copied()
        .expect("scale_hub present in subnet directory");

    let worker_count_before = registry
        .iter()
        .filter(|entry| {
            entry.role == canister::SCALE && entry.entry.parent_pid == Some(scale_hub_pid)
        })
        .count();

    // Create a worker via the scale_hub canister.
    let worker_pid: Result<Result<Principal, PublicError>, PublicError> =
        pic.update_call(scale_hub_pid, "create_worker", ());
    let _worker_pid = worker_pid
        .expect("create worker via scale_hub (transport)")
        .expect("create worker via scale_hub (app)");

    // Allow any async cascades to settle.
    pic.tick_n(10);

    // Registry on root should show a new worker under scale_hub.
    let SubnetRegistryView(registry_after) = pic
        .query_call(root_id, protocol::CANIC_SUBNET_REGISTRY, ())
        .expect("registry after worker creation");
    let worker_count_after = registry_after
        .iter()
        .filter(|entry| {
            entry.role == canister::SCALE && entry.entry.parent_pid == Some(scale_hub_pid)
        })
        .count();

    assert_eq!(
        worker_count_after,
        worker_count_before + 1,
        "worker should be registered under scale_hub"
    );

    // Scale_hub's view of its children should include the worker (auth is parent-only).
    let children_page: Page<CanisterSummaryView> = pic
        .query_call(
            scale_hub_pid,
            protocol::CANIC_CANISTER_CHILDREN,
            (PageRequest {
                limit: 100,
                offset: 0,
            },),
        )
        .expect("scale_hub subnet children");
    let worker_children = children_page
        .entries
        .iter()
        .filter(|entry| entry.role == canister::SCALE)
        .count();

    assert!(
        worker_children >= worker_count_after,
        "scale_hub children should include the new worker"
    );
}
