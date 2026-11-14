# M0 first journey decisions and evidence

Status: M0 in progress, 2026-11-14. The [architecture](canic-system-design.md)
is accepted; local creation and direct-cycle funding commit-boundary experiments
pass. The M1 CLI now exercises retained signed host attempts, deployment and
same-canister upgrade locally. Mainnet creation, installation, greeting and the
profile entry/worker deployment and persistent worker upgrade have passed.
A local infrastructure-owner maintenance window now preserves both effect
journals across a changed executable and failed upgrades. A bounded artifact
fixture also installs a retained worker module without the operator. The profile
demo now qualifies a one-slot manager that creates and installs its worker
after admission without the host, including normal mainnet growth and successful
mainnet manager maintenance. Production
management-owner persistence and comparative adoption measurements remain unqualified.
This record contains scoped decisions and evidence, not a second implementation
backlog. The first product is the [M1 Hello World loop](../../README.md), followed
by the [persistent mainnet profile pair](../demos/profile.md#current-mainnet-demo).

## 0.201.0 release review

The release review covers app-owned definitions versus fleet bindings, canister-only
trees, shared management ownership, and the separation of public Root reports from
authenticated completion evidence. The growth fixture's obsolete report-heading
assertion now checks the explicit uncertified Manager label.
The installed Root CLI fixture pauses automatic simulator progress around manual
time advancement, preventing its clock from racing the live gateway.

One naming cleanup remains: `crates/canic-host/src/lib.rs` exports the shared
management protocol's `Phase` as `ProfileGrowthPhase`, used by CLI renderers and
their tests. Rename that export and its callers to a management-owned name in the
next increment, without a compatibility alias. It does not pull the profile
application protocol into Host or change persisted/wire evidence.

## Read-only Root progress

The next 2026-11-14 increment closes the observation gap between Root admission
and retained completion. The existing fleet collector now selects each planned
Root by exact network binding and principal, alongside explicitly selected
profile managers. Terminal output, saved reports and localhost watch reuse that
collector; no new command, journal or polling owner is introduced. Automatic
selection stays network-scoped even when two replicas reuse the same principal;
explicit principal-only selection still refuses that ambiguity. The combined
selection remains capped at sixteen owners and the existing snapshot deadline.

Reuse the same pinned legacy component-operation and progress-rendering research
recorded in the managed-child and readable-status sections below, including the
request-correlation and unknown-readiness regressions and their inspected path
history. Preserve their separation of reported progress and terminal evidence.
A public report must match the retained approval before its child enters the
observation inventory. Mismatches stay unknown; an empty pre-admission Root is
allowed. Neither reported readiness nor independently observed child metadata
establishes retained child custody, changes slot bindings or writes receipts.
Trees still contain canisters only; external controllers remain metadata.

Native tests cover exact network selection, explicit/automatic deduplication,
selection budgets and refusal of foreign approvals and self-child reports.
The installed CLI fixture observes Scheduled then Ready without applying,
checks both terminal and HTML Root labels, and proves the child's retained
identity and receipt remain absent until authenticated completion. Watching
creates no signed observation attempt and spends no new management allowance.
The simulator uses synthetic cycles and opens no browser. This is bounded
single-level Root observation, not general cascading subtree discovery.

## Managed-child CLI approval

The next 2026-11-14 increment connects the generated app to the supplied Root
through the existing fleet plan/apply/status workflow. Explicit `--root-module`
selects Root-owned creation; parent declarations alone never authorize it.
Resolve the immediate parent from a configured identity or a completed named
receipt, reject ambiguous/conflicting identities and refuse multiple children
for this one-slot owner. Each apply checks certified Root code and sole operator
control. The child receipt and fleet projection name Root as its controller.

Research at legacy revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` inspected
[component operation workflow](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/workflow/mod.rs),
[operation regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/tests.rs)
and [transport selection tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/ops/transport/tests/mod.rs).
The path history includes `4176ee97405185bf4780b5ada0f9206ead750313`
(native timer ownership) and `aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`
(the aligned supplier hard cut). Carry forward intent before submission,
immutable review identity, incomplete successful progress, authority-drift
refusal and monotonic output custody. The narrow Root needs no legacy release
registry, capacity pool or generalized component-operation journal.

The existing typed plan adds an optional management binding; absent bindings
serialize exactly as before. Golden checks preserve retained direct v1/v2
plan digests; no new format version, migration or parallel reader is introduced.
The single retained plan owns Root identity/code, child bytes, signer, exact 1T
attachment and schedule. New schedules default to ten minutes after planning,
with an effect deadline ten minutes later. Reopening a named slot returns its
original authority, including a past schedule; it cannot renew an allowance.
A declared child refuses fresh direct-host creation. Existing direct operations
remain recoverable by their original digest even if the declaration changes.

Staging uses the existing exact signed-attempt owner. A successful pending result
is separate from completion. Root adds a controller-guarded replicated
`observe` endpoint that only reads committed status. Its retained reply can
recover lost staging evidence without re-signing admission. Only that exact
read method, recipient and payload may receive fresh signing authority after
the effect deadline; it cannot reconcile or dispatch effects. Public queries do
not establish created-principal custody. Observation attempts are bounded at 32;
an uncertain expired observation stays blocked. Child identity, attempts,
request commitments and outcome evidence cannot regress. Root readiness plus
certified child module/controllers are required before retaining a receipt.

Focused host/CLI tests cover planning, exact replay, changed authority, output
substitution, signer scope and preserved direct-plan encodings. Installed CLI
qualification creates the generated config, binds a fixture Root, plans and
admits its app, loses the staging reply, reconstructs admission through a
replicated read, observes one ready child, projects its fleet membership and
recovers a missing completion receipt without invoking the signer. Existing
Root maintenance qualification remains passing. No mainnet mutation or browser
launch occurred. Whole-fleet convergence, child upgrades, arbitrary subtree
operations and general infrastructure maintenance approvals remain separate.

## Generated starter implementation

The next 2026-11-14 increment extends the existing `app init` action with
`--source DIRECTORY --canic-source CHECKOUT`. It generates a new standalone
workspace, its config, an editable greeting application, and a declaration for
the supplied Root. The default initialization path still edits declarations for
existing code. Source generation cannot be combined with `--config`; it owns the
new project's config rather than editing a second existing document.

Reuse the pinned legacy scaffold implementation, tests and canonical-Root/source-
alias history inspected in the Root section below. Carry forward refusal of
occupied targets and validation before writes. Replace the legacy multi-file
workspace mutation/rollback with one complete directory publication through
Host FS: the new project's parent must exist, staging is private, publication
cannot replace an existing name, and failures retain source/error/path evidence.
The generator does not edit the containing Cargo workspace or current fleet
config. Tests cover malformed names/source, existing and symlink destinations,
source aliases, shared config semantics and competing initializers.

The selected Canic checkout must match the CLI version and expose the supplied
Root and greeting contract packages. Resolve its physical path before writing
dependency and build references. The generated workspace inherits its selected
compiler, supplier declarations and lint policy; it keeps publication disabled.
The user prepares and retains the generated lockfile explicitly. Generation
does not fetch dependencies, invoke Cargo, create journals or authorize effects.
No crates.io Canic package or copied Root runtime is inserted into the app.

`artifact build --output WASM_FILE` atomically publishes a replaceable build
input after selecting and admitting the Cargo artifact. The generated config's
`artifacts/root.wasm` and `artifacts/app.wasm` paths resolve beside that config.
Existing planning continues to retain immutable bytes under its original owner;
build outputs are not deployment receipts or new runtime authority.

The focused host/CLI tests and installed Root fixture passed. The latter now uses
the actual CLI generator and builder, prepares the generated lockfile offline,
and installs the generated greeting under Root on PocketIC with synthetic cycles.
It retains the controller, admission, maintenance and replay checks from the
previous Root increment. Source generation and managed-child CLI approval are
implemented; whole-fleet convergence remains pending. No mainnet mutation or
browser launch occurred. See the [starter commands](../demos/root.md).

## One-child Root implementation

On 2026-11-14, the next local implementation supplies `canic-root` for one
greeting application. The canister tree is `Root → application`; Root's external
operator is controller metadata, not a canister node. This is a bounded Root
implementation, not generated source or whole-fleet convergence.

Legacy research used revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`:

- [Canonical Root materialization](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/canonical_root/mod.rs)
  and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/canonical_root/tests.rs)
  supply the ownership lesson: application generation must consume Canic's
  supplied Root rather than fork its management implementation into each app.
- [Root bootstrap](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/workflow/bootstrap/root/mod.rs)
  establishes synchronous custody before deferred work and withholds readiness
  while required releases/activation evidence are absent. Retain that ordering;
  the selected one-child workload needs neither its Store nor its Coordinator
  and capacity-import machinery.
- [Scaffold implementation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/scaffold/mod.rs)
  and [regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/scaffold/tests.rs)
  refuse existing targets and validate planned writes before mutation. Preserve
  these cases when generation is implemented; they are not proved by this runtime
  increment.
- [History f2f88e1](https://github.com/dragginzgame/canic-legacy/commit/f2f88e1c89c38629aaff0a299f9e32c373dea1b0)
  fixes canonical Root identity through retained source aliases. Future source
  materialization must bind to the resolved config identity; this increment
  builds explicit packages and adds no source-discovery mechanism.

Extract the already installed profile manager's one-slot engine into
`canic-management`, with Wire/evidence types in `canic-management-protocol`.
The profile manager and Root each mount lifecycle and endpoints explicitly and
supply their own bounded readiness decoder. The engine owns effect issuance,
retained artifact bytes, journal transitions and reconciliation. Preserve RPC
numbers, field/enum order, stable-memory keys and timer identity. Move every
consumer to the single protocol owner without a compatibility re-export. The
native host graph no longer depends on the profile application's protocol.

`make root-test` passed with CLI-built Root and Hello World Wasm, synthetic cycles
and an external identity controlling Root. Qualification proves unauthorized
admission refusal, an upgrade while the original approval is scheduled, one
created child with Root as sole controller, observed module binding, a greeting,
exact replay and changed-approval refusal. `make growth-test` passed all five
interruption windows, expiry/reserve refusal, artifact custody and maintenance
against the extracted engine. Lost creation evidence remains blocked, not retried.

An additional installed upgrade from the retained pre-extraction profile manager
Wasm (`4fd7e504d88e4f0139a834f3b27f8aa027edb9a6a861284669411a37f0b0d1c4`)
preserved its admitted authority and artifact, resumed creation and reached
profile readiness through the new engine. Reproduce that evidence with an exact
retained previous artifact using `CANIC_PREVIOUS_MANAGER` and the explicitly
selected ignored test `previous_profile_manager_preserves_admitted_work` in
`qualification/m1-tests/tests/root.rs`; the default Root test needs no historical
binary. The shared protocol frame/budget regression moved with its owner.

Root/management and affected native/Wasm callers passed Clippy; Root/management
Rustdoc passed. No mainnet canister, signer, controller or balance was changed.
Source generation, persistent application upgrades through Root and whole-fleet
CLI convergence remain the next separate product outcomes. See the
[hands-on Root guide](../demos/root.md).

## Reusable app definition implementation

The app-definition portion of 0.201.0 implements shared app definitions, independently
of the Root and source generator. `apps.APP.canisters` owns roles, parent relationships
and optional Wasm input paths; each fleet owns only network, journal selection
and existing-ID bindings. A new fleet projects the app's complete declared layout
without copying another fleet's IDs. The existing plan/apply and observation
owners consume a resolved snapshot, and retained deployment-plan formats and
digests remain unchanged.

`app canister add/list` edits and inspects the shared model. `fleet canister
bind/list` binds existing identities for observation and lists the resolved fleet.
The retired fleet-local add command has no alias. The config is unversioned and
rejects the previous layout; [README migration instructions](../../README.md#initialize-an-app-and-fleet)
describe the explicit external edits. This checkout retains all original
principals, network selections and `.canic` journal paths. Old source/layout
examples later in this evidence record describe their historical increments.

Research reuses the pinned declaration mutation/config regression and history
review below, plus legacy
[topology finalization](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_topology/mod.rs)
and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_topology/tests.rs).
Retain shared logical definitions versus explicit deployment facts, deterministic
projection, bounded validation and no fabricated principals. Do not import the
general Spec catalogue, warm pools or Coordinator dependency to describe the
current two demos. Static parent declarations still grant no spawning authority.

Focused regressions cover two fleets sharing one definition with independent
bindings/plans/journals, malformed/conflicting bindings, no fleet-level topology
or artifact overrides, duplicate names, complete config limits, unchanged files
on refusal, concurrent initialization, locks/symlinks, config-relative Wasm paths,
source edits refusing new authority for occupied slots, and retained v1/v2 plan
recovery. The installed M1 journey uses the new declaration command while retaining
the same create/install/call/upgrade effect path. Generated source, generalized
management ownership, whole-fleet convergence and runtime cascades are still
separate unimplemented outcomes; this increment does not qualify them.

Validation: 22 focused host config tests, nine CLI configuration/context tests,
Host/CLI Clippy and Host Rustdoc passed. The rebuilt CLI passed the installed
PocketIC create/call/replay/receipt-recovery/same-canister-upgrade journey. Its
first launch was blocked by sandbox localhost binding; the rerun with localhost
access passed. Formatting and tooling agreement passed. Actual checkout app/fleet
listings preserve the profile tree and IDs. No mainnet mutation or push occurred.

## Cascading and workflow scope review

The maintainer's 2026-11-14 answers admit a reusable app definition, a generated
Rust starter with one Root-controlled application canister, persistent local
development, whole-fleet convergence, composable scaling/sharding, cascading
discovery and project-subtree operations. ICP conversion is excluded; operators
supply cycles manually. Application frontend work is deferred. The simple
handover default retains workspace evidence and separately provisions signer
authority; journal-free disaster recovery is not promised. These are accepted
design requirements, not shipped CLI capabilities. The canonical scope is in
[architecture section 1](canic-system-design.md#accepted-developer-and-operator-journeys).

Evaluation used legacy revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`.
The recommendation is to adapt the behavior through narrow owners, not move the
old Core, Coordinator/Registry and Store dependencies wholesale:

| Legacy evidence | Decision for the rewrite |
| --- | --- |
| [Topology cascade](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/workflow/cascade/topology.rs), [validation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/ops/cascade/mod.rs) and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/ops/cascade/tests/mod.rs) | Preserve exact receiver/role/parent checks, branch slicing, explicit leaf rows and refusal of missing/duplicate/foreign children. Adapt allocation-generation binding; do not add a Store just to retain its topology endpoint. |
| [Component runtime discovery](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/workflow/component_runtime/mod.rs) and [regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/workflow/component_runtime/tests/mod.rs) | Preserve local validated lookup, all exact role matches in member-path order, exact replay, monotonic directory progression and conflict refusal. Discovery is not permission or a live health check. General group/service catalogues are not prerequisites. |
| [Directory synchronization](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-control-plane/src/workflow/component_directory_synchronization/mod.rs) | Preserve intent before dispatch and exact target coverage before marking synchronization complete. Replace required Coordinator/Fleet Registry ownership with the selected management owner's committed allocation projection for the initial single-subnet journey. |
| [State cascade](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/workflow/cascade/state.rs) and [bounded report regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/ops/cascade_report/tests.rs) | Preserve local application versus descendant completion, per-target uncertainty, reconciliation failures, complete counts with bounded detail and no mutation on malformed aggregation. Use durable operation generations and bounded resumable scheduling rather than copying one recursive awaited call chain. |
| [Installed partial-failure/retry fixture](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/qualification/canic-tests/src/pic/fleet_registry/baseline/tests/state_cascade/mod.rs) | Bring forward wrong-caller refusal, immediate-parent enforcement, stopped descendant with successful sibling outcomes, unchanged-state retry completing propagation, and recovery endpoints during logical pause. Add selected-subtree isolation and overlapping-transition tests. |

History matters here: [commit c04cd56](https://github.com/dragginzgame/canic-legacy/commit/c04cd56c11d287cf5baebb472baac61706434dc5)
replaced unit/first-error cascade results with bounded per-target reports,
role-correct transport and a recheck before later sends. This addressed loss of
partial outcome information; it was not cosmetic abstraction. The pinned native
Wire implementation retains those lessons. Its
[endpoint admission tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-core/src/domain/policy/pure/fleet_activation/native_cascade_admission_tests.rs)
also show that the topology endpoint is Store-specific while managed components
receive runtime configuration. Therefore "move cascading" must include both
directory configuration and state propagation, not just the topology module.

Changed assumptions and unproven paths:

- Legacy's pre-send state-value comparison is useful but does not establish a
  durable generation fence across pause/resume/pause or out-of-order delivery.
  Bind commands and replies to generations and preserve pending work across upgrades.
- The inspected installed fixture starts at a Root; it does not establish
  arbitrary project-subtree selection, overlapping ancestor/descendant changes,
  child creation during a pause, or atomic subtree upgrades. Qualify those directly.
- The transport uses an unbounded response wait and sequential fanout. A stopped
  child returning failure is not proof of progress past an indefinitely pending
  call. Select and qualify bounded scheduling/in-flight exposure separately from
  each effect's transport wait contract.
- A configuration tree is currently descriptive. Do not reuse its `parent` field
  as endpoint authorization or management authority without an installed binding.

No legacy tests were run or copied for this documentation-only evaluation. Their
source supplies regression cases for implementation, not new qualification.
Search of current `crates/` and `apps/` found no ICP conversion implementation to
delete. Removed its prospective effect contract and scope entries; preserved
historical external conversion receipts, public wallet observations and required
cycle-provider accounting. No mainnet authority or configuration changed.

## Domain and configuration review

On 2026-11-14, the maintainer identified an implementation that put apps inside a
fleet, contradicting the existing definition of a fleet as one application
instance. The implementation and declaration were corrected to **App → Fleets →
Canisters**. This review compared domain/configuration ownership, host planning,
operation selection and recovery, inventory projections, and the narrow manager
custody graph with the architecture. It is not a full security audit of every
supplier or a production qualification claim.

| Finding | Resolution and witness |
| --- | --- |
| Hierarchy and deployment boundaries were implemented inconsistently | App/fleet selection, independent default journals and shared-journal filtering; tests cover two fleets of one app and a second app |
| Upgrade planning could retain an unrelated target before apply refused it | Refuse outside-fleet targets before writing the plan; installed CLI still upgrades an admitted target and preserves recovery |
| One physical identity could be declared in two fleets on one exact network binding | Reject conflicting membership across the declaration; equal principals on different trust bindings remain valid |
| Duplicate JSON map keys silently replaced declarations | Reject duplicate app, fleet and canister keys before selection |
| Speculative command names and stale implementation claims obscured the actual product | Keep the shipped vocabulary canonical and distinguish the one-slot manager from a general executor/Root; remove the obsolete claim that no implementation exists |

The architectural distinctions remain: local names do not grant controller
authority; config edits cannot rewrite approved requests; a shared operator or
executable does not merge fleet budgets; a report does not establish readiness or
authorize a retry. The current declarations are local selection, not an on-chain
membership registry. Rename/transfer, general activation bindings, distributed
fencing and production restore require their own admitted contracts.

The existing dispatch/callback distinction and wait-policy exposure remain
consistent with the reviewed [IC system API](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/)
and [execution properties](https://docs.internetcomputer.org/references/message-execution-properties/).
This source review does not extend the previously qualified provider windows.

Validation: 24 host tests, 15 CLI tests, the installed PocketIC deployment/upgrade/
recovery journey, M1 Clippy/Rustdoc, formatting and tooling agreement passed.
Regression cases live in [host config tests](../../crates/canic-host/src/config/tests.rs),
[CLI config tests](../../crates/canic-cli/src/config_tests.rs) and the
[installed journey](../../qualification/m1-tests/tests/journey.rs). Mainnet
canister code and operation evidence were not changed by these fixes. Comparative
adoption measurements and broader production lifecycle remain unqualified; this
review does not close M0 or admit the optional capability catalogue.

### Legacy evidence applied to configuration and terminal output

The 0.200.1 configuration increment was compared with committed legacy revision
`377fbceb3994c9d1f3033979ef4e4ce9e6b85801`. This is a scoped source and history
review, separate from the earlier working-tree inventory in the architecture.

- [Declaration mutation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/release_set/config/mutation/declare.rs)
  checked app identity, rejected duplicate names and validated the resulting
  configuration before publishing it. Current app/fleet/slot edits retain those
  protections. The legacy package bindings, reserved roles and topology compiler
  are not requirements of the current local slot declaration.
- [Configuration implementation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/release_set/config/mod.rs)
  and its [regression tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/release_set/config/tests.rs)
  keep projections on one validated snapshot, retain decoding evidence and refuse
  malformed mutations. Canic's owned `Config` and `SelectedFleet` provide the
  snapshot boundary; the existing shared durable writer owns atomic publication.
  A single-file edit does not need legacy's multi-file package-rename rollback.
- [Discovery diagnostics tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/config_discovery/tests.rs)
  escape hostile paths while preserving original evidence. This exposed raw
  config paths in Canic's three declaration success messages and incomplete
  Unicode escaping in its private fleet renderer. Both now use
  `ic_host_text::escape::terminal_text`; paths, configuration and JSON retain
  their original values. Explicit config selection needs no recursive discovery
  or directory traversal subsystem.
- [CLI configuration tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/config/tests/mod.rs)
  distinguish a fleet name from its bound app. Current tests already exercise
  `hello/demo`, multiple fleets and multiple apps. Local declarations remain
  distinct from live membership and authority observations.

History comparison included the config tests before `aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`
(parent `3700c6e1bf78cb04a60dcfaf0f789d4dd992c9e0`) and the CLI config module before
`e8605c44ffaf77ceb7ddaf85b374f42b7a48703d`
(parent `e6a061999dfae207bc20917ffefa2082e7ffe2e7`). The snapshot, escaping and
app/fleet regression cases predate those reorganizations; moving ownership did
not create the underlying guarantees.

Adapted witnesses in the current normal test paths:

| Legacy invariant | Current regression |
| --- | --- |
| One loaded snapshot survives a later config edit | `loaded_config_and_selection_retain_one_validated_snapshot` |
| Nested malformed input refuses mutation and retains its decoding source | `nested_unknown_fields_refuse_edits_without_changing_config` |
| Display escaping preserves original paths and machine output | `declaration_messages_escape_paths_while_json_retains_original_paths` |
| External diagnostics cannot inject terminal direction or line controls | `terminal_escapes_external_unicode_controls_without_changing_evidence` |

Validation: 19 host configuration tests and all 19 CLI tests passed, along with
M1 Clippy/Rustdoc, formatting and tooling agreement. The CLI was rebuilt with
`make canic`. Viewer tests required localhost access; no browser was launched.
The installed deployment simulator was already qualified for the preceding slot
declaration change and was not rerun for this presentation-only production edit.

This review qualifies local declarations and fleet text presentation, not all
legacy subsystems, every CLI error path or production management authority.

### Existing canister declarations from the CLI

The same 0.200.1 declaration path now accepts `fleet canister add --id PRINCIPAL`.
This closes the documented manual JSON step for observing an existing canister.
It uses the existing config schema, principal parser, membership validation and
atomic writer. It does not read journals or contact a provider. Existing names
remain immutable through `add`; creation in a declared-ID slot remains refused.
The same principal on another exact network binding remains a separate identity.

The declaration research above applies unchanged. The additional boundary check
read legacy [fleet import](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/fleet/import/mod.rs)
and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/fleet/import/tests.rs).
That workflow reviews exact source identities, controller transfer, code/state
clearing and bounded debits, then applies retained approval. Its tests keep new
review inputs separate from replay authority. The path history includes
`aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`; comparing the tests with parent
`3700c6e1bf78cb04a60dcfaf0f789d4dd992c9e0` found them unchanged.

Local observation declarations do not have those destructive effects, so this
increment introduces no import command, Root, capacity enrollment or approval
mechanism. The applicable invariant is that declaring an identity cannot silently
approve effects: CLI tests reject identity/network/apply inputs on `add`; host
tests refuse new creation in the declared slot and preserve config bytes on
invalid IDs, duplicate membership and attempted rebinding. Controller authority
and existence still require independent evidence at the existing observation and
deployment boundaries.

Validation: all 20 host configuration tests and 6 CLI configuration/help tests
passed, including `existing_id_declarations_are_network_scoped_and_do_not_authorize_creation`
and `existing_canister_declaration_lists_exact_id_without_deployment_authority`.
M1 Clippy/Rustdoc, formatting and tooling agreement passed; `make canic` rebuilt
the CLI. No deployment, mainnet observation or browser launch was needed.

### Initializing additional apps

`app init` now also adds a new app to an existing config. This closes the manual
`apps` map edit while retaining a single initialization command. The declaration
and snapshot research above remains applicable at the same pinned legacy
revision: refuse duplicate identities, validate the whole result, preserve
unrelated declarations and keep local configuration separate from effects.
Legacy's one-app configuration assumption does not apply to the selected
App → Fleets → Canisters catalogue; its ownership and refusal invariants do.

The host owns both initialization branches. A missing file uses exclusive
creation; an existing file uses the current sidecar lock, reload and atomic
replacement. A first-creation race may refuse one caller, but cannot overwrite a
successful declaration. Invalid existing files and symlinks are never treated as
missing configs. Duplicate app names direct the operator to `fleet init` rather
than modifying the app's fleets implicitly. JSON returns the complete resulting
config from that mutation, without an independent reload.

Regression cases cover preservation of existing IDs and journal bytes, separate
default journals for the same fleet name under different apps, concurrent first
creation, lock contention, duplicate names, malformed/oversized config, symlinks
including dangling links, the 16-app ceiling, and explicit app selection through
the CLI. This changes local declaration behavior only.

Validation: 23 host configuration tests and 7 CLI configuration/help tests passed,
along with M1 Clippy/Rustdoc, formatting and tooling agreement. The CLI was rebuilt
with `make canic`; the checkout's mainnet config and deployments were unchanged.

### Platform 0.33 adoption for the 0.200.1 GitHub snapshot

Registry availability was checked on 2026-11-14: Platform 0.33.0 is published
and non-yanked. Both workspace manifests now select it; unchanged feature sets
leave its new `nns-registry-types` codecs disabled. Testkit 0.104.0 retains
Platform 0.32.3 in its independent native harness. Canic's host graph uses 0.33.0;
the manager and M0 Wasm graphs select one Runtime/guard 0.22.3 and Types 0.20.3,
without Agent, PocketIC or Testkit in those Wasm graphs. Locked offline metadata
and consumer compilation admit both workspaces, rather than requiring matching
facade versions across independent executables.

The first-party registry review found the retained locks current: Host
FS/Process/Artifacts 0.77.2, Text 0.75.0, Storage 0.74.1, Timers 0.44.2, Wire
0.22.1, Testkit 0.104.0 and Toko Tooling 0.43.0, alongside Runtime and Types
above. Compatible manifest minimums remain unchanged; this is not a bulk
dependency refresh. Publication remains disabled for every Canic package.

Validation on Linux passed: both locked offline metadata graphs, the scoped
host/manager/M0 dependency checks, `make lint`, and `make test m0-test
m0-artifact-test m1-test profile-test growth-test`. The installed fixtures cover
creation/funding interruption, owner maintenance, retained artifact custody,
CLI deployment/recovery/upgrade, private profile preservation and autonomous
growth through all five injected interruption windows. These use synthetic cycles;
no mainnet deployment or browser launch occurred. Native macOS was not rerun.

### Slot summaries preserve conflicting evidence

The next increment corrects a presentation gap: the host already refused to
rebind retained plans after a config network edit, but the slot summary could
still label the old principal as the current slot identity. Both renderers now
compare the exact network binding before presenting a configured or retained
identity. A conflicting configured principal is also explicit beside the slot.
Operations remain in the report, with their original network and principal; no
stored plan, approval or receipt is rewritten.

This review read legacy [network resolution](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/network/mod.rs)
and [network regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/network/tests.rs),
especially `changed_anchor_cannot_rebind_an_existing_profile` and contradictory
anchor/digest refusal. Path history includes `8c87ae4c8e09cdbb6f2eb3562ecb0a821e871bcf`;
the changed-anchor case already exists in its parent
`fa92d886ef9beee824de3ec900832722fef85bd2`. Legacy derived a canonical network
identity from enrolled trust bytes. Canic currently binds the exact URL, root
key and routing principal, so it retains that existing digest encoding rather
than importing environment profiles or silently changing identity semantics.

The adapted regression keeps app, fleet, slot, URL and principal equal while
changing only the root key. Terminal and HTML must show a network conflict and
retain original evidence. It also checks same-network principal conflicts and
an unresolved operation against a newly configured ID. Host checks preserve the
previous fingerprint encoding and distinguish URL, root and routing changes.

Validation passed: all 36 Host library tests and 22 CLI tests, plus
`make m1-clippy m1-doc canic fmt-check toko-tooling-check`. The CLI tests use
loopback viewer fixtures; this increment performs no mainnet deployment.

### Readable retained-operation status

The pending 0.200.2 CLI increment gives `fleet status OPERATION` a human summary
of its existing local evidence. It uses IC Host's table and escaping APIs and
leaves `--json`, scope admission and journal ownership unchanged. It does not
query the network, infer success from retained replies, or treat an expired
approval as evidence of no effect. Upgrade creation is inapplicable only when
the retained creation class is `not_submitted`; contradictory evidence stays
visible. Receipt details are labelled historical, without a current-health claim.

Research at legacy revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801` read
[progress rendering](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/fleet/progress/render/mod.rs)
and its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/fleet/progress/tests.rs),
especially `unknown_readiness_and_all_applied_counts_never_claim_deployment_success`.
The [Host progress projection](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/ops/progress/mod.rs)
and [operation tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_operation/tests.rs)
also require exact request correlation and preserve uncertain intent. Path history
includes `afc2521ed02744c1e1a376d59dae5e2b80d9dec5`; its parent
`ac55e50334dd6479ec36f404e89e60bcfe9184d6` already has the same identity checks,
with earlier package imports. This change adapts the truthful-output invariant
to a local journal summary; it does not import legacy live progress machinery.

Regression cases cover retained replies without receipts, expired uncertain
attempts, upgrade evidence, historical receipt wording and hostile display text.
The configured CLI journey checks human output alongside unchanged JSON and
existing cross-fleet rejection.

Validation passed: the three status renderer regressions, the configured CLI
planning/inspection test, CLI Clippy and `make canic fmt-check`. A rebuilt-binary
smoke check created an isolated local declaration and plan under `target/`,
printed its status, and compared JSON with the original plan. It submitted no
effect and used neither the private mainnet journal nor a browser.

### Compact fleet terminal view

Maintainer feedback on the four-canister mainnet profile display identified
duplicated identities, stacked role badges, excessive colour and routine help
text that obscured the fleet. The default view now uses one IC Host table per
network binding, with named canisters, one principal per row, one role,
controller references and code/controller evidence. External controller
principals appear once below their network's table. Display references use
`external:N`, which cannot collide with valid configured slot names. They are
snapshot-local, not configuration or authority identifiers.

Names resolve through the same slot-state decision as conflict summaries,
including exact network binding. Conflicted or unresolved slots stay visible
without lending their names to contradictory retained identities. Existing
observation errors, unknowns, incomplete operations and inventory issues remain
in the default view. Hashes, timestamps, manager attempt counts and explanatory
text move to `--details`. The JSON and browser report remain unchanged.

Legacy research reused revision `377fbceb3994c9d1f3033979ef4e4ce9e6b85801`, reading
[list rendering](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/render.rs),
[renderer regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/tests.rs)
and [live-query failure tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/live/tests.rs).
Its shared table owns escaping/alignment; caller-selected colour is applied
after rendering, and failed observations remain explicit. Those mechanisms
already exist before path-history commit `aa02dc0db13d6ae6f2749a09f84de6f7791d0c8f`,
in parent `3700c6e1bf78cb04a60dcfaf0f789d4dd992c9e0`. The initial compact view
adapted that table approach. The following correction restores explicit tree
semantics while keeping readiness claims separate.

Validation passed: seven terminal regressions, the slot conflict regression,
the configured CLI planning/inspection test, CLI Clippy and `make canic fmt-check`.
The profile-shaped fixture checks a compact layout, one occurrence of each
principal, colour-independent alignment, opt-in detail, preserved JSON, and
name/role isolation across trust bindings. Existing escaping, failed observation
and uncertain-effect regressions continue to pass.

A read-only mainnet check with the rebuilt `canic fleet show --app profile`
confirmed four named rows, verified code/controllers, the manager's uncertified
Ready report, and five retained receipts. The command is available through the
existing `~/.local/bin/canic` link; no browser or deployment was started.

### Application tree and role correction

The maintainer explicitly values legacy Canic's application tree design. Reading
the actual [registry entry](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/registry/mod.rs)
and [tree renderer](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/support/registry_tree.rs)
showed independent `role` and `parent_pid` fields. Tree order follows immediate
parents, not IC controllers or routing. The renderer sorts siblings, preserves
parent-before-child ordering and exposes children whose parent is outside the
visible inventory. Its [list regressions](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-cli/src/list/tests.rs)
exercise nested branches, subtree selection and repeated-role ambiguity.
Path history includes `d32fa61ed431e40fdda25c6dc05963f49b52e1f5`; the same tree
and ambiguity handling exist in parent
`51e99819cf55ddc7f9b5f27541a8185ad33d76e9` before its error-type migration.

The research also read legacy [topology planning](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_topology/mod.rs),
its [tests](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-host/src/component_topology/tests.rs)
and [child-binding validation](https://github.com/dragginzgame/canic-legacy/blob/377fbceb3994c9d1f3033979ef4e4ce9e6b85801/crates/canic-contracts/src/deployment/topology/validation/mod.rs).
Those protect admitted roles, immediate parent identities, placement and resource
bounds. A display relationship cannot substitute for those authorities.

Current concrete slot declarations now have optional application-defined `role`
and immediate `parent` slot fields. Validation rejects absent/cross-fleet parents,
self-parenting, cycles and invalid role names within the existing config bounds.
`canister add --role ... --parent ...` uses the existing atomic config writer.
Repeated roles are allowed; commands continue to select unique slots. Listings
and HTML declarations share one tree layout, while observed rows bind names and
roles only on the original exact network and unambiguous slot identity. A missing
or conflicted parent is reported and its observed child remains visible without
nesting. The renderer is bounded against malformed cyclic snapshots as well.

The profile config declares entry, worker and manager responsibilities and only
the manager/managed-worker parent edge. Entry/shared-worker discovery is still
a routing relationship. Controller observations stay in their own column;
controlling a child no longer renames a manager to `controller`. Config role
names do not select query adapters, authorize effects or renew existing plans.
General topology deployment and dynamic-child admission remain unimplemented;
this increment preserves the model without importing a Root/Coordinator stack.

Validation passed: 24 Host config tests, two tree regressions, seven terminal
regressions, the slot conflict regression and eight CLI context/config tests;
Host/CLI Clippy, Host Rustdoc and `make canic fmt-check toko-tooling-check`.
The tests preserve named plan identity after role/parent edits, reject malformed
topology without replacement, retain unknown observations and separate routes
from parentage. Public canister identities, journals and network selections
remain unchanged.
The rebuilt read-only mainnet view confirmed the manager role and nested managed
worker, with four observed canisters and five retained receipts. No management
effects or browser launches were performed.

## First capability and consumer invariant

M1 exercises build, reviewed installation, invocation and status of one greeting
canister, followed by a changed executable with explicit preservation semantics.
It needs no database, browser frontend, Fleet admission or autonomous scaling.

The proposed M2 capability is managed installation and readiness of a shared
stateful worker, followed in M3 by a qualified maintenance-window upgrade.
The application retains user registration, routing, database schema and resource
authorization. The selected consumer is the Canic-owned profile demo; Toko
integration is outside the requested scope.

The first multi-canister demo also requires a runnable browser fleet view,
delivered with that increment and no later than M2. The team must be able to
inspect deployed canisters, their relationships, observed state and interrupted
operations. The [visualization acceptance contract](canic-system-design.md#demo-fleet-visualization-from-the-first-multi-canister-increment)
owns the scope; the local growth slice extends the same view to an autonomous
worker using `--manager`. The CLI supplies a read-only `canic fleet show` JSON/HTML snapshot,
qualified against two local greeting deployments, retained uncertain upgrades and
an unavailable provider. It groups principals by exact network trust binding and
shows certified module/controller observations separately from historical receipts.
The report remains a point-in-time view of locally retained deployments; stateful
cycle balances, broader application call edges and general autonomous discovery still belong
to the full multi-canister acceptance gate.

The design also admits an optional `ic-relay` module alongside direct platform
calls. Its [managed pilot and comparison](canic-system-design.md#direct-calls-and-optional-ic-relay)
use two demo canisters and one bounded application command once that journey is
available. Select a compatible supplier revision and measure batching gains,
latency, storage and recovery against the direct path before adoption. Relay's
existing supplier experiments do not qualify this rewrite. This choice neither
adds a Relay dependency to M1 nor expands the M0 closure conditions.

The selected invariant is concrete: an application-supplied user ID and its owner
principal mapping remain unchanged; private reads/writes by another caller remain
rejected. The upgrade preserves the profile name and revision. The application
owns these records; Canic's deployment layer does not reinterpret them.

On 2026-11-13 the maintainer selected a simple Canic-focused journey and excluded
Toko integration. The earlier read-only Toko observation at
`44d4e2c6d41e3575b2503e79ffa369129ff3ecf2` remains research only. No Toko build,
port, dependency update or adoption measurement is needed to advance this demo.

## Topology candidate and retained responsibilities

### Runnable stateful baseline

Use a Canic-owned profile demo: a standalone worker owns a
supplied nonzero user ID, immutable caller-to-user mapping, private profile read,
bounded display name and optimistic revision. The native demo client invokes
that application's Wire contract through ICP CLI. The existing Canic deployment
path installs the worker; its domain, storage and endpoints remain application-owned.
This is the selected hands-on consumer and baseline for subsequent managed
composition. A standalone worker does not by itself complete M2's multi-canister,
management-effect and composition requirements.

Before implementation, select these initial qualification ceilings: 16 profiles,
128 UTF-8 bytes per display name, 4 KiB RPC frames, two declared Storage
allocations and at most 32 physical stable pages (2 MiB). The complete worker
must fit the current CLI's 1,500,000-byte install limit. No timers or idle work
are admitted. Measure installation and full-capacity footprint against these
bounds; report supplier overhead separately from profile payloads. IC Storage
0.74.0 owns the backing and collection engines. Do not reuse raw-memory M0
fixtures as the worker's persistence implementation.

Executable B changes its reported application version while keeping the same
record format. Qualification must preserve IDs, owners, names and revisions,
reject unauthorized reads/writes and stale revisions, permit exact replay without
another revision, and refuse the first profile beyond capacity. A failed upgrade
must retain executable A and its data. The entry increment is recorded below;
management autonomy remains separate. This baseline creates no
Coordinator, Store or generic runtime framework.

The original standalone rehearsal passed on 2026-11-13 (`make profile-test`). Canic created
and installed the worker on PocketIC; the native client read readiness through
ICP CLI. Distinct PocketIC caller identities exercised private reads, immutable
ownership, bounded registration and revision replay. At full capacity, a failed
post-upgrade retained executable A and its records; a subsequent Canic upgrade
to B preserved all records and refused further registration. Reapplying the
completed operation with no signer returned its retained receipt. The generated
fleet view identified the current module and both deployment operations.

| Baseline measurement | Observed | Selected ceiling |
| --- | ---: | ---: |
| Raw worker Wasm | 238,598 bytes | 1,500,000 bytes |
| Code section / functions | 162,328 bytes / 394 | Included in total Wasm |
| Initial stable memory | 1,179,648 bytes (18 pages) | 2,097,152 bytes (32 pages) |
| Stable memory with 16 profiles | 1,179,648 bytes (18 pages) | 2,097,152 bytes (32 pages) |
| Initial / full-capacity Wasm memory | 1,179,648 / 1,179,648 bytes | Measured baseline |

The initial stable allocation is supplier/collection overhead with no profiles.
The filled fixture adds fifteen maximum-length names and one shorter updated
name (less than 2,048 name bytes), plus IDs, owners, revisions and record encoding,
without another physical page. This measures allocated
memory, not exact live heap or a comparative efficiency claim. There are no
application timers. The worker owns both endpoints and lifecycle hooks; Storage
owns backing, allocation admission and collections. The native client imports
the application protocol and host process supplier, not the worker implementation.

Original standalone inputs (SHA-256; preceding the entry increment):

- Root lockfile: `924c4a447808a40964daf8e326635a8bc493df6ffcc139748e8e83750902ea65`.
- Qualification lockfile: `03c2afa061914716f75d673b2ad2957dfa8b548ec3ecb5df3fa67bbb24925553`.
- Executable A: `1ec70bb3652ed781ac08476e9d36fd9fc067b223f36d33a651a25d6ca6f3c774`.
- Executable B: `edbb2137db47e6bd1d9802d5b90bb104ac5424b3cc5a74e895f54bc9876aefa4`.

The test source is `qualification/m1-tests/tests/profile.rs`; the build entrypoint
is `scripts/dev/test-profile.sh`. This qualifies the unchanged storage schema
across two executables, not management-operation preservation or entry/worker
composition. Mainnet use is described in [the demo guide](../demos/profile.md).

### Entry and worker composition

The next installed increment passed on 2026-11-13 using the same focused
`make profile-test` entrypoint. Canic deploys both canisters. The entry owns one
immutable worker address in an IC Storage cell; its controller may bind it once
or replay the exact binding. Other callers, invalid targets and replacement
bindings are refused. Its separate lifecycle strictly restores the binding on
upgrade. The worker's storage schema and ownership rules remain unchanged.

The client resolves that route through a certified replicated response, then
sends the original user's signed operation directly to the worker. No forwarded
caller assertion or impersonation authority is introduced. The entry separately
makes a controller-requested, bounded direct `probe` call to the worker. One
active check, a five-second wait, zero attached cycles and no retries bound that
observation; a callback failure can occupy the volatile slot until an entry
upgrade clears it. No management effects or spending allowance depend on that slot.

Qualification covers unbound and unauthorized requests, same-binding replay,
replacement refusal, oversized frames, native-client resolution, anonymous
identity preservation, direct worker readiness, strict entry upgrade restoration,
stopped-worker failure and successful observation after restart. The generated
fleet report contains two canisters, three deployment operations and an explicitly
operator-declared route. That arrow is not an on-chain observation. Existing
profile preservation and wrong-owner rejection cases remain in the same journey.

The current worker is 238,276 raw Wasm bytes (162,903 code-section bytes, 391
functions), with unchanged 18-page stable allocation. The entry is 325,566 raw
Wasm bytes (229,961 code-section bytes, 446 functions) and uses 10 stable pages
(655,360 bytes), within its selected
1,500,000-byte install and 16-page stable ceilings. It owns one named allocation
and no timers. These are allocation/size measurements, not a performance comparison.

Current input SHA-256 values:

- Root lockfile: `19685a9502f37b44116b7c37540bbc0f530ac91d66745f79246c6c2639eabf5c`.
- Qualification lockfile: `69b432f33babce34bd1badba9fee5a7d80800500c4ce0939b3a7f249ca183c14`.
- Worker A: `16dcabb91273504d2bc4f4cced4d0f68b2525a14c398fd8b3914aea5b520b638`.
- Worker B: `2017e8b6d4fe89ff7bd1fbd6f18ea0faa9f6b413ffc49ef3e5cfef0140aa8e85`.
- Entry: `9a8fe4927940940512f99fef91fa8784ea835aca2202622dc0453188d1bf8bfd`.

This proves discovery, direct observation and separate persistent ownership.
It does not yet qualify managed growth, surviving management-operation custody,
Relay transport or fleet-wide admission. M0/M2 remain open for those selected
management requirements; the entry is not the management-effect owner E.

### Management-effect owner candidate

Use R → E → application as the first experiment candidate. R is the operator
principal; E is one subnet-local management owner. Compare direct-host bootstrap
for its authority and cost, not as a substitute for operator-absent execution.
Select the product arrangement after the custody experiment establishes its limits.

E retains intent, reservations, exact effect identities and committed outcomes.
R retains E's maintenance operation and the inventory/exposure needed to recover
it outside E. Application code cannot clear E's unresolved obligations.
For autonomous installation, first evaluate a bounded set of approved artifact
bytes retained by E before granting growth authority; unplugging the host must
not remove those bytes. No Coordinator or separate Store enters this experiment.
Artifact size limits and storage costs must be measured before selecting that path.

## First effect experiment

The [runnable fixture](../../qualification/README.md) exercises actual management
creation and direct-cycle funding with explicit trap injection. It introduces no
product operation engine or configuration language.

| Boundary | Evidence the experiment must establish |
| --- | --- |
| Host submission | Exact signed-attempt bytes/identity survive process interruption before submission; restart does not invent another attempt |
| Executor dispatch | Intent, reservation and outgoing-call enqueueing commit together; a trap before return produces no external call from that execution |
| Returned result | Creation principal or funding completion/refund evidence survives a committed callback; writing then trapping in that callback is a separate negative case |
| Subsequent work | A later message can fail without erasing the committed outcome; reconciliation uses that outcome without repeating the paid effect |
| Maintenance | R can stop/upgrade/recover E while keeping its operation and exposure outside the affected canister; governance is investigated only if selected |

The [system API](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/)
defines the dispatch commit boundary; the
[execution properties](https://docs.internetcomputer.org/references/message-execution-properties/)
define rollback within a trapping callback. These are specification evidence,
not results for a selected executor implementation. Rust `await` syntax alone
does not establish a new message boundary.

Classify each injected window as proven no effect, proven effect with retained
evidence, reconstructible uncertainty, or blocked uncertainty within a stated
bound. Harness inventory may establish what happened in a test, but cannot be
used as a production recovery mechanism unavailable to R or E. Never infer the
created principal from inventory similarity or payment identity from balances.

### Creation results on 2026-11-13

`make m0-test` passed on Linux x86-64 with Rust 1.100.0, PocketIC 16.1.0,
Testkit 0.102.0, Platform 0.32.1, Runtime 0.22.1 and Wire 0.22.0. The lockfile
records the complete selected graph. The maximum-evidence codec check and all
five installed scenarios passed; native/Wasm Clippy, Rustdoc, shell and tooling
agreement checks also passed.

| Injected window | New canisters | Committed executor evidence | Outcome |
| --- | --- | --- | --- |
| After enqueue, before dispatch commit | 0 | Empty slot; no reservation | Proven no creation effect; execution still costs cycles |
| Before result write | 1 | Issued intent, no principal, 2T cycles exposure | Blocked uncertainty; another creation is refused |
| After result write, in the same callback | 1 | Issued intent, no principal, 2T cycles exposure | Blocked uncertainty; writing the record did not commit it |
| Separate continuation after result commit | 1 | Principal/refund retained; continuation write rolled back | Original outcome reconstructed by reading committed evidence; replay creates nothing |
| No fault | 1 | Principal/refund retained; child controlled by executor | Proven effect with retained evidence; replay creates nothing |

The implementation consequence is to keep irreplaceable-result capture small and
commit it before fallible subsequent work. This does not eliminate traps before
that commit. A single uncertain attempt blocks this fixture's creation capacity;
product acceptance of that blocked exposure and its disposition remain decisions.
No claim of automatic recovery for every creation window is justified.

### Direct-cycle funding results on 2026-11-13

The same `make m0-test` run passed all ten funding interruption cases: five
windows against each of a live empty recipient and a deleted recipient on the
same subnet. Each fresh executor attaches 1T synthetic cycles. The
[management API](https://docs.internetcomputer.org/references/ic-interface-spec/management-canister/)
defines `deposit_cycles`; the selected path uses an unbounded call and captures
the matching callback refund before further work. No ledger is involved.

| Injected window | Live recipient | Deleted recipient | Committed executor evidence |
| --- | --- | --- | --- |
| After enqueue, before dispatch commit | No credit | No deposit dispatched | Empty slot, zero reservation; proven no deposit effect |
| Before result write | Credited 1T | Attachment refunded | Issued intent, no receipt or refund observation, full 1T exposure blocked |
| After result write, in the same callback | Credited 1T | Attachment refunded | Same blocked uncertainty; uncommitted writes were discarded |
| Separate continuation after result commit | Credited 1T | Attachment refunded | Completion/zero refund, or rejection code 3/full 1T refund retained; zero unresolved exposure |
| No fault | Credited 1T | Attachment refunded | Same retained outcomes; identical replay returns evidence without another deposit |

For dispatch traps, repetition still produces no credit. For every occupied
slot, changed amount and changed destination are refused. Admission tests also
reject zero/over-limit amounts, oversized principal bytes and non-controller
callers without consuming the slot, then successfully admit one valid deposit.
Both maximum-evidence Wire codec checks pass under the original finite limits.
Native/Wasm Clippy, Rustdoc, formatting and shell checks pass.

Recipient balances and executor balance decreases are independent test oracles,
not inputs to recovery. In the deleted-recipient callback-trap cases, observed
executor decreases were 13,337,427 and 13,337,441 cycles respectively, rather than
the 1T attachment. This establishes a physical refund in this fixture but supplies
no committed operation receipt to the executor. It therefore keeps 1T reserved
and refuses replay. A reject code alone would not release exposure: the probe
retains any unrefunded portion even when a rejection record commits.

The bounded result is deliberately conservative: no callback-loss window was
reconstructed from balances. A retained receipt survives later work; absent that
receipt, a single uncertain attempt blocks the funding slot. Product acceptance
and disposition of this bound remain open. Local dispatch-resource failures,
cross-subnet deposits and upgrade persistence have not been qualified here.

### Qualified inputs

Creation was rerun successfully after sharing its dispatch injection with the
funding probe. SHA-256 digests below identify that combined passing run; paths
are relative to the repository:

```text
6a0f87ca7a9b37a369e6a8f8fd020da8de0268c0266767b4022121a57638f709  qualification/Cargo.lock
5d2801ce434ba03e75d53acf8944eaa9678f8850d3b92984f73d389c777253d8  qualification/m0-probe/src/lib.rs
add2c1182af05ae42107016b8d799e755e07dafee88dce6d6b48a3dcf9da81da  qualification/m0-probe/src/canister/creation.rs
b943d5b8079997ce47bbd8ccde6c17f474c35df0a4f2fe386c874941c08f4cc2  qualification/m0-probe/src/canister/funding.rs
e92fcc8110ade8fbf27ce0389108c019c7a87423662ce9c37045709afd942d8a  qualification/m0-probe/src/canister/mod.rs
6cdbc9aeb059bc5d1bdb1f8fa0d20c757bc2d676e415c1a6735873b9e08765ae  qualification/m0-probe/src/tests.rs
097984a2af24b618afda29c1b47055874360255266ba2c966a72b93af0c77cb9  qualification/m0-tests/tests/creation.rs
b742937cc45f496e7a48a0013430dce579e10b7afeebd19ab4f63868c5dfc48b  qualification/m0-tests/tests/funding.rs
18f497e7f4d307e816dd4e2c074f29e3640f699c5537f27bccfba27bfad9fcde  qualification/m0-tests/tests/support/mod.rs
bc1737fc22075d576fe26887b098052b7412bcbe72d5958a15498022e44bbdde  qualification/target/wasm32-unknown-unknown/release/canic_m0_probe.wasm
b44e1eccd66e02328146b3e209e3403e3a3428a25d25555aca2f67500e5ae9db  target/m0/server/16.1.0-x86_64-linux/pocket-ic
```

### Owner-maintenance results on 2026-11-13

`make m0-test` passed again with the maintenance hooks, on Linux x86-64 with
Rust 1.100.0, PocketIC 16.1.0, Testkit 0.102.0, Platform 0.32.2, Runtime 0.22.2
and Wire 0.22.1 from the current qualification lockfile. This includes the original
five creation cases, ten funding cases, funding admission and both bounded codec
checks. `make m0-maintenance-test` selects the new rehearsal alone.
Native/Wasm Clippy, Rustdoc, formatting and shell checks also passed. Native
macOS execution remains unqualified.

Six maintenance cases cover empty journals after dispatch traps, retained
creation/deposit successes, lost callback evidence, retained evidence after later
failure, a retained funding rejection/refund and a rejection whose callback
evidence was lost. Each case establishes R → E and, when its creation result is
retained, E → child. R is a distinct simulated operator principal; an unrelated
principal cannot upgrade E.

The harness retains original evidence outside E, stops E, and attempts a changed
executable whose post-upgrade hook restores the journals, overwrites stable bytes
and traps. The old module, stable bytes and both journals survive unchanged. R
can restart E, then stop and successfully upgrade it to the changed executable.
Repeating the failing upgrade with populated stable memory also preserves the
running candidate and its evidence. These observations agree with the atomic
upgrade contract in the [system API specification](https://docs.internetcomputer.org/references/ic-interface-spec/canister-interface/#canister-upgrades).

Replaying the original commands after maintenance creates no extra canister or
deposit. Lost outcomes still reserve the full 2T creation and 1T deposit exposure,
even when the test oracle observed a refund. Retained outcomes remain readable;
the upgrade does not turn uncertainty into another allowance.

The test owns one raw 64 KiB stable page containing two independently bounded
Wire frames. It reuses the existing journal schemas and limits. This is an
isolated persistence experiment, not a production Storage integration. Host
observations are retained in harness memory, so the result does not qualify a
durable R journal or host crash recovery. It also excludes callbacks still in
flight, schema migration, recovery from an unconditionally trapping pre-upgrade
hook, governance and mainnet executor maintenance. Those limits prevent this
result from closing the entire infrastructure-recovery requirement.

The qualification lockfile SHA-256 is
`9d196c90c9f4ed0dbc5b95b2cf8d218da7c330d074f6d90c94424ce58676285a`.
The two separately retained executables are `target/m0/original.wasm`
(`bd1624f19e1f569f7267b983d5d22fd1b300617ebdce7ac8c80e24fecb180094`)
and `target/m0/maintenance-candidate.wasm`
(`8e91ccfef31e28fd0074effc2ef88f243a1986c3491550a4a5c524897e6c558c`).
The maintenance source SHA-256 is
`e70078f9cc972fad74b8c93f2bfc1c4f001fc4493a969bc45877a5c571eed12d`;
its installed test is
`1ed73914f578c3722625c0507841a8ce40044eea1d29e7ba5371a00f91bcae90`.

### Artifact-custody results on 2026-11-13

`make m0-artifact-test` passed two native admission/codec checks and four installed
scenarios on Linux x86-64, Rust 1.100.0 and PocketIC 16.1.0. The selected suppliers
are Platform 0.32.2, Runtime 0.22.2, Wire 0.22.1, Testkit 0.102.0 and SHA-256
implementation `sha2` 0.11.1. Native/Wasm Clippy, Rustdoc and shell checks passed.

The fixture retains one approved raw Wasm, exact hash and target before a future
dispatch time. A failed upgrade preserves the approval; a successful upgrade
restores it into a changed executable. Advancing only the simulator clock and
rounds then installs the actual Hello World worker, which returns its greeting.
No operator update or host artifact delivery occurs during that interval.

Platform's released direct `InstallCodeArgs` export is native-only. The qualified
on-chain alternative uses `upload_chunk` into E's own management chunk store,
then `install_chunked_code` with the approved hash and target, following the
[management API](https://docs.internetcomputer.org/references/management-canister/#install_chunked_code).
There is one chunk and no separate Store canister. Each request commitment is
retained before dispatch; an unknown result does not create another attempt.

| Case | Retained evidence and disposition |
| --- | --- |
| Successful autonomous installation | One upload and one install; callback retained; module hash and sole executor controller verified |
| Upload callback writes then traps | One upload, zero installs; original upload commitment remains blocked; physical chunk presence is only a test oracle |
| Install callback writes then traps | One install, no committed success reply; after executor maintenance, module/controller observation reconstructs the installed binding without reinstalling |
| Target already has code | Installation rejected; original code/version retained; approval cannot be replaced to retry |

All cases also reject unauthorized staging, oversized upload frames, wrong hashes
and changed approvals. Replaying the original approval never repeats an upload
or install. Worker versions independently prove no reinstall; observing the
installed binding does not manufacture a missing callback receipt. These claims
assume the selected sole executor and its operator do not independently replace
the worker; application readiness remains a separate future obligation.

The rebuilt worker is 43,632 uncompressed bytes; its upload frame is 43,699 bytes.
At the observed post-install point E uses 1,441,792 bytes of Wasm heap, 65,536
bytes of stable memory and 1,048,576 bytes of IC-managed chunk storage. These are
total fixture measurements, not incremental attribution or a production budget.
The 65,536-byte artifact ceiling fits a 65,619-byte upload and a 65,758-byte
maximum-evidence checkpoint, requiring two stable pages. Chunk storage is retained
after this experiment; cleanup and retention economics have not been qualified.
In the successful 61-second simulated interval, E's balance fell 374,600,091
cycles and the worker's 136,804,576 cycles. These include fixture execution,
storage time and observations; they are not installation-only costs or mainnet
prices. The heartbeat is test scheduling, not the product's idle-cost policy.

This supports executor-local custody for the bounded first autonomous experiment.
The host-only path remains simpler for M1 but requires the operator to supply
installation. A separate Store would add a canister without a demonstrated need
for this one-artifact workload; its costs and benefits have not been measured.
Product selection still needs governed storage, demand-driven scheduling and the
stateful consumer's actual artifact budget. This fixture pre-creates the worker
and does not yet compose creation, installation and readiness into the M2 path.

SHA-256 inputs: qualification lockfile
`a14cd0d337142a9dc550e4fa3c2e5f9c14de6eabca99164523de3071e52b4bf9`;
executor A `3ebb48eecbc840651390fcdb727a88f37e0726127753193dc43ee7ea994986bd`;
executor B `b4fc51bb87a334050c61c97d23b8b1657db7630437eea66aec83f6fa63d59d60`;
worker `0ac16f95cca67500fc8f6615012365cd81fcc09bde253035cde783476e89635f`.
The executables are retained separately under `target/m0/artifact/`. The earlier
digests identify their own qualification runs, not this expanded lockfile.

## First CLI journey on 2026-11-13

The direct-host arrangement is selected for the stateless M1 greeting: one
operator controls one application canister. Host retains the approval, artifact,
signed attempts and returned results in `.canic/`; ICP CLI retains key custody.
The application-scoped manager selected below supplies the first bounded enduring
owner; this host-only path remains its bootstrap and comparison.

`make m1-test` passed on Linux x86-64 with Rust 1.100.0, PocketIC 16.1.0 and
repository-prepared ICP CLI 1.7.0. Eight host checks cover plan/artifact admission,
writer exclusion, immutable retained bytes, external creation DTOs, signed
request/status binding and refusal to replace expired uncertain attempts.
The installed test invokes the actual CLI process to deploy executable A, call
`Hello from Canic, team!`, recover its receipt without a signer, and upgrade the
same principal to executable B returning `Hello again from Canic, team!`.

The harness also removes the saved creation reply, and later the upgrade reply,
to model result loss after network completion. Apply recovers using the original
signed requests, without a signer, unchanged signed bytes or an extra canister.
These cases qualify resubmission within the original validity window. They do
not emulate host power failure; synchronized file/parent publication belongs to
Host FS. Expired unresolved attempts remain blocked. Deleting the only real
deployment journal is never a recovery instruction.

Native/Wasm Clippy, library Rustdoc, formatting, shell and tooling agreement
checks passed. The demo's Wasm dependency graph has no Host, Agent, CLI or simulator
dependency. Qualification invokes the CLI process so PocketIC's older pinned
`thiserror` does not force a downgrade of the product graph.

An anonymous mainnet `icrc1_fee` query returned 100,000,000 cycles, matching the
[provider contract](https://github.com/dfinity/cycles-ledger/blob/main/README.md).
The operator subsequently completed ledger/CMC creation and mainnet installation
through this CLI. Native macOS execution remains unqualified. This path assumes
the ledger fee remains unchanged
between preflight and submission, and one coordinated operator uses the retained
state directory. Cross-host fencing and concurrent external upgrades are not
qualified. The stateless demo's upgrade proves principal/controller preservation,
not the M3 application-data or infrastructure-journal promise.

The preceding M0 input digests identify their original combined probe run. The
qualification lockfile now also includes the M1 process harness; that addition
does not replace the older evidence's source identity.

### Mainnet observation on 2026-11-13

The operator applied operation
`50dea196b16ff449b86f0000fde720971745b50f8b1c349bdc3e0dc7cdb1f786`,
creating `z4fqh-uqaaa-aaaan-q6yrq-cai` under the sole controller
`o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe`.
The retained creation and installation replies and terminal receipt are present.
An independent completed-apply invocation with no available signer revalidated
the certified controller and module hash
`cdb19d3a2a115eea571730fe9598af5b0319e9aa1b3545f9c595cb3ac19dd42b`.
A fresh, explicitly uncertified greeting query returned
`Hello from Canic, team!` at approximately 17:04 UTC.

The next executable's hash is
`187f6d99668f93609c8adc72a800fb6c0ca1f8ff2d85147f22497137f0f7832c`.
Its prepared upgrade binds the same canister and exact previous module, with
zero creation funding. Mainnet execution of that upgrade remains pending; the
local same-canister rehearsal already passed. A subsequent read-only fleet
snapshot found the original one-hour upgrade approval expired with both effects
still `not_submitted`; a new reviewed plan is needed before signing. This does
not authorize replacement of an uncertain submitted attempt. Authority-bearing signed files
remain in `.canic/`, outside Git; these observations are not a backup of them.

## Bounds and preservation witness

### Profile mainnet deployment and preservation

On 2026-11-13 the operator unlocked `canic-mainnet` directly in a desktop terminal.
ICP CLI minted 2,000,000,012,799 cycles (rounding around the requested 2T), bringing
the ledger balance to 2,405,555,110,034. The conversion result is retained in
`.canic/profile-mainnet-funding/result.json`; it must not be used to justify
repeating a payment after uncertainty. Each following creation had an approved
1,000,000,000,000-cycle debit ceiling, including the ledger fee.

| Step | Canister | Operation digest |
| --- | --- | --- |
| Worker creation/install | `4pkhu-oaaaa-aaaai-ra2ta-cai` | `fe6ff221535c6938683da088b22abd72b94776e166b094988dbae1dad85a4563` |
| Entry creation/install | `bisqm-biaaa-aaaas-qhfeq-cai` | `b2aa17980e465da49cd9f416f5665161ad972aee00b6d0c6167ede4d5a4be3e7` |
| Worker upgrade | `4pkhu-oaaaa-aaaai-ra2ta-cai` | `96c32c1933b72bb68ea99475497f09fe1d885c5251418dd407b877f9fe7f9be3` |

The creation artifacts match the worker A and entry hashes recorded above. The
upgrade installed worker B (`2017e8b6d4fe89ff7bd1fbd6f18ea0faa9f6b413ffc49ef3e5cfef0140aa8e85`).
Certified metadata independently confirmed both current modules and the original
sole controller. All three terminal receipts and exact attempts remain under
their original operation directories in `.canic/`.

The entry's binding and direct readiness check succeeded. User 1 was created,
updated to revision 2, read, and then read again after the worker upgrade. The
owner bytes, user ID, display name and revision matched exactly. These private
reads remain uncertified queries; the write replies, deployment metadata and
entry's replicated readiness response use certified paths. The post-upgrade
readiness response reported `profile-v2` and `ready: true` under the same route.

The final account observation was 405,555,110,034 cycles and 6.42852601 ICP. The
browser snapshot contained both profile canisters and the existing Hello World
canister, with no inventory or metadata-observation failures. Its declared route
remains distinct from observed controller/module metadata. Localhost serving
does not imply that the canisters themselves are local.

### Remaining management witness

Start with one executor, one application and at most one unresolved paid attempt;
an unresolved creation pauses new creation instead of replenishing its allowance.
Use synthetic local cycles first. Record attached-cycle ceilings, protected
recovery reserve and observed fees separately. The operator restored
`canic-mainnet`; the first proposed deployment budgets 1T cycles from its default
cycles-ledger account. Approval is an explicit apply of the reviewed digest and
requires local identity unlock. Local results do not authorize a live debit.

Measure the Hello World setup and operator steps first. Before M2, measure the
selected stateful consumer's current lifecycle glue, endpoint/memory/timer owners,
dependencies, total Wasm and code-section sizes, initial stable memory and idle
work. Record numeric acceptance budgets before implementing that adoption slice;
no footprint or cost advantage has yet been measured.

For M3, executable B changes observable application behavior while preserving
the user ID, owner mapping, profile revision and private-read rule from executable A.
Across the transition, an interrupted management operation retains its original
identity, evidence and outstanding exposure at its surviving owner; B cannot
turn it into a new allowance. Separate this from the experiment upgrading E:
application preservation does not prove infrastructure-owner recovery.

M0 closes on the bounded effect results, selected topology/custody/artifact path,
agreed consumer scope and baseline, numeric budgets and this concrete preservation
scenario. A documented blocked window is acceptable only with a bounded exposure
and an explicit disposition; it is not automatic recovery. The profile mainnet
upgrade is now qualified above; the earlier expired Hello World plan remains
unsubmitted. The standalone
consumer baseline and budgets are now recorded above. The manager below selects
the first product custody/scheduling path. Normal mainnet growth has passed;
comparative adoption measurements remain outstanding, and broader lifecycle work
needs consumer admission. These
results alone do not close M0.

## Application-scoped autonomous manager: local qualification

On 2026-11-13, select `operator → profile manager → one new profile worker` for
the first autonomous slice. The manager owns management authority, the immutable
one-use approval, retained worker Wasm and operation journal. The worker owns
its application lifecycle and private records. The existing mainnet entry keeps
its immutable original route; the new worker is addressed directly. This is not
a subnet-wide executor or general scaling policy.

Two IC Storage cells and one IC Timers 0.44.1 registration retain the work through
operator absence and manager maintenance. No separate Store or Coordinator is
needed for this bounded artifact set. The operator's controller authority supplies
the maintenance path; no governance arrangement is selected.

Qualification bounds: one approval; one create/upload/install issuance each;
1T creation attachment; at least 0.5T liquid reserve before each effect (plus the
attachment before creation); 512 KiB raw worker Wasm; 64 stable pages. Status
frames remain 4 KiB, with a 64 KiB allocation allowance and 1,024 nodes. The larger
staging envelope accounts for Wire's conservative allocation charge; native tests
cover maximum artifact size and fully populated evidence. These are resource
ceilings, not a lifetime funding or total execution-cost guarantee.

`make growth-test` passed with the actual Canic CLI and profile client:

| Evidence | Observed result |
| --- | --- |
| Admit once, then no mutating operator ingress | One ready worker, sole manager controller, same subnet; counts `[1, 1, 1]` |
| Authenticated profile write on the created worker | Original caller owns the retained profile |
| Distinct manager maintenance executable | Original authority, child and dispatch counts survive; no additional child |
| Trap before create dispatch commit | No child or committed attempt; original scheduled authority resumes after maintenance |
| Create callback writes then traps | Child exists, principal/refund absent; `CreateIssued` never creates again |
| Separate upload message traps before dispatch | Previously committed child principal survives |
| Upload callback writes then traps | Stored-chunk observation reconstructs outcome without another upload |
| Install callback writes then traps | Module/controller observation reconstructs outcome without another installation |
| Changed approval, unauthorized caller, bad hash | Refused without replacing original authority |
| Wrong native network/root key, expired dispatch, inadequate admission reserve | Refused or stopped; no extra child |
| Exact approval replay, including after maintenance | Same slot and evidence; no new allowance |

Independent subnet counts verify effects; the manager is never given that oracle
to recover a lost principal. Creation-result loss remains blocked with full 1T
attachment exposure after maintenance. This is an experimental limitation, not
automatic recovery or production approval. Intent and outgoing-call issuance
commit with their IC message; callback writes commit only on successful message
completion. The next effect starts in a separate timer message, so a later
failure cannot erase a committed outcome.

The native growth file binds the selected network/root key and exact Wire request,
retaining both before invoking ICP CLI. Manager bootstrap uses the existing signed
host-attempt journal. Growth admission instead relies on the manager's identical-
approval idempotence; the client does not claim custody of ICP CLI's signed ingress.

Measured default manager: 684,139 raw Wasm bytes, 500,616 code-section bytes,
875 defined functions and 18 physical stable pages (1,179,648 bytes), including
retained worker bytes. The management chunk store additionally retains one worker
module; its bytes are outside the stable-page measurement. SHA-256:
`dd316c9ab7bc628abea68a5384eff3c689a12476382b413f26b8841e17e925b8`.
Worker: 238,329 raw bytes, SHA-256
`0be0704fb8e13a43650b457a98dfb88ee4a409ff349c6a03b53ab5603845b2a5`.
These are whole-artifact measurements, not supplier attribution or evidence of
size/cost savings. Earlier mainnet artifact hashes remain historical.

The fleet snapshot reports both manager and created worker. The manager journal
is labelled uncertified; module/controller metadata is independently certified.
Observation never resumes work. See the [growth guide](../demos/growth.md) for
reproduction, mainnet preparation and maintenance limits. No autonomous manager
was deployed to mainnet in this qualification run.

## Autonomous mainnet worker

On 2026-11-13, deploy the exact default manager qualified above through operation
`2cfe76adf294be8d03c2afeca2f5f64dc4be51ca4ba470dd66517f629dd6ce42`:

| Role | Principal | Sole controller |
| --- | --- | --- |
| Manager | `rk6n2-wyaaa-aaaad-qmoua-cai` | `canic-mainnet` (`o5trf-oqyg7-cawjp-xs4pw-aomb3-iwki5-hyezf-qahfz-j3ffd-jh4fc-oqe`) |
| Worker | `rn7lo-3aaaa-aaaad-qmouq-cai` | `rk6n2-wyaaa-aaaad-qmoua-cai` |

The initial wallet observation was 6.42852601 ICP and 405,555,110,034 cycles.
The single requested 2.5T conversion deposited 2,500,000,005,152 cycles; the small
rounding difference is retained in `.canic/growth-mainnet/funding.json`.
The manager's creation budget was 2.5T. Its single immutable worker approval
attached 1T with the selected 0.5T minimum liquid reserve check. Transfers and
provider fees remain separate concepts; no cost advantage is claimed.

The certified admission reply was `Scheduled` with counts `[0, 0, 0]`. Scheduled
dispatch was 21:00:24 UTC. Until an anonymous query reported `Ready`, host commands
were observations only. A subsequent controller update returned certified
`Ready` evidence: counts `[1, 1, 1]`, all three request commitments retained, zero
creation refund, no failure, matching approved/observed worker hash and 18 stable
pages. The maintenance executable marker was false.

Independent certified fleet observations matched the manager hash
`dd316c9ab7bc628abea68a5384eff3c689a12476382b413f26b8841e17e925b8`
and worker hash
`0be0704fb8e13a43650b457a98dfb88ee4a409ff349c6a03b53ab5603845b2a5`,
with the controller graph above. The snapshot contained five canisters, six host
operations and no inventory or observation failures. Historical expired Hello
World approval remains unsubmitted; it is not this manager's authority.

An authenticated update created user 1 on the new worker at revision 1 with the
name `Canic autonomous mainnet worker`. Its private query returned the same full
profile. The write reply was certified; the private read was explicitly an
uncertified query. Existing entry routing and its original worker were unchanged.

The final wallet observation was **5.10925125 ICP** and **405,555,115,186 cycles**.
Host signed attempts remain in the operation directory; approval, admission,
anonymous observation sequence, certified outcome, profile checks and fleet
snapshot are retained in `.canic/growth-mainnet/`. The completed execution artifact
reported exit zero. Do not rerun its funding payment or replace the spent slot.

This qualifies the normal one-worker mainnet path. At this point, interruption
and maintenance evidence was local; the subsequent mainnet maintenance run is
recorded below. General worker lifecycle is not qualified. The [current demo commands](../demos/growth.md#current-mainnet-demo)
use the existing canisters without another deployment.

## Mainnet manager maintenance

On 2026-11-14, upgrade `rk6n2-wyaaa-aaaad-qmoua-cai` in place using the exact
maintenance artifact from the passing local growth fixture: 684,263 raw bytes,
SHA-256 `5824826b6b715762223e5a0cec21239379d4ff415478341dddfa1b5a12078118`.
Operation `8b54397d04474aa916e93bcb89f46b8db7e96fccd05391d4eeb4224930f601d6`
binds the original manager module, same controller and zero creation funding.
Only `maintenance-candidate` was selected; no fault-injection build was used.

The first apply stopped after retaining the successful installation reply and
before persisting its terminal receipt. The exact cause of that initial
verification failure was not retained. A subsequent query observed the changed
executable. Resuming the original operation with identity `anonymous` and an
unavailable signer path `/nonexistent/canic-no-signer` settled the receipt through
fresh certified module/controller reads. The existing reply was reused; there
was no second signature, installation or replacement operation. This is direct
mainnet evidence of post-install receipt recovery, not a generic retry guarantee.

Certified manager observations before and after maintenance matched in every
field except the intended executable marker (`false` → `true`). This includes
the original approval and deadlines, worker principal, `[1, 1, 1]` dispatch
counts, all request commitments, zero refund, matching observed module, no
failure, `Ready` phase and 18 stable pages. Replaying the original, now expired,
approval returned identical certified evidence; it did not renew the spent slot.

The worker `rn7lo-3aaaa-aaaad-qmouq-cai` kept its exact module hash and sole manager
controller. Its before/after private profile queries matched the full record:
user 1, original owner, revision 1 and `Canic autonomous mainnet worker`. These
private reads remain explicitly uncertified. The fleet snapshot contained five
canisters and seven host operations, with no inventory or observation failures.

Retained evidence lives in `.canic/growth-mainnet-maintenance/`, including the
before/after observations, recovered receipt, replay result, profile checks and
fleet snapshot. The first execution artifact records exit 1; the subsequent
verification records exit 0. The original host operation directory retains its
signed attempt, reply and now-complete receipt.

The wallet preflight remained 5.10925125 ICP and 405,555,115,186 cycles; no new
deposit was visible. Maintenance used the manager's existing cycles, without a
wallet conversion or new canister creation. The preservation guarantee remains
limited to this same-format upgrade. Injected failures and blocked-creation
preservation were tested locally; cross-schema migration and general worker
administration remain separate scope.
