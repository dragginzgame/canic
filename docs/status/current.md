# Current Status

Last updated: 2026-07-21

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the source, design, audit, or changelog files needed for the current task.

Historical detail is archived at:

- [status through 2026-06-30](archive/2026-06-30-precompact.md); and
- [status through the 0.90.2 release](archive/2026-07-13-precompact.md).

## Current Release

- The workspace package version is `0.96.3`.
- The latest published release is `v0.96.3` at
  `e5c3b7be7014b67fbb1ad18a30aae7843b2ae83d`.
- The `v0.96.3` source tree is
  `09974e599161d8275939812980a2728b025c1f25`; its product-tree hash is
  `b105dbf33206f6b694f62247fb74e610446e2bd1b08a219e518f272e56421cc2` and
  its Cargo.lock SHA-256 is
  `b8d15e2dbdacfb218b6126dcea33e5781eda5470ac4ba9e867d171a6ba482cd4`.
- D13 workspace-only release lock synchronization and the executable
  `v0.91.6` compatibility accounting are released in `v0.92.12`.
- The immutable `v0.92.12` closeout recorded
  `closeout_verdict: pass_with_limitations` with three deferred P2 watchpoints.
- The maintainer subsequently accepted D14, released in `v0.92.13`. Its auth
  performance checkpoints fix `CANIC-092-PERF-001`; the two remaining deferred
  P2s are dependency-upstream and trust-path-complexity watchpoints. The
  verdict remains `pass_with_limitations`, with no P0/P1, waiver, or blocked
  run.
- The maintainer then accepted bounded D15 work on those final P2s, released in
  `v0.92.14`. The three measured auth/root-proof production hubs are split by
  existing ownership and the dependency risk has one exact fail-closed
  CI/release inventory. The complexity finding is fixed; the four upstream
  transitives remain an accepted external limitation. The live ledger is 42
  fixed, one accepted P2, zero deferred, and zero blocked.
- Released `v0.93.0` hard-cuts the unreachable 0.74 issuer-renewal
  attempt model, storage slot, metric, outcome/failure counters, and status
  projection. The current chain-key batch is the sole renewal-work authority,
  and CLI/medic status warns when a reachable issuer has no usable active
  proof.
- Released `v0.93.1` hard-cuts uncalled host ICP CLI convenience,
  snapshot upload/restore, replica-display, and legacy text snapshot-ID
  surfaces while preserving the maintained Candid-aware, rooted replica,
  JSON snapshot-create, and backup-owned restore paths.
- Released `v0.93.2` makes the Wasm-store retirement lifecycle generation
  bound and non-reentrant, permanently excludes stores in GC from publication,
  and makes restore operation-receipt outcome and attempt identity mandatory.
- Released `v0.93.3` bounds ICP-refill lookup and metric work through
  lifecycle-rebuilt derived indexes, retains renewal-timer failure causes in
  runtime diagnostics, and restores the workflow/storage layering guard.
- Released `v0.93.4` through `v0.93.7` hard-cut permissive ICP response
  parsing, duplicate registry and auth response models, and local query
  transport fallback. Maintained query paths use canonical Candid DTOs and
  retain typed command, envelope, decoding, endpoint, and replica causes.
- Released `v0.93.8` gives project config discovery, duplicate fleet
  admission, and fleet selection one typed authority. Install, deployment
  verification, and deployed-list operations fail closed on project-root
  resolution rather than continuing through optional fallbacks.
- Released `v0.93.9` gives Cargo workspace, ICP project, and
  canister-manifest discovery typed filesystem, TOML, metadata, missing, and
  ambiguity failures. It removes the public explicit-path-to-current-workspace
  fallback and rejects uncanonicalized manifest search roots.
- Released `v0.93.10` makes selected-network artifact lookup exact, makes
  replay terminal transitions fail closed on missing or corrupt persisted
  receipts, gives root install a typed phase/cause boundary, and preserves the
  host-owned installed-deployment error through CLI commands without duplicate
  mappings.
- Released `v0.93.11` makes cost-guard rollback and ICP-refill/pool-create
  settlement fail closed, hard-cuts the retired Wasm-store internal-proof
  classifier and duplicate endpoint model, and preserves typed backup-manifest,
  fleet-create, fleet-config, and config-list child causes.
- Released `v0.93.12` makes replay receipts the durable recovery authority for
  costed refill, pool-create, root-provision, and root-upgrade completion, and
  preserves both typed root-install failure causes.
- Released `v0.93.13` completes that invariant for `RequestCycles` and every
  staged replay response, removes the duplicate direct response-commit and
  cycles-specific replay paths, and makes the reviewed production cost-flow
  inventory executable.
- Released `v0.93.14` prevents replay cleanup or recovery-marker failures from
  replacing the primary typed failure across root RPC, cycles funding, pool
  creation, ICP refill, and authentication preparation. It also keeps pending
  pool-reset records authoritative through asynchronous recovery so recycled
  canister metadata and duplicate-work exclusion survive the transition.
- Released `v0.93.15` fails delegated-token verification closed when required
  local subnet state is absent, returns typed state-cascade partial failures,
  and binds prevalidated shard allocation to one deterministic root replay
  identity per owner, pool, and slot.
- Released `v0.93.16` replaces the separate scaling, directory, and
  sharding create flows with one receipt-backed placement-allocation authority.
  Unknown directory outcomes retain their claims, tracked retries replay the
  exact root placement operation, capability transport preserves its full
  32-byte identity, and committed placement receipts remain recoverable until
  response-idempotent acknowledgement completes. Terminal placement intents
  own bounded acknowledgement retry across init and upgrade without retaining
  redundant terminal intent records after cleanup. Evidence-backed directory
  repair and disposal skip nonexistent root acknowledgement while still
  removing their terminal local intent.
- Released `v0.93.17` reserves `canic:` intent resource keys for one
  runtime authority. Consumer begin, lookup, settlement, commit, and rollback
  fail closed on that namespace; placement acknowledgement accepts only its
  canonical reserved resource shape, and cost-guard plus pool-import intents
  use the same boundary.
- Released `v0.93.18` makes placement and pool-import recovery
  interruption-safe, gives terminal placement receipt cleanup one durable
  owner, and makes expired-intent cleanup use the checked abort authority.
  The pre-`0.93.17` cost namespace remains a documented hard cut with no old-key
  read, alias, or migration path.
- Released `v0.93.19` gives pending pool reset recovery one bounded
  cursor sweep so blocked oldest rows neither spin nor starve later work. It
  also makes sharding assignment and release accounting reject dangling
  references, counter underflow, and counter overflow before stable mutation.
- Released `v0.93.20` serializes child cycles funding through the
  existing durable replay authority. A competing pending `RequestCycles`
  operation now fails with a typed conflict before it can race whole-ledger
  rollback, cumulative budget, or cooldown accounting. Failure to persist the
  pre-transfer replay marker also settles the cost guard without losing the
  typed replay cause or retaining an unused cycle reservation.
- Released `v0.93.21` routes workflow test fixtures through ops,
  prepares the complete locked dependency graph before offline role-package
  and dependency-risk inspection, and activates the dependency-risk rejection
  fixtures in CI, making layering and fresh-cache validation deterministic.
- Released `v0.93.22` binds every token-backed replay transition to
  the complete immutable receipt identity, so an expired asynchronous
  continuation cannot mutate or abort a newer request that reused its stable
  slot. ICP-refill context lookup and delegated-token lazy repair revalidate
  that ownership before post-await local mutation. Ordinary receipt collection
  now matches admission at the exact expiry boundary.
- Released `v0.93.23` makes release-set artifact and root-manifest
  path projection pure and canonical. Deployment registration no longer
  follows duplicate error-swallowing branches or creates artifact directories;
  the durable writer remains the sole filesystem-mutation authority.
- Released `v0.93.24` distinguishes selected, build, and artifact target
  concepts across ICP command routing, build evidence, deployment truth,
  backup, and restore. Normal named installs verify their fresh local build
  outputs, supplied plans retain exact artifact lookup, and missing roots
  preserve expected role rows so the artifact gate fails closed instead of
  accepting empty evidence.
- Released `v0.93.25` corrects the selected-target name to
  `environment` across CLI, host, evidence, deployment, backup, restore,
  diagnostics, and JSON. `build_network` remains the local/IC compile class,
  `artifact_environment` remains the exact `.icp` artifact namespace, and an
  `icp.yaml` environment may separately reference a backing network.
- Released `v0.93.26` bounds the public delegated-token preparation
  surface at 64 active entries per caller and 512 globally for both durable
  replay responses and caller-bound prepared-token metadata. Expired entries
  release capacity, exact committed replay remains available at saturation,
  and the duplicate issuer-proof metadata authority is removed.
- Released `v0.93.27` gives supplied deployment plans one prepared
  artifact authority. Digest-pinned raw and gzip sources normalize under the
  selected ICP environment, deployment truth, release publication, and root
  activation consume those same revalidated bytes, and root resolution waits
  until the safety and preflight gates pass.
- Released `v0.93.28` binds executable restore snapshot operations, journals,
  and receipts to one expected checksum. Restore execution consumes only a
  private no-follow copy of the verified artifact. It also removes the
  redundant 0.50-bound adoption availability field; recommendation support is
  the sole current decision and projection.
- Released `v0.93.29` gives Wasm-store inventory one deterministic
  registry projection carrying canister identity and persisted creation time.
  Missing roles are empty inventories, while post-create registration absence
  fails closed instead of inventing metadata. The raw stable-record accessor
  and dead role-not-found error flow are removed from the control-plane bridge.
  Root index validation now shares the index builder's direct-child scope, so
  nested canisters cannot create false duplicate service-role failures.
- Released `v0.93.30` hard-cuts the duplicate delegated-auth network
  model and duplicate host build-network enum. Config, bootstrap, verification,
  renewal, provisioning, environment resolution, build tooling, and runtime
  introspection now use the canonical `BuildNetwork` authority. The strict
  config and runtime-status field is `build_network`; the old auth key,
  mainnet/PocketIC/testnet auth labels, and empty runtime `network` projection
  have no aliases or fallback parsing.
- Released `v0.93.31` makes human-readable cycle parsing and Candid
  `Nat` narrowing exact and checked. Oversized management balances preserve a
  typed cause; oversized CMC notification totals terminate without entering
  child-funding accounting; corrupt oversized refill totals cannot rebuild the
  derived index. Lossy conversions and unchecked convenience arithmetic are
  hard-cut without aliases.
- Released `v0.93.32` makes bounded strings invariant-preserving
  across construction, Serde/Candid input, and stable decoding. Mutable inner
  access, lossy UTF-8 replacement, and overlong truncation are removed.
  Malformed stable intent, operation, and replay-slot identities fail closed
  rather than aliasing all-zero keys.
- Released `v0.93.33` hard-cuts missing-field defaults from current
  backup and restore v1 documents. Download-journal topology receipts are
  mandatory and stale receipts fail before snapshot creation rather than being
  overwritten during resume. Exact backup- and topology-bound journal rows
  recover interrupted snapshot creation without repeating the external call.
  Current manifest provenance, execution metadata, restore verification,
  lifecycle counts, operation counts, and receipt collections must be
  explicitly present. Persisted backup and restore plans now require
  `plan_version: 1`; restore plan projections are recomputed and validated
  before persistence or apply preparation. Ordinary config and private helper
  types no longer carry misleading `V1` suffixes.
- Released `v0.93.34` validates restore apply dry-runs before they
  can create durable journals. Version, readiness, operation, sequence, and
  artifact projections must match their concrete rows. Checksum algorithm and
  hash validation use the canonical artifact authority, and contradictions
  preserve a typed cause rather than being converted into executable state.
- Released `v0.93.35` makes every nullable field in maintained backup
  and restore documents explicit and required, while stable restore runner
  JSON retains one key set across modes. It also removes two behavior-neutral
  PascalCase `rename_all` declarations; remaining declarations belong only to
  Serde JSON, configuration, report, evidence, or external CBOR enums, while
  Candid continues to require explicit item-level names.
- Released `v0.93.36` makes the canonical backup phase builder the sole
  persisted operation authority. Backup plans reject incomplete or altered
  operation projections, malformed topology hashes, target cycles, internal
  depth contradictions, and selected subtree graphs that are disconnected or
  rooted beneath another selected target. Root-omitted deployment targets must
  likewise remain connected to their declared root.
- Released `v0.94.0` replaces path-existence and `Drop`-owned backup/restore
  journal locking with one no-follow, close-on-exec kernel authority. Live
  owners remain exclusive, abrupt process death releases ownership, and stale
  regular sidecars no longer block recovery.
- Released `v0.94.1` retains one restart-visible command-lifetime lock through
  each mutating backup/restore command tree and hard-cuts unsafe pending-reset
  recovery. Quiescent unknown effects halt without blind replay.
- Released `v0.94.2` freezes the executable 106-case protocol and proves
  execution-journal publication plus read-only verification across process
  death.
- Released `v0.94.3` proves backup preflight and pending-claim publication,
  resumes exact local pending work, and reconciles committed stop from typed
  target status without a second stop.
- Released `v0.94.4` reconciles committed snapshot creation from one exact
  inventory delta and reconstructs lost execution receipts from complete
  artifact evidence. Versioned backup documents remain version 1; superseded
  shapes are hard-rejected with no migration or fallback reader.
- Released `v0.94.5` gives stop and start one lifecycle-status recovery
  authority. Committed start is adopted without a second command, exact
  stopped state justifies one start, and returned-failure retry reconciles
  before mutation.
- Released `v0.94.6` replaces only uncommitted private download staging
  after an exact `Created` artifact claim, rejects unsafe staging entries, and
  preserves `Downloaded` or later evidence for its canonical recovery
  boundary. Artifact-journal states now reject fields owned by another
  transition.
- Released `v0.94.7` reconciles both sides of the `Downloaded`
  artifact-journal write. A non-durable write retains `Created` and justifies
  one redownload; a durable exact row rebuilds the normal receipt and proceeds
  to checksum without another command. Missing or mismatched staging rejects
  before execution-journal mutation.
- Released `v0.94.8` proves a checksum completed only in memory is safely
  recomputed after process death from unchanged staged bytes. Missing or
  unsafe input fails closed without claiming checksum-verified state. The
  existing production runner and secure traversal required no change.
- Released `v0.94.9` reconciles checksum-verified staging and canonical
  artifact publication through one checksum-bound authority. Exact durable
  evidence is adopted while missing, changed, or conflicting evidence rejects.
- Released `v0.94.10` reconciles durable artifact state and final manifest
  publication. Canonical bytes are reverified in place, and only the exact
  immutable manifest is published or adopted.
- Released `v0.94.11` proves terminal state and its receipt publish in one
  durable execution-journal document for every post-preflight operation.
  Restart reconciles pre-write interruption and skips post-write completion
  without duplicate mutation or receipt. A lost final response replays the
  same completed backup with no command, receipt, or layout change.
- Released `v0.94.12` hard-cuts the competing snapshot-download flow and
  unsafe failed-layout pruning, preserves progressed restore authority during
  preparation, restores availability after backup failures, and reconciles
  interrupted restore effects from exact status or inventory evidence. A real
  two-canister local-ICP journey restored application state `A -> B -> A`; a
  deterministic real upload crash then reconciled one committed effect with
  zero duplicate upload. The checksum-bound maintainer and CI toolchain pins
  ICP CLI 1.1.0 and the maintained compatibility floor is now
  `>=1.1.0,<2.0.0`. Both durable sides of initial restore-plan and apply-
  journal publication now survive acknowledged process death, bringing the
  frozen matrix to 60 passing cases with 46 pending.
- Released `v0.94.13` completes private restore staging, all pending-claim and
  terminal publication sides, and final-response loss. Upload staging skipped
  by process death is removed before terminal publication or reconciliation.
- Released `v0.94.14` completes the remaining stopped-precondition,
  effect/receipt, command-tree, and rejection cases. All 106 frozen protocol
  cases and all seven required journeys pass; no case remains pending.
- Released `v0.95.0` closes 0.94's status records, anchors the timer line
  to `v0.94.14`, inventories every timer and bounded host wait, reproduces
  seven findings, freezes every owner disposition and the public hard-cut
  surface, and adds an executable source inventory guard. Production timer
  behavior is unchanged in this audit-only batch.
- Released `v0.95.1` gives every current canister timer one common
  generation-safe workflow, removes guarded/fixed-rate/raw-CDK bypasses, adds
  consuming application cancellation, and projects live registration and
  process condition independently. Owner-specific idle-poll and full-scan
  removal remains in Slice C.
- Released `v0.95.2` replaces hourly local-intent polling with one
  lifecycle-rebuilt stable finite-expiry index, exact earliest-deadline
  scheduling, bounded 32-row continuation, and truthful idle state for
  TTL-free work. Other Slice C owners remain separate.
- The accepted 0.95 duration amendment rejects inherited round-number
  cadences. Every delay must be semantic zero, an authoritative deadline,
  bounded retry policy, explicit safety observation, or application-supplied.
  Released `.3` makes local intent invariant/storage failure stop failed rather
  than repeat the one-minute retry released in `.2`.
- Released `v0.95.3` removes the permanent root-pool maintenance interval.
  One `pool:pending` owner reconstructs from durable work, remains idle when
  empty, and retries only local-build importability failure through the
  accepted bounded backoff. Direct inspection also corrected the log audit:
  entry-count retention is still sweep-owned, so log migration remains an
  isolated later batch rather than being folded into pool work.
- Released `v0.95.4` adds one lifecycle-rebuilt stable index containing
  only terminal placement acknowledgements. Empty roles execute no callbacks;
  root transport failure uses the frozen 1/2/4/8/16/30-minute backoff, while
  root rejection and local contradictions stop failed. A maintained scaling
  PocketIC journey drains real acknowledgements back to idle.
- Released `v0.95.5` completes Slice C. Count and byte retention are
  enforced during append by one ordered runtime-log authority; optional age
  retention removes at most 256 rows at the exact oldest deadline. The default
  no-age policy executes zero callbacks. Append-only allocations 31 and 32 are
  hard-cut in favor of the modeled `runtime_log` domain at allocation 35, with
  no migration or compatibility reader for non-authoritative old log history.
- Released `v0.95.6` begins Slice D. It replaces coupled hourly cycle
  tracking/top-up with event-owned balance history and one configuration-gated
  `cycles:topup` safety owner. Released `v0.95.7` makes automatic
  funding nonroot-only, keeps root ICP conversion manual, removes its obsolete
  threshold and automatic workflow, bounds unattended balance observation to
  one hour, and prevents successful child requests from undercutting the
  configured parent cooldown. Parent admission, replay, cost, request,
  cumulative child-budget, cooldown, kill-switch, and balance controls remain
  authoritative.
- Released `v0.95.8` hard-cuts the one-minute delegated-proof recurrence and its
  duplicate start-soon flow. One auth renewal owner reconstructs exact
  registry-bound batch and issuer refresh deadlines, preserves typed failure
  outcomes, retries only bounded external causes, and reconciles disabled
  templates to idle. Runtime-log age deadline overflow now fails closed.
- Released `v0.95.9` reconciles configured child funding when authoritative
  topology arrives, checks every cycle deadline, and hard-cuts the
  behaviorless role-attestation timer route. Released `v0.95.10` closes the
  measured line by preserving typed insufficient-capacity rejection,
  permitting one bounded same-round hierarchy recovery attempt, and retaining
  the final owner/cost evidence in the maintained test and status surfaces.
- The [0.95 closeout report](../audits/release-lines/0.95-closeout.md) records a
  pass verdict at the immutable `v0.95.10` release identity.
- Released `v0.96.0` anchors Slice A to `v0.95.10`, freezes the complete
  in-repository receipt consumer and authority inventory, and traces the
  sibling Toko mint flow read-only. Toko has not adopted the API and has no old
  receipt rows. Released `v0.96.1` measures the current 100,000-row
  stable-capacity envelope, corrects the totals record's undersized stable
  bound, and removes exact zero-total rows. Terminal reclamation remains blocked
  on Toko's per-action identity, recovery/rate/resource-cardinality policy, and
  the final eligibility-allocation envelope. Released `v0.96.2` implements the
  independently safe replay-deadline admission hard cut. Released `v0.96.3`
  corrects its lifecycle reconciliation to one ordered linear pass. Open
  `0.96.4` freezes a 24-hour terminal observation grace, provisions exact
  settlement-index capacity at admission, and persists ordered terminal
  eligibility without enabling deletion.
- The completed 0.92 line design is
  [0.92 holistic audit and audit-system validation](../design/0.92-holistic-audit-and-audit-system-validation/0.92-design.md).
- The active line design is
  [0.96 receipt replay horizon and terminal reclamation](../design/0.96-receipt-replay-horizon-and-terminal-reclamation/0.96-design.md).
- Current development notes are in the
  [0.96 changelog](../changelog/0.96.md); released 0.95 notes remain in the
  [0.95 changelog](../changelog/0.95.md), released 0.94 notes remain in the
  [0.94 changelog](../changelog/0.94.md), and the completed 0.92 line remains in the
  [0.92 changelog](../changelog/0.92.md).

## Current Decision

0.93 is closed at `v0.93.36`. Its structural hard-cut purpose is complete, and
no known blocking structural residue is carried forward. The line deliberately
does not claim process-death recovery or realistic multi-canister
backup/restore readiness. Those bounded operational proofs move to the active
0.94 design rather than extending 0.93 as another open-ended audit.
Known non-blocking structural residue deferred from 0.93: none.

0.94 is closed at `v0.94.14`. All 106 frozen process-death/rejection cases and
all seven required journeys pass, every finding is fixed, and the realistic
multi-canister state restore is complete.

0.95 is closed at `v0.95.10`. It focused only on timer authority and
scheduling consolidation. Released `v0.95.0` completes Slice A and released
`v0.95.1` completes the Slice B common authority: one direct platform owner,
fixed built-in keys, opaque application identities, request/generation
arbitration, after-completion recurrence, consuming cancellation, truthful
live status, and one lifecycle facade.
Released `.2` completes the first Slice C owner: finite local-intent expiry.
Released `.3` removes idle pool polling and corrects intent invariant failure.
Released `.4` owns placement acknowledgement through one terminal-only
derived index and pending-only scheduler. Released `.5` gives log count, byte,
and age retention one ordered mutation authority and removes the final Slice C
polling interval. Released `.6` separates diagnostic cycle observations from
automatic funding. Released `.7` restricts that funding owner to nonroot parent
requests, keeps root ICP conversion manual, and tightens its observation and
abuse bounds. Released `.8` replaces the last fixed built-in recurrence with
exact delegated-proof refresh, durable batch, and typed retry deadlines.
Released `.9` repairs the topology/funding initialization race and completes
the hard-cut routing residue. Released `.10` fixes the measured
child-before-parent funding order with one bounded retry and records the final
24-hour comparison. The obsolete `canic-core` lifecycle-helper boolean is
removed.

0.96 is active. Audit-only Slice A is released as `v0.96.0`; the Canic-side
receipt authority is inventoried and guarded. Released `v0.96.1` records the
existing 100,000-row stable footprint: the primary, placement acknowledgement,
and resource totals allocations have a 3,969-page, 248.0625-MiB physical
subtotal in the measured ascending high-water case through base MemoryManager.
It also corrects the valid totals encoding bound from 64 to 69 bytes and
removes exact zero totals after abort or rollback. Released `.2` requires an
absolute application deadline, closes absent operations at equality, enforces
the source-backed 24-hour maximum, and stores one exact adjunct at allocation
46. Released `.3` replaces the resulting lookup-amplified lifecycle validation
with one ordered pass over each canonical map. Open `.4` adds the sole ordered
terminal-eligibility authority at allocation 47, a fixed 24-hour observation
grace, and admission-time reservation of the measured 726-page high-water
envelope. Settlement now persists exact eligibility before its primary and
aggregate transition; no cleanup timer or deletion is enabled. The complete
application receipt high-water subtotal is 4,737 physical pages, or 296.0625
MiB, through the pinned base `MemoryManager`. Production placement keeps its
separate acknowledgement-owned removal and stores no application deadline.
The read-only Toko snapshot uses Canic 0.71.3 and contains no receipt consumer,
so adoption needs no old-state reader or migration. It still needs a per-mint
action identity, recovery path, explicit batch/rate/resource-cardinality
bound, and removal or integration of its
parent-only stack mint before reclamation can be enabled. General cleanup,
dependency work, backup/restore changes, and
compatibility layers remain excluded.

0.92 treats Canic as feature complete for this line, but not as 1.0-ready.
The audit machinery has been inventoried, corrected, and frozen. Phase C has
attempted all 22 retained definitions read-only against the published
`v0.92.0` product snapshot. Corrected instruction-footprint v2 is
valid/partial, and corrected Wasm-footprint v2 completes the retained-method
ledger at 22 valid and zero invalid active results. Evidence-only PocketIC
coverage completes the two formerly partial mandatory traces. The aggregate is
now a valid `fail` with six pass and four fail results, zero partial, and zero
blocked. The Phase C product baseline gate is complete; product fixes still
require explicit finding review and bounded slice acceptance. Phase D D1 is
released in `v0.92.1`; D2/D3 are released in `v0.92.2`; D4 is released in
`v0.92.3`; D5 is released in `v0.92.4`; D6 in `v0.92.5`; and D7 in
`v0.92.6`. D8 is released in `v0.92.7`. Root runtime records and lifecycle
diagnostics no longer contain
absolute build paths, isolated root/bootstrap artifacts reproduce byte for
byte, and build provenance requires explicit optional-transform identity and
outcome. This fixes `CANIC-092-BUILD-001` and `CANIC-092-BUILD-002`; the
current mandatory-trace ledger is ten pass and zero fail. D9 is released in
`v0.92.8`: 13 external Action
executions use immutable commits, downloaded tools are version/checksum bound,
and one governance matrix owns the Ubuntu 24.04 x86_64 native/Wasm support
cell. This fixes `CANIC-092-RELEASE-001`, `-002`, and `-004`; the dedicated
scanner gap remained blocked after D9 and is fixed by D12. D10 is released in
`v0.92.9` and
focused
validation passes: published feature documentation matches the owning
manifests, active CLI proof asserts only maintained commands, warning-as-error
core rustdoc passes, and installed plus packaged downstream artifact proofs
pass before registry publication. This fixes `CANIC-092-PUBLISH-001`,
`CANIC-092-RESIDUE-001`, and `CANIC-092-DOCS-002`. D11 is released in
`v0.92.10` and
focused validation passes: shared decision inputs now belong to model,
root-proof admission belongs to workflow, and ops no longer imports policy.
This fixes `CANIC-092-LAYERING-005`, and the live layering guard passes with
zero violations. D12 is released in `v0.92.11`: Gitleaks 8.30.1 is
version/checksum bound, scans complete history with full redaction, and reports
zero unreviewed findings. This fixes `CANIC-092-RELEASE-003` without a waiver.
Slice E then found that the 0.92.11 version-only release transaction advanced
six unrelated external packages. D13 fixes `CANIC-092-RELEASE-005` in released
`v0.92.12` by synchronizing only workspace lock entries offline. Post-closeout
D14 adds stage-level root-proof and delegated-token instruction checkpoints
and is released in `v0.92.13`, fixing `CANIC-092-PERF-001`. Two P2 findings
remained after D14. Released D15 fixes the concrete trust-path
ownership concentration and makes the exact upstream dependency limitation
fail closed in CI and patch releases. No deferred finding remains; the one
accepted upstream P2 keeps the verdict `pass_with_limitations`. No P0 or P1
remains. Executable compatibility accounting passes: generated root and
Wasm-store Candid are byte-identical, production CLI/config/stable/backup and
package-feature owners agree, and `v0.91.6` state upgrades to `v0.92.11` in
PocketIC. The accepted 0.92.7 provenance hard cut rejects old envelopes and
requires regeneration as documented. The final release-line verdict is
`pass_with_limitations`; D14 and D15 were separately and explicitly authorized
by the maintainer after closeout.

Pre-1.0 removals remain hard cuts. Do not add aliases, compatibility wrappers,
duplicate command paths, deprecated APIs, anti-resurrection tests, or fallback
behavior unless the maintainer explicitly requests it. Named Canic
environments resolve through the upstream `environments` section in
`icp.yaml`; only `local` and `ic` are implicit, and no staging/mainnet aliases
exist. Custom environments may reference separately named backing networks.

Toko mint remains downstream-owned. Canic provides generic primitives only;
automated work must not edit the Toko repository or move mint-specific
requests, receipts, evidence, retry, cancellation, or tests into Canic.

## 0.92 Audit-System Outcome

- Phase A's
  [inventory report](../audits/reports/2026-07/2026-07-14/0.92-audit-system-inventory.md)
  found six confirmed P1 audit-system defects.
- Phase B's
  [hardening report](../audits/reports/2026-07/2026-07-14/0.92-audit-system-hardening.md)
  prepared and validated all six corrections.
- The
  [method-freeze report](../audits/reports/2026-07/2026-07-14/0.92-method-freeze.md)
  closes those findings at `v0.92.0` and admits the Phase C baseline.
- One canonical [method catalog](../audits/METHODS.md) owns 22 active
  definitions: 14 system, 7 authentication, and 1 manual-only module-surface
  method.
- The frozen method manifest is
  `fa92c4102efe74391c51f1f829aec7ac9c0b64941da73ee6dad1ebf2b292df07`.
- The frozen source tree is
  `fd31bb8289365a38f2bea7f8ebd6973908ee959f`.
- The frozen product-tree hash is
  `c2b932cfda4cd3060d8fb171a6005595c8c9e6c8b65d8bfd8ae34a4516e0802e`.
- The compatibility baseline remains `v0.91.6`, product-tree hash
  `8fce43e41ce430d9b505e19f8d596ed440b291d4c6ecb19c4a1cfdf71656a9b6`.
- The committed delta is fully classified as audit-system, operator/CI
  contract, requested documentation, and human-owned release-version changes.
  Runtime/public/serialized/stable behavior is unchanged. The 0.92.11 version
  transaction advanced six compatible transitive lock entries; current audit
  evidence records them and D13 prevents repetition.
- At least three months of real-world use remains a separate prerequisite for
  any future 1.0 discussion.

## 0.92 Phase C Baseline and Phase D

- The frozen Phase C baseline remains immutable at
  `91736337fc1cfeb891f17d7d62affb5e671348e2`.
- Phase D changes only accepted finding-backed slices and compares them to the
  immediate parent and frozen baseline.
- D1 is released in `v0.92.1`, D2/D3 in `v0.92.2`, D4 in `v0.92.3`, D5 in
  `v0.92.4`, D6 in `v0.92.5`, D7 in `v0.92.6`, D8 in `v0.92.7`, D9 in
  `v0.92.8`, D10 in `v0.92.9`, D11 in `v0.92.10`, and D12 in `v0.92.11`.
  D13 and Slice E compatibility accounting are released in `v0.92.12`; the
  release-line closeout is complete. Post-closeout D14 is released in
  `v0.92.13`, and post-closeout D15 is released in `v0.92.14`.
- Missing evidence remains partial/blocked, never pass, and historical Phase C
  results are not rewritten by later fixes.

First primary results:

- [dependency hygiene v1](../audits/reports/2026-07/2026-07-14/0.92-dependency-hygiene-v1.md)
  remains invalid history. Corrected
  [dependency hygiene v2](../audits/reports/2026-07/2026-07-14/0.92-dependency-hygiene-v2.md)
  is a valid pass at risk 3/10: all 484 external packages identify license
  metadata, the cached advisory scan finds zero known vulnerabilities, and
  four reachable unmaintained transitive packages remain watchpoints. This is
  metadata hygiene, not legal review. D15 now gives those four packages an
  exact advisory/package/checksum/immediate-introducer inventory that rejects
  vulnerabilities or any warning/graph drift; the upstream limitation remains
  accepted until maintained owners remove or replace it.
- [product baseline identity correction](../audits/reports/2026-07/2026-07-14/0.92-product-baseline-identity-correction.md)
  fixes `CANIC-092-AUDIT-017`: the previously cited `cfc49c36...` belonged to
  the Phase B implementation commit; exact published `v0.92.0` product identity
  is `c2b932cf...`. Full source commit identities and product evidence remain
  valid.
- [CI and release integrity](../audits/reports/2026-07/2026-07-14/0.92-release-integrity.md)
  is a valid `fail`: permissions/triggers and `actionlint` pass, but external
  Actions use mutable tags, executable tool downloads/installations lack
  immutable verified identities, the required dedicated secret scanner is
  absent, and no canonical supported host/target matrix exists.
- [D9 release execution integrity](../audits/reports/2026-07/2026-07-15/0.92-d9-release-execution-integrity.md)
  fixes the action, executable-tool, and support-matrix findings. Its affected
  subchecks pass. The dedicated-scanner gap that remained after D9 is fixed by
  D12 below, so the current affected release-integrity rerun passes.
- [D10 active documentation and hard-cut residue](../audits/reports/2026-07/2026-07-15/0.92-d10-active-documentation-and-hard-cut-residue.md)
  fixes the package-feature, active legacy-proof residue, and public-rustdoc
  findings. Manifest-derived docs proof, installed/packaged CLI proof, and
  generated/canonical packaged Wasm-store proof pass without product behavior
  or serialized-surface changes.
- [D11 canonical layering closure](../audits/reports/2026-07/2026-07-15/0.92-d11-canonical-layering-closure.md)
  fixes the remaining 18 ops-to-policy dependencies. Shared data belongs to
  model, root delegation-proof admission belongs to workflow, and the current
  executable layering guard passes with zero violations.
- [D12 dedicated secret scan](../audits/reports/2026-07/2026-07-16/0.92-d12-dedicated-secret-scan.md)
  fixes the final P1 evidence gap. The version/checksum-bound scanner runs over
  complete history with full redaction; 11 reviewed false positives are
  admitted only by exact fingerprints, and the rerun reports zero findings.
- [D13 workspace-only release lock synchronization](../audits/reports/2026-07/2026-07-16/0.92-d13-workspace-lock-sync.md)
  fixes the version transaction after the 0.92.11 bump advanced six unrelated
  transitive packages. A disposable comparison proves the workspace-only
  offline update changes only workspace versions and locks zero external
  packages. The fix is released in `v0.92.12`.
- [`v0.91.6` compatibility accounting](../audits/reports/2026-07/2026-07-16/0.92-v0916-compatibility-accounting.md)
  passes every required surface with the documented pre-1.0 hard cuts.
  Independently generated root Candid is byte-identical, old persisted state
  upgrades in PocketIC, and old build provenance rejects at the explicit
  regeneration boundary.
- Findings `CANIC-092-AUDIT-007`, `CANIC-092-DEPENDENCY-001`, and
  `CANIC-092-RELEASE-001` through `-004` are indexed in the 0.92 tracker.
- [layer boundary](../audits/reports/2026-07/2026-07-14/0.92-layer-boundary.md)
  remains invalid v1 history. Corrected
  [layer boundary v2](../audits/reports/2026-07/2026-07-14/0.92-layer-boundary-v2.md)
  uses fingerprinted direct/grouped import fixtures and an executable
  ops-to-policy rule. The guard lists 25 production violations and the valid
  frozen result fails at risk 7/10, fixing `CANIC-092-AUDIT-012`. Its
  API/DTO/product authority findings were subsequently fixed by D4 through D7
  and D11; the current guard reports zero violations.
- [build integrity v1](../audits/reports/2026-07/2026-07-14/0.92-build-integrity-v1.md)
  remains invalid history. Corrected
  [build integrity v2](../audits/reports/2026-07/2026-07-14/0.92-build-integrity-v2.md)
  excludes only observation timestamps and their derived digest from semantic
  provenance comparison. Two isolated lanes reproduce ordinary app and
  bootstrap-store raw/gzip bytes and app semantic provenance; final root
  Wasm/gzip bytes and semantic provenance still differ because absolute build
  paths enter generated runtime records and lifecycle logs. The valid result
  fails, fixes `CANIC-092-AUDIT-008`, and left `CANIC-092-BUILD-001` and
  `CANIC-092-BUILD-002` open at the frozen baseline.
- [D8 reproducible root artifacts](../audits/reports/2026-07/2026-07-15/0.92-d8-reproducible-root-artifacts.md)
  fixes both build findings. Two isolated offline lanes now produce identical
  root/bootstrap raw and gzip artifacts plus identical semantic provenance;
  final root Wasm contains neither temporary root. The host builder records
  transform role, kind, optional mode, tool, version, and applied, unavailable,
  or unrequested outcome through the required hard-cut provenance shape.
- [authentication invariants](../audits/reports/2026-07/2026-07-14/0.92-auth-invariants.md)
  found no accepting bypass: invalid trust, audience, subject, scope, replay,
  and attestation inputs reject in focused unit/PocketIC evidence. The original
  audience/replay v1 attempts remain invalid history. Corrected
  [audience/replay v2](../audits/reports/2026-07/2026-07-14/0.92-auth-invariants-v2.md)
  methods use current exact filters through a zero-test-refusing runner and
  validly pass at risk 3/10, fixing `CANIC-092-AUDIT-009`. D2 fixes
  `CANIC-092-ERROR-001` by preserving typed proof/provisioning causes. D7
  subsequently fixes `CANIC-092-LAYERING-004` by deleting the accidental
  public install-state DTO surface.
  The generated/session integration gap is fixed by the July 15 evidence slice.
- [mandatory trace admission](../audits/reports/2026-07/2026-07-14/0.92-mandatory-trace-admission.md)
  is `blocked` and `invalid`: the accepted design names ten mandatory trace
  IDs and requires a trace method ID/version/fingerprint, but no trace method
  was cataloged, fingerprinted, or frozen at that attempt. It remains invalid
  history. The superseding
  [mandatory traces v1](../audits/reports/2026-07/2026-07-14/0.92-mandatory-traces-v1.md)
  catalogs/fingerprints the protocol and executes all ten IDs, fixing
  `CANIC-092-AUDIT-010`. The later
  [evidence completion](../audits/reports/2026-07/2026-07-15/0.92-mandatory-trace-evidence-completion.md)
  closes auth/control execution gaps. Six traces pass and deploy/auth/control/
  blob fail on existing product findings; the valid aggregate `fail` completes
  the gate with no partial or blocked trace.
- [control-plane publication](../audits/reports/2026-07/2026-07-14/0.92-control-plane-publication.md)
  supporting evidence passes controller/root authorization, exact conflict
  refusal, and completed post-upgrade binding reconciliation. The later
  [D1 publication-safety slice](../audits/reports/2026-07/2026-07-15/0.92-d1-publication-safety.md)
  fixes `CANIC-092-COST-001` and `CANIC-092-ERROR-002` with one workflow-owned
  quota/cycle permit and typed publication causes. Quota, conflict, capacity,
  authorization, and interrupted-recovery PocketIC cases pass; store-GC
  behavior and public Candid shapes are unchanged.
- [D2 auth typed-cause preservation](../audits/reports/2026-07/2026-07-15/0.92-d2-auth-typed-causes.md)
  fixes `CANIC-092-ERROR-001`. All seven auth methods and the current auth
  trace pass; wrong-issuer, expired, and corrupted proofs reject without
  changing the installed active proof. Candid and stable state are unchanged.
- [D3 canonical layer contract](../audits/reports/2026-07/2026-07-15/0.92-d3-canonical-layer-contract.md)
  fixes `CANIC-092-LAYERING-003` and `CANIC-092-DOCS-001`. `AGENTS.md` is the
  sole active authority; public core docs, module headers, and hygiene guidance
  agree on the strict direction and model/storage ownership. No runtime,
  public, serialized, or stable behavior changes.
- [D4 root-issuer admission ownership](../audits/reports/2026-07/2026-07-15/0.92-d4-root-issuer-admission-ownership.md)
  fixes `CANIC-092-TEST-001` and removes seven root-issuer/model violations
  from `CANIC-092-LAYERING-005`. Workflow invokes pure policy before ops
  mutation; model owns state-shaped values. Public and stable shapes are
  unchanged. The canonical layering finding remained open after D4 and is
  fixed by D11.
- [D5 blob-billing workflow ownership](../audits/reports/2026-07/2026-07-15/0.92-d5-blob-billing-workflow-ownership.md)
  fixes `CANIC-092-LAYERING-001`. API now delegates Cashier sequencing,
  reserve/recovery, gateway sync, and readiness to one workflow over pure
  policy and single-step ops. Public DTOs, Candid, billing prices/protocol, and
  stable records are unchanged; the current blob trace passes.
- [D6 passive RPC DTO ownership](../audits/reports/2026-07/2026-07-15/0.92-d6-passive-rpc-dto-ownership.md)
  fixes `CANIC-092-LAYERING-002`. Request DTOs retain only boundary data and
  neutral constructors; one workflow command owns capability family, replay
  identity, and metadata attachment, while ops owns signed-payload projection.
  Public protocol, replay behavior, and stable state are unchanged.
- [D7 internal surface hard cuts](../audits/reports/2026-07/2026-07-15/0.92-d7-internal-surface-hard-cuts.md)
  fix `CANIC-092-LAYERING-004` and `CANIC-092-SURFACE-001`. The duplicate
  public proof-install request/outcome and direct core error root are deleted;
  the internal ops plan, model failure classification, and deliberate
  control-plane support bridge retain one owner. No Candid endpoint or stable
  schema changes.
- [security boundary ordering](../audits/reports/2026-07/2026-07-14/0.92-security-boundary-ordering.md)
  is a valid pass with watchpoints: generated endpoint, trust proof,
  subject/scope, capability, replay, and recovery-required paths retain their
  owning gate before handler execution or state mutation. It does not clear
  the owner-specific auth and publication findings.
- [bootstrap lifecycle symmetry](../audits/reports/2026-07/2026-07-14/0.92-bootstrap-lifecycle-symmetry.md)
  is a valid pass with watchpoints: root/non-root init and upgrade restore
  synchronously, failure traps before continuation, and bootstrap/user work is
  scheduled through zero-delay timers. Structural guards and three PocketIC
  lifecycle cases pass.
- [capability surface v1](../audits/reports/2026-07/2026-07-14/0.92-capability-surface-v1.md)
  remains invalid history because its broad workspace Clippy requirement
  conflicts with canonical targeted-test authority. Corrected
  [capability surface v2](../audits/reports/2026-07/2026-07-14/0.92-capability-surface-v2.md)
  uses its owning package test and targeted Clippy contract. Six retained DIDs
  rebuild, 19 protocol tests and Clippy pass, and the valid result passes at
  risk 4/10, fixing `CANIC-092-AUDIT-011`.
- [publish surface v1](../audits/reports/2026-07/2026-07-14/0.92-publish-surface.md)
  is a valid pass and first frozen-method baseline at risk 4/10. All eight
  public packages verify from isolated offline archives. D3 fixes
  `CANIC-092-DOCS-001`; D10's current-tree rerun fixes
  `CANIC-092-PUBLISH-001` and `CANIC-092-RESIDUE-001` with complete
  manifest-derived feature docs and maintained-command-only proof.
- [module structure v1](../audits/reports/2026-07/2026-07-14/0.92-module-structure-v1.md)
  is a valid fail and first frozen-method baseline at risk 7/10. It confirms 25
  production ops-to-policy imports, direct policy decisions in ops, and
  policy-owned values used by stable mappers (`CANIC-092-LAYERING-005`). It
  finds no cycle, public record leak, test/fleet seam breach, or module-layout
  escape. D10's current-tree warning-as-error rustdoc rerun fixes the separate
  `CANIC-092-DOCS-002` link without changing the frozen result.
- [DRY consolidation v1](../audits/reports/2026-07/2026-07-14/0.92-dry-consolidation-v1.md)
  is a valid fail and first frozen-method baseline at risk 6/10. Operator,
  evidence, backup, and release-proof responsibilities retain clear owners,
  but root-issuer policy admission duplicates the existing ops/policy
  authority defect. `CANIC-092-TEST-001` records the missing direct rejection
  and unchanged-state proof.
- [complexity accretion v1](../audits/reports/2026-07/2026-07-14/0.92-complexity-accretion-v1.md)
  remains invalid history. Corrected
  [complexity accretion v2](../audits/reports/2026-07/2026-07-14/0.92-complexity-accretion-v2.md)
  maps all 546 files, reproduces its normalized mechanical result, retains
  exact manual evidence, and applies one score. The valid first v2 baseline
  fails at risk 8/10, fixes `CANIC-092-AUDIT-013`, and retains P2
  `CANIC-092-COMPLEXITY-001`; 178 focused test selections pass and no auth
  correctness failure is inferred from the pressure measurements. Released D15
  fixes the live concentration by splitting all three named auth/root-proof
  hubs along existing responsibilities; the immutable baseline remains
  unchanged.
- [change friction v1](../audits/reports/2026-07/2026-07-14/0.92-change-friction-v1.md)
  remains invalid history. Corrected
  [change friction v2](../audits/reports/2026-07/2026-07-14/0.92-change-friction-v2.md)
  maps all 546 current files, freezes five exact feature slices, reproduces
  its normalized output twice, and applies one score. The valid first v2
  baseline fails at risk 8/10, fixes `CANIC-092-AUDIT-014`, and creates no
  separate product finding; its pressure remains deduplicated into layering
  and complexity. Seventy-four focused tests pass.
- [instruction footprint v1](../audits/reports/2026-07/2026-07-14/instruction-footprint.md)
  remains blocked/invalid history before producing any perf row. Pinned PocketIC 14.0.0
  starts and 11 scenario tuples are retained, but root-probe direct Cargo Wasm
  compilation is rejected by the authoritative `canic build` boundary. The
  runner composite is root-dependent, the exact scan misses 57 namespaced
  product checkpoints, and four required flow classes are absent
  (`CANIC-092-AUDIT-015`). Corrected
  [instruction footprint v2](../audits/reports/2026-07/2026-07-15/instruction-footprint-v2.md)
  uses only authoritative root-harness artifacts and a fixed 12-scenario
  update/install roster. All rows execute, 21 checkpoint deltas and 57 static
  checkpoints are retained, and the root-independent evidence hashes verify.
  The valid first baseline is `partial` at risk 6/10, fixes
  `CANIC-092-AUDIT-015`, and records P2 `CANIC-092-PERF-001` for missing
  root-proof and delegated-token internal checkpoints.
- [Wasm footprint v1](../audits/reports/2026-07/2026-07-14/wasm-footprint.md)
  is blocked/invalid before producing any artifact metric. The required clean
  linked-worktree run passes its Cargo/ICP/`ic-wasm`/`twiggy` prerequisites,
  then the first `app` direct Cargo Wasm build is rejected by the authoritative
  `canic build` hard cut. Its executable composite is root-dependent
  (`CANIC-092-AUDIT-016`). It remains invalid history. Corrected
  [Wasm footprint v2](../audits/reports/2026-07/2026-07-15/wasm-footprint-v2.md)
  removes the direct-Cargo/pre-shrink flow and builds all six release/debug
  role pairs through the authoritative host builder. Gzip, `ic-wasm`,
  `twiggy`, source-mutation, and evidence-hash checks pass. The valid first v2
  baseline passes at risk 4/10, fixes `CANIC-092-AUDIT-016`, and creates no
  product finding.
- [canic-core module surface hardening](../audits/reports/2026-07/2026-07-14/canic-core-module-surface-hardening.md)
  is a valid first frozen-method failure at risk 4/10. It confirms
  `CANIC-092-LAYERING-004` and adds P2 `CANIC-092-SURFACE-001` for the
  unnecessary hidden-but-public core error path. Generated, replay, state,
  sibling-support, and test-only surfaces otherwise retain current owners. D7
  fixes both product findings without rewriting the frozen result.
- [Phase C baseline review](../audits/reports/2026-07/2026-07-14/0.92-phase-c-baseline-review.md)
  remains the original blocked synthesis. The live ledger now has 22 valid
  and zero invalid active results after the instruction and Wasm corrections.
  The final mandatory trace result is valid and complete at aggregate `fail`.
  D1 fixes two non-waivable publication P1 findings, D2 fixes the auth cause
  P1, D3 fixes one P1 authority conflict plus one P2 documentation drift, D7
  fixes two P2 accidental public surfaces, and D8 fixes the root-build P1 plus
  transform-provenance P2. D9 fixes the two release-execution P1s plus the
  support-matrix P2. D10 fixes two P2 active package/proof findings and the P3
  rustdoc drift. D11 fixes the remaining layering P1, and D12 fixes the
  dedicated-scanner P1 without a waiver. D14 fixes the performance P2 and D15
  fixes the complexity P2; one accepted upstream dependency P2 remains.
  All current trace reruns pass; the frozen Phase C aggregate remains
  historical.

## Focused Validation

- Clean release/tag/origin identity, method-path equality, method fingerprints,
  product-tree hash, and path classification pass.
- Audit catalog, affected Bash syntax, operator guards, `actionlint`, focused
  instruction identity/baseline tests, changelog governance, formatting, and
  diff hygiene pass.
- Locked/offline Cargo metadata/tree and the dependency-v2 declaration rule
  pass. `cargo-audit 0.22.2` found zero vulnerabilities using advisory DB
  commit `9f3e1380...` from 2026-07-13.
- The original credential-pattern scan found no match. D12's dedicated
  Gitleaks 8.30.1 full-history scan also passes after 11 false positives were
  classified and admitted only by exact historical fingerprints.
- Layering v2 detector fixtures pass. Its immutable baseline validly fails on
  25 production ops-to-policy dependencies; D4's affected-scope rerun reduces
  the live set to 18. Policy purity, passive DTO, and root-issuer ownership
  checks pass.
- Build-integrity v2's frozen baseline retains the root-path failure. D8's
  affected-scope rerun executes two new isolated root lanes: root/bootstrap
  raw and gzip artifacts plus semantic provenance reproduce exactly, neither
  final root contains a temporary lane path, and all four local transforms
  record `ic-wasm 0.9.11`.
- Focused authentication tests pass across macro/access, proof-chain, audience,
  scope, replay, role-attestation, chain-key batch, root facade, and capability
  rejection paths. Audience/replay v2 selections are all nonempty and passing;
  their runner rejects a successful Cargo zero-test selection with exit 3.
- The exact delegated-session PocketIC case passes pre-bootstrap rejection,
  successful bootstrap and guarded-call parity, idempotent replay,
  conflicting-wallet rejection, and unchanged authority after rejection.
- D2 focused validation passes 263 all-feature auth library tests, every
  nonempty audience/replay v2 selection, trust/capability selections, strict
  targeted Clippy, and PocketIC facade/session/renewal/attestation/capability
  cases. Wrong-issuer, expired, and corrupt proof installs return typed codes
  without active-state mutation.
- D3 active-contract scans, changed-file Rust formatting, strict `canic-core`
  Clippy, and package verification pass. Layering self-tests pass; the full
  guard retains the same 25 known product-code violations. D10 later fixes the
  separately indexed public rustdoc link.
- D4 validation passes 881 all-feature `canic-core` library tests, four pure
  policy/passive DTO guards, 19 public protocol tests, strict targeted Clippy,
  and two root proof/renewal PocketIC regressions. Direct workflow tests prove
  positive policy/template admission, every request rejection boundary,
  unchanged state/epoch, and skipped timer start.
- D5 validation passes 878 all-feature `canic-core` library tests, 50 focused
  blob-billing tests, four policy/DTO guards, 19 protocol tests, strict
  all-feature/all-target core Clippy, and four PocketIC billing selections.
  Reserve refusal performs no partial top-up, transient Cashier failure
  releases the guard for retry, status boundaries remain distinct, and billing
  configuration survives upgrade.
- D6 validation passes 51 workflow RPC tests, eight RPC ops tests, four replay
  manifest tests, four policy/DTO guards, 19 protocol tests, strict
  all-feature/all-target core Clippy, and exact PocketIC identical-replay and
  cross-family-conflict cases. The capability trace remains pass and the live
  layering set at the D6 boundary remains 18.
- D7 validation passes all-feature/all-target core and control-plane checks,
  four provisioning tests, 20 chain-key batch tests, three DTO/serialization
  guards, 19 protocol tests, all-feature facade check, strict
  core/control-plane Clippy, offline core/facade package verification, the
  feature-owned control-plane facade test, and exact PocketIC new-issuer
  provisioning. The live layering set at the D7 boundary remains 18. D10's
  later current-tree rustdoc rerun passes with warnings denied.
- Focused D1 publication validation passes 18 all-feature publication tests,
  30 replay/capability policy tests, four core and one publication-owned
  cost-permit structural checks, strict targeted Clippy, and five root/store
  PocketIC filters. Ten publications are admitted in one quota window; the
  eleventh rejects before fleet mutation. Conflict, capacity, missing release,
  transport, and invalid-state outcomes retain typed public codes.
- The exact interrupted-publication PocketIC case commits target material
  before the root mirror, upgrades root, reuses the same store, and converges
  to exactly one root-projected owner.
- Focused security-ordering additions pass 18 tests across token prepare,
  lazy repair, public prepare, replay abort, and recovery-required receipt
  preservation.
- Lifecycle validation passes 2 structural boundary tests, 1 trap boundary
  test, and 3 PocketIC install/upgrade/failure tests.
- Capability validation rebuilds six retained artifacts, passes 19 protocol
  tests and targeted Clippy, and confirms intended root/non-root/issuer
  placement. Required workspace Clippy is blocked by policy and not fabricated.
- Publish validation passes locked/offline metadata, seven workspace-manifest
  tests, the release package/install definition guard, installed and packaged
  CLI proof, and generated plus canonical packaged Wasm-store builds.
- Structure validation passes isolated public-surface mapping, module layout,
  crate-cycle, test/fleet seam, and five focused DTO/policy boundary tests.
  D10's current-tree core rustdoc passes with warnings denied.
- DRY validation passes 8 core root-policy, 19 host registry-related, 2 host
  response-parser, 30 CLI output-filtered, and 19 backup persistence tests.
  Direct ownership traces find no competing registry, output, evidence,
  backup, or release-proof flow beyond the indexed issuer-policy defect.
- Complexity v2 reproduces its complete scope/counter output, maps every file,
  and applies one deterministic score. Five focused filters pass 178 selected
  executions; the valid result fails at risk 8/10 on the bounded P2 hotspot.
- Change-friction v2 maps all 546 current files, reproduces normalized digest
  `aac8db07...` twice, applies one score, and validly fails at risk 8/10.
  Five exact slices and 74 focused RPC/capability/delegation tests complete.
- Instruction v1 fails closed at the obsolete fixture build path and remains
  invalid history. Corrected v2 passes 37 focused tests plus its ignored live
  generator, executes all 12 isolated PocketIC scenarios, retains 12
  normalized rows and 21 checkpoint deltas, finds 57 static checkpoints, and
  verifies every compact evidence hash. The valid result is partial only on
  the two indexed auth-flow checkpoint gaps.
- Wasm v1 fails closed at the obsolete direct build and remains invalid
  history. V2 completes 12 authoritative role/profile builds in a clean
  detached worktree, verifies every builder gzip pair, analyzes all six
  release artifacts with `ic-wasm` and `twiggy`, preserves tracked product
  source, and verifies the primary report plus ten compact artifacts.
- Module-surface D7 validation passes all-feature `canic-core` check, focused
  provisioning and facade proof-surface tests, protocol and package checks,
  and direct consumer scans. The duplicate serialized batch request/outcome
  and direct core-error root are removed; the maintained control-plane support
  bridge remains.
- D8 validation passes 7 artifact-transform tests, 15 build-provenance/policy
  tests, 12 release-set manifest tests, targeted checks and strict Clippy, and
  two isolated offline root builds. The current deploy trace now passes; the
  finding-backed two-lane proof is the admitted affected-method rerun. The
  frozen 12-artifact Wasm v2 baseline remains unchanged historical evidence.
- D9 validation passes the 13-action immutable-pin guard, positive and
  rejection checksum proof, all five external executable install/version
  probes, canonical caller-override rejection and exact IC tool checks,
  workflow and shell lint, 22 focused Rust tests, targeted strict Clippy, the
  host package probe, and matrix/changelog/diff guards. The integrity guard
  discovers the maintained version/checksum surface instead of duplicating a
  variable ledger, and `update-dev` no longer updates unrelated toolchains or
  workspace dependencies. Release mutation is transactional on failure,
  one-shot phases are sequential under parallel Make, and push rejects unless
  the clean release commit and annotated tag match before an atomic ref update.
- D10 validation passes seven manifest-derived package tests, positive CLI
  unit proof, warning-as-error core rustdoc, strict targeted Clippy, installed
  CLI proof, packaged CLI proof, and generated plus canonical packaged
  Wasm-store builds. Bash syntax, ShellCheck, targeted formatting, and diff
  hygiene pass.
- D11 validation passes focused environment, funding, topology, placement,
  metrics, pool, and auth tests; default and sharding core checks; strict
  all-target/all-feature core Clippy; and the executable layering guard with
  zero production ops-to-policy dependencies. Public, serialized, stable,
  configuration, and dependency surfaces are unchanged.
- D12 validation passes the checksum-bound Gitleaks install and exact-version
  probe, redacted full-history scan, unavailable/near-match version,
  rule-configuration override, shallow-history, and argument rejection probes,
  release-integrity and validation-matrix guards, `actionlint`, Bash syntax,
  changed-script ShellCheck, and `make gitleaks-scan`.
  The admitted result has zero unreviewed findings and retains no raw report.
- D13 validation passes a disposable 0.92.10-to-0.92.11 workspace-only lock
  synchronization with zero external updates, the release transaction rollback
  test, current locked/offline metadata and license checks, cached advisory
  scanning, the release-integrity guard, Bash syntax, and ShellCheck.
- D15 focused validation passes the exact dependency inventory and rejection
  fixtures, release guards, `actionlint`, ShellCheck, 165 auth tests, 11 auth
  prepare workflow tests, strict all-target core Clippy, formatting, and diff
  hygiene. Every resulting auth/root-proof production owner is below 600
  logical lines. The slice is released in `v0.92.14`.
- Released 0.93.0 stale-renewal cleanup passes targeted core renewal, chain-key
  batch, metrics, CLI auth, Candid protocol, changelog-governance, and isolated
  installed-CLI proofs; strict all-target Clippy for `canic-core`, `canic`, and
  `canic-cli`; targeted package checks; formatting; Bash syntax; and diff
  hygiene.
- Released Wasm-store lifecycle and restore-receipt development passes 21
  focused control-plane publication/GC tests, 34 restore apply-journal tests,
  targeted package checks, and strict all-target Clippy for
  `canic-control-plane` and `canic-backup`.
- Released ICP-refill, renewal diagnostics, and test-layering development passes
  80 focused ICP-refill tests, 15 renewal tests, 22 control-plane publication
  tests, strict all-feature Clippy for `canic-core` and
  `canic-control-plane`, the layering guard, and Cargo Machete.
- Slice E compatibility validation passes tagged root/Wasm-store Candid and
  production CLI/config/package comparisons, a `v0.91.6`-to-`v0.92.11`
  PocketIC state upgrade, 52 current stable-record tests, 19 protocol tests, 7
  manifest tests, 15 provenance/policy tests, and 195 backup/restore tests.

## Next Action

Freeze Toko's per-mint action identity, recovery flow, stack-mint disposition,
and batch/rate/resource-cardinality envelope. The
read-only downstream tree remains unchanged at the recorded snapshot and still
supplies none of those missing contracts. Eligibility shape, observation grace,
and physical settlement reservation are now fixed in open `0.96.4`; use the
accepted downstream values before enabling bounded deletion or timer work. Do
not extend 0.96 into unrelated cleanup.

The [0.92 release-line closeout](../audits/release-lines/0.92-closeout.md) is
preserved at its immutable `v0.92.12` anchor with
`closeout_verdict: pass_with_limitations`. Post-closeout D14 is released in
`v0.92.13` and fixes the performance watchpoint with existing instrumentation.
D15 is released in `v0.92.14`; it fixes the concrete complexity concentration
and controls the remaining upstream dependency risk through an exact
fail-closed inventory. All 28 P1 findings are fixed; no deferred or blocked
finding remains, and one accepted P2 external limitation keeps the 0.92
verdict at `pass_with_limitations`.

The `0.93.0` through `0.93.36` audit slices are released and the line is
closed. They hard-cut stale
runtime, host, transport, discovery, replay, placement, intent, recovery, and
validation authority while preserving the intentionally read-only endpoint
metadata/Candid behavior. Released `.25` corrects `.24`'s selected target
terminology: `staging`, `local`, and `ic` are environments selected by
`canic --environment` and ICP CLI `-e`; they are not direct network arguments.
Keep `environment`, `artifact_environment`, `build_network`, backing network,
and `runtime_variant` distinct as defined in the active build-artifact
vocabulary. Do not reopen removed selected-target `network` fields, CLI flags,
JSON keys, aliases, or direct named-network paths.

The accepted 0.94 design has confirmed its exact operation and durable-
transition inventory and completed the early disposable-platform gate.
Snapshot create, upload, stopped-target restore observation, and exact repeated
restore are available through a managed local ICP deployment. Journal and
command-lifetime ownership plus the restore pending-recovery hard cut are
released through `v0.94.1`; the frozen executable protocol baseline is
released in `v0.94.2`.

Released `v0.94.3` through `v0.94.5` prove preflight, every post-preflight
pending claim, stop, snapshot creation, created-artifact publication, and
start across their assigned process-death boundaries. Exact lifecycle status
and snapshot inventory reconcile committed effects without duplicate
commands.

Released `v0.94.6` completes `B09`: a pending snapshot download resumes only
from exact `Created` artifact authority, replaces uncommitted private staging,
and rejects unsafe entries.

Released `v0.94.7` completes both `B10` write sides. A non-durable
`Downloaded` transition retains `Created` and performs one redownload; a
durable exact transition reconstructs the normal receipt and proceeds to
checksum with zero download commands.

Released `v0.94.8` completes `B11` with deterministic process-death proof. A
checksum lost with child memory is recomputed from unchanged staged bytes,
while missing or unsafe input never becomes verified progress.

Released `v0.94.9` completes both `B12` and both `B13` write sides. Durable
checksum rows, initial publication, and canonical recovery now share exact
checksum-bound artifact authority.

Released `v0.94.10` completes both `B14` and both `B15` write sides. Restart
verifies every durable artifact in place, then publishes or adopts only the
exact manifest derived from current authority. Missing or changed bytes,
conflicting manifests, and premature manifests fail closed.

Released `v0.94.11` completes all 12 `B16` terminal execution receipt/state
publication cases and `B17` final-successful-response loss.

Released `v0.94.12` retains the complete `B18` command-tree proof and fixes
the defects exposed by closeout tracing and the first realistic recovery run.
`canic backup create` is now the sole capture authority; prepare and prune
cannot erase recovery evidence; backup failures restore availability; and
restore upload, lifecycle, load, and verification each have one
operation-specific recovery rule. Restore verification proves running module
identity rather than command success alone.

Initial restore preparation now also has deterministic process-death evidence.
An unpublished plan or journal is ignored, an exact directory-synced document
is adopted, and journal creation follows only an exact validated plan. All
four `CANIC-094-R01` and `CANIC-094-R02` cases pass without a production
failpoint or alternate persistence flow.

The disposable local-ICP journey backed up deterministic `user_hub` and
`user_shard` state `A`, resumed after a real ICP 1.1 inventory-shape failure,
mutated both canisters to `B`, restored across independent runner processes,
and queried both as `A`. Completed backup and restore replay executed zero
operations. A separate acknowledged `SIGKILL` case killed the runner after a
real snapshot upload and before its receipt; restart adopted exactly one new
inventory identity without another upload and completed the journal. The
temporary network is stopped.

The 0.94.12 hard cut removes the old snapshot CLI/library path and failed
prune selector, changes the exact version-1 restore-document field set, and
requires ICP CLI `>=1.1.0,<2.0.0`. No version 2, alias, legacy parser, or
compatibility fallback exists; Candid and Cargo package versions are
unchanged.

Released 0.94.13 completes `R03`, all 12 `R04` pending-claim cases, all 12
`R12` terminal state/receipt cases, and `R13` final-response loss. It also
removes private upload staging before terminal persistence and during
committed-effect reconciliation, so process death cannot retain a completed
staged snapshot after the journal advances.

Released `v0.94.14` completes `R05`, `R07` through `R11`, all four `R14`
owner-dead command-tree cases, and `C01` through `C10`. Returned stopped-state
observation I/O failures now persist one failed operation/receipt pair before
preserving the typed cause. All 106 frozen cases and all seven required
journeys pass. The line is closed; further backup/restore work requires a new
design.
