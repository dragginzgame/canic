use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const MANAGED_START_MARKERS: &[&str] = &[
    "canic::start!()",
    "canic::start!(",
    "canic::start_local!()",
    "canic::start_local!(",
    "canic::start_wasm_store!()",
    "canic::start_wasm_store!(",
    "canic::start_fleet_coordinator!()",
    "canic::start_fleet_coordinator!(",
];

const RAW_ENDPOINT_MARKERS: &[&str] = &[
    "#[ic_cdk::query",
    "#[ic_cdk::update",
    "#[::ic_cdk::query",
    "#[::ic_cdk::update",
    "#[query]",
    "#[query(",
    "#[update]",
    "#[update(",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate directory should have a parent")
        .parent()
        .expect("workspace root should exist")
        .to_path_buf()
}

fn collect_files(root: &Path, filename: Option<&str>, extension: Option<&str>) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = fs::read_dir(&directory)
            .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|error| panic!("read entry under {}: {error}", directory.display()));
        entries.sort_by_key(std::fs::DirEntry::path);
        for entry in entries {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .unwrap_or_else(|error| panic!("inspect {}: {error}", path.display()));
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file()
                && filename.is_none_or(|expected| entry.file_name() == expected)
                && extension
                    .is_none_or(|expected| path.extension().is_some_and(|ext| ext == expected))
            {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

#[test]
fn managed_canisters_export_endpoints_only_through_canic_macros() {
    let workspace = workspace_root();
    let mut managed_sources = BTreeSet::new();

    for source_root in ["apps", "canisters"] {
        for manifest in collect_files(&workspace.join(source_root), Some("Cargo.toml"), None) {
            let package_root = manifest.parent().expect("package manifest parent");
            let sources = collect_files(&package_root.join("src"), None, Some("rs"));
            let managed = sources.iter().any(|path| {
                let source = fs::read_to_string(path)
                    .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
                MANAGED_START_MARKERS
                    .iter()
                    .any(|marker| source.contains(marker))
            });
            if managed {
                managed_sources.extend(sources);
            }
        }
    }

    let mut violations = Vec::new();
    for path in managed_sources {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for marker in RAW_ENDPOINT_MARKERS {
            if source.contains(marker) {
                violations.push(format!(
                    "{} contains raw managed endpoint marker {marker}",
                    path.strip_prefix(&workspace).unwrap_or(&path).display()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "managed Canister endpoints bypass the Canic activation dispatcher: {violations:#?}"
    );
}

#[test]
fn prepared_managed_init_defers_application_work_while_standalone_local_starts_it() {
    let macro_path = workspace_root().join("crates/canic/src/macros/start.rs");
    let source = fs::read_to_string(&macro_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", macro_path.display()));
    let managed = source
        .split("macro_rules! __canic_start_nonroot_lifecycle_core")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! __canic_start_local_lifecycle_core")
                .next()
        })
        .expect("managed non-root lifecycle macro");
    let managed_init = managed
        .split("#[$crate::__internal::cdk::init]")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[$crate::__internal::cdk::post_upgrade]")
                .next()
        })
        .expect("managed non-root init body");

    assert!(
        managed_init.contains("LifecycleApi::init_nonroot_canister_before_bootstrap"),
        "managed non-root init must enter the canonical Prepared lifecycle"
    );
    assert!(
        !managed_init.contains("schedule_init_nonroot_bootstrap")
            && !managed_init.contains("TimerApi::defer_lifecycle")
            && !managed_init.contains("canic_install("),
        "Prepared managed init must not schedule bootstrap, timers, or application hooks"
    );

    let local = source
        .split("macro_rules! __canic_start_local_lifecycle_core")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! __canic_root_lifecycle_core")
                .next()
        })
        .expect("standalone-local lifecycle macro");
    let local_init = local
        .split("#[$crate::__internal::cdk::init]")
        .nth(1)
        .and_then(|rest| {
            rest.split("#[$crate::__internal::cdk::post_upgrade]")
                .next()
        })
        .expect("standalone-local init body");

    assert!(
        local_init.contains("LifecycleApi::init_local_nonroot_canister_before_bootstrap")
            && local_init.contains("schedule_init_nonroot_bootstrap")
            && local_init.contains("canic_install(args)"),
        "standalone-local init must retain its explicit local lifecycle and application startup"
    );
    assert!(
        !local.contains("CanisterInitPayload") && !local.contains("FleetBinding"),
        "standalone-local lifecycle must not fabricate managed Fleet identity"
    );
}

#[test]
fn prepared_activation_schedules_each_current_application_install_hook_once() {
    let workspace = workspace_root();
    let macro_path = workspace.join("crates/canic/src/macros/start.rs");
    let source = fs::read_to_string(&macro_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", macro_path.display()));
    let nonroot = source
        .split("macro_rules! __canic_start_nonroot_lifecycle_core")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! __canic_start_wasm_store_lifecycle_core")
                .next()
        })
        .expect("managed non-root lifecycle macro");
    let wasm_store = source
        .split("macro_rules! __canic_start_wasm_store_lifecycle_core")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! __canic_start_local_lifecycle_core")
                .next()
        })
        .expect("Wasm Store lifecycle macro");
    let root = source
        .split("macro_rules! __canic_root_lifecycle_core")
        .nth(1)
        .and_then(|rest| {
            rest.split("// Run the optional init block from a lifecycle timer")
                .next()
        })
        .expect("managed root lifecycle macro");

    assert!(
        nonroot.contains("fn __canic_schedule_prepared_activation_init(args: Option<Vec<u8>>)")
            && nonroot.contains("canic_install(args).await;"),
        "managed non-root activation must receive durable init bytes from its transition"
    );
    assert!(
        wasm_store.contains("fn __canic_schedule_prepared_activation_init(args: Option<Vec<u8>>)")
            && wasm_store.contains("canic_install(args).await;"),
        "Wasm Store activation must receive durable init bytes from its transition"
    );
    assert!(
        root.contains("fn __canic_schedule_prepared_activation_init()")
            && root.contains("canic_setup().await;")
            && root.contains("canic_install().await;"),
        "managed root activation must schedule its current application install hooks"
    );

    let duplicate_guard = "__CANIC_PREPARED_APPLICATION_INIT_SCHEDULED.replace(true)";
    for (adapter, lifecycle) in [
        ("managed non-root", nonroot),
        ("Wasm Store", wasm_store),
        ("managed root", root),
    ] {
        assert_eq!(
            lifecycle.matches(duplicate_guard).count(),
            1,
            "{adapter} activation adapter must suppress duplicate hook scheduling"
        );
    }
    assert_eq!(
        source.matches(duplicate_guard).count(),
        3,
        "only the root, non-root and Wasm Store activation adapters may schedule application install hooks"
    );

    let nonroot_path = workspace.join("crates/canic/src/macros/endpoints/role.rs");
    let nonroot_endpoints = fs::read_to_string(&nonroot_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", nonroot_path.display()));
    assert!(
        nonroot_endpoints.contains("__canic_schedule_prepared_activation_init(")
            && nonroot_endpoints.contains("transition.application_init_args,"),
        "managed non-root activation must hand durable init bytes to the lifecycle adapter"
    );

    let root_path = workspace.join("crates/canic/src/macros/endpoints/root.rs");
    let root_endpoints = fs::read_to_string(&root_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", root_path.display()));
    assert!(
        root_endpoints.contains("__canic_schedule_prepared_activation_init();"),
        "managed root activation must hand success to the lifecycle adapter"
    );
}

#[test]
fn standalone_local_emits_only_local_status_and_standards() {
    let workspace = workspace_root();
    let start_path = workspace.join("crates/canic/src/macros/start.rs");
    let start = fs::read_to_string(&start_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", start_path.display()));
    let bundles_path = workspace.join("crates/canic/src/macros/endpoints/bundles.rs");
    let bundles = fs::read_to_string(&bundles_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", bundles_path.display()));

    let local_start = start
        .split("macro_rules! start_local")
        .nth(1)
        .and_then(|rest| rest.split("macro_rules! start_wasm_store").next())
        .expect("standalone-local start macro");
    assert!(
        local_start.contains("__canic_emit_local_status_endpoint!()")
            && local_start.contains("canic_emit_icrc_standards_endpoints!()")
            && !local_start.contains("__canic_emit_managed_command_endpoint!()")
            && !local_start.contains("__canic_emit_managed_status_endpoint!()"),
        "standalone-local startup must expose only local status and standards"
    );

    let store_bundle = bundles
        .split("macro_rules! canic_bundle_wasm_store_runtime_endpoints")
        .nth(1)
        .expect("Wasm Store endpoint bundle");
    assert!(
        store_bundle.contains("canic_emit_local_wasm_store_endpoints!()")
            && store_bundle.matches("canic_emit_").count() == 1,
        "Wasm Store control must be owned by its role command/status dispatcher"
    );
}

#[test]
fn fleet_admission_projection_is_managed_only_and_authenticates_before_state_access() {
    let workspace = workspace_root();
    let role_path = workspace.join("crates/canic/src/macros/endpoints/role.rs");
    let role = fs::read_to_string(&role_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", role_path.display()));
    let managed_status = role
        .split("macro_rules! __canic_emit_managed_status_endpoint")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! __canic_emit_local_status_endpoint")
                .next()
        })
        .expect("managed status emitter");
    let managed_command = role
        .split("macro_rules! __canic_emit_managed_command_endpoint")
        .nth(1)
        .expect("managed command emitter");
    let local_status = role
        .split("macro_rules! __canic_emit_local_status_endpoint")
        .nth(1)
        .and_then(|rest| {
            rest.split("macro_rules! __canic_emit_managed_command_endpoint")
                .next()
        })
        .expect("standalone-local status emitter");

    assert!(
        managed_status.contains(
            "#[cfg(canic_capability_fleet_admission_projection)]\n            Admission("
        ) && managed_status.contains("CanisterStatusRequest::Admission"),
        "an explicitly enrolled managed role must expose the local Fleet-admission projection"
    );
    for variant in [
        "ActivateFleetAdmission(",
        "OpenFleetAdmission(",
        "PrepareFleetAdmission(",
    ] {
        let position = managed_command
            .find(variant)
            .unwrap_or_else(|| panic!("managed admission command variant {variant}"));
        let prefix = &managed_command[..position];
        assert!(
            prefix.ends_with("#[cfg(canic_capability_fleet_admission_projection)]\n            "),
            "managed admission command variant {variant} must be role-pruned"
        );
    }
    let auth = managed_status
        .find("is_controller_or_root(caller)")
        .expect("controller-or-Root authorization");
    let dispatch = managed_status
        .find("FleetAdmissionProjectionApi::status")
        .expect("Fleet-admission projection facade dispatch");
    assert!(
        auth < dispatch,
        "managed status must authorize before projection state dispatch"
    );
    assert!(
        !managed_command.contains("RuntimeWhitelist")
            && !managed_command.contains("runtime_whitelist"),
        "removed local whitelist mutation authority must not survive"
    );
    assert!(
        !local_status.contains("FleetAdmissionProjection")
            && !local_status.contains("CanisterStatusRequest::Admission"),
        "standalone-local status must not expose managed Fleet-admission state"
    );

    for relative in [
        "crates/canic/src/macros/endpoints/root.rs",
        "crates/canic/src/macros/endpoints/fleet_coordinator.rs",
        "crates/canic/src/macros/endpoints/wasm_store.rs",
    ] {
        let source = fs::read_to_string(workspace.join(relative))
            .unwrap_or_else(|error| panic!("read {relative}: {error}"));
        assert!(
            !source.contains("FleetAdmissionProjectionApi"),
            "specialized infrastructure surface must not expose target-local projection: {relative}"
        );
    }
}

#[test]
fn managed_start_remains_a_thin_profile_surface_composer() {
    let workspace = workspace_root();
    let start_path = workspace.join("crates/canic/src/macros/start.rs");
    let source = fs::read_to_string(&start_path)
        .unwrap_or_else(|error| panic!("read {}: {error}", start_path.display()));
    let managed_start = source
        .split("macro_rules! start")
        .nth(1)
        .and_then(|rest| rest.split("macro_rules! start_local").next())
        .expect("managed start macro");

    for emitter in [
        "__canic_root_lifecycle_core!",
        "__canic_start_nonroot_lifecycle_core!",
        "__canic_start_ingress_payload_inspect!",
        "__canic_emit_managed_command_endpoint!",
        "__canic_emit_managed_status_endpoint!",
        "canic_bundle_root_only_endpoints!",
    ] {
        assert!(
            managed_start.contains(emitter),
            "managed start macro must compose {emitter}"
        );
    }
    assert!(
        !managed_start.contains("workflow::")
            && !managed_start.contains(".await")
            && !managed_start.contains("fn canic_status")
            && !managed_start.contains("fn canic_command"),
        "start! must compose lifecycle and role emitters without owning orchestration or protocol handlers"
    );
}
