// Category C - System-level artifact test (no embedded config).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const PRODUCTION_SOURCE_ROOTS: [&str; 3] = ["apps", "canisters", "crates"];
const TIMER_OWNERSHIP_SIGNALS: [&str; 10] = [
    "AsyncJobOwner",
    "AsyncJobRecovery",
    "AsyncJobWorkflow",
    "CanisterTimerStatus",
    "CoreAsyncJobRecovery",
    "TimerApi",
    "TimerAuthorityWorkflow",
    "TimerExecutionOutcome",
    "async_job_recovery",
    "ic_timers",
];
const PROHIBITED_AUTHORITY_FRAGMENTS: [&str; 10] = [
    "ClaimKey::Transient",
    "NEXT_TRANSIENT_ID",
    "TimerClaimId",
    "TimerKey",
    "TimerWorkflow",
    "cdk::timers",
    "enum TimerClaim",
    "ic_cdk_timers",
    "static CLAIMS",
    "struct TimerHandle",
];
const NATIVE_REGISTRATION_FRAGMENTS: [&str; 6] = [
    "register_after_completion(",
    "register_once(",
    "reconcile_after_completion(",
    "reconcile_once(",
    "reconcile_watchdog(",
    "Registration>",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OwnershipClass {
    DomainAsyncJobRecovery,
    DtoOrMetricsProjection,
    FixedCanicConsumer,
    IndependentApplicationCustody,
    NativeRegistrationCustody,
    PrivateLifecycleConsumer,
    ProhibitedSchedulingAuthority,
}

impl OwnershipClass {
    const fn label(self) -> &'static str {
        match self {
            Self::DomainAsyncJobRecovery => "domain async-job recovery",
            Self::DtoOrMetricsProjection => "DTO/metrics projection",
            Self::FixedCanicConsumer => "fixed Canic consumer",
            Self::IndependentApplicationCustody => "independent application custody",
            Self::NativeRegistrationCustody => "native registration custody",
            Self::PrivateLifecycleConsumer => "private lifecycle consumer",
            Self::ProhibitedSchedulingAuthority => "prohibited scheduling authority",
        }
    }
}

#[test]
fn timer_ownership_inventory_is_semantically_classified() {
    let root = workspace_root();
    let expected = expected_ownership_inventory();
    let mut observed = BTreeSet::new();

    for source_root in PRODUCTION_SOURCE_ROOTS {
        collect_rust_sources(&root.join(source_root), &root, &mut |path, source| {
            if !excluded_test_source(path)
                && TIMER_OWNERSHIP_SIGNALS
                    .iter()
                    .any(|signal| source.contains(signal))
            {
                observed.insert(path.to_string());
            }
        });
    }

    let classified = expected.keys().copied().map(str::to_string).collect();
    assert_eq!(observed, classified, "timer ownership inventory changed");
    let prohibited = OwnershipClass::ProhibitedSchedulingAuthority;
    assert!(expected.values().all(|class| *class != prohibited));
    let documented_source = read_source(
        &root,
        "docs/audits/working/0.104-timer-ownership/consumer-inventory.tsv",
    );
    let documented = documented_ownership_inventory(&documented_source);
    let expected_documented = expected
        .iter()
        .map(|(path, class)| (path.to_string(), class.label()))
        .collect();
    assert_eq!(documented, expected_documented);

    for (path, class) in expected {
        let source = read_source(&root, path);
        assert_semantic_class(path, source.as_str(), class);
    }
}

#[test]
fn timer_provider_graph_and_manifest_consumers_are_closed() {
    let root = workspace_root();
    let lock = read_source(&root, "Cargo.lock");

    assert_eq!(locked_package_versions(&lock, "ic-timers"), ["0.6.1"]);
    assert_eq!(locked_package_versions(&lock, "ic-cdk-timers"), ["1.0.0"]);
    assert_eq!(locked_package_versions(&lock, "icydb"), ["0.230.2"]);

    let workspace_manifest = read_source(&root, "Cargo.toml");
    assert!(workspace_manifest.contains("ic-timers = \"=0.6.1\""));
    assert!(workspace_manifest.contains("icydb = { version = \"=0.230.2\""));
    assert!(!workspace_manifest.contains("ic-cdk-timers ="));

    let mut timer_consumers = BTreeSet::from(["Cargo.toml".to_string()]);
    let mut raw_provider_consumers = BTreeSet::new();
    for source_root in PRODUCTION_SOURCE_ROOTS {
        collect_named_files(
            &root.join(source_root),
            &root,
            "Cargo.toml",
            &mut |path, manifest| {
                if manifest
                    .lines()
                    .any(|line| line.trim_start().starts_with("ic-timers ="))
                {
                    timer_consumers.insert(path.to_string());
                }
                if manifest
                    .lines()
                    .any(|line| line.trim_start().starts_with("ic-cdk-timers ="))
                {
                    raw_provider_consumers.insert(path.to_string());
                }
            },
        );
    }

    assert_eq!(timer_consumers, expected_timer_manifest_consumers());
    assert!(raw_provider_consumers.is_empty());
}

#[test]
fn maintained_runtime_docs_do_not_advertise_the_removed_timer_facade() {
    let root = workspace_root();

    for path in [
        "crates/canic/README.md",
        "docs/features/runtime/README.md",
        "docs/features/runtime/native-timers.md",
    ] {
        let source = read_source(&root, path);
        for forbidden in [
            "TimerApi::cancel",
            "TimerApi::set",
            "canic::timer!(",
            "canic::timer_interval!(",
            "use canic::api::timer",
        ] {
            assert!(
                !source.contains(forbidden),
                "maintained runtime document {path} advertises removed facade `{forbidden}`"
            );
        }
    }
}

#[test]
fn direct_raw_timer_provider_access_is_absent_from_production() {
    let root = workspace_root();
    let mut violations = Vec::new();

    for source_root in PRODUCTION_SOURCE_ROOTS {
        collect_rust_sources(&root.join(source_root), &root, &mut |path, source| {
            if excluded_test_source(path) {
                return;
            }
            for forbidden in ["ic_cdk_timers", "cdk::timers"] {
                if source.contains(forbidden) {
                    violations.push(format!("{path}: {forbidden}"));
                }
            }
        });
    }

    assert!(
        violations.is_empty(),
        "production code bypasses the shared timer provider: {violations:?}"
    );
}

#[test]
fn timed_host_wait_inventory_remains_explicit() {
    let root = workspace_root();
    let mut waits = BTreeMap::new();

    for source_root in PRODUCTION_SOURCE_ROOTS {
        collect_rust_sources(&root.join(source_root), &root, &mut |path, source| {
            if excluded_test_source(path) {
                return;
            }

            let wait_count = [
                "thread::sleep(",
                "recv_timeout(",
                "park_timeout(",
                "sleep_until(",
                "tokio::time::sleep(",
            ]
            .into_iter()
            .map(|fragment| source.matches(fragment).count())
            .sum();
            if wait_count > 0 {
                waits.insert(path.to_string(), wait_count);
            }
        });
    }

    assert_eq!(waits, expected_wait_inventory());
}

#[test]
fn pool_and_snapshot_paths_use_exact_native_owners() {
    let root = workspace_root();
    let timer = read_source(&root, "crates/canic-core/src/workflow/runtime/timer/mod.rs");
    let timer_api = read_source(&root, "crates/canic-core/src/api/timer.rs");
    let pool = read_source(
        &root,
        "crates/canic-control-plane/src/workflow/canister_pool/mod.rs",
    );
    let lifecycle = read_source(&root, "crates/canic-control-plane/src/api/lifecycle.rs");
    let coordinator = read_source(
        &root,
        "crates/canic-control-plane/src/api/fleet_coordinator.rs",
    );
    let authority = read_source(
        &root,
        "crates/canic-core/src/workflow/runtime/authority_restore.rs",
    );

    for forbidden in [
        "static CLAIMS",
        "enum TimerClaim",
        "register_snapshot_resume_participant",
        "register_async_job_recovery_participant",
    ] {
        assert!(!timer.contains(forbidden));
        assert!(!timer_api.contains(forbidden));
    }
    for required in [
        "static MAINTENANCE_TIMER: RefCell<Option<AfterCompletionRegistration>>",
        "static RECOVERY_WATCHDOG: RefCell<Option<WatchdogRegistration>>",
        "reconcile_after_completion(",
        "reconcile_watchdog(",
    ] {
        assert!(
            pool.contains(required),
            "Root pool custody lacks `{required}`"
        );
    }
    assert!(
        lifecycle.contains("canister_pool::suspend_for_authority_snapshot()")
            && lifecycle.contains("canister_pool::resume_after_authority_snapshot()")
            && lifecycle.contains("AuthorityRestoreApi::prepare_root_snapshot(")
            && lifecycle.contains("AuthorityRestoreApi::resume_root_snapshot("),
        "Root snapshot lifecycle must invoke its exact timer owner"
    );
    assert!(
        coordinator.contains("AuthorityRestoreApi::prepare_coordinator_snapshot(")
            && coordinator.contains("AuthorityRestoreApi::resume_coordinator_snapshot("),
        "Coordinator snapshot lifecycle must not link Root timer owners"
    );
    for required in [
        "TimerAuthorityWorkflow::suspend_root",
        "TimerAuthorityWorkflow::suspend_coordinator",
        "TimerAuthorityWorkflow::resume_root",
        "TimerAuthorityWorkflow::resume_coordinator",
    ] {
        assert!(
            authority.contains(required),
            "authority restore lacks exact role path `{required}`"
        );
    }
}

fn assert_semantic_class(path: &str, source: &str, class: OwnershipClass) {
    for forbidden in PROHIBITED_AUTHORITY_FRAGMENTS {
        assert!(
            !source.contains(forbidden),
            "{path} contains prohibited scheduling authority `{forbidden}`"
        );
    }

    match class {
        OwnershipClass::DomainAsyncJobRecovery => {
            for forbidden in [
                "AfterCompletionRegistration",
                "OnceRegistration",
                "TimerIdentity",
                "TimerRegistrationStatus",
                "TimerSchedule",
                "TimerSnapshot",
                "WatchdogRegistration",
                "register_after_completion(",
                "register_once(",
                "reconcile_watchdog(",
            ] {
                assert!(
                    !source.contains(forbidden),
                    "domain recovery file {path} owns provider state `{forbidden}`"
                );
            }
        }
        OwnershipClass::DtoOrMetricsProjection => {
            for forbidden in NATIVE_REGISTRATION_FRAGMENTS {
                assert!(
                    !source.contains(forbidden),
                    "projection file {path} owns registration capability `{forbidden}`"
                );
            }
        }
        OwnershipClass::FixedCanicConsumer => {
            assert!(
                source.contains("Registration") || source.contains("timer_identity"),
                "fixed Canic consumer {path} lacks named registration custody"
            );
        }
        OwnershipClass::IndependentApplicationCustody => {
            assert!(
                source.contains("ic_timers"),
                "independent application {path} does not use the shared provider"
            );
            assert!(
                !source.contains("TimerIdentity::try_new(\"canic\""),
                "independent application {path} claims the Canic timer owner"
            );
        }
        OwnershipClass::NativeRegistrationCustody => {
            assert!(
                NATIVE_REGISTRATION_FRAGMENTS
                    .iter()
                    .any(|fragment| source.contains(fragment)),
                "native custody file {path} lacks a native registration capability"
            );
        }
        OwnershipClass::PrivateLifecycleConsumer => {
            for forbidden in NATIVE_REGISTRATION_FRAGMENTS {
                assert!(
                    !source.contains(forbidden),
                    "private lifecycle consumer {path} retains native custody `{forbidden}`"
                );
            }
        }
        OwnershipClass::ProhibitedSchedulingAuthority => {
            panic!("{path} remains a prohibited scheduling authority")
        }
    }
}

fn expected_ownership_inventory() -> BTreeMap<&'static str, OwnershipClass> {
    application_ownership()
        .into_iter()
        .chain(control_plane_ownership())
        .chain(core_boundary_ownership())
        .chain(core_recovery_ownership())
        .chain(core_consumer_ownership())
        .chain(facade_ownership())
        .collect()
}

const fn application_ownership() -> [(&'static str, OwnershipClass); 7] {
    use OwnershipClass::{
        DtoOrMetricsProjection as Projection, IndependentApplicationCustody as Application,
        PrivateLifecycleConsumer as Lifecycle,
    };

    [
        ("apps/saltz/burner/src/lib.rs", Application),
        ("apps/test/test/src/lib.rs", Application),
        (
            "canisters/test/canic_icydb_lifecycle_probe/src/lib.rs",
            Application,
        ),
        (
            "canisters/test/delegation_root_stub/src/lib.rs",
            Application,
        ),
        ("canisters/test/intent_authority/src/lib.rs", Lifecycle),
        ("canisters/test/runtime_probe/src/lib.rs", Application),
        ("crates/canic-cli/src/inspect/mod.rs", Projection),
    ]
}

const fn control_plane_ownership() -> [(&'static str, OwnershipClass); 8] {
    use OwnershipClass::{
        NativeRegistrationCustody as Custody, PrivateLifecycleConsumer as Lifecycle,
    };

    [
        (
            "crates/canic-control-plane/src/api/fleet_coordinator.rs",
            Lifecycle,
        ),
        ("crates/canic-control-plane/src/api/lifecycle.rs", Lifecycle),
        (
            "crates/canic-control-plane/src/workflow/canister_pool/mod.rs",
            Custody,
        ),
        (
            "crates/canic-control-plane/src/workflow/component_provisioning.rs",
            Lifecycle,
        ),
        (
            "crates/canic-control-plane/src/workflow/component_registry/mod.rs",
            Lifecycle,
        ),
        (
            "crates/canic-control-plane/src/workflow/fleet_coordinator/mod.rs",
            Lifecycle,
        ),
        (
            "crates/canic-control-plane/src/workflow/fleet_registry_mirror/mod.rs",
            Lifecycle,
        ),
        (
            "crates/canic-control-plane/src/workflow/fleet_subnet_root.rs",
            Lifecycle,
        ),
    ]
}

const fn core_boundary_ownership() -> [(&'static str, OwnershipClass); 10] {
    use OwnershipClass::{
        DomainAsyncJobRecovery as Recovery, DtoOrMetricsProjection as Projection,
        PrivateLifecycleConsumer as Lifecycle,
    };

    [
        ("crates/canic-core/src/api/runtime/mod.rs", Projection),
        ("crates/canic-core/src/api/timer.rs", Lifecycle),
        ("crates/canic-core/src/control_plane_support.rs", Recovery),
        ("crates/canic-core/src/domain/runtime.rs", Projection),
        ("crates/canic-core/src/dto/runtime.rs", Projection),
        ("crates/canic-core/src/lifecycle/init/nonroot.rs", Lifecycle),
        ("crates/canic-core/src/lifecycle/init/root.rs", Lifecycle),
        (
            "crates/canic-core/src/lifecycle/upgrade/nonroot.rs",
            Lifecycle,
        ),
        ("crates/canic-core/src/lifecycle/upgrade/root.rs", Lifecycle),
        (
            "crates/canic-core/src/ops/runtime/metrics/mod.rs",
            Projection,
        ),
    ]
}

const fn core_recovery_ownership() -> [(&'static str, OwnershipClass); 9] {
    use OwnershipClass::DomainAsyncJobRecovery as Recovery;

    [
        (
            "crates/canic-core/src/ops/storage/async_job_recovery/mod.rs",
            Recovery,
        ),
        ("crates/canic-core/src/ops/storage/mod.rs", Recovery),
        (
            "crates/canic-core/src/role_contract/allocation.rs",
            Recovery,
        ),
        ("crates/canic-core/src/role_contract/catalog.rs", Recovery),
        ("crates/canic-core/src/role_contract/model.rs", Recovery),
        ("crates/canic-core/src/state_contract.rs", Recovery),
        (
            "crates/canic-core/src/storage/stable/async_job_recovery/mod.rs",
            Recovery,
        ),
        ("crates/canic-core/src/storage/stable/mod.rs", Recovery),
        (
            "crates/canic-core/src/workflow/runtime/async_job/mod.rs",
            Recovery,
        ),
    ]
}

const fn core_consumer_ownership() -> [(&'static str, OwnershipClass); 9] {
    use OwnershipClass::{
        FixedCanicConsumer as Fixed, NativeRegistrationCustody as Custody,
        PrivateLifecycleConsumer as Lifecycle,
    };

    [
        (
            "crates/canic-core/src/workflow/placement/acknowledgement.rs",
            Fixed,
        ),
        ("crates/canic-core/src/workflow/runtime/auth/mod.rs", Fixed),
        (
            "crates/canic-core/src/workflow/runtime/auth/renewal.rs",
            Fixed,
        ),
        (
            "crates/canic-core/src/workflow/runtime/authority_restore.rs",
            Lifecycle,
        ),
        (
            "crates/canic-core/src/workflow/runtime/cycles/mod.rs",
            Fixed,
        ),
        ("crates/canic-core/src/workflow/runtime/intent.rs", Fixed),
        ("crates/canic-core/src/workflow/runtime/log.rs", Fixed),
        ("crates/canic-core/src/workflow/runtime/root.rs", Lifecycle),
        (
            "crates/canic-core/src/workflow/runtime/timer/mod.rs",
            Custody,
        ),
    ]
}

const fn facade_ownership() -> [(&'static str, OwnershipClass); 2] {
    use OwnershipClass::PrivateLifecycleConsumer as Lifecycle;

    [
        ("crates/canic/src/macros/endpoints/wasm_store.rs", Lifecycle),
        ("crates/canic/src/macros/start.rs", Lifecycle),
    ]
}

fn expected_timer_manifest_consumers() -> BTreeSet<String> {
    [
        "Cargo.toml",
        "apps/saltz/burner/Cargo.toml",
        "apps/test/test/Cargo.toml",
        "canisters/test/canic_icydb_lifecycle_probe/Cargo.toml",
        "canisters/test/delegation_root_stub/Cargo.toml",
        "canisters/test/runtime_probe/Cargo.toml",
        "crates/canic-control-plane/Cargo.toml",
        "crates/canic-core/Cargo.toml",
        "crates/canic-tests/Cargo.toml",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn documented_ownership_inventory(source: &str) -> BTreeMap<String, &str> {
    source
        .lines()
        .skip(1)
        .map(|line| {
            let mut columns = line.split('\t');
            let path = columns.next().expect("inventory path").to_string();
            let class = columns.next().expect("inventory ownership class");
            assert!(columns.next().is_some(), "inventory current authority");
            assert!(columns.next().is_some(), "inventory closeout");
            assert!(columns.next().is_none(), "unexpected inventory column");
            (path, class)
        })
        .collect()
}

fn expected_wait_inventory() -> BTreeMap<String, usize> {
    BTreeMap::from([
        (
            "crates/canic-backup/src/persistence/command_lifetime_lock/mod.rs".to_string(),
            4,
        ),
        (
            "crates/canic-backup/src/persistence/journal_lock/mod.rs".to_string(),
            2,
        ),
        (
            "crates/canic-host/src/canister_build/cache.rs".to_string(),
            2,
        ),
        ("crates/canic-host/src/icp/command.rs".to_string(), 1),
        ("crates/canic-host/src/terminal/activity.rs".to_string(), 1),
    ])
}

fn locked_package_versions<'a>(lock: &'a str, wanted: &str) -> Vec<&'a str> {
    lock.split("[[package]]")
        .skip(1)
        .filter_map(|package| {
            let mut name = None;
            let mut version = None;
            for line in package.lines() {
                let line = line.trim();
                name = name.or_else(|| quoted_value(line, "name"));
                version = version.or_else(|| quoted_value(line, "version"));
            }
            if name == Some(wanted) { version } else { None }
        })
        .collect()
}

fn quoted_value<'a>(line: &'a str, field: &str) -> Option<&'a str> {
    line.strip_prefix(field)
        .and_then(|rest| rest.strip_prefix(" = \""))
        .and_then(|rest| rest.strip_suffix('"'))
}

fn collect_rust_sources(directory: &Path, root: &Path, visit: &mut impl FnMut(&str, &str)) {
    collect_named_files(directory, root, "rs", visit);
}

fn collect_named_files(
    directory: &Path,
    root: &Path,
    name_or_extension: &str,
    visit: &mut impl FnMut(&str, &str),
) {
    let mut entries = fs::read_dir(directory)
        .unwrap_or_else(|err| panic!("read {}: {err}", directory.display()))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|err| panic!("read entry below {}: {err}", directory.display()));
    entries.sort_by_key(std::fs::DirEntry::path);

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_named_files(&path, root, name_or_extension, visit);
            continue;
        }
        let matches = path
            .file_name()
            .is_some_and(|name| name == name_or_extension)
            || path
                .extension()
                .is_some_and(|extension| extension == name_or_extension);
        if !matches {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .unwrap_or_else(|err| panic!("relativize {}: {err}", path.display()))
            .to_string_lossy()
            .replace('\\', "/");
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        visit(&relative, &source);
    }
}

fn excluded_test_source(path: &str) -> bool {
    path.starts_with("crates/canic-testing-internal/")
        || path.contains("/tests/")
        || path.ends_with("/tests.rs")
        || path.ends_with("/test_support.rs")
}

fn read_source(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).unwrap_or_else(|err| panic!("read {relative}: {err}"))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(PathBuf::from)
        .expect("workspace root")
}
