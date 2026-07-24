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
        2,
        "both non-root start paths must schedule argument-free internal bootstrap"
    );
    assert_eq!(
        source.matches("canic_install(args).await;").count(),
        2,
        "both non-root start paths must preserve application init arguments"
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
        body.contains(
            "fn init(identity: ::canic::dto::fleet_activation::CurrentRootInstallIdentity)"
        ),
        "root init must accept the exact current install identity"
    );
    assert!(
        body.contains("let _ = canic_install;"),
        "root lifecycle must retain the application install-hook contract without executing it"
    );
    for forbidden in [
        "schedule_init_root_bootstrap",
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
            "TimerWorkflow::set_application_once",
            "canic:bootstrap:init_nonroot_canister",
            "bootstrap_init_nonroot_canister().await",
        ],
    },
    ScheduleHelper {
        path: "crates/canic-core/src/lifecycle/upgrade/nonroot.rs",
        function: "schedule_post_upgrade_nonroot_bootstrap",
        required_fragments: &[
            "Duration::ZERO",
            "TimerWorkflow::set_application_once",
            "canic:bootstrap:post_upgrade_nonroot_canister",
            "bootstrap_post_upgrade_nonroot_canister().await",
        ],
    },
    ScheduleHelper {
        path: "crates/canic-control-plane/src/api/lifecycle.rs",
        function: "schedule_init_root_bootstrap",
        required_fragments: &[
            "FleetActivationApi::require_active()?",
            "Duration::ZERO",
            "TimerApi::defer_lifecycle",
            "canic:bootstrap:init_root_canister",
            "bootstrap_init_root_canister().await",
        ],
    },
    ScheduleHelper {
        path: "crates/canic-control-plane/src/api/lifecycle.rs",
        function: "schedule_post_upgrade_root_bootstrap",
        required_fragments: &[
            "Duration::ZERO",
            "TimerApi::defer_lifecycle",
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

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("workspace root")
}
