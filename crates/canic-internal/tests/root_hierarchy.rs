use canic::{
    Error,
    cdk::types::Principal,
    core::{
        dto::page::{Page, PageRequest},
        ids::{CanisterRole, SubnetRole},
        ops::storage::{CanisterEntry, CanisterSummary, directory::PrincipalList, env::EnvData},
    },
    types::TC,
};
use canic_internal::canister;
use canic_testkit::pic::{Pic, pic};
use std::{collections::HashMap, env, fs, io, path::PathBuf, sync::OnceLock};

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

// -----------------------------------------------------------------------------
// TESTS
// -----------------------------------------------------------------------------

static SETUP: OnceLock<Setup> = OnceLock::new();

struct Setup {
    pic: &'static Pic,
    root_id: Principal,
    registry: HashMap<CanisterRole, CanisterEntry>,
}

fn setup_root() -> &'static Setup {
    SETUP.get_or_init(|| {
        let root_wasm = load_root_wasm().expect("load root wasm");

        let pic = pic();

        let root_id = pic
            .create_and_install_canister(CanisterRole::ROOT, root_wasm)
            .expect("install root canister");

        // Fund root so it can create children using its configured cycle targets.
        pic.add_cycles(root_id, 50 * TC);

        // The root performs child installation via timers.
        for _ in 0..100 {
            pic.tick();
        }

        let registry_vec: Vec<CanisterEntry> = pic
            .query_call(root_id, "canic_subnet_canister_registry", ())
            .expect("query registry");

        let registry = registry_vec
            .into_iter()
            .map(|entry| (entry.role.clone(), entry))
            .collect::<HashMap<_, _>>();

        Setup {
            pic,
            root_id,
            registry,
        }
    })
}
#[test]
fn root_builds_hierarchy_and_exposes_env() {
    // setup
    let setup = setup_root();
    let pic = setup.pic;
    let root_id = setup.root_id;
    let by_type = &setup.registry;

    let expected = [
        (CanisterRole::ROOT, None),
        (canister::APP, Some(root_id)),
        (canister::AUTH, Some(root_id)),
        (canister::SCALE_HUB, Some(root_id)),
        (canister::SHARD_HUB, Some(root_id)),
    ];

    for (ty, parent) in expected {
        let entry = by_type
            .get(&ty)
            .unwrap_or_else(|| panic!("missing {ty} entry in registry"));

        assert_eq!(entry.parent_pid, parent, "unexpected parent for {ty}");
    }

    let children = [
        canister::APP,
        canister::AUTH,
        canister::SCALE_HUB,
        canister::SHARD_HUB,
    ];

    for child_ty in children {
        let entry = by_type
            .get(&child_ty)
            .unwrap_or_else(|| panic!("missing {child_ty} entry in registry"));

        let env: EnvData = pic
            .query_call(entry.pid, "canic_env", ())
            .expect("query child env");

        assert_eq!(
            env.canister_role,
            Some(child_ty.clone()),
            "env canister type for {child_ty}",
        );
        assert_eq!(env.parent_pid, Some(root_id), "env parent for {child_ty}",);
        assert_eq!(env.root_pid, Some(root_id), "env root for {child_ty}",);
        assert_eq!(
            env.prime_root_pid,
            Some(root_id),
            "env prime root for {child_ty}",
        );
        assert_eq!(
            env.subnet_role,
            Some(SubnetRole::PRIME),
            "env subnet type for {child_ty}",
        );

        assert!(
            env.subnet_pid.is_some(),
            "env subnet pid should be set for {child_ty}"
        );
    }
}

#[test]
fn directories_are_consistent_across_canisters() {
    // setup
    let setup = setup_root();
    let pic = setup.pic;
    let root_id = setup.root_id;
    let by_type = &setup.registry;

    //    let print_counts = env::var("PRINT_DIR_COUNTS").is_ok();
    let print_counts = true;

    let root_app_dir: Page<(CanisterRole, PrincipalList)> = pic
        .query_call(root_id, "canic_app_directory", (PageRequest::new(100, 0),))
        .expect("root app directory");
    let root_subnet_dir: Page<(CanisterRole, PrincipalList)> = pic
        .query_call(
            root_id,
            "canic_subnet_directory",
            (PageRequest::new(100, 0),),
        )
        .expect("root subnet directory");

    if print_counts {
        eprintln!(
            "root app directory entries: {}, root subnet directory entries: {}",
            root_app_dir.entries.len(),
            root_subnet_dir.entries.len()
        );
    }

    for (ty, entry) in by_type.iter().filter(|(ty, _)| !ty.is_root()) {
        let app_dir: Page<(CanisterRole, PrincipalList)> = pic
            .query_call(
                entry.pid,
                "canic_app_directory",
                (PageRequest::new(100, 0),),
            )
            .expect("child app directory");
        let subnet_dir: Page<(CanisterRole, PrincipalList)> = pic
            .query_call(
                entry.pid,
                "canic_subnet_directory",
                (PageRequest::new(100, 0),),
            )
            .expect("child subnet directory");

        if print_counts {
            eprintln!(
                "{ty} app directory entries: {}, subnet directory entries: {}",
                app_dir.entries.len(),
                subnet_dir.entries.len(),
            );
        }

        assert_eq!(
            app_dir.entries, root_app_dir.entries,
            "app directory mismatch for {ty}"
        );
        assert_eq!(
            subnet_dir.entries, root_subnet_dir.entries,
            "subnet directory mismatch for {ty}"
        );
    }
}

#[test]
fn subnet_children_matches_registry_on_root() {
    // setup
    let setup = setup_root();
    let pic = setup.pic;
    let root_id = setup.root_id;
    let by_type = &setup.registry;

    let mut expected_children: Vec<CanisterSummary> = by_type
        .values()
        .filter(|entry| entry.parent_pid == Some(root_id))
        .map(|entry| CanisterSummary {
            pid: entry.pid,
            role: entry.role.clone(),
            parent_pid: entry.parent_pid,
        })
        .collect();

    assert!(
        !expected_children.is_empty(),
        "registry should contain root children"
    );

    let mut page: Page<CanisterSummary> = pic
        .query_call(
            root_id,
            "canic_subnet_canister_children",
            (PageRequest::new(100, 0),),
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
    let by_type = &setup.registry;

    let scale_hub = by_type
        .get(&canister::SCALE_HUB)
        .expect("scale_hub present in registry");

    // Create a worker via the scale_hub canister.
    let worker_pid: Result<Result<Principal, Error>, Error> =
        pic.update_call(scale_hub.pid, "create_worker", ());
    let worker_pid = worker_pid
        .expect("create worker via scale_hub (transport)")
        .expect("create worker via scale_hub (app)");

    // Allow any async cascades to settle.
    for _ in 0..10 {
        pic.tick();
    }

    // Registry on root should show the worker as a child of scale_hub.
    let registry_after: Vec<CanisterEntry> = pic
        .query_call(root_id, "canic_subnet_canister_registry", ())
        .expect("registry after worker creation");
    let worker_entry = registry_after
        .iter()
        .find(|entry| entry.pid == worker_pid)
        .expect("worker present in registry after creation");
    assert_eq!(
        worker_entry.parent_pid,
        Some(scale_hub.pid),
        "worker should be registered under scale_hub"
    );

    // Scale_hub's view of its children should include the worker (auth is parent-only).
    let mut children_page: Page<CanisterSummary> = pic
        .query_call(
            scale_hub.pid,
            "canic_subnet_canister_children",
            (PageRequest::new(100, 0),),
        )
        .expect("scale_hub subnet children");
    children_page
        .entries
        .retain(|c| c.parent_pid == Some(scale_hub.pid));

    assert!(
        children_page.entries.iter().any(|c| c.pid == worker_pid),
        "scale_hub children should include the new worker"
    );
}
