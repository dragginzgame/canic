# README and linked documentation drift audit

Date: 2026-09-23. Reviewer: Codex. Result: **fail — confirmed documentation
drift**. All findings below are open. This is a documentation review, not a
runtime qualification, release verdict, B1 acceptance or minor closeout.

## Scope and method

The review starts at the repository README, covers its 23 directly reachable
Markdown documents (including crate directory READMEs), and follows the current
feature, onboarding, configuration, architecture and operator references.
Semantic findings are checked against current source or normative policy.
Historical status entries and frozen measurements are not required to describe
today's implementation. Detailed transitive contracts receive source checks
where relevant; this is not an exhaustive proof of every runtime contract.

The separate recursive link pass visits 248 Markdown documents and checks 1,140
local link occurrences, including historical documents reached through the
current status and feature guides. It finds 87 missing targets in seven archived
documents, no unresolved checked heading fragments, and no missing direct README
targets. The [link inventory](readme-documentation-links.json) retains every
visited document and missing target. The pass recognizes inline Markdown links
and HTML `src`/`href`, skips fenced examples, and checks ATX heading fragments;
it is not a complete Markdown-renderer implementation. The 68 external link
occurrences were not fetched. External project descriptions and external tool
behavior were not independently certified.

Method: ad hoc manual README review, revision 1: (1) resolve links; (2) compare
commands with dispatch/scaffolding; (3) compare configuration and init contracts
with schema/source; (4) compare feature and lifecycle claims with their owners;
(5) compare policy and version claims with repository authorities. No new
recurring audit method or explanatory-prose gate is introduced.

Source anchor: `64024b5aed48b54f7e9537aefefc4b8887f286b4` (`v0.110.37` checkout),
HEAD tree `6827bb435c1d00ac1fc2e250f25b5dd47e8d5f57`, with existing dirty work.
The handoff records separately published `.38`; this review does not reconcile
that release or modify concurrent B1/recovery work. Cargo.lock SHA-256 at review:
`8fddc467e615c43fc62abe766b720f5e57b8e688546c46033c3b20a7cb621e6d`.
No clean or immutable current-product snapshot is claimed. Compiler execution,
target, feature selection and benchmark comparison are not applicable: this is
a read-only source trace. Only this report and its link inventory are added.

## Findings

Priority P2 means a misleading current contract or procedure; P3 means navigation
or maintenance drift. All findings have high confidence from the cited source.
Locations refer to the checkout at review time.

### D01 — P2: first-App instructions scaffold an already-created canister

[INSTALLING.md](../../../../../INSTALLING.md), lines 122–124, and the
[operator walkthrough](../../../../architecture/v1-operator-walkthrough.md),
lines 30–32, run `app create`, then `scaffold canister ... app`, then attach it.
`app create` already writes the `app/` package and attaches its Component Spec.
The next command returns `TargetExists`; a new user cannot finish the sequence.
Evidence: [scaffold owner](../../../../../crates/canic-cli/src/scaffold/mod.rs),
lines 327–372, 402–414 and 951–977. Remove the duplicate steps, or use a genuinely
new role when teaching scaffolding. An existing-target regression already exists.

### D02 — P2: onboarding bypasses the maintained desired-state generator

[INSTALLING.md](../../../../../INSTALLING.md), line 140, the
[minimal guide](../../../../getting-started/minimal-managed-fleet.md), line 220,
and the [walkthrough](../../../../architecture/v1-operator-walkthrough.md)
instruct users to write the low-level Fleet document. Their build examples only
build one role. The [Fleet owner](../../../../features/operations/fleet-ensure.md),
lines 247–250, explicitly requires generation after a complete App build, binding
the finalized release, protected Fleet policy and estate seed. The tutorials
should show that complete build/generate/review sequence, including `--fresh`
when starting empty, and reserve low-level TOML for reference.

### D03 — P2: CONFIG describes a superseded managed init payload

[CONFIG.md](../../../../../CONFIG.md), lines 46–49, says children receive
`EnvBootstrapArgs` inside `CanisterInitPayload` and that concrete
`ComponentBinding` is future work. The current
[payload](../../../../../crates/canic-core/src/dto/abi/v1/payload.rs), line 32,
contains typed `authority` and protected `component_deployment`; the
[nonroot initializer](../../../../../crates/canic-core/src/workflow/runtime/nonroot.rs),
lines 48–85, consumes concrete Component/ComponentChild authority now.
Document those bindings; do not teach manual reconstruction of the old payload.

### D04 — P2: CONFIG advertises cycle units rejected by its parser

[CONFIG.md](../../../../../CONFIG.md), line 235, permits `K` and `M` in cycle
configuration. The human-config entry point in
[cycles.rs](../../../../../crates/canic-core/src/cdk/types/cycles.rs), line 127,
requires uppercase `B`, `T` or `Q`; Component Spec fields use that deserializer.
The generic internal `FromStr` accepting smaller units does not make them valid
TOML input. Correct the human-input contract and distinguish internal parsing.

### D05 — P2: the canonical config reference omits deployed composition

[CONFIG.md](../../../../../CONFIG.md) explains Component Specs but has no schema
reference for `component_groups`, `component_group_deployments` or `services`.
All three are maintained top-level fields in
[ConfigModel](../../../../../crates/canic-core/src/config/schema/mod.rs),
lines 180–194. Initial Component placement and managed qualification depend on
these declarations; Specs alone do not explain how to get deployed occurrences.
Add concise field references and one complete composition example, linking the
existing schema owners instead of making readers infer them from old designs.

### D06 — P2: multiple guides still assume application-owned Roots

[INSTALLING.md](../../../../../INSTALLING.md), line 89, tells readers how to
declare Root packages. [Native timers](../../../../features/runtime/native-timers.md),
line 202, includes Root in the application lifecycle-participant example.
[Root proof provisioning](../../../../operations/root-proof-provisioning.md),
lines 56–68 and its later repair advice, recommends an application Root endpoint.
Root package paths now reject in
[configuration validation](../../../../../crates/canic-core/src/config/validation/mod.rs),
line 189; [start_fleet_root!](../../../../../crates/canic/src/macros/start.rs),
line 332, explicitly has no application hooks or participant. Restrict application
examples to managed nonroot roles and describe the canonical Root renewal owner.
The provisioning source map also names the nonexistent
`crates/canic/src/macros/endpoints/nonroot.rs`.

### D07 — P2: CONFIG promises a removed memory diagnostic endpoint

[CONFIG.md](../../../../../CONFIG.md), line 248, says
`diagnostics.memory_ledger = true` emits `canic_memory_ledger`. Current endpoint
macros do not emit that method. The maintained controller-only read is
`canic_observability(MemoryAllocations)`, documented in
[public observability](../../../../features/runtime/public-observability.md)
and implemented by [role endpoints](../../../../../crates/canic/src/macros/endpoints/role.rs),
lines 134–203, and the Root endpoint owner. The config field remains accepted
and hashed, which does not establish the claimed endpoint behavior. Correct the
guide; disposition of that residual config field is a separate source decision.

### D08 — P2: the managed-test recipe pins an old Canic release

[Managed-App qualification](../../../../features/build-and-evidence/managed-app-qualification.md),
line 13, instructs consumers to add `canic = "=0.110.7"`. This conflicts with the
same-version runtime/CLI contract and can introduce an old test facade alongside
current product dependencies. Use the application's exact selected Canic version
or its workspace dependency. Keep historical qualification versions in dated
evidence, not in a current dependency recipe.

### D09 — P2: the feature index incorrectly removes backup/restore CLI support

[Feature index](../../../../features/README.md), lines 26–27, calls backup a
Rust-only retained domain and says its former CLI workflow is absent. Current
[CLI dispatch](../../../../../crates/canic-cli/src/lib.rs), lines 252 and 266,
implements `backup` and `restore`; the linked feature guide documents them.
Update the index to describe the current same-release recovery workflow.

### D10 — P2: the core README teaches a forbidden dependency direction

[canic-core README](../../../../../crates/canic-core/README.md), line 32, shows
`endpoints → workflow → policy → ops → model`. Policy must never call ops.
The [architecture contract](../../../../contracts/ARCHITECTURE.md) and AGENTS
require independent workflow-to-policy and workflow-to-ops branches. Correct
the diagram so new contributors do not implement the wrong layering.

### D11 — P2: the README retains an obsolete release-policy exception

[README.md](../../../../../README.md), lines 162–163, says reinstall-only
applies unless a design says otherwise. Current [AGENTS.md](../../../../../AGENTS.md)
requires every pre-1.0 transition to be reinstall-only without an active
exception. Remove that escape clause while preserving same-release recovery.
This does not reinterpret historical decisions as current authorization.

### D12 — P2: runtime recovery documentation has an incomplete owner inventory

[Runtime guide](../../../../features/runtime/README.md), lines 74–95, lists four
recovery-critical owners but omits fixture import. The
[stable record](../../../../../crates/canic-core/src/storage/stable/async_job_recovery/mod.rs),
lines 65–82, also retains its attempt fence, retry pressure and typed failure.
Describe this domain without conflating durable business retry with provider state.
The [recovery runbook](../../../../operations/recovery-retry-runbooks.md), line
260, additionally tells operators to inspect `canic/async_recovery/watchdog`;
the exact [timer identity](../../../../../crates/canic-core/src/workflow/runtime/timer/mod.rs),
line 344, is `canic/async_job_recovery/watchdog`.

### D13 — P3: current navigation points to old release lines as active

[Scaling guide](../../../../features/scaling-and-placement/README.md), line 28,
calls 0.101 active. The [architecture index](../../../../architecture/README.md),
lines 24–27, presents archived 0.100 design/status as the current implementation
handoff. Link current status for current delivery; label those designs as
historical background. Do not rewrite their frozen conclusions.

### D14 — P3: the CLI README's command-family list omits frontend

[CLI README](../../../../../crates/canic-cli/README.md), lines 9–33, omits
`frontend`, although its own later text and current
[dispatch](../../../../../crates/canic-cli/src/lib.rs), line 259, expose it.
Add it in ASCII order between `fleet` and `info`.

### D15 — P3: current composition guides disagree on the IcyDB dependency

[Native timers](../../../../features/runtime/native-timers.md), line 36, names
IcyDB 0.261.2 as the composition fixture; the
[memory guide](../../../../features/runtime/stable-memory-layout.md), lines 49
and 74, names 0.258.0. Current [Cargo.toml](../../../../../Cargo.toml), line 98,
selects 0.261.7. Separate historical measured adoption claims from current
dependency guidance. Link the manifest/lock for current selection and retain
exact historical versions only with dated qualification evidence. Merely changing
the prose version cannot establish that a new combination passed PocketIC.

### D16 — P3: TESTING still recommends serializing the entire workspace

[TESTING.md](../../../../../TESTING.md), lines 29–37, applies single-threaded
execution to workspace tests generally. The maintained
[runner](../../../../../scripts/ci/run-workspace-tests.sh), lines 364–375 and
454–474, runs ordinary tests with normal libtest parallelism and serializes the
PocketIC suites separately. Scope the restriction to the simulator lanes and
describe the ordinary-test barrier, preserving the validation speed work.

### D17 — P3: archive moves broke 87 reachable link occurrences

The [complete inventory](readme-documentation-links.json) identifies all source
lines and targets. Breakdown:

| Archived source | Missing occurrences |
| --- | ---: |
| 0.104 timer design | 5 |
| 0.101 composition design | 1 |
| 0.101 status | 6 |
| 2026-08-12 status archive | 66 |
| 2026-08-26 status archive | 4 |
| 0.103 Candid design | 3 |
| 0.100 Coordinator design | 2 |

Examples include the runtime guide's directly linked 0.104 design, lines
515–520, whose audit links still use the depth from before archival. Repair
relative paths where the original target survives; otherwise identify the
historical missing target honestly. Preserve evidence content and verdicts.

## Repair order and validation

First repair the first-App build/generate path, CONFIG's current authority and
composition schema, and unsupported Root/memory examples. Then reconcile the
feature/index/lifecycle/version/testing prose and restore archived navigation.
These can be one coherent documentation batch; they do not require a patch per
finding or a runtime change.

README MSRV/internal-toolchain badges agree with Cargo.toml/rust-toolchain.toml.
ICP, ic-wasm and Binaryen pins in installation/build guidance agree with the
repository pins. Current facade/control-plane feature defaults and managed-test
constructor signatures checked in source agree with their guides. Those checks
are source observations, not compiler or installed-binary qualification.

No Cargo/Make compilation, full validation, PocketIC, network effect, version
transaction, staging or commit was performed. The existing dirty worktree is
preserved. Suggested follow-up validation is local link checking, current config
example validation and a narrowly scoped scaffold/example check if examples
change. Do not add string-match tests for these explanatory sentences.
