# Canic local instructions

This repository adopts the reviewed Toko Tooling engineering baseline in
[TOKO-TOOLING.md](TOKO-TOOLING.md). Read that baseline before changing this repo.
Shared files are managed by `.toko-tooling.snapshot`; update them through the
authenticated tooling installer.

## Repository choices

- The maintainer treats legacy Canic (`canic-legacy`) as an intensely iterated design document
  spanning four years and four generations of LLMs. Derive Canic from user
  outcomes and platform constraints. Old code, terminology, package boundaries
  and roadmaps are research inputs; they may be discarded or redesigned. The
  current design is also revisable. Coverage does not require feature parity.
- Preserve legacy Canic's distinction between application roles, immediate
  parent/child topology and IC controller relationships. The maintainer values
  that tree design. Do not replace an application role with `controller` or infer
  application parentage from controllers or routing links. App definition `role`
  and `parent` fields describe the tree instantiated in each fleet; they grant no effect authority.
  Keep canister trees homogeneous: only canisters occupy indented tree nodes.
  Show external operator/controller principals separately as controller metadata,
  never as parents above Root in a canister tree. Apply this to CLI output,
  visualizations, documentation and explanatory examples.
- Keep the virtual Cargo workspace at the root and product packages in `crates/`.
- Keep one canonical workflow for each task. Do not add convenience shortcuts,
  command aliases or parallel configuration mechanisms without a demonstrated need.
  The hierarchy is App → Fleets → Canisters. App definitions own shared
  roles, parent relationships and optional Wasm input paths; fleets bind network
  and existing identities. Spawn permissions, placement and budgets still require
  their admitted management contracts. Initialization declares an app and its
  first fleet; optional source generation creates a standalone greeting workspace.
  Added buildable canisters use the existing `--manifest`, `--package` and `--wasm`
  recipe fields together; paths remain relative to the app config.
  Whole-fleet plan/apply accepts a new Root, an optional declared `wasm-store`,
  and one or two direct application children with bounded serial upgrades. Store
  resolves Root's approved creation output and completes handover/readiness before
  application admission; it never consumes an application slot. The total operator
  debit includes Store's separate 1T creation allowance. Preserve nested Store
  attempts under the same fleet journal and cancellation boundary. Refuse Store
  addition/removal on retained fleets. A changed Store artifact now selects its
  separate Root-owned maintenance approval after completed child work. One
  immutable fleet approval binds all artifacts, permanent name/slot assignments and the Root's typed creation
  output; all steps reuse the existing signer/effect owners under the journal
  lock. Retain nested Root/child attempts, preserve original deadlines on replan,
  and refuse competing individual plans. `fleet cancel` may release an approval
  only before effect preparation; retain its revocation and never apply it again.
  Once any effect preparation begins, preserve the reservation and resume it.
  Completed creation and upgrade receipts permit the next bound child upgrade through the existing
  managed Plan contract, under the same journal lock. Resolve target, previous code
  and Root authority from retained completion; require matching authenticated Ready
  evidence. Pending upgrades take precedence over creation receipts. No-change
  compares retained completion only, never claims fresh network convergence. Root
  maintenance is a separate direct upgrade after completed child work; larger
  trees and existing-fleet adoption remain unsupported.
  Whole-fleet planning validates one ordered chain under the journal lock before
  publishing each existing managed Plan; a pending tip cannot be overtaken.
  Host operations use one explicit `PlanAction`; do not reintroduce independent
  optional authority fields. Fleet children have permanent identities in one ordered
  collection. Keep historical field names and directory layouts in record codecs;
  preserve approval digests and signed attempts. The current one/two-child limit
  is a qualification boundary, not the shape of every host consumer.
  Root admission treats `execute_at_ns` as an earliest dispatch bound and may
  accept elapsed timestamps before the original expiry. The same timer and
  journal own execution; preserve finite intervals and never renew deadlines.
  `bash scripts/dev/test-store.sh dispatch` qualifies the local runtime contract.
  New Host plans bind earliest dispatch to plan creation and expire ten minutes
  later; prerequisites consume the same window. Explicit future and retained
  historical schedules keep their pre-dispatch admission cutoff. Preserve exact
  times and digests on replan; never renew expired authority. Older Root code is
  not assumed to support elapsed-time admission or selected through fallback.
  Individually created children retain their existing first-upgrade planning path.
  Change one artifact per approval. Existing one-child fleets cannot gain a child
  by replanning or rename a retained slot. Maintenance binds every child's
  exact completed journal, reads it through the selected controller-only observations
  before installation and verifies preservation before retaining a receipt.
  Reuse direct install/signed-attempt custody; never reset slots or rewrite prior
  approvals. Only completed maintenance advances the Root hash for future child
  plans. The replacement must support this observation contract. One exact retained
  single-upgrade Root is admitted by module hash with separate controller-only
  creation and upgrade observations before installation; its pinned binary fixture
  qualifies the transition. Never probe endpoints or fall back on errors for other
  old owners. The operator must keep the maintenance window exclusive.
- The managed starter is a small buildable Rust app with one application
  canister controlled by a Root/management owner. Whole-fleet reviewed convergence,
  cascading discovery and subtree operations are admitted product requirements;
  their general implementations are not yet shipped. Scaling and sharding should
  be optional composable building blocks, with application-owned data semantics.
  Preserve all four admitted allocation patterns in design section 8: singleton,
  scaling workers, shards and keyed instances. Keep cardinality/assignment separate
  from subnet placement, application parentage and controller authority. Each
  selected pattern must qualify upgrades and recovery, not creation alone.
- The selected managed-fleet design uses a separate dynamic, versioned Wasm Store;
  see design section 12. Store owns durable artifact bytes, immutable release
  history and pins; Root owns approvals, immutable references and effect evidence.
  Do not grow Root into an artifact repository. Current Root payload storage and
  limits below remain supported implementation constraints until the replacement
  and custody transition qualify. `canic-wasm-store` now owns the standalone
  publication canister; `canic-wasm-store-protocol` is passive and Host-safe.
  It owns caller/operation/artifact pins and bounded management-cache delivery.
  Root exposes explicit Store-bound creation/upgrade/custody methods; legacy
  inline contracts remain supported. Complete Store references and history must
  survive maintenance. Root also mounts `store_publication`: one controller-only,
  deadline-bound prepare/put/publish/observe call. Store owns durable outcomes;
  Root retains no publication catalog, payload or retry journal. A reply trap
  cannot undo Store commits; recover by observation and exact idempotent commands.
  Fault point 12 exercises this remote-commit boundary only in qualification.
  Root's explicit `observe_store` mount reads initialized Store state through
  replicated `observe_info`, under the same controller/deadline admission. It
  reserves no app slot and certifies neither module identity nor sole control.
  `make store-readiness-test` qualifies handover, refusal, reply fault 12 and Root
  maintenance. Retained bootstrap combines this observation with certified
  code/controller evidence.
  Fleet publication uses existing plan/apply/status/reconcile with explicit version,
  bound Root/Store code and original signed attempts. V3 publication plans never
  reserve deployment slots or produce installation receipts. Reconcile grants only
  typed release observation, not fresh mutation authority. Use `make publication-test`
  for the installed CLI/signer witness. A named `wasm-store` plan with
  `--root-module` now creates/installs through the direct effect owner, retains
  Root's certified subnet before creation, and hands sole control to that Root.
  Root must already have a configured identity. Retain the exact handover attempt;
  certified sole control plus signed replicated readiness can resolve a lost
  handover reply without renewing effect authority. Reconcile cannot create,
  install or hand over. Never consume a greeting-child slot for Store. Whole-fleet
  bootstrap composes this same action before the application children.
  Named child `fleet plan --release PUBLICATION_OPERATION` selects a completed
  signed publication in the same fleet, binding its exact manifest and Store code.
  Retain source-bound staging/reconciliation and complete Store custody reads;
  never recover through an inline observation that omits artifact custody. The
  app input must match publication bytes. This qualifies creation and the named
  planner's first upgrade. Fresh whole fleets with a declared Store publish each
  child's `(name, full-content-hash)` release before admitting either child, using
  the approved Root/Store creation outputs. Both manifests belong to the original
  fleet approval. Child upgrades first publish through a separate existing Plan,
  then replan for installation after retained publication completion. Completed
  release evidence remains reusable across reviewed Root maintenance; unfinished
  publication authority stays bound to its original Root code and deadlines. No new CLI
  selector, mutable latest lookup or runtime effect owner is needed. Whole-fleet
  serial upgrades retain their existing eight-upgrade bound. Retained inline
  fleets keep inline delivery. Root maintenance for Store-backed fleets carries
  the exact Store code binding and complete per-slot sources alongside all child
  journals. Before any Store operation exists, select `observe_stored_children`
  for both preflight and postflight;
  compare full custody including historical pins and empty slots, and verify live
  Store authority/code/placement as well as child identity/code/controllers. Reuse
  the original direct install attempt and refuse incomplete or mismatched evidence.
  Inline-to-Store migration remains pending.
  Store's `observe_custody` returns one release's progress and all pins on its
  shared artifact; Root mounts `observe_store_custody` through the existing typed
  controller/deadline admission. This reads metadata only, adds no stable owner
  or pin authority, and is not an atomic catalog snapshot. `bash scripts/dev/test-store.sh custody` qualifies this read
  across Store/Root maintenance, shared versions, former owners and reply loss.
  Store owns its persistent mutation gate in CWS2 settings under the original
  allocation. Recognized CWS1 settings load without a prior attempt; malformed
  or unknown state never initializes. Close/observe/activate binds a monotonic
  sequence, operation and captured controller. Exact replay cannot re-close an
  activated attempt or reopen a newer one. Root mounts only this fixed contract
  as `store_admission`; it grants no install authority. Cache continuations retain
  their admission epoch and recheck it after awaits, including the final reply.
  Closed is not drained: already-issued remote cache work may still finish.
  `bash scripts/dev/test-store.sh admission` qualifies the gate, two-executable
  preservation and in-flight cache fencing. Root's explicit Store-maintenance
  stage/advance/observe mounts retain one approval outside Store and qualify
  closure followed by a single stop issuance. New child/publication mutations
  and raw gate changes are fenced while it exists. Lost callbacks recover by
  observation, including after expiry; never reissue stop. Full child custody
  refuses this pending record so Host Root maintenance cannot omit it.
  `bash scripts/dev/test-store.sh quiescence` qualifies sole-Root authority,
  callback faults 13/14, Root replacement and predecessor migration. This runtime
  now scans the complete closed catalog one entry per continuation before stop,
  retaining only counts, a cursor and ordered metadata commitment. Store checks
  the exact closed attempt and captured owner on every page. Concurrent replies
  may commit a cursor once; counts and an explicit end page establish completion.
  This does not hash payload bytes. Stores lacking this contract remain blocked.
  Prior CUJ5 records load without preservation evidence; never invent it for an
  already stopped attempt. Explicit `install_store_maintenance` accepts up to
  1 MiB of approved raw Wasm, uploads once into Store's management cache, and
  authorizes one chunked installation and restart with admission closed. Host
  retains bytes; Root retains only request commitments and original authority.
  After restart, reuse the same scan to verify preservation; only explicit
  `activate_store_maintenance` can open the exact attempt. Callback faults 15–18
  and dispatch fault 19 qualify recovery without repeating upload/install/start/
  activation. `bash scripts/dev/test-store.sh maintenance` owns this installed proof.
  Completion releases the mutation fence but retains the operation. Full child-only
  custody must still refuse that retained Store record, including Ready. Root's
  explicit `observe_root_custody` returns both child journals, their artifact
  sources and the Store operation synchronously. An approved Host Root-maintenance
  `StoreMaintenance.operation` selects this complete read before and after install;
  compare every field, including original operator, deadlines and uncertain effects.
  Absence preserves the older read contract and digest, never an automatic fallback.
  Whole-fleet planning selects this operation only from the Store workflow's
  authenticated completion, never public diagnostics. Existing plan/apply/status/
  reconcile now drive one Store replacement: bind Root/Store code, original operator,
  deadline and bytes; retain signed admission before continuation. Reconcile only
  observes issued effects and verifies metadata; apply explicitly installs and activates.
  Completion is a pointer to the exact signed Ready observation, not a parallel
  receipt owner. Subsequent child sources bind that completed maintenance attempt
  and select complete Root custody, refusing missing, stale or unready evidence.
  Each apply performs at most 32 continuations under 4096 retained observations.
  One attempt only: no cancellation, reset or successor history yet. Live use
  remains unqualified; use the local installed publication fixture for this workflow.
  Root's public `store_maintenance_status` query projects the retained metadata
  without scheduling work. `fleet show` selects it only from retained Store
  bootstrap authority and a known Store identity, never configuration roles alone.
  Reports are uncertified and cannot establish completion custody, current Store
  admission or readiness. Missing methods and mismatched Store identities remain
  unknown; the existing snapshot deadline bounds these observations as well.
  Root's controller-only `recover_store_maintenance` accepts the exact retained
  approval and observes only already-issued effects (plus post-install catalog
  verification). It cannot close, stop, upload, install, start or activate anew.
  Keep Host reconciliation on this restricted contract;
  generic `advance_store_maintenance` may dispatch new effects and is not a
  substitute. No recovery request renews the original deadline or operator.
  Cache preparation never grants install authority. It has no pin release or
  deletion; do not clear pins on expiry.
  Use `make store-test`, `make store-clippy` and `make store-doc` for this scope.
  The installed fixture uses synthetic cycles and two different executables;
  `maintenance-candidate` is a qualification marker, not a live release choice.
- Exclude ICP conversion from Canic: operators supply cycles manually; bounded
  distribution of those cycles remains in scope. Application frontend integration
  is deferred. Keep operator handover based on retained workspace evidence and
  separately provisioned signer authority; do not promise journal-free recovery.
- Group CLI actions under `app`, `artifact` and `fleet`; wallet inspection takes
  a public principal. Keep command help alphabetical. Only `--json` is global;
  scope options belong after the action that actually consumes them. Reject
  unrelated context rather than silently ignoring it.
- Render terminal tables with IC Host's `ic-host-text::table` API. Canic owns
  columns, cell values and output policy; the shared renderer owns escaping,
  display widths and alignment. Do not hand-pad columns or add a formatter alias.
- `crates/canic` owns the small greeting Wire contract and optional actor adapters.
  Its `blob` feature mounts the independent supplier through explicit local handlers;
  no Blob runtime dependency is enabled by default. Host and CLI packages
  remain native; the demo canister is in `apps/hello/canic-hello`.
- Follow the accepted [endpoint ownership boundary](docs/design/canic-system-design.md#endpoint-ownership-and-actor-composition):
  Canic composes actor endpoints, Platform owns generic IC entrypoint macros,
  and Wire owns codecs without endpoint knowledge. Keep shared protocols passive
  and Testkit Wire test-only. Migrate callers before retiring Wire Endpoint;
  do not add another codec/executor implementation or implicit lifecycle owner.
- `crates/canic-root` qualifies up to two greeting children through the runtime and
  installed CLI, including independent upgrades, complete Root maintenance and fleet views.
  `canic-management` owns the shared creation/artifact/evidence engine;
  `canic-management-protocol` owns its bounded Wire contract independently of
  runtime dependencies. Root and the profile manager explicitly mount their own
  lifecycle/endpoints and supply application readiness checks. Preserve existing
  manager journal keys, authority and wire bytes across extraction and upgrades.
  Use `make root-test`, `make root-clippy` and `make root-doc` for Root, and
  `make growth-test` when changing the shared engine's effect boundaries.
  `make endpoint-test` focuses on installed management mounts, authorization,
  frame admission and reply-encoding rollback using synthetic cycles. Root and
  the profile manager select exports with `canic_management::mount_endpoints!`;
  their lifecycle and readiness remain explicit. Fault points 9/10 exercise
  staging/reconciliation reply encoding only in `fault-injection` builds.
  Root admits two permanent child slots and at most eight serial upgrades per child.
  Scope every callback, timer and reconciliation explicitly to its stable slot.
  Reserve all pending creation attachments against one shared liquid-cycle budget.
  Creation/current-upgrade artifacts share a 1,048,512-byte budget; refuse overflow
  before replacing either slot. Keep both checked storage leases and the original
  directory capacities. Original endpoint contracts retain first-slot authority;
  legacy full-custody observation must refuse when the second slot is occupied.
  The new complete custody observation always includes both slots, even Empty. Each new
  approval requires a Ready predecessor without failure; retain every historical
  approval, request commitment and outcome. Uncertain, expired or failed work
  cannot be replaced, and history exhaustion never evicts records. Keep creation
  evidence immutable. The v6 journal retains optional Store-maintenance custody
  and before/after catalog scans and installation evidence, migrating v1/v2/v3/v4/v5 payloads without inventing
  missing authority or preservation evidence inside the original
  two fixed IC Storage allocations; current and creation artifact bytes stay
  there, while historical artifact hashes and host-retained bytes preserve custody.
  Never rewrite the supplier's directory or add a second
  backing owner. Downgrading the payload to an older executable is unsupported.
  A named child plan requires explicit `--root-module` authority. Resolve its
  parent from a configured identity or completed named deployment; check exact
  Root code and sole operator control before admission and observation. Keep
  staging's original signed attempt. Only Root's matching replicated, read-only
  observations selected by the retained plan (original or child-scoped) may receive
  fresh read authority after the effect deadline; public
  queries cannot establish the created child's custody. Upgrade observations use
  approval-bound ID 29; unit ID 25 remains public latest-status only. Read old
  on-disk responses without rewriting them; child recovery must not fall back to
  unbound live observations or re-sign an old pending request. Only the exact-hash
  single-upgrade Root maintenance preflight may use its unit ID-25 observation,
  comparing the complete single slot (including Empty) with approved custody.
  Explicit `fleet reconcile`
  has separate, short-lived request authority bound to the exact retained approval;
  it never renews effect deadlines, budgets or spent slots. Keep its signed attempt
  and deadline under the existing journal lock, ordered before the next observation.
  Unresolved recovery cannot be overtaken or re-signed. New reconciliation IDs
  27/28 replace unbound 22/26 without fallback; old owners require separate
  qualified maintenance, never automatic rebinding. Preserve existing direct
  plan encodings/digests and keep Root as the child's receipt controller.
  Fleet views automatically query planned Roots under their exact network binding.
  Query upgrade progress only for a retained upgrade plan and match its full
  approval. Multiple upgrades must form a completed predecessor chain; query
  only its tip and retain historical rows. Creation and upgrade queries share
  the existing owner deadline;
  unavailable/mismatched upgrade reports remain unknown without erasing creation.
  Keep those public reports separate from retained output custody: mismatched
  approvals cannot add a reported child, and watching must never sign, apply,
  reconcile or publish completion receipts. Reuse the shared bounded collector
  for terminal, saved reports and localhost refreshes.
- `apps/profile/` owns the profile protocol, native client, discovery entry and
  persistent worker and autonomous profile manager. Its original inline journey
  uses one worker; the selected Store-backed journey also uses one worker and
  the shared engine's bounded Store protocols. Profile readiness remains
  application-owned. Use `bash scripts/dev/test-root.sh profile_store` to qualify
  publication, worker/Store/owner maintenance, private-user preservation and
  refusal of incompatible readiness. No existing fleet is implicitly migrated.
  It uses IC Storage; Toko application integration is outside
  the current scope. Use `make profile-test`, `make profile-clippy` and
  `make profile-doc` for its affected checks.
  `make growth-test` qualifies operator-absent creation and manager maintenance
  using synthetic cycles. Never select `fault-injection` for a live deployment.
- Use Rust 1.100.0 and the inherited workspace lint policy.
- Packages start with `publish = false`; no first-party public licence is selected.
- The rewrite uses versions 0.200.0 and later to distinguish it from legacy Canic.
  Keep crates.io publication disabled until the product is mature and the
  maintainer explicitly authorizes publication. GitHub pushes do not authorize it.
- There are no local dependency exceptions or release adapters. The experimental
  deployment path selects local/mainnet authority through an explicit plan.
- `canic app init` creates local `canic.json` declarations by default: it creates a missing
  config or adds a new app and its first fleet, refusing existing app names and
  preserving other apps. Configured journals are not created by initialization.
  `--source DIRECTORY` instead creates a new standalone
  source workspace with its own config, supplied Root/Store and generated app declarations.
  The supplying checkout defaults to the CLI build source; `--canic-source` overrides it.
  It conflicts with `--config`, requires the destination's parent to exist, and
  refuses occupied destinations. Publish source/config together through the Host
  FS no-replace directory owner; preserve uncertain staging on publication errors.
  Resolve the matching Canic checkout physically; use its contract dependency,
  supplier selections, toolchain and inherited lints. Do not select legacy Canic
  from crates.io or copy Root management code into generated apps. Tool
  preparation remains explicit. `app build` owns declared workspace/package recipes,
  prepares only missing Cargo locks, and builds all three canisters through the locked
  artifact builder. Preserve existing locks. `artifact build --output` publishes a replaceable
  build input; retained deployment artifacts still belong to the plan/journal owner.
  `canic fleet init` adds a fleet projecting the shared app definitions, with
  no existing identity bindings, journal creation or network effects. Config
  updates use atomic replacement and a persistent `CONFIG.canic-lock` sidecar.
  `canic app canister add/list` owns reusable definitions; `fleet canister bind/list`
  owns observation bindings and resolved listings. There is no `fleet canister add`
  alias. Definitions never grant controller or spawn authority. Binding an existing
  ID is observation-only, refuses replacement and conflicting membership, and
  never rewrites journals. App-defined Wasm paths resolve beside the config;
  named creation refuses a competing `--wasm` override.
  Existing-app selectors infer only a sole configured app; existing-fleet
  selectors infer only a sole fleet within that app. Resolve app first, never
  infer it from a fleet or canister name. Unknown explicit names and ambiguity
  fail before effects. Keep selectors scoped to commands that use them, with
  no aliases or saved current context. `app list` and unfiltered `fleet list`
  list all declarations; names being created by initialization remain required.
  Explicit `--config` wins. Listings and selection share a bounded catalog of the
  current and direct-child `canic.json` files. Match declared app identity, reject
  duplicates and malformed configs, skip hidden/generated trees and child symlinks.
  Never search ancestors or recursively scan. Use this same catalog for declarations,
  edits and operations; creation of a new app
  still targets the current config unless `--config` or `--source` is supplied.
  The current config is unversioned, with `apps.APP.canisters` definitions and
  `apps.APP.fleets.FLEET.canisters` identity bindings. Refuse the retired versioned
  layout rather than maintaining a second reader. Retained v1/v2 deployment plans
  are unchanged; preserve their digests and same-operation recovery.
  Configured journals default to `.canic/apps/APP/fleets/FLEET/` beside the config.
  This checkout's `hello-world/mainnet` and `profile/mainnet` retain `journal: ".canic"`.
  Keep original records in place and filter observations by exact membership.
  `profile_manager: true` on an existing-ID binding selects only the demo's
  read-only status adapter.

## Local Blob composition

The original witness used the maintainer-selected, read-only local checkout. Its
qualified library is now committed in Blob 0.100.6. Cargo pins the exact supplier
Git revision `decde204acb6d7c8362a0003e904beef72ff89b6`; keep the sibling checkout
read-only and do not substitute its pending work. Authenticated Git access is
required for initial dependency preparation. Blob remains optional and absent
from the default application's runtime graph.

Use `make blob-test`, `make blob-clippy` and `make blob-doc`. The installed
witness is `qualification/canic-asset-catalog`; it calls the real Service locally and
uses the application's one Storage bootstrap. Its fault endpoint is qualification
only. `scripts/dev/blob-inputs.pl` resolves the immutable source through locked
Cargo metadata and records its source identity, file hashes and both consumer
locks before/after the installed run. Changed inputs fail the runner. Requalify
when those selected source bytes or contracts change. Publication stays disabled.

Blob owns its Service, identities, accounting and maintenance gate. Canic owns
endpoint mounting; the app owns user/catalog/publication policy. Never rewrite
all ingress callers to the application authority, create a second Blob ledger,
initialize after reopen failure or activate during endpoint/lifecycle mounting.
Current fleet effects send empty lifecycle arguments and do not own Blob's exact
drain/activation sequence. Do not advertise fleet-managed Blob deployment or
bypass its admission checks; see [composition scope](docs/demos/blob.md).

## Legacy research before subsystem work

Before substantive subsystem implementation or redesign, inspect the relevant
`canic-legacy` implementation, regression tests and Git history. Treat it as
accumulated engineering evidence, not merely a source of feature names.

- Identify the problem the legacy solution addressed, its failure cases and
  safety requirements, and which platform or consumer assumptions have changed.
- Reuse or adapt sound solutions where they fit the current Canic design. Explain
  why a relevant solution is being replaced or omitted rather than rediscovering
  the same failure through trial and error.
- Bring applicable regression cases into Canic and run them against the new
  implementation. Preserve the demonstrated invariant, not incidental legacy
  structure or terminology.
- Record the inspected revision, relevant source/test paths and the resulting
  decision in the subsystem's existing design/work notes. Reuse prior research
  when it covers the same revision and question; identify gaps in the evidence.

This research does not require feature parity or give an existing mechanism a
presumption of inclusion. Derive the implementation from current user outcomes
and platform constraints, retaining protections for threats that still apply.
Inspect the legacy repository read-only unless changes there are separately
authorized.

## Post-push design review

Apply the shared [post-push review](TOKO-TOOLING.md#post-push-review) to Canic’s
App → Fleets → Canisters model, explicit authority boundaries and qualified
recovery paths. Keep Canic-specific deferred findings in the existing design or
qualification notes.

## Changelog examples

At each user-facing increment, include a few short, copyable `canic` command
examples in fenced `sh` blocks in the root `CHANGELOG.md`. Place them near the
top of the current entry or beside the relevant feature, rather than burying
usage in prose or only linking to another document. Show supported fleet tasks,
state necessary prerequisites and effects, and keep browser launches opt-in.
`make canic` owns building and installing the CLI on PATH and verifying command
resolution. User-facing instructions use plain `canic ...`, never build-directory
executable paths, PATH exports or shell aliases. Keep explicit app/fleet/config
selection appropriate to the stated working directory.
Normal apply examples use the prepared default signer; reserve `--icp-bin` for
explicit overrides and qualification fixtures.

## Development checks

Qualification lives in `qualification/`: `canic-qualification` owns all native
installed harnesses and explicit `make qualification-setup`; `canic-effect-probe`
and `canic-artifact-probe` retain isolated platform-boundary fixtures. Use descriptive
Make targets (`effect-test`, `artifact-custody-test`, `journey-test`) without
milestone aliases. Simulator preparation and tests share `IC_TESTKIT_SERVER_DIRECTORY`,
defaulting to `target/qualification/server`; tests only verify prepared assets.
Keep dated decisions in `docs/qualification/history.md` and current test scope in
`qualification/README.md`. Retire a probe only when its failure cases have a
qualified product owner; renaming alone does not replace regression coverage.

Do not automatically open desktop windows, terminals or browsers. The maintainer
has paused automatic launches; only launch a window when explicitly requested.
Browser-facing demos and reports must be served over localhost. Give the
maintainer a clickable `http://localhost:PORT/` URL; do not offer a filesystem
HTML link or expect VS Code to render it. If explicitly requested, open previews
in the normal external desktop browser, not VS Code's embedded browser.
Keep a requested viewer running for review and state how to stop it. Use
`canic fleet view REPORT` for a saved fleet snapshot, `canic fleet show --serve` for a fresh
saved view, or `canic fleet show --watch` for bounded, read-only automatic observations.
Omit `--open` unless a browser launch is explicitly requested.

Use `make check`, `make test`, `make clippy` or `make doc` for the affected library
check. Use `make fmt-check` for Rust and manifest formatting, and
`make toko-tooling-check` for snapshot and dependency agreement. `make ci` is the
aggregate repository check. Tool preparation is explicit: `make install-tools`,
then `make tools-check`. The formatting hook is available via `make install-hooks`.
`make lint` checks both workspaces' Clippy, Rustdoc, formatting and tooling
agreement without running application or simulator tests. Every package inherits
its workspace lint table; scoped `expect` attributes need an explicit reason.
Use `make journey-test`, `make journey-clippy` and `make journey-doc` for the CLI/demo journey;
simulator and pinned ICP signer preparation remain explicit.
