// Category C - System-level artifact test (no embedded config).

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use syn::{
    Expr, ExprCall, ExprMethodCall, ExprPath, ItemUse, Macro, UseTree, Visibility,
    visit::{self, Visit},
};

const PRODUCTION_SOURCE_ROOTS: [&str; 3] = ["apps", "canisters", "crates"];
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
const NATIVE_REGISTRATION_CAPABILITIES: [&str; 5] = [
    "register_after_completion",
    "register_once",
    "reconcile_after_completion",
    "reconcile_once",
    "reconcile_watchdog",
];
const NATIVE_REGISTRATION_ACTIONS: [&str; 7] = [
    "cancel",
    "ensure_scheduled",
    "ensure_scheduled_immediately",
    "reconcile_schedule",
    "resume",
    "suspend",
    "unregister",
];
const RAW_TIMER_MECHANICS: [&str; 5] = [
    "clear_timer",
    "global_timer_set",
    "set_global_timer",
    "set_timer",
    "set_timer_interval",
];

#[derive(Debug, Default, Eq, PartialEq)]
struct TimerSyntax {
    has_timer_semantics: bool,
    uses_native_provider: bool,
    raw_provider_accesses: BTreeSet<String>,
    native_registration_references: BTreeMap<String, usize>,
    native_registration_calls: BTreeMap<String, usize>,
    native_registration_actions: BTreeMap<String, usize>,
    violations: Vec<String>,
}

#[derive(Default)]
struct TimerSyntaxVisitor {
    analysis: TimerSyntax,
}

impl<'ast> Visit<'ast> for TimerSyntaxVisitor {
    fn visit_ident(&mut self, identifier: &'ast proc_macro2::Ident) {
        if timer_semantic_identifier(identifier.to_string().as_str()) {
            self.analysis.has_timer_semantics = true;
        }
        visit::visit_ident(self, identifier);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        if segments.iter().any(|segment| segment == "ic_cdk_timers") {
            self.analysis
                .raw_provider_accesses
                .insert("ic_cdk_timers".to_string());
        }
        if segments
            .windows(2)
            .any(|window| window == ["cdk", "timers"])
        {
            self.analysis
                .raw_provider_accesses
                .insert("cdk::timers".to_string());
        }
        for mechanic in segments
            .iter()
            .filter(|segment| RAW_TIMER_MECHANICS.contains(&segment.as_str()))
        {
            self.analysis
                .raw_provider_accesses
                .insert((*mechanic).clone());
        }
        if path
            .segments
            .first()
            .is_some_and(|segment| segment.ident == "ic_timers")
        {
            self.analysis.uses_native_provider = true;
        }
        if path
            .segments
            .iter()
            .any(|segment| timer_semantic_identifier(segment.ident.to_string().as_str()))
        {
            self.analysis.has_timer_semantics = true;
        }
        visit::visit_path(self, path);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(function) = call.func.as_ref()
            && let Some(function_name) = function
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
        {
            if NATIVE_REGISTRATION_CAPABILITIES.contains(&function_name.as_str()) {
                self.analysis.has_timer_semantics = true;
                *self
                    .analysis
                    .native_registration_calls
                    .entry(function_name)
                    .or_default() += 1;
            } else if NATIVE_REGISTRATION_ACTIONS.contains(&function_name.as_str())
                && qualified_native_registration_action(&function.path)
            {
                self.analysis.has_timer_semantics = true;
                *self
                    .analysis
                    .native_registration_actions
                    .entry(function_name)
                    .or_default() += 1;
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let action = call.method.to_string();
        if NATIVE_REGISTRATION_ACTIONS.contains(&action.as_str()) {
            self.analysis.has_timer_semantics = true;
            *self
                .analysis
                .native_registration_actions
                .entry(action)
                .or_default() += 1;
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_expr_path(&mut self, expression: &'ast ExprPath) {
        if let Some(capability) = expression
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            && NATIVE_REGISTRATION_CAPABILITIES.contains(&capability.as_str())
        {
            *self
                .analysis
                .native_registration_references
                .entry(capability)
                .or_default() += 1;
        }
        visit::visit_expr_path(self, expression);
    }

    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        let mut imports = Vec::new();
        flatten_use_tree(&item.tree, &mut Vec::new(), &mut imports);
        let public = !matches!(item.vis, Visibility::Inherited);

        for import in imports {
            let is_provider = import.path.first().is_some_and(|part| part == "ic_timers");
            let is_raw_provider = import
                .path
                .first()
                .is_some_and(|part| part == "ic_cdk_timers")
                || import
                    .path
                    .windows(2)
                    .any(|window| window == ["cdk", "timers"])
                || import
                    .path
                    .iter()
                    .any(|part| RAW_TIMER_MECHANICS.contains(&part.as_str()));
            let is_authority = import.path.iter().any(|part| {
                NATIVE_REGISTRATION_CAPABILITIES.contains(&part.as_str())
                    || NATIVE_REGISTRATION_ACTIONS.contains(&part.as_str())
                    || matches!(
                        part.as_str(),
                        "AfterCompletionRegistration"
                            | "OnceRegistration"
                            | "TimerApi"
                            | "TimerAuthorityWorkflow"
                            | "TimerIdentity"
                            | "TimerSchedule"
                            | "TimerSnapshot"
                            | "WatchdogRegistration"
                    )
            });
            let aliases_provider_module = is_provider
                && import
                    .path
                    .last()
                    .is_some_and(|part| matches!(part.as_str(), "ic_timers" | "self"));

            if is_provider {
                self.analysis.has_timer_semantics = true;
                self.analysis.uses_native_provider = true;
            }
            if is_raw_provider {
                self.analysis.has_timer_semantics = true;
                self.analysis
                    .raw_provider_accesses
                    .insert(import.path.join("::"));
            }
            if import.rename.is_some() && (aliases_provider_module || is_authority) {
                self.analysis.violations.push(format!(
                    "timer scheduling authority is renamed in `{}`",
                    import.path.join("::")
                ));
            }
            if public && (is_provider || is_authority) {
                self.analysis.violations.push(format!(
                    "timer scheduling authority is publicly re-exported from `{}`",
                    import.path.join("::")
                ));
            }
        }
        visit::visit_item_use(self, item);
    }

    fn visit_macro(&mut self, macro_invocation: &'ast Macro) {
        visit_token_stream(&macro_invocation.tokens, &mut self.analysis);
        visit::visit_macro(self, macro_invocation);
    }
}

fn qualified_native_registration_action(path: &syn::Path) -> bool {
    path.segments.len() > 1
        && (path
            .segments
            .first()
            .is_some_and(|segment| segment.ident == "ic_timers")
            || path.segments.iter().any(|segment| {
                matches!(
                    segment.ident.to_string().as_str(),
                    "AfterCompletionRegistration" | "OnceRegistration" | "WatchdogRegistration"
                )
            }))
}

struct UseImport {
    path: Vec<String>,
    rename: Option<String>,
}

fn analyze_timer_syntax(path: &str, source: &str) -> TimerSyntax {
    let file = syn::parse_file(source).unwrap_or_else(|error| panic!("parse {path}: {error}"));
    let mut visitor = TimerSyntaxVisitor::default();
    visitor.visit_file(&file);
    let mut analysis = visitor.analysis;
    if analysis.native_registration_references != analysis.native_registration_calls {
        analysis.violations.push(format!(
            "native scheduling capability references are not direct calls: references={:?}, calls={:?}",
            analysis.native_registration_references, analysis.native_registration_calls
        ));
    }
    analysis
}

fn flatten_use_tree(tree: &UseTree, prefix: &mut Vec<String>, imports: &mut Vec<UseImport>) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            flatten_use_tree(&path.tree, prefix, imports);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let mut path = prefix.clone();
            path.push(name.ident.to_string());
            imports.push(UseImport { path, rename: None });
        }
        UseTree::Rename(rename) => {
            let mut path = prefix.clone();
            path.push(rename.ident.to_string());
            imports.push(UseImport {
                path,
                rename: Some(rename.rename.to_string()),
            });
        }
        UseTree::Glob(_) => imports.push(UseImport {
            path: prefix.clone(),
            rename: None,
        }),
        UseTree::Group(group) => {
            for tree in &group.items {
                flatten_use_tree(tree, prefix, imports);
            }
        }
    }
}

fn timer_semantic_identifier(identifier: &str) -> bool {
    let lowercase = identifier.to_ascii_lowercase();
    lowercase.contains("async_job_recovery")
        || identifier.starts_with("AsyncJob")
        || matches!(
            identifier,
            "AfterCompletionRegistration"
                | "CanisterTimerStatus"
                | "CoreAsyncJobRecovery"
                | "OnceRegistration"
                | "TimerApi"
                | "TimerAuthorityWorkflow"
                | "TimerExecutionOutcome"
                | "TimerIdentity"
                | "TimerRegistrationStatus"
                | "TimerSchedule"
                | "TimerSnapshot"
                | "WatchdogRegistration"
                | "ic_cdk_timers"
                | "ic_timers"
        )
        || NATIVE_REGISTRATION_CAPABILITIES.contains(&identifier)
        || RAW_TIMER_MECHANICS.contains(&identifier)
}

fn visit_token_stream(tokens: &proc_macro2::TokenStream, analysis: &mut TimerSyntax) {
    for token in tokens.clone() {
        match token {
            proc_macro2::TokenTree::Group(group) => visit_token_stream(&group.stream(), analysis),
            proc_macro2::TokenTree::Ident(identifier) => {
                let identifier = identifier.to_string();
                if timer_semantic_identifier(identifier.as_str()) {
                    analysis.has_timer_semantics = true;
                }
                if NATIVE_REGISTRATION_CAPABILITIES.contains(&identifier.as_str()) {
                    *analysis
                        .native_registration_references
                        .entry(identifier.clone())
                        .or_default() += 1;
                    *analysis
                        .native_registration_calls
                        .entry(identifier.clone())
                        .or_default() += 1;
                }
                if NATIVE_REGISTRATION_ACTIONS.contains(&identifier.as_str()) {
                    *analysis
                        .native_registration_actions
                        .entry(identifier.clone())
                        .or_default() += 1;
                }
                if identifier == "ic_cdk_timers" {
                    analysis.raw_provider_accesses.insert(identifier.clone());
                }
                if RAW_TIMER_MECHANICS.contains(&identifier.as_str()) {
                    analysis.raw_provider_accesses.insert(identifier);
                }
            }
            _ => {}
        }
    }
}

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
    let mut observed_native_calls = BTreeMap::new();
    let mut observed_native_actions = BTreeMap::new();

    for source_root in PRODUCTION_SOURCE_ROOTS {
        collect_rust_sources(&root.join(source_root), &root, &mut |path, source| {
            if excluded_test_source(path) {
                return;
            }

            let syntax = analyze_timer_syntax(path, source);
            assert!(
                syntax.violations.is_empty(),
                "{path} contains disguised or exported scheduling authority: {:?}",
                syntax.violations
            );
            if syntax.has_timer_semantics {
                observed.insert(path.to_string());
            }
            if !syntax.native_registration_calls.is_empty() {
                observed_native_calls.insert(path.to_string(), syntax.native_registration_calls);
            }
            if !syntax.native_registration_actions.is_empty() {
                observed_native_actions
                    .insert(path.to_string(), syntax.native_registration_actions);
            }
        });
    }

    let classified = expected.keys().copied().map(str::to_string).collect();
    assert_eq!(observed, classified, "timer ownership inventory changed");
    assert_eq!(
        observed_native_calls,
        expected_native_registration_calls(),
        "native timer registration custody changed"
    );
    assert_eq!(
        observed_native_actions,
        expected_native_registration_actions(),
        "native timer registration actions changed"
    );
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
        let syntax = analyze_timer_syntax(path, source.as_str());
        assert_semantic_class(path, source.as_str(), &syntax, class);
    }
}

#[test]
fn semantic_inventory_ignores_comments_and_string_literals() {
    let syntax = analyze_timer_syntax(
        "synthetic.rs",
        r#"
            const NOTE: &str = "ic_timers::register_once";
            // TimerApi::set was removed.
            fn ordinary_business_logic() {}
        "#,
    );

    assert_eq!(syntax, TimerSyntax::default());
}

#[test]
fn semantic_inventory_rejects_aliased_provider_authority() {
    let syntax = analyze_timer_syntax(
        "synthetic.rs",
        "use ic_timers::register_once as schedule_later;",
    );

    assert!(syntax.has_timer_semantics);
    assert_eq!(syntax.violations.len(), 1);
}

#[test]
fn semantic_inventory_rejects_value_aliased_provider_authority() {
    let syntax = analyze_timer_syntax(
        "synthetic.rs",
        "fn alias() { let schedule = ic_timers::register_once; schedule(); }",
    );

    assert!(syntax.has_timer_semantics);
    assert_eq!(syntax.violations.len(), 1);
}

#[test]
fn semantic_inventory_observes_unclassified_and_duplicate_native_calls() {
    let syntax = analyze_timer_syntax(
        "synthetic.rs",
        r"
            fn duplicate_custody() {
                ic_timers::register_once(first());
                ic_timers::register_once(second());
            }
        ",
    );

    assert!(syntax.has_timer_semantics);
    assert_eq!(syntax.native_registration_calls["register_once"], 2);
}

#[test]
fn semantic_inventory_observes_native_registration_method_actions() {
    let syntax = analyze_timer_syntax(
        "synthetic.rs",
        r"
            fn mutate(first: &Registration, second: &Registration) {
                first.ensure_scheduled(schedule());
                first.ensure_scheduled_immediately();
                first.cancel();
                second.cancel();
                second.unregister();
            }
        ",
    );

    assert!(syntax.has_timer_semantics);
    assert_eq!(syntax.native_registration_actions["ensure_scheduled"], 1);
    assert_eq!(
        syntax.native_registration_actions["ensure_scheduled_immediately"],
        1
    );
    assert_eq!(syntax.native_registration_actions["cancel"], 2);
    assert_eq!(syntax.native_registration_actions["unregister"], 1);
}

#[test]
fn semantic_inventory_rejects_aliased_native_registration_actions() {
    let syntax = analyze_timer_syntax(
        "synthetic.rs",
        "use ic_timers::OnceRegistration::cancel as cancel_later;",
    );

    assert!(syntax.has_timer_semantics);
    assert_eq!(syntax.violations.len(), 1);
}

#[test]
fn semantic_inventory_counts_macro_generated_native_calls() {
    let syntax = analyze_timer_syntax(
        "synthetic.rs",
        r"
            macro_rules! schedule {
                () => { ic_timers::register_once(identity(), lifetime(), callback()) };
            }
        ",
    );

    assert!(syntax.has_timer_semantics);
    assert_eq!(syntax.native_registration_calls["register_once"], 1);
}

#[test]
fn semantic_inventory_detects_raw_timer_mechanics() {
    let syntax = analyze_timer_syntax(
        "synthetic.rs",
        "fn bypass() { ic_cdk::api::set_global_timer(1); }",
    );

    assert!(syntax.has_timer_semantics);
    assert!(syntax.raw_provider_accesses.contains("set_global_timer"));
}

#[test]
fn semantic_inventory_detects_aliased_raw_provider_imports() {
    let syntax = analyze_timer_syntax("synthetic.rs", "use ic_cdk_timers as raw;");

    assert!(syntax.has_timer_semantics);
    assert!(syntax.raw_provider_accesses.contains("ic_cdk_timers"));
}

#[test]
fn timer_provider_graph_and_manifest_consumers_are_closed() {
    let root = workspace_root();
    let lock = read_source(&root, "Cargo.lock");

    assert_eq!(locked_package_versions(&lock, "ic-timers"), ["0.7.0"]);
    assert_eq!(locked_package_versions(&lock, "ic-cdk-timers"), ["1.0.0"]);
    assert_eq!(locked_package_versions(&lock, "icydb"), ["0.252.1"]);

    let workspace_manifest = read_source(&root, "Cargo.toml");
    assert!(workspace_manifest.contains("ic-timers = \"=0.7.0\""));
    assert!(workspace_manifest.contains("icydb = { version = \"=0.252.1\""));
    assert!(!workspace_manifest.contains("icydb-model ="));
    assert!(!workspace_manifest.contains("ic-cdk-timers ="));

    let mut direct_icydb_model_consumers = BTreeSet::new();
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
                if manifest
                    .lines()
                    .any(|line| line.trim_start().starts_with("icydb-model ="))
                {
                    direct_icydb_model_consumers.insert(path.to_string());
                }
            },
        );
    }

    assert!(direct_icydb_model_consumers.is_empty());
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
            for forbidden in analyze_timer_syntax(path, source).raw_provider_accesses {
                violations.push(format!("{path}: {forbidden}"));
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

#[test]
fn recovery_takeovers_recheck_each_owners_authoritative_domain_demand() {
    let root = workspace_root();
    let cases = [
        (
            "root issuer renewal",
            "crates/canic-core/src/workflow/runtime/auth/renewal.rs",
            [
                "AsyncJobWorkflow::has_expired_attempt(owner, now_ns)",
                "AuthOps::has_enabled_root_issuer_renewal_templates()",
                "ConfigOps::delegated_tokens_config()",
                "AsyncJobWorkflow::claim_expired(owner, now_ns)",
                "Self::run_scheduled().await",
            ],
        ),
        (
            "automatic cycle top-up",
            "crates/canic-core/src/workflow/runtime/cycles/mod.rs",
            [
                "AsyncJobWorkflow::has_expired_attempt(owner, now_ns)",
                "Self::automatic_topup_config()",
                "AsyncJobWorkflow::claim_expired(owner, now_ns)",
                "Self::run_attempt(attempt).await",
                "attempt.operation_id()",
            ],
        ),
        (
            "placement receipt acknowledgement",
            "crates/canic-core/src/workflow/placement/acknowledgement.rs",
            [
                "AsyncJobWorkflow::has_expired_attempt(owner, now_ns)",
                "ReceiptBackedIntentOps::has_placement_acknowledgements()",
                "AsyncJobWorkflow::claim_expired(owner, now_ns)",
                "Self::run_scheduled().await",
                "let operation_id = intent.operation_id;",
            ],
        ),
    ];

    for (owner, path, required) in cases {
        let source = read_source(&root, path);
        for fragment in required {
            assert!(
                source.contains(fragment),
                "{owner} recovery lost authoritative binding `{fragment}`"
            );
        }
    }

    let pool = read_source(
        &root,
        "crates/canic-control-plane/src/workflow/canister_pool/mod.rs",
    );
    for required in [
        "AsyncJobRecoveryOps::expired_deadline(AsyncJobOwner::CanisterPoolMaintenance, now_ns)",
        "dispatch_async_job_recovery()",
        "finish_maintenance_timer(attempt, maintain_once_inner().await)",
        "CanisterPoolOps::pending_creation()",
        "CanisterPoolOps::pending_reset_canisters()",
        "CanisterPoolOps::ready_count()",
    ] {
        assert!(
            pool.contains(required),
            "pool recovery lost authoritative binding `{required}`"
        );
    }
}

fn assert_semantic_class(path: &str, source: &str, syntax: &TimerSyntax, class: OwnershipClass) {
    for forbidden in PROHIBITED_AUTHORITY_FRAGMENTS {
        assert!(
            !source.contains(forbidden),
            "{path} contains prohibited scheduling authority `{forbidden}`"
        );
    }

    match class {
        OwnershipClass::DomainAsyncJobRecovery => {
            assert!(
                syntax.native_registration_calls.is_empty(),
                "domain recovery file {path} owns a native registration capability"
            );
            assert!(
                syntax.native_registration_actions.is_empty(),
                "domain recovery file {path} performs a native registration action"
            );
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
            assert!(
                syntax.native_registration_calls.is_empty(),
                "projection file {path} owns a native registration capability"
            );
            assert!(
                syntax.native_registration_actions.is_empty(),
                "projection file {path} performs a native registration action"
            );
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
                syntax.uses_native_provider,
                "independent application {path} does not use the shared provider"
            );
            assert!(
                !source.contains("TimerIdentity::try_new(\"canic\""),
                "independent application {path} claims the Canic timer owner"
            );
        }
        OwnershipClass::NativeRegistrationCustody => {
            assert!(
                !syntax.native_registration_calls.is_empty(),
                "native custody file {path} lacks a native registration capability"
            );
        }
        OwnershipClass::PrivateLifecycleConsumer => {
            assert!(
                syntax.native_registration_calls.is_empty(),
                "private lifecycle consumer {path} retains native registration custody"
            );
            assert!(
                syntax.native_registration_actions.is_empty(),
                "private lifecycle consumer {path} performs a native registration action"
            );
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
        .chain(operator_projection_ownership())
        .chain(facade_ownership())
        .collect()
}

fn expected_native_registration_calls() -> BTreeMap<String, BTreeMap<String, usize>> {
    [
        (
            "canisters/test/runtime_probe/src/lib.rs",
            [("register_after_completion", 1), ("register_once", 4)].as_slice(),
        ),
        (
            "crates/canic-control-plane/src/workflow/canister_pool/mod.rs",
            [("reconcile_after_completion", 1), ("reconcile_watchdog", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/placement/acknowledgement.rs",
            [("register_once", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/auth/renewal.rs",
            [("register_once", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/cycles/mod.rs",
            [("register_once", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/intent.rs",
            [("register_once", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/log.rs",
            [("register_once", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/timer/mod.rs",
            [("reconcile_watchdog", 1), ("register_once", 1)].as_slice(),
        ),
    ]
    .into_iter()
    .map(|(path, calls)| {
        (
            path.to_string(),
            calls
                .iter()
                .map(|(capability, count)| (capability.to_string(), *count))
                .collect(),
        )
    })
    .collect()
}

fn expected_native_registration_actions() -> BTreeMap<String, BTreeMap<String, usize>> {
    [
        (
            "canisters/test/runtime_probe/src/lib.rs",
            [("cancel", 1), ("ensure_scheduled", 4)].as_slice(),
        ),
        (
            "crates/canic-control-plane/src/workflow/canister_pool/mod.rs",
            [("cancel", 2)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/placement/acknowledgement.rs",
            [("ensure_scheduled", 1), ("reconcile_schedule", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/auth/renewal.rs",
            [("reconcile_schedule", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/cycles/mod.rs",
            [("reconcile_schedule", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/intent.rs",
            [("ensure_scheduled", 1), ("reconcile_schedule", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/log.rs",
            [("reconcile_schedule", 1)].as_slice(),
        ),
        (
            "crates/canic-core/src/workflow/runtime/timer/mod.rs",
            [("cancel", 1), ("ensure_scheduled", 1), ("unregister", 2)].as_slice(),
        ),
    ]
    .into_iter()
    .map(|(path, actions)| {
        (
            path.to_string(),
            actions
                .iter()
                .map(|(action, count)| (action.to_string(), *count))
                .collect(),
        )
    })
    .collect()
}

const fn application_ownership() -> [(&'static str, OwnershipClass); 5] {
    use OwnershipClass::{
        IndependentApplicationCustody as Application, PrivateLifecycleConsumer as Lifecycle,
    };

    [
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
    ]
}

const fn control_plane_ownership() -> [(&'static str, OwnershipClass); 10] {
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
            "crates/canic-control-plane/src/workflow/component_registry/lifecycle_drivers/mod.rs",
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
        (
            "crates/canic-control-plane/src/workflow/root_admission/mod.rs",
            Lifecycle,
        ),
    ]
}

const fn core_boundary_ownership() -> [(&'static str, OwnershipClass); 11] {
    use OwnershipClass::{
        DomainAsyncJobRecovery as Recovery, DtoOrMetricsProjection as Projection,
        PrivateLifecycleConsumer as Lifecycle,
    };

    [
        ("crates/canic-core/src/api/runtime/mod.rs", Projection),
        (
            "crates/canic-core/src/api/runtime/root_funding.rs",
            Lifecycle,
        ),
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

const fn operator_projection_ownership() -> [(&'static str, OwnershipClass); 1] {
    use OwnershipClass::DtoOrMetricsProjection as Projection;

    [("crates/canic-cli/src/inspect/mod.rs", Projection)]
}

fn expected_timer_manifest_consumers() -> BTreeSet<String> {
    [
        "Cargo.toml",
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
        ("crates/canic-host/src/lib.rs".to_string(), 1),
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
