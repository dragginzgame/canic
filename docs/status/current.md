# Current Status

Last updated: 2026-08-29

## Purpose

This is the compact handoff for new agent sessions. Read it first, then inspect
only the active source, validation, design, or changelog material needed for the
current task.

Historical handoffs: [through 2026-06-30](archive/2026-06-30-precompact.md),
[through 0.90.2](archive/2026-07-13-precompact.md),
[through 0.101.52 Q4](archive/2026-08-12-precompact.md), and
[through published 0.109.12](archive/2026-08-26-pre-root-repair-hard-cut.md).

Published `v0.109.19` at
`8be7c210e6ef7a2b707139c34a47989ac4fd810b` is the immutable maintained
release. Published `v0.109.16` at
`045f131224506bfadabfdb258471cd9b9745d8c8` remains immutable but unqualified:
its complete gate stopped at warning-denied Clippy while the former release
shell continued through versioning, tagging, push and package publication.
Published `0.109.19` closes the public Binaryen installer's staged writer
before executing the checksum-admitted candidate, correcting Linux `ETXTBSY`
without weakening optimizer authority. It also makes an in-progress Fleet
operation self-sufficient: new plans digest and retain their reviewed desired
authority, and the exact current-schema zero-debit operation already issued by
the downstream staging Fleet can be observed and closed without a second
command before newer desired state is planned separately.

The interrupted registry publication exposed six immutable `0.109.16`
packages (`canic-backup`, `canic-core`, `canic-control-plane`, `canic-macros`,
`canic`, and `canic-fleet-coordinator`). `canic-host`, `canic-cli`, and
`canic-wasm-store` were not published at that version when reconciled. No yank
or further `0.109.16` publication was performed by this correction pass.

Release versioning no longer treats that descriptive lineage prose as a
machine authority. Uniform package versions, the immutable tag and the exact
validated-source marker remain the governed release facts.

## Current Decision

CANIC-059 is now the maintained host architecture. `canic fleet ensure
<fleet>` is the sole Fleet installation/convergence workflow. Its plan-only
form observes the current configured estate and retains one immutable reviewed
plan. `--apply <plan_sha256>` advances only that plan. Fresh install,
deployment-plan, historical-plan, retained-recovery, retained-Root repair,
recovery-bundle, installed-Fleet cache, adoption, and autonomous Root-deletion
owners are removed rather than adapted.

The current contract is schema `v1` and reads no historical operator evidence.
Its only local authorities are the current desired Fleet document and
`.canic/fleet-ensure/<environment>/<fleet>/` current-generation plan, retained
reviewed desired authority, journal, identity map, and operation lock. Unknown
schemas, changed desired/artifact digests, changed authority, and unreviewed
balance drift fail closed.

Every mutating action has a retained intent before the platform call. Lost
responses are resolved by exact live-state reconciliation or an idempotent
Ledger/drain operation identity before retry. The stall count is consecutive
and resets only on durable progress. A terminal invocation completes in the
same call; its immediate successor plan has no mutation actions when the live
estate already equals desired state.

New plans retain their normalized reviewed desired input inside the plan hash.
An in-progress plan is returned before newer working-file bytes are considered,
and every platform adapter must rebind to the retained input before observation
or effect handling. With an explicit environment, the CLI can reopen that
authority even if the working desired TOML is absent. Plans created before this
retention field fail closed unless the exact original input is supplied, except
for one current-schema no-debit terminal boundary: an all-reused plan whose
preceding actions are applied and whose final typed Component-provisioning
action is issued may use a current document only to prove the exact retained
name/Principal set and query terminal state. That path cannot issue an update,
funding or canister effect; protected terminal inventory and cycle conservation
remain mandatory before the journal closes.

The same verified pre-retention path compacts any inline Store chunks into the
maintained content-addressed projection before its first resumed platform call.
The logical plan digest and journal remain unchanged, so the retained 37 MiB
projection disappears without weakening replay authority or waiting for a
later successor plan.

## Cycle-Safety Boundary

The plan reports the complete observed controlled balance, retained balance,
scheduled transfers, maximum fees, bounded observation/update burn, maximum
new funding, maximum operator debit, every canister disposition, and the
reviewed post-operation conservation equation. Apply refuses a changed plan,
insufficient operator balance, or a debit/burn above that bound. Terminal
evidence records:

```text
observed starting cycles
+ received new funding
- measured execution/observation burn
= final controlled cycles
```

Controllers cannot pull cycles out of an arbitrary IC canister. A canister
with a material balance may therefore be replaced or deleted only when its
current desired entry supplies an exact treasury-bound, idempotent drain
method and Candid contract. Otherwise planning returns `NoSafeDrain` and leaves
the canister untouched. A stopped-state and residual-balance check immediately
precedes deletion. Creation charges, Ledger fees, requested initial funding,
and execution/observation burn remain separate quantities.

An accepted drain response is not conservation proof. The journal retains
source and treasury balances from before the call and requires a fresh bounded
source debit plus the exact controlled-treasury credit before stop or delete.
Similarly, a successful control-plane update marks only `issued`; later work
remains fenced until the exact status query proves terminal application.

The current host journal prevents duplicate effects across interrupted or
repeated invocations sharing the operator-state root. Ledger create/withdraw
and configured drain effects additionally use exact replay identities. A
globally distributed lock across independent operator-state roots is not yet
provided; do not run concurrent apply commands from different Canic state
roots.

Fleet Ensure schema `v1` now owns one exact JSON projection for the complete
current plan. Every nested `u128` cycle amount is bounded decimal text, and the
matching human-readable `Cycles` decoder accepts that exact form even inside
Serde's internally tagged current protocol enum. Candid and binary encodings
remain exact `u128`; plan and action hashing retain the same decimal authority,
so reopening a retained plan does not change its digest or replay identity.
New plan documents retain only the Store chunk hash, bounded size, template,
version and index. Exact bytes live under
`.canic/fleet-ensure/objects/sha256/`; write verifies any retained prepared
authority before retention, and read rejects missing, unsafe, oversized or
hash-mismatched objects before an action can be observed or issued. Both
projection directions discriminate the outer Fleet-protocol action before
reading Store-specific fields; generic protocol steps remain byte-for-byte
outside the content-addressing owner. Object hydration now rejects an invalid
declared bound before access, preflights the no-follow descriptor size before
allocation and reads no more than the expected size plus one byte. Concurrent
growth, truncation, links and same-length hash drift all fail closed.

## Current Completion State

The current candidate converges canister existence, code, controllers, running
state and cycles. It now compiles one ordered current protocol graph from exact
role authority: Store release-set and artifact staging, Root Store adoption and
bootstrap, deterministic Coordinator Registry joins, Root synchronization,
Registry activation, Root-mirror activation, exact local Component Registry
preparation, then Component provisioning and Fleet-catalog publication. Every
response remains issued until its exact protected status proves terminal state.
The terminal observation calculation extends its Component-derived bound only
for `ProvisionComponents`; every preceding supported Fleet protocol action
retains the accumulated bound rather than entering a Component-only branch.
Store adoption retains the protected operator plus owning Root as its one
terminal controller set. An explicitly seeded retained Store may begin
Root-only; the Root durably records the exact current authority, adds only the
protected operator, and re-observes the final set before the Store install can
run. Foreign controller sets fail closed, and no temporary/final compatibility
schema is retained.

Retained convergence now preserves distinct deterministic Root and Store
installation identities. Root retains a Store child authority bound to the
exact Fleet, operation, principals, release build, topology, controllers,
manifest and credential generation; Root-owned state continues to use the Root
identity while Store prepare, status, resume and activation calls use the Store
identity. Typed retryable-pending observation may replay the exact retained
issued Component-provisioning command once without recompiling it. A stopped
retained Coordinator, Root or Store is started under its existing Principal
before role queries, without funding, reinstall, replacement or recreation.
The focused real-Wasm PocketIC journey reaches terminal Root activation and
non-empty inventory with conserved cycles and identities, then converges again
without effects. Further fresh-estate expansion remains paused behind this
retained-estate correction.

The high-level `canic fleet generate <fleet>` owner now compiles the low-level
desired document from protected Fleet policy and an exact finalized complete
release set. Retained generation accepts one explicit live estate seed and
verifies its Coordinator, Root, Store, pool and treasury identities against
the active operator, Registry-backed placement, direct management evidence and
protected Root inventory. Fresh generation instead creates or exactly replays
one durable no-effect seed containing a random Fleet ID, exact Cycles Ledger
and management creation fee, and logical Coordinator, Root, Store and initial
pool roles. Release metadata invents no Principal in either mode.
Generation binds both child digests in the finalized complete release set,
derives each infrastructure Candid sidecar from the manifest-bound Wasm path,
reads it without following links and verifies the retained digest, rejects
duplicate retained identities and unexpected co-controllers, and checks the
complete live Fleet/Coordinator/Root/Store relationship before output. Output
is create-once by default; a changed generated document requires the exact
current file SHA-256 through `--replace`, rejects concurrent drift and is
published atomically. Every paid retained Root-owned asset remains inside the
observed conservation total through idle, claimed and workload lifecycle
states; a missing retained identity still fails instead of becoming a
replacement creation. Fresh generation performs no paid effect. Its ordinary
Fleet Ensure plan records unallocated roles and the exact maximum debit, and
apply durably journals each Cycles Ledger creation before resolving dependent
controller and treasury Principals from retained results. Both modes keep
observation and update burn as conservative measured ceilings.

The governed production five-Component PocketIC journey now begins from a
fresh estate and traverses that complete typed graph through terminal catalog
publication. It then recompiles the exact live successor Registry against the
retained Component operation receipt and proves an immediate second run has no
nonterminal action or update effect. The control-plane convergence evidence gap
is closed without restoring a deleted install or recovery owner.

The direct Canic runtime exact-pins `ic-timers 0.7.0` and uses its
policy-specific watchdog reconciliation state without changing Canic's
cadence-backed recovery contract. The composed-framework lifecycle fixture
resolves the exact published IcyDB 0.246.0 runtime and model family. Both now
share the one locked `ic-timers 0.7.0` provider. Dependency edges into that
family are confined to the two unpublished fixture packages, while published
Canic package graphs remain IcyDB-free. The host-only published
`canic::testing` feature now owns the generic managed-App test boundary: exact
grouped init and Directory authority, initial fencing/activation, protected
status, successor fencing, same-release upgrade and standalone-local install.
It exposes no runtime storage, endpoint, timer or lifecycle authority and
removes the need for downstream test adapters to pin private `canic-core` or
`ic-testkit` construction APIs.

The validation runner retains the August shared PocketIC server, one-process
governed suite, persistent artifact cache and ordinary-before-PocketIC barrier.
The release lane now rejects malformed draft/status metadata before entering
that runner and retains an exact-HEAD local success receipt after the clean
complete gate. A later release-only failure can resume the same immutable
candidate without repeating the complete suite; any source commit change
invalidates the receipt.
The enlarged 0.109 graph had nevertheless regressed ordinary validation to six
sequential Cargo invocations and expanded the internal governed inventory from
22 to 32 cases. The current runner batches the four package-owned integration
groups into one multi-package invocation, distinguishes libtest parallelism
from suite concurrency, reports total wall time and compiler-cache deltas, and
prints the ten slowest governed cases. Make retains `sccache` for the complete
two-hour test envelope with a 40 GiB default cache; a server reset is explicit
rather than indistinguishable from an unused cache.

`app config`, admission, auth-renewal status, backup, blob-storage, cycles,
funding status, `info endpoints`, `info env`, `info list`, `info metrics`,
`info subnets`, inspection, Medic, restore, status and token operations are
exposed against terminal current ensure inventory. Protocol-bound operations
resolve exact Registry-retained Candid bindings and fail closed when the current
inventory does not retain them. Subnet reporting requires the retained
Coordinator/Root bindings and a complete agreeing live Registry/Root snapshot.
Medic reports current desired/plan drift, exact topology, Registry authority
and reviewed conservation bounds without reading or recommending deleted
install or recovery state. The old funding-policy rotation flags are not
restored because their mutation authority came from the deleted install plan;
`cycles funding` is current protected status only.

## Scope Removed

- Historical install contracts, version-specific plan loaders and patch-pair
  recovery allowlists.
- Retained Root repair, provisional successor authority, repair receipts and
  content-addressed recovery-bundle import/verification.
- Former fresh-install/deploy/adoption/installed-catalog host owners and their
  public CLI modes, aliases, diagnostics and compatibility-only tests. Fresh
  estate seeding is a no-effect input to the sole current Fleet Ensure owner,
  not a restored installer.
- The dedicated retained-repair fixture and Root-deletion examples.

Historical release notes and archived audits remain truthful history; they are
not active contracts.

## Validation State

Open `0.109.16` coding-time evidence passes at the published `0.109.15`
predecessor worktree: locked `canic-host`/`canic-cli` all-target compilation,
warning-denied all-target Clippy for both changed packages, changelog
governance, and focused tests for the process-backed retained-estate planner,
same-module reinstall plus effect-free replay, Store-before-Root ordering,
target-local funding diagnostics, stable Component-failure progress identity
and ICP status canister-version decoding. Focused generation and current-plan
tests reject a 4.8T pool target for a 5T admitted Component before effects, and
the Store-adoption predicate rejects immutable authority without the exact
operation receipt. Native funding also remains issued until a fresh canister
observation reaches the reviewed post-balance; a Ledger receipt alone is not
completion. The exact Toko-shaped Fleet Ensure PocketIC test also passes
with a same-module Root reset proved by a strictly newer management canister
version and an effect-free successor apply. No broad workspace or broad
PocketIC gate was run during coding, as required by repository policy.

The Canic-side `ic-timers 0.7.0` slice passes locked all-target checks and
warning-denied all-target Clippy for `canic-core` and `canic-control-plane`,
five core timer-custody tests, five Root canister-pool tests, the focused native
ownership guards and changelog governance. Published IcyDB 0.246.0 now resolves
the same timer provider, so the combined lifecycle fixture is active again. Its
targeted governed PocketIC journey passes in 11 seconds with one shared timer
inventory across install, prepared, active and upgrade boundaries.

Published `0.109.17` centralizes complete and fast versioning in
one `set -euo pipefail` owner. Validation failure, fast-eligibility failure,
dirty state or source drift now exits before the version bumper receives its
validated-source environment. Its executable fixture proves each negative path
and the exact successful authority handoff. The two timing-report format calls
that stopped `0.109.16` Clippy use the maintained inline argument form.

Published `0.109.18` aligns IcyDB `0.246.0` on the single
`ic-timers 0.7.0` provider and repairs the current Fleet Ensure JSON boundary.
Focused production-writer coverage passes for Registry join and activation
with `u128::MAX` cycle values. An isolated copy of the exact downstream
37,131,114-byte plan and 8,448-byte issued journal decodes and re-encodes
without changing the source files, embedded plan digest, operation ID,
conservation totals or ordered action hashes. Its isolated canonical rewrite
removes every inline Store payload and is less than one tenth of the retained
file size. The evidence covers all eleven current protocol variants. The issued
action is observed terminal without a second command, and its immediate
successor ensure performs zero mutations. A focused object-store regression
also proves fail-closed rejection after content tampering.

That release exposes one supported feature-selected standalone-local build.
Its declaration pass produces the exact adjacent `.did`; its deployable runtime
uses the same Cargo feature set without the declaration cfg, pointer export or
embedded public Candid. Finalization compares the runtime query/update export
inventory to the parsed sidecar and fails before `ICP_WASM_OUTPUT_PATH` copying
on any mismatch. Normal managed builds retain their existing local metadata
policy, while IC builds remain metadata-free.

Release-profile artifacts now have one canonical finalizer for configured
Components, Fleet Coordinator and Wasm Store. It requires the checksum-bound
official Binaryen 108 identity, derives the input's admitted IC feature flags,
applies `-Oz`, and rejects export, embedded public-Candid or feature drift
before replacing the staged input. Gzip, artifact hashes, release sets, Store
publication and module-hash authority consume only those optimized bytes; fast
and debug builds do not request the transform, and release has no unoptimized
fallback. Admission verifies the platform-specific executable SHA-256 before
running the selected path, and `canic toolchain install` provides the published
checkout-independent repair path. Provenance records that executable digest
alongside the exact version and before/after raw, gzip, code-section, data-
section and defined-function values. The recurring footprint method covers all
nine Canic-owned roles and requires exact identity across two clean release
builds.

The focused real App build reduced section 10 from 2,997,977 to 2,827,666
bytes through Binaryen. Named post-optimization Twiggy evidence then identified
only stable sorting as a material Canic-owned residual. Removing hidden
`BTreeMap::collect` sorts in Component topology and sharing the unique-ID total
order used by three chain-key batch selectors reduced the final measured code
section again to 2,800,001 bytes and defined functions from 5,009 to 4,963.
Fourteen topology, sixteen Component-deployment and twenty-two chain-key batch
tests pass, alongside the focused host artifact/provenance suites and
warning-denied host and core Clippy. IcyDB's retained-kernel scan residual is
recorded as upstream feedback only; no IcyDB repository was changed.

The CANIC-079/080 follow-up has targeted authority and I/O evidence passing:
seven Binaryen-filtered tests, all fifteen artifact finalizer tests, nine build-
provenance tests, fifteen durable-I/O tests, three direct Store-object tests,
two CLI toolchain tests, top-level ordering and the recursive CLI help check.
The selected executable is rejected on digest mismatch before execution and
the diagnostic names its path and public repair command. Warning-denied all-
target Clippy for `canic-host` and `canic-cli`, the release-integrity contract,
the Binaryen installer ShellCheck and changelog governance also pass. No
complete workspace or broad PocketIC gate was run during this coding slice.

Read-only Toko Miner inspection confirms the managed and standalone-local
PocketIC journeys cover install, Canic/IcyDB readiness, admission, operations,
state and timer restoration, same-Wasm upgrade and fencing. Its current dirty
qualification script remains pinned to Canic `0.109.17` and builds the fast
profile, so the supplied optimized managed journey is useful evidence but not
the final frozen-candidate release-pipeline gate. Toko must bind that gate to
the candidate's canonical release artifact; Canic did not mutate Toko.

Published `0.109.17` also reduces endpoint-heavy Canic Wasm without
removing managed behavior. Macro-generated handlers share non-generic
instrumentation, standard update payload limits use the runtime fallback, and
default Fleet guards skip unused caller/context construction. Final artifacts
also omit the declaration-pass-only `get_candid_pointer` export while retaining
their extracted `.did` and local Candid metadata. On the controlled 256-endpoint
release fixture these changes reduce section 10 from 3,360,770 to 3,257,052
bytes and defined functions from 6,458 to 6,260. Canonical finalization now
warns at 9.25 MiB and rejects section 10 above the IC's exact 10 MiB limit for
configured roles and both built-in infrastructure canisters.

Focused evidence passes: 12 endpoint-expansion tests, five access-policy tests,
six artifact-build authority/command tests, six Wasm section/limit tests,
locked compilation and warning-denied all-target Clippy for the four changed
packages. The controlled release-Wasm comparison used identical source,
toolchain, profile, shrink and metadata paths. No broad workspace or PocketIC
gate was run during this coding slice.

Published `0.109.19` CANIC-081/082 has focused evidence passing for the
real Linux staged-executable publication path, all Binaryen host tests, both
CLI toolchain tests, retained desired round-trip and changed-input resumption,
the bounded pre-retention zero-debit final observation, typed protocol replay,
current plan JSON round-trip, and CLI recovery with the working TOML absent.
An ignored evidence test also copied and reopened the exact downstream
37,131,114-byte plan and 8,448-byte issued journal in isolated temporary state;
its compacted copy is less than one tenth of the original, retains the exact
plan/action identities and leaves the copied journal plus original evidence
unchanged. The focused 25-test Fleet Ensure module, including the governed
Toko-shaped PocketIC case, passes after retaining Canic-owned generic protocol
steps in the reviewed authority. Locked host/CLI checking,
warning-denied host/CLI Clippy, scoped formatting and diff hygiene pass. No
broad workspace or PocketIC gate was run, as required for an implementation
slice.

The adjacent version-rollback regression also passes using exact restored file
bytes and clean repository state rather than brittle console prose.

<!-- canic-release-state: source-development -->
Published `0.109.15` added a governed fast release lane for exact
non-runtime changes. It preserves immutable-tag ancestry, targeted release and
dependency checks, locked compilation, candidate sealing and atomic push while
skipping the workspace/PocketIC matrix. Its release receipt records
`gate=fast`, so it cannot be confused with complete validation. The same batch
updates the compatible yanked `chacha20` transitive lock entry from `0.10.1` to
`0.10.2` without changing Canic production source.

Targeted `0.109.15` evidence passes: 13 release-flow regressions, changelog
governance, current-document and release-matrix semantics, ShellCheck, the
release-integrity contract, zero-vulnerability dependency risk, locked offline
metadata and the locked workspace all-targets check. The last check compiled
the corrected `chacha20 0.10.2` graph in 43 seconds. No PocketIC or complete
workspace test gate was run, matching the maintained fast-lane boundary.

Published `0.109.14` qualification evidence follows.
Current operator-surface rebinding, focused governed runtime
qualification and active sediment/documentation reconciliation are complete.

Targeted evidence for the published source candidate:

- The published managed-App support has two pure authority-compilation tests
  passing with warning-denied `canic` Clippy. A composed-framework PocketIC
  consumer builds through the public facade and is assigned to the governed
  targeted lifecycle tier; downstream application-specific assertion cleanup
  remains downstream-owned after publication.
- The focused release remote-state fixture uses a local bare `origin` and
  proves accepted fast-forward state, rejection after concurrent branch
  advancement, rejection of an occupied planned tag, idempotent acceptance of
  an identical published tag and rejection of a conflicting tag object. The
  release-integrity contract binds the guard immediately before version
  mutation and before atomic push readiness.
- Focused `canic-host` Fleet-ensure tests pass, including lost responses at all
  seven mutation kinds,
  conservation, unsafe-retirement rejection, plan-tamper rejection, and a
  Toko-shaped PocketIC estate that converges then immediately replans/applies
  with zero mutation actions. Separate tests prove authority validation and
  treasury reuse, live Ledger-fee drift,
  rejects before intent/effect, a short paid result closes safely into a new
  reviewed plan without duplicating the retained creation, two-sided treasury
  receipt proof gates retirement, update issuance remains fenced until status
  proves terminal application, and consecutive stalls reach the configured
  bound before later genuine progress resets it.
- The same focused host suite now includes typed Root-placement compilation and
  exact one-command issuance/terminal-status replay for current Component
  provisioning. Warning-denied `canic-host` and `canic-testing-internal`
  package Clippy pass. The governed targeted production five-Component
  PocketIC case passed in 77 seconds with the shared compiler, complete
  Store/Registry/Component Registry convergence, terminal runtime activation,
  Fleet-catalog publication and an effect-free immediate replay. Peak reported
  RSS was 414,212 kB with 19 threads.
- The generator/current-release follow-up has targeted checks passing: seven
  focused public-generator tests within the 36 `canic-host` Fleet-ensure tests,
  the current-release manifest test, five
  `canic-cli` Fleet parsing/publication tests, and the focused control-plane
  Store-controller test. Targeted locked checks for `canic-host`,
  `canic-control-plane` and `canic-cli`, plus warning-denied Clippy for those
  changed packages, also pass. These prove create-once generation, no invented
  Fleet or treasury identity, exact retained-identity/controller sets,
  Root-owned idle/claimed/workload classification, exact Root-only-to-
  Root/operator Store preparation and foreign-controller rejection. The added
  retained multi-Component public-generator-to-workflow journey binds the live
  random Fleet ID, admits old 2T policy only behind the current Root reinstall,
  retains both paid assets while one is a 4.9T workload under the desired 5T
  policy, applies only three exact infrastructure reinstalls, emits no workload
  top-up, conserves the full observed balance and proves an immediate zero-
  effect replay. Generated Coordinator, Root and Store init
  bytes round-trip against every authority-bearing field; a missing seeded
  identity rejects instead of becoming a replacement creation. The generator
  now binds the queried live Ledger fee, authorizes no creation fee for its
  adoption-only estate, and keeps observation/update burn as measured
  conservative ceilings. The public generator journey uses a process-backed
  deterministic live-observation adapter; the separate governed PocketIC case
  proves the real current control-plane graph.
- CANIC-065's public retained multi-Component generator journey plans a Store
  protocol action before Component provisioning, retains one canonical plan
  digest and performs zero mutations before apply. This directly guards the
  terminal-observation bound against treating every typed Fleet protocol action
  as Component provisioning.
- The first complete-gate run passed every cheap invariant, workspace check and
  warning-denied Clippy tier, then stopped at the ordinary-test barrier before
  PocketIC. It exposed four propagation defects: the host-only public `testing`
  feature was being compared with canister-role features, Fleet subcommands
  were not declared in ASCII order, read-only CLI timer inspection was absent
  from the ownership inventory, and release-flow fixture repositories did not
  install the new remote-state guard. Focused regressions for all four now pass.
  A subsequent rerun exposed an exact-sentence check for the downstream minor
  block; that prose coupling is removed. The maintained 0.110 status and
  closeout audit still carry the actual no-mutation boundary.
  The adjacent guard audit also removes historical sentence/value assertions,
  runbook and validation-matrix heading/command inventories, README badge prose,
  root-changelog summary formatting and subjective pending-narrative scans.
  Source-development and validated-source authority now use structured status
  markers rather than exact English sentences. Structured release headers,
  package versions,
  schemas, hashes, executable command ownership, immutable audit fingerprints,
  support cells and required file/link presence remain enforced.
  The following complete-gate ordinary tier found one stale internal Fleet
  subcommand-order expectation after the public help ordering had already been
  corrected. The expectation now uses the same ASCII order and its focused
  unit and recursive help regressions pass; PocketIC was skipped on that failed
  run as designed.
  The final PocketIC tier then exposed one stale Store test that still expected
  the protected operator to lose mutation authority. The maintained endpoint
  and current Fleet Ensure contract retain the exact Root plus operator set;
  the journey now proves both callers and continues to reject anonymous access.
  Its exact targeted PocketIC rerun passed in 63 seconds.
  The next complete-gate ordinary tier stopped before PocketIC on one redundant
  test-only clone; that warning-denied Clippy finding is corrected.
  Release readiness is determined by the final unmodified `make validate`
  outcome on this exact source, reported in the maintainer handoff.
- The multi-Root generator now indexes policy Roots, retained identity Roots
  and compiled topology Roots by exact parsed `SubnetId` before joining them.
  Its focused two-Root regression deliberately uses Principals whose text and
  typed byte order differ, proving that Root-local authority cannot cross-bind
  through positional sorting.
- Six focused current-protocol compiler tests now also prove deterministic
  Registry-chain construction, exact Root/Store authority rejection,
  path-confined qualified Store bytes, content-bound chunk publication and
  deterministic replay identities. The sixth binds a post-publication Registry
  successor to the exact retained Component operation authority and rejects
  Registry or plan drift. Focused control-plane adoption/stable-state
  tests prove the exact retained Root/operator Store controller set. The
  canonical Wasm Store Candid was regenerated and its five focused surface
  checks pass.
- Current operator rebinding has 38 focused Medic tests, 48 focused cycles
  tests, the recursive CLI ordering/help check, three focused top-level
  dispatch/global-option checks, and warning-denied `canic-cli --all-targets`
  Clippy passing. The terminal ensure inventory regression also passes with its
  exact retained plan/journal authority assertion.
- Terminal current inventory now derives its complete authority from the exact
  active Coordinator Registry, Root provisioning receipt, Component Registry
  partitions, pool rows and bounded sharding-child pages. Two focused observer
  tests prove current module/profile binding, and the effect-free-successor
  regression proves the active Registry, protocol-created topology and its
  independently observed cycles remain retained across the successor plan.
- `info subnets` is restored as a current-only leaf. Five focused tests cover
  its terminal-authority binding and complete live aggregation, and the
  recursive CLI ordering/help test passes with the restored surface.
- The final targeted governed five-Component PocketIC rerun passed in 73
  seconds. It reached terminal runtime activation and Fleet-catalog publication,
  then proved an immediate replay issued no update. Reported shared-server
  high-water RSS was 421,668 kB with 19 threads.
- The earlier `0.109.13` maintainer `release-patch` attempt stopped in `check-invariants`
  before the broad compile/test tiers because the active operations index
  omitted the recovery-runbook link. The link and its missing current
  `pending_send` ICP-refill procedure are restored, the exact focused runbook
  guard passes, and the top-level changelog now has the one canonical
  `0.109.13` summary required by versioning.
- The IcyDB 0.245.1 lifecycle fixture passes targeted compilation plus its
  direct-ingress and same-release transition/recovery PocketIC proofs. The
  cold fixture proof passed in 49 seconds; its cached recovery proof passed in
  5 seconds.
- Earlier warning-denied package Clippy and maintained layering, timer,
  current-status, release-matrix, release-integrity and local v1 readiness
  checks passed.

The current empty-estate correction has focused evidence passing for durable
fresh-seed creation and exact replay, changed-fee rejection, no-effect desired
generation, logical controller and treasury resolution, create convergence and
an immediate effect-free successor plan. All 55 focused `canic-host` Fleet
Ensure tests and all eight focused `canic-cli` Fleet tests pass. No broad
workspace or broad PocketIC gate was run during this coding slice.

The distinct Root/Store correction passes focused exact-authority model and
stable-state tests, Root/Store init decoding, issued-command retry, stopped
same-Principal startup, formatting and warning-denied all-target Clippy for the
four changed packages. The layering guard also passes with the Store child
authority owned by the layer-neutral identity model rather than a transport
DTO. Its governed five-Component PocketIC journey rebuilt
the changed Root, Coordinator and Store Wasms and passed in 364 seconds. It
retained one issued operation through a typed Root-acceptance failure, restarted
the same Store, replayed the exact command, activated all five Components,
published the Fleet catalog and found no nonterminal update on immediate
replay. Reported shared-server high-water RSS was 423,508 kB with 19 threads.

This correction is a current reinstall-only contract. A successor CLI does not
inject corrected runtime behavior into an already-installed `0.109.19` Root,
and the issued predecessor plan is not a cross-patch repair authority. Affected
operators must discard the predecessor's local in-progress ensure evidence and
review a new current plan that reuses the controlled Principals and balances
while reinstalling the corrected infrastructure artifacts. No compatibility
bridge, journal edit or live repair path was added.

No version, commit, tag, package publication, push, deployment, identity
switch, Ledger call, live canister call, or sibling-repository mutation was
performed.

## Next Action

Finish the combined open `0.109.20` candidate with targeted host/CLI hygiene,
then let the maintainer-selected release flow choose its validation boundary.
After immutable publication, affected retained estates must generate and review
a current reinstall plan rather than reopen a predecessor issued operation.
Downstream launchers must use the ordinary Fleet Ensure generate/plan/apply
sequence and qualify it in their own repository. The distinct pooled-canister
Ledger-account recovery operation and finalized artifact build-network binding
remain later accepted work. Do not begin 0.110. Do not begin 0.111 from this
batch.
