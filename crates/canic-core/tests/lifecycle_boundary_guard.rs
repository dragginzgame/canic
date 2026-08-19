// Category C - System-level artifact test (no embedded config).

use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn before_bootstrap_lifecycle_adapters_remain_synchronous() {
    let mut violations = Vec::new();

    for adapter in BEFORE_BOOTSTRAP_ADAPTERS {
        let source = read_source(adapter.path);
        let body = function_body(&source, adapter.function);

        for forbidden in FORBIDDEN_BEFORE_BOOTSTRAP_FRAGMENTS {
            if body.contains(forbidden) {
                violations.push(format!(
                    "{}::{} contains forbidden lifecycle-before-bootstrap fragment `{forbidden}`",
                    adapter.path, adapter.function
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "lifecycle before-bootstrap boundary changed: {violations:?}"
    );
}

#[test]
fn async_lifecycle_bootstrap_stays_in_zero_delay_schedule_helpers() {
    let mut violations = Vec::new();

    for helper in SCHEDULE_HELPERS {
        let source = read_source(helper.path);
        let body = function_body(&source, helper.function);

        for required in helper.required_fragments {
            if !body.contains(required) {
                violations.push(format!(
                    "{}::{} is missing lifecycle scheduling fragment `{required}`",
                    helper.path, helper.function
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "lifecycle async bootstrap scheduling boundary changed: {violations:?}"
    );
}

#[test]
fn nonroot_init_arguments_reach_only_the_application_hook() {
    let source = read_source("crates/canic/src/macros/start.rs");

    assert_eq!(
        source.matches("schedule_init_nonroot_bootstrap();").count(),
        3,
        "all three non-root start paths must schedule argument-free internal bootstrap"
    );
    assert_eq!(
        source.matches("canic_install(args).await;").count(),
        3,
        "all three non-root start paths must preserve application init arguments"
    );
}

#[test]
fn root_init_stays_prepared_without_scheduling_bootstrap_or_application_hooks() {
    let source = read_source("crates/canic/src/macros/start.rs");
    let body = macro_section(
        &source,
        "macro_rules! __canic_root_lifecycle_core",
        "// Run the optional init block from a lifecycle timer",
    );
    let init = function_body(body, "init");

    assert!(
        body.contains("fn init(args: ::canic::dto::fleet_subnet_root::FleetSubnetRootInitArgs)"),
        "root init must accept the exact Fleet Subnet Root authority"
    );
    assert!(
        body.contains("let _ = canic_install;"),
        "root lifecycle must retain the application install-hook contract without executing it"
    );
    for forbidden in [
        "TimerApi::defer_lifecycle",
        "canic_setup().await",
        "canic_install().await",
    ] {
        assert!(
            !init.contains(forbidden),
            "Prepared root init must not schedule `{forbidden}`"
        );
    }
}

#[test]
fn root_post_upgrade_schedules_services_and_hooks_only_when_active() {
    let macro_source = read_source("crates/canic/src/macros/start.rs");
    let root = macro_section(
        &macro_source,
        "macro_rules! __canic_root_lifecycle_core",
        "// Run the optional init block from a lifecycle timer",
    );
    let post_upgrade = function_body(root, "post_upgrade");
    assert!(
        post_upgrade.contains("let active =")
            && post_upgrade.contains("if active {")
            && post_upgrade.contains("schedule_post_upgrade_root_bootstrap();")
            && post_upgrade.contains("canic_upgrade().await;"),
        "root post-upgrade must gate bootstrap and application hooks on Active"
    );

    let runtime_source = read_source("crates/canic-core/src/workflow/runtime/root.rs");
    let runtime = function_body(
        &runtime_source,
        "post_upgrade_root_canister_after_memory_init",
    );
    assert!(
        runtime.contains("FleetActivationOps::status(true)")
            && runtime.contains("TimerAuthorityWorkflow::is_suspended()")
            && runtime.contains("if active && !timers_suspended {")
            && runtime.contains("RuntimeWorkflow::start_all_root()")
            && runtime.contains("Ok(active && !timers_suspended)"),
        "root runtime restoration must gate service startup on protected Active and unsealed state"
    );
}

#[test]
fn automatic_topup_reachability_requires_the_compiled_capability() {
    let start = read_source("crates/canic/src/macros/start.rs");
    let nonroot = macro_section(
        &start,
        "macro_rules! __canic_start_nonroot_lifecycle_core",
        "// Lifecycle core for the host-installed sibling Wasm Store.",
    );
    let local = macro_section(
        &start,
        "macro_rules! __canic_start_local_lifecycle_core",
        "// Lifecycle core for the root Canic canister.",
    );
    for lifecycle in [nonroot, local] {
        assert!(
            lifecycle.contains("#[cfg(canic_capability_automatic_topup)]")
                && lifecycle.contains("#[cfg(not(canic_capability_automatic_topup))]"),
            "non-root lifecycle must select its runtime owner from the compiled capability"
        );
    }

    let endpoints = read_source("crates/canic/src/macros/endpoints/role.rs");
    assert!(
        endpoints.contains("LifecycleApi::configure_component_runtime_with_automatic_topup")
            && endpoints.contains("ComponentRuntimeApi::configure;"),
        "managed activation must select the exact capability-pruned runtime path"
    );

    let runtime = read_source("crates/canic-core/src/workflow/runtime/mod.rs");
    assert!(!function_body(&runtime, "start_all").contains("CycleWorkflow"));
    assert!(function_body(&runtime, "start_all_with_automatic_topup").contains("CycleWorkflow"));
    assert!(!function_body(&runtime, "start_all_root").contains("CycleWorkflow"));

    let timer = read_source("crates/canic-core/src/workflow/runtime/timer/mod.rs");
    assert!(!function_body(&timer, "suspend_root").contains("CycleWorkflow"));
    assert!(!function_body(&timer, "recover_expired_async_jobs").contains("CycleWorkflow"));
    assert!(
        function_body(&timer, "recover_expired_async_jobs_with_automatic_topup")
            .contains("CycleWorkflow")
    );

    let topology = read_source("crates/canic-core/src/workflow/cascade/topology.rs");
    assert!(
        !topology.contains("CycleWorkflow"),
        "topology linkage alone must not grant automatic top-up custody"
    );
}

#[test]
fn lifecycle_participant_is_paired_safe_and_ordered_before_deferred_work() {
    let source = read_source("crates/canic/src/macros/start.rs");
    let nonroot = macro_section(
        &source,
        "macro_rules! __canic_start_nonroot_lifecycle_core",
        "// Lifecycle core for the host-installed sibling Wasm Store.",
    );
    let local = macro_section(
        &source,
        "macro_rules! __canic_start_local_lifecycle_core",
        "// Lifecycle core for the root Canic canister.",
    );
    let root = macro_section(
        &source,
        "macro_rules! __canic_root_lifecycle_core",
        "// Run the optional init block from a lifecycle timer",
    );

    assert_lifecycle_participant_grammar(&source, nonroot, local, root);
    assert_lifecycle_participant_ordering(nonroot, local, root);
    assert_specialized_start_macros_reject_participants(&source);
}

fn assert_lifecycle_participant_grammar(source: &str, nonroot: &str, local: &str, root: &str) {
    for (name, section) in [("managed", nonroot), ("local", local), ("Root", root)] {
        assert!(
            section.contains("init = $lifecycle_init:path,")
                && section.contains("post_upgrade = $lifecycle_post_upgrade:path,")
                && section.contains("__canic_typecheck_lifecycle_participant_pair!"),
            "{name} lifecycle must accept one paired path declaration and coerce both paths to safe fn() -> ()"
        );
    }
    let typecheck = macro_section(
        source,
        "macro_rules! __canic_typecheck_lifecycle_participant_pair",
        "// Lifecycle core for non-root Canic canisters.",
    );
    assert_eq!(
        typecheck.matches("let _: fn() -> ()").count(),
        2,
        "the shared destination-crate type check must require two safe synchronous functions"
    );

    for (name, section) in [
        (
            "managed/Root",
            macro_section(
                source,
                "macro_rules! start",
                "/// Configure a local-only non-root Canic canister",
            ),
        ),
        (
            "local",
            macro_section(
                source,
                "macro_rules! start_local",
                "/// Configure lifecycle hooks and the canonical endpoint bundle",
            ),
        ),
    ] {
        let matcher = section
            .split("$crate::__canic_require_finish!();")
            .next()
            .expect("public start matcher");
        assert_eq!(
            matcher.matches("lifecycle_participant(").count(),
            1,
            "{name} start grammar must accept at most one lifecycle participant pair"
        );
        assert!(
            matcher.contains("init = $lifecycle_init:path,")
                && matcher.contains("post_upgrade = $lifecycle_post_upgrade:path"),
            "{name} start grammar must reject closures and partial participant declarations"
        );
    }
}

fn assert_lifecycle_participant_ordering(nonroot: &str, local: &str, root: &str) {
    assert_ordered(
        function_body(nonroot, "init"),
        &[
            "init_nonroot_canister_before_bootstrap(",
            "$(($lifecycle_init)();)?",
        ],
        "managed init participant ordering",
    );
    assert_ordered(
        function_body(nonroot, "post_upgrade"),
        &[
            "let active = restore_runtime(",
            "$(($lifecycle_post_upgrade)();)?",
            "if active {",
        ],
        "managed post-upgrade participant ordering",
    );
    assert_ordered(
        function_body(local, "init"),
        &[
            "initialize_runtime(",
            "$(($lifecycle_init)();)?",
            "$crate::__canic_after_optional_start_init_hook!",
        ],
        "local init participant ordering",
    );
    assert_ordered(
        function_body(local, "post_upgrade"),
        &[
            "let _active = restore_runtime(",
            "$(($lifecycle_post_upgrade)();)?",
            "$crate::__canic_after_optional_start_init_hook!",
        ],
        "local post-upgrade participant ordering",
    );
    assert_ordered(
        function_body(root, "init"),
        &[
            "init_root_canister_before_bootstrap(",
            "$(($lifecycle_init)();)?",
        ],
        "Root init participant ordering",
    );
    assert_ordered(
        function_body(root, "post_upgrade"),
        &[
            "post_upgrade_root_canister_before_bootstrap(",
            "$(($lifecycle_post_upgrade)();)?",
            "if active {",
        ],
        "Root post-upgrade participant ordering",
    );
}

fn assert_specialized_start_macros_reject_participants(source: &str) {
    let wasm_store = macro_section(
        source,
        "macro_rules! start_wasm_store",
        "/// Configure the dedicated built-in Fleet Coordinator canister surface.",
    );
    let coordinator = &source[source
        .find("macro_rules! start_fleet_coordinator")
        .expect("Fleet Coordinator start macro")..];
    assert!(
        !wasm_store.contains("lifecycle_participant")
            && !coordinator.contains("lifecycle_participant"),
        "specialized infrastructure start macros must reject application lifecycle participants"
    );
}

struct FunctionRef {
    path: &'static str,
    function: &'static str,
}

struct ScheduleHelper {
    path: &'static str,
    function: &'static str,
    required_fragments: &'static [&'static str],
}

const BEFORE_BOOTSTRAP_ADAPTERS: &[FunctionRef] = &[
    FunctionRef {
        path: "crates/canic-core/src/lifecycle/init/root.rs",
        function: "init_root_canister_before_bootstrap",
    },
    FunctionRef {
        path: "crates/canic-core/src/lifecycle/init/nonroot.rs",
        function: "init_nonroot_canister_before_bootstrap",
    },
    FunctionRef {
        path: "crates/canic-core/src/lifecycle/upgrade/root.rs",
        function: "post_upgrade_root_canister_before_bootstrap",
    },
    FunctionRef {
        path: "crates/canic-core/src/lifecycle/upgrade/nonroot.rs",
        function: "post_upgrade_nonroot_canister_before_bootstrap",
    },
    FunctionRef {
        path: "crates/canic-control-plane/src/api/lifecycle.rs",
        function: "init_root_canister_before_bootstrap",
    },
    FunctionRef {
        path: "crates/canic-control-plane/src/api/lifecycle.rs",
        function: "post_upgrade_root_canister_before_bootstrap",
    },
];

const FORBIDDEN_BEFORE_BOOTSTRAP_FRAGMENTS: &[&str] = &[
    ".await",
    "async {",
    "async move",
    "TimerOps::set",
    "TimerWorkflow::set",
    "TimerApi::defer_lifecycle",
    "defer_lifecycle",
    "workflow::bootstrap",
    "schedule_",
];

const SCHEDULE_HELPERS: &[ScheduleHelper] = &[
    ScheduleHelper {
        path: "crates/canic-core/src/lifecycle/init/nonroot.rs",
        function: "schedule_init_nonroot_bootstrap",
        required_fragments: &[
            "Duration::ZERO",
            "TimerApi::defer_lifecycle_required",
            "canic:bootstrap:init_nonroot_canister",
            "bootstrap_init_nonroot_canister().await",
        ],
    },
    ScheduleHelper {
        path: "crates/canic-core/src/lifecycle/upgrade/nonroot.rs",
        function: "schedule_post_upgrade_nonroot_bootstrap",
        required_fragments: &[
            "Duration::ZERO",
            "TimerApi::defer_lifecycle_required",
            "canic:bootstrap:post_upgrade_nonroot_canister",
            "bootstrap_post_upgrade_nonroot_canister().await",
        ],
    },
    ScheduleHelper {
        path: "crates/canic-control-plane/src/api/lifecycle.rs",
        function: "schedule_post_upgrade_root_bootstrap",
        required_fragments: &[
            "Duration::ZERO",
            "TimerApi::defer_lifecycle_required",
            "canic:bootstrap:post_upgrade_root_canister",
            "bootstrap_post_upgrade_root_canister().await",
        ],
    },
];

fn read_source(relative_path: &str) -> String {
    let path = workspace_root().join(relative_path);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn function_body<'a>(source: &'a str, function: &str) -> &'a str {
    let signature = format!("fn {function}");
    let start = source
        .find(&signature)
        .unwrap_or_else(|| panic!("source should contain `{signature}`"));
    let body_start = source[start..].find('{').map_or_else(
        || panic!("`{signature}` should have a body"),
        |offset| start + offset,
    );

    let mut depth = 0usize;
    for (offset, ch) in source[body_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth
                    .checked_sub(1)
                    .unwrap_or_else(|| panic!("unbalanced braces in `{signature}`"));
                if depth == 0 {
                    return &source[body_start..=body_start + offset];
                }
            }
            _ => {}
        }
    }

    panic!("`{signature}` body should close")
}

fn macro_section<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    let start = source.find(start).expect("macro section start");
    let end = source[start..]
        .find(end)
        .map(|offset| start + offset)
        .expect("macro section end");
    &source[start..end]
}

fn assert_ordered(source: &str, fragments: &[&str], context: &str) {
    let mut cursor = 0usize;
    for fragment in fragments {
        let offset = source[cursor..]
            .find(fragment)
            .unwrap_or_else(|| panic!("{context} is missing `{fragment}`"));
        cursor = cursor.saturating_add(offset).saturating_add(fragment.len());
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("workspace root")
}
