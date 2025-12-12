use std::{collections::HashMap, env, fs, io, path::PathBuf};

use canic::{
    Error,
    cdk::types::Principal,
    core::{
        ids::{CanisterRole, SubnetRole},
        model::memory::{CanisterEntry, CanisterSummary},
        ops::model::memory::{
            directory::DirectoryPageDto, topology::subnet::SubnetCanisterChildrenPage,
        },
    },
    types::{PageRequest, TC},
};
use canic_internal::canister;
use canic_testkit::pic::{Pic, PicBuilder};

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
/// 3. Return `None` on GitHub Actions (because wasm is not built in CI).
///
/// This allows running tests without `make test` while still supporting
/// end-to-end PIC tests on local machines.
fn load_root_wasm() -> Option<Vec<u8>> {
    // CI environments skip PIC wasm-dependent tests.
    if option_env!("GITHUB_ACTIONS") == Some("true") {
        return None;
    }

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

struct Setup {
    pic: Pic,
    root_id: Principal,
    registry: HashMap<CanisterRole, CanisterEntry>,
}

fn setup_root() -> Option<Setup> {
    let root_wasm = load_root_wasm()?;

    let pic = PicBuilder::new()
        .with_nns_subnet()
        .with_application_subnet()
        .build();

    let root_id = pic
        .create_and_install_canister(CanisterRole::ROOT, root_wasm)
        .expect("install root canister");

    // Fund root so it can create children using its configured cycle targets.
    pic.add_cycles(root_id, 50 * TC);

    // The root performs child installation via timers.
    // Run timers enough times for root+children to finish bootstrapping.
    for _ in 0..100 {
        pic.tick();
    }

    let registry: Vec<CanisterEntry> = pic
        .query_call(root_id, "canic_subnet_canister_registry", ())
        .expect("query registry");

    let registry = registry
        .into_iter()
        .map(|entry| (entry.ty.clone(), entry))
        .collect();

    Some(Setup {
        pic,
        root_id,
        registry,
    })
}

#[test]
fn root_builds_hierarchy_and_exposes_env() {
    let Some(setup) = setup_root() else {
        eprintln!(
            "skipping root_builds_hierarchy_and_exposes_env — \
             run `make test` to build canisters or set {ROOT_WASM_ENV}"
        );
        return;
    };

    let Setup {
        pic,
        root_id,
        registry: by_type,
    } = setup;

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

        let env: canic::core::model::memory::env::EnvData = pic
            .query_call(entry.pid, "canic_env", ())
            .expect("query child env");

        assert_eq!(
            env.canister_type,
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
            env.subnet_type,
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
    let Some(setup) = setup_root() else {
        eprintln!(
            "skipping directories_are_consistent_across_canisters — \
             run `make test` to build canisters or set {ROOT_WASM_ENV}"
        );
        return;
    };

    let Setup {
        pic,
        root_id,
        registry,
    } = setup;

    //    let print_counts = env::var("PRINT_DIR_COUNTS").is_ok();
    let print_counts = true;

    let root_app_dir: DirectoryPageDto = pic
        .query_call(root_id, "canic_app_directory", (PageRequest::new(100, 0),))
        .expect("root app directory");
    let root_subnet_dir: Result<DirectoryPageDto, Error> = pic
        .query_call(
            root_id,
            "canic_subnet_directory",
            (PageRequest::new(100, 0),),
        )
        .expect("root subnet directory");
    let root_subnet_dir =
        root_subnet_dir.unwrap_or_else(|err| panic!("root subnet directory result: {err}"));

    if print_counts {
        eprintln!(
            "root app directory entries: {}, root subnet directory entries: {}",
            root_app_dir.entries.len(),
            root_subnet_dir.entries.len()
        );
    }

    for (ty, entry) in registry.iter().filter(|(ty, _)| !ty.is_root()) {
        let app_dir: DirectoryPageDto = pic
            .query_call(
                entry.pid,
                "canic_app_directory",
                (PageRequest::new(100, 0),),
            )
            .expect("child app directory");
        let subnet_dir: Result<DirectoryPageDto, Error> = pic
            .query_call(
                entry.pid,
                "canic_subnet_directory",
                (PageRequest::new(100, 0),),
            )
            .expect("child subnet directory");
        let subnet_dir = subnet_dir
            .unwrap_or_else(|err| panic!("child subnet directory result for {ty}: {err}"));

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
    let Some(setup) = setup_root() else {
        eprintln!(
            "skipping subnet_children_matches_registry_on_root — \
             run `make test` to build canisters or set {ROOT_WASM_ENV}"
        );
        return;
    };

    let Setup {
        pic,
        root_id,
        registry,
    } = setup;

    let mut expected_children: Vec<CanisterSummary> = registry
        .values()
        .filter(|entry| entry.parent_pid == Some(root_id))
        .map(|entry| CanisterSummary {
            pid: entry.pid,
            ty: entry.ty.clone(),
            parent_pid: entry.parent_pid,
        })
        .collect();

    assert!(
        !expected_children.is_empty(),
        "registry should contain root children"
    );

    let mut page: canic::core::ops::model::memory::topology::SubnetCanisterChildrenPage = pic
        .query_call(
            root_id,
            "canic_subnet_canister_children",
            (PageRequest::new(100, 0),),
        )
        .expect("query root subnet children");

    expected_children.sort_by(|a, b| a.ty.cmp(&b.ty));
    page.children.sort_by(|a, b| a.ty.cmp(&b.ty));

    assert_eq!(
        page.total,
        expected_children.len() as u64,
        "reported total mismatch"
    );
    assert_eq!(
        page.children, expected_children,
        "child list from endpoint must match registry"
    );
}

#[test]
fn worker_topology_cascades_through_parent() {
    let Some(setup) = setup_root() else {
        eprintln!(
            "skipping worker_topology_cascades_through_parent — \
             run `make test` to build canisters or set {ROOT_WASM_ENV}"
        );
        return;
    };

    let Setup {
        pic,
        root_id,
        registry,
    } = setup;

    let scale_hub = registry
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
    let mut children_page: SubnetCanisterChildrenPage = pic
        .query_call(
            scale_hub.pid,
            "canic_subnet_canister_children",
            (PageRequest::new(100, 0),),
        )
        .expect("scale_hub subnet children");
    children_page
        .children
        .retain(|c| c.parent_pid == Some(scale_hub.pid));

    assert!(
        children_page.children.iter().any(|c| c.pid == worker_pid),
        "scale_hub children should include the new worker"
    );
}
