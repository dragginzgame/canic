# Canic 0.109 Implementation Status

Date: 2026-08-23
Last updated: 2026-08-24

## Status

- State: B1-B7 are functionally implementation-complete. B8 now owns the
  0.109 release and downstream go-live support that must precede complexity
  remediation. The inherited pool-bootstrap and Registry-recovery defects plus
  the retry-diagnostic gap were intended to be corrected in published
  `v0.109.0`, but a post-publication executable-path audit found the successor
  check unreachable from two earlier restart gates and the host able to exhaust
  its poll bound before scheduled pool retry. Published `v0.109.2` fixes those
  paths and proves zero-to-five pool creation. Published `v0.109.1`
  corrects `CANIC-027`; the exact downstream graph, complete CI and managed/
  standalone PocketIC qualification pass. The downstream fresh install then
  exposed `CANIC-029`: paid infrastructure journals were intact, but recovery
  still required the original maximum operator debit before replay. Published
  `v0.109.2` derives remaining debit from the exact retained
  session and creation journals, reports its next replay phase, retries only
  transient verification unavailability within a fixed bound and admits the
  exact 0.109.1-to-0.109.2 host recovery path without changing installed
  0.109.1 Wasms. Exact downstream 0.109.2 CI passes, and an isolated checkout
  of the retained deployment source produces a blocker-free recovery plan with
  all three creations fenced, zero remaining debit and Store-bootstrap
  verification next. The authorized resume then exposed `CANIC-031` before
  any IC update: accepted retained-builder recovery still entered the ordinary
  successor-version workspace build validator before finalized-artifact reuse.
  Published `v0.109.3` separates finalized recovery into a read-only snapshot,
  validates exact topology, packages, manifests and bytes without Cargo or
  artifact mutation, and preserves fresh-build version enforcement. The exact
  downstream resume crossed that gate, then exposed `CANIC-033`: the first
  correlated Coordinator provisioning status had already advanced beyond
  `Planned`. The open correction retains every exact monotonic nonterminal
  status, maps complete terminal evidence directly to the terminal host phase
  and extends the explicit 0.109.1 recovery successor set through 0.109.4.
  The accepted post-implementation complexity audit has a
  `fail` closeout verdict; B9 must
  supersede it with a pass before B10, 0.109 minor closeout or 0.110 promotion.
  Direct managed-application ingress architecture is positively qualified, but
  B8 remains open on maintainer validation/publication of the `CANIC-033`
  correction, then the separately authorized downstream plan/resume and
  deployed-state/admission evidence.
- Outcome: one Coordinator-owned, bounded Fleet admission policy projected and
  enforced locally by every managed instance whose exact role declares
  `fleet_admission = true`.
- Runtime impact: B4 hard-cuts the retained 0.108 runtime whitelist into one
  exact target-bound managed projection. Fresh managed non-Root canisters
  retain it fenced, open it only after existing Root activation and enforce the
  observed caller locally. B6 advances effective mutations through one durable
  Coordinator/Root reserve, fence, activate and open protocol. A stale reserved
  aggregate releases before target effects with a typed `CatalogChanged`
  result; exact replay converges or returns that same terminal result without a
  second policy or participant owner.
- Predecessors: published 0.107 supplies bounded local whitelist primitives;
  human-accepted 0.108 closed in published `v0.108.2`.
- Implementation approval: the maintainer accepted B1's managed-only scope,
  authority and bounds on 2026-08-23 and explicitly continued 0.109. B2 through
  B7 are implementation-complete with passing focused checks. On 2026-08-24
  the maintainer required all Canic-owned Toko go-live support to precede the
  post-adoption complexity-remediation stage, accepted that audit's findings
  and required their in-repository remediation before 0.110 promotion.
- Successors: 0.110 estates, 0.111 stateful adoption and 0.112 observatory are
  renumbered but otherwise retain their accepted dependency order.
- Surface posture: the design hard-cuts `[app.whitelist]`,
  `caller::is_whitelisted()` and independent per-canister mutation into
  protected Fleet input, one Coordinator command/status authority, local
  projections and `caller::is_fleet_admitted()`. The role, caller-adapter and
  CLI names remain the exact B1 contract.
- Downstream posture: Toko Miner remains read-only from Canic. Its IcyDB App
  has adopted the published synchronous managed-projection adapter and passed
  exact built-Wasm qualification. Its live install and retained recovery
  session remain downstream-owned; Canic only inspected its local path
  authority read-only for the targeted recovery proof.

Design: [Fleet-wide ingress admission](0.109-design.md)

Binding audit:
[0.109 post-implementation complexity audit](../../audits/release-lines/0.109-post-implementation-complexity-audit.md)

B8 correction evidence:
[release and go-live support](../../audits/working/0.109-fleet-wide-ingress-admission/b8-release-go-live-support.md)

## Release-Batch Tracker

| Batch | Outcome | Direct evidence and fallout | Focused validation | Status |
| --- | --- | --- | --- | --- |
| B1 | Exact authority and baseline | 0.107 whitelist inventory, Toko direct-ingress trace, selector/participant contract, bounds and hard-cut acceptance | Source/config/Candid inventory, reproducible sizing fixture and explicit maintainer review | Accepted |
| B2 | Protected policy compilation | Fleet-input schema, selectors, canonical digest, effective projections, plan/Registry/install authority and config hard cut | Config/hash/plan/first-excess tests | Ready |
| B3 | Coordinator policy authority | Stable policy, add/remove generation, replay, paged status and diagnostics | Model/policy/storage/replay/capacity tests | Ready |
| B4 | Managed-role projections | Root distribution, local storage/predicate, endpoint manifests, fresh activation and whitelist removal | Role builds, access/macro, restart and multi-Root PocketIC tests | Ready |
| B5 | Composed-framework adapter | Synchronous managed-projection caller guard and generic IcyDB-style fixture | Native parity and direct-ingress PocketIC journey | Ready |
| B6 | Runtime convergence | Pre-effect catalog reserve/release, fence/activate/open journals, participant fences, exact retry and forward recovery | Stale-catalog, interruption, unavailable-target, add/remove and new-Component PocketIC matrix | Ready |
| B7 | Security closeout and propagation | Docs/generated surfaces, residue cleanup, measurements and read-only Toko adoption review | Targeted repository gates and adversarial multi-Root journey | Ready |
| B8 | Release and downstream go-live support | Correct the accepted fresh-deployment and interrupted-install recovery blockers, including `CANIC-033`; publish an immutable matching `canic`/`canic-cli` 0.109 pair, freeze the current Fleet-input/planning and App-only adoption contracts, consume separately authorized qualification evidence and correct every resulting Canic-owned blocker | Fresh-install/restart/retry-status evidence, finalized-artifact reuse without predecessor-workspace rejection, monotonic first-status reconciliation, targeted checks, maintainer release gate, package-pair equality, read-only downstream review and retained qualified-adoption/recovery evidence | Active: published 0.109.3 crossed retained-artifact reuse; the open correction for CANIC-033 is targeted-qualified and awaits maintainer validation/publication before downstream resume |
| B9 | Post-adoption complexity contraction | Canonical complexity/change-friction/structure/duplication evidence, localized admission decisions, finding-backed decomposition of the three gravity wells, active handoff below 250 lines, bounded PocketIC resource envelope, `CANIC-028` artifact advice, retained-decision source-drift diagnostics and 0.110 scope retriage | Immutable method reruns, targeted changed-package checks and accepted superseding verdict | Accepted; blocked on B8 |
| B10 | Published managed-App qualification support | One bounded downstream test-support surface for exact managed init/activation, admission fencing, fresh install and same-release recovery without private `canic-core`/`ic-testkit` reconstruction | Public-package consumer build plus managed/standalone lifecycle qualification and Toko adapter removal | Scheduled; blocked on B9 |

Ten batches fit the normal minor-line guideline. They are not preassigned
patch releases.

## Release And Downstream-Support Gate

B8 must close every Canic-owned prerequisite exposed by the current Toko Miner
go-live review before B9 simplification begins. Known defects must be corrected
before the package publication step:

1. the three accepted fresh-deployment blockers below are reproduced against
   the current 0.109 candidate, corrected and covered by direct recovery
   evidence;
2. the maintainer-owned release flow publishes matching `canic` and
   `canic-cli` 0.109 packages from one validated immutable commit;
3. Canic's maintained authoring and planning surfaces support the exact
   App-only topology: Coordinator on PZP, Root/Store/App on PAE4O,
   `funding_profile = "preview_multi_subnet"`, one top-level `operator`,
   `[admission]`, PZP Fiduciary-cost acknowledgement, Coordinator and Root
   funding policy, and current node-scaled funding values;
4. the maintained live planner, not downstream-authored fields, obtains the
   operator account, balance and observation evidence. At the reviewed 34/13
   node counts the infrastructure creation amount is 310T cycles plus three
   Cycles Ledger creation fees, before App and pool funding;
5. the public managed-App contract remains exactly
   `caller::is_fleet_admitted()` for the Canic login endpoint and
   `fleet_admission::require_caller()` as the first operation in each of the
   five protected caller-owned Robot endpoints, while intentional anonymous
   catalogue and database reads remain public; and
6. a separately authorized downstream adoption uses the exact published pair,
   removes `[app.whitelist]`, enrolls the App role with
   `fleet_admission = true`, regenerates its input and bindings, passes its own
   `make ci`, proves public/admitted/denied/anonymous/fenced/local calls plus a
   fresh 0.109 managed App install and same-release upgrade/restore rehearsal
   with retained IcyDB state and reconstructed timers. If installation has not
   started, it retains a reviewed interactive `make staging-plan` digest,
   placements, fees and maximum debit. If installation has started, it instead
   retains a read-only recovery plan naming the exact existing session,
   retained release build, next replay phase, original maximum debit and
   journal-derived remaining debit before an explicitly authorized resume.
   Any Canic-owned blocker found by that work is fixed in B8 before its evidence
   is accepted.

### Accepted Fresh-Deployment Blockers

The downstream run used Canic 0.108.1, passed `cargo check --workspace`, built
all Toko Wasms, produced a zero-blocker Fleet plan and passed shell syntax plus
`git diff --check`. The fresh deployment then exposed these inherited Canic
defects before 0.109 adoption. The downstream trace reports that the relevant
0.108.1 modules are byte-for-byte unchanged from 0.108.0; that provenance does
not permit carrying the defects into 0.109:

- `CANIC-109-GOLIVE-001`: Component provisioning requires five Ready pool
  Canisters, but fresh installation reaches Component-provisioning acceptance
  with zero and does not seed the configured pool. The Coordinator retries
  until the host's bounded retry limit. B8 must either seed the exact plan-bound
  Ready target before acceptance or make acceptance wait durably while the sole
  pool owner is guaranteed to produce capacity, with bounded interruption/
  restart behavior and no manual pool injection.
- `CANIC-109-GOLIVE-002`: after manual pool supply, the five services publish
  and the Fleet Registry legitimately advances from revision 3 to 4. Host
  restart still accepts only the exact planned Joining or all-Active snapshot.
  B8 must accept a validated monotonic successor while preserving exact Fleet,
  authority, root, Component and immutable plan bindings, and must continue to
  reject rollback, incompatible status, missing entries and unrelated
  successor mutation. One canonical successor validator must own that decision.
- `CANIC-109-GOLIVE-003`: the Coordinator logs a useful per-Root retry failure
  but exposes only a generic bounded-retry outcome. B8 must retain and expose a
  bounded typed last per-Root failure in protected status without making prose
  or logs authoritative.

The observed downstream operation is stuck at `ConfirmingDirectories`; its
Root is `Published` but not runtime-active. B8 evidence must reproduce the
current 0.109 behavior, converge a fresh deployment without manual pool work,
restart after service publication, reach runtime-active and show the exact
per-Root failure while a retry is pending. The fixes are forward-only in 0.109;
released 0.108 sources and tags remain immutable.

Published `v0.109.0` added the Root-owned pool pass, successor predicate and
typed failure, but its evidence covered only one pool asset and invoked the
successor predicate after earlier exact-snapshot gates. Published `v0.109.2`
drives the sole Root-owned Cycles-Ledger journal to the exact
accepted-batch demand, waits between unchanged host polls and applies one
strict successor authority at join, Root synchronization and activation. Its
IC-profile PocketIC journey starts with zero, creates and provisions five
assets, and exposes then clears the typed pending Root failure. A complementary
local-profile journey still reaches `RuntimesActivated`. Exact Joining/all-
Active/successor replay passes while missing, premature, wrong-plan and later
successors reject. The detailed commands and results are retained in the B8
correction evidence. Downstream adoption and local qualification on 0.109.1
remain historical positive evidence. Its interrupted fresh install is the
current recovery subject; B8 must not replace that evidence with another fresh
staging attempt.

### `CANIC-029`: interrupted fresh-install recovery

The downstream 0.109.1 install durably created, funded and installed the
Coordinator, Root and Wasm Store before stopping during exact Store-bootstrap
verification. The Root journal is retained at sequence 15 and the exact status
query now agrees with it. The App was not created, the frontend was not
changed, and only 14,788 operator cycles remain. Requiring the original
310,000,300,000-cycle maximum before inspecting those journals would make a
safe resume impossible and encourage a duplicate top-up.

The published 0.109.2 correction loads the exact immutable install session, retained
plan and Coordinator/Root creation journals without acquiring an install lock
or changing their bytes. Every creation journal beyond `Planned` permanently
fences its amount and exact Cycles Ledger creation fee from a second debit.
`CreationInFlight` is fenced but remains an uncertain observation-only outcome;
it is never treated as successful or recreated. Planning and installation
recompile the original authority and digest, preserve the original maximum
debit, and compare the live operator balance only with the checked remaining
debit. The report binds the operation/session identity, retained 0.109.1
release build, retained builder, original plan digest, fenced/total creation
counts, uncertain outcomes, next replay phase and remaining debit.

The only cross-patch allowance is a host-side 0.109.2, 0.109.3 or 0.109.4 recovery of
one validated retained 0.109.1 install session using its exact installed
0.109.1 artifacts.
It is not a canister upgrade, migration, adoption, rollback or mixed-version
runtime contract. The Store-bootstrap verification retries only typed
`STATE_UNAVAILABLE`, at most five exact queries separated by one second; the
effectful update remains outside that query loop and later recovery retains its
same idempotent operation identity. A read-only probe against the
reported downstream state reproduced three fenced operator creations, zero
remaining operator debit and the Store-bootstrap verification as the next
phase without touching the downstream repository or network.

The 0.109.3 package passed the maintainer release flow. The trusted operator's
exact-source recovery plan also passes with zero remaining debit. Its
authorized resume exposed `CANIC-031` before any network update because the
host accepted the 0.109.1 recovery pair and then applied 0.109.2 ordinary
workspace build validation. The open correction derives recovery inputs from
the finalized release authority, exact current topology/package declarations
and retained manifests. It revalidates raw/gzip bytes without writing Cargo or
artifact state; fresh builds still require the current Canic version. The
0.109.1 retained-session regression and package/topology/manifest/byte drift
rejections pass, as does warning-denied host Clippy. The exact downstream
resume then crossed finalized-artifact reuse and exposed `CANIC-033` after
Root preparation: the Coordinator's zero-delay private advance raced the
host's exact-`Planned` first-status requirement. The open correction accepts
any exact nonterminal phase and derives the next advance request from it;
complete `RuntimesActivated` evidence advances directly to catalog
publication. The explicit successor set now includes 0.109.4 and still rejects
later or different predecessor pairs. All 126 focused install-root tests and
warning-denied host Clippy pass. B8 closes only after the correction passes the
maintainer release flow, the retained session finishes and deployed-state/
admission checks pass;
frontend publication and fixture changes remain downstream-owned and later.

The downstream repository, its CI and every deployment effect remain outside
this repository's mutation authority. In particular, this gate does not
authorize `make staging`; the maintainer must authorize that live effect
separately after the reviewed recovery plan. Do not start a fresh staging
operation while the retained session is recoverable.

## Post-Adoption Complexity Gate

The binding audit rejects the completed functional candidate as the baseline
for beginning another distributed control-plane line. Its six accepted
findings cover 0.109 change radius, mature gravity-well modules, the oversized
current handoff, validation-capacity growth, the downstream-loop prerequisite
now assigned to B8 and the unearned breadth of the scheduled 0.110 plan.

B9 remediation must not begin until B8 closes. It must then fix every remaining
complexity finding and retain a passing superseding verdict from one immutable
candidate. Publication and adoption establish the real product loop that B9
audits; neither event alone closes the complexity gate or authorizes 0.110.

B9 remains a pure simplification batch. It localizes admission decisions and
decomposes `ops::component_registry`, `workflow::component_registry` and
`ops::fleet_coordinator` along existing owner seams, contracts this handoff
below 250 lines and freezes a bounded PocketIC time/RSS/process envelope.
Correcting `CANIC-028` preserves exact selected-root observation but tells a
named-environment fresh plan that initial install owns build/finalization; it
must not add fallback lookup, manual copying or a new build capability. The
same B9 diagnostic slice must classify a retained-decision mismatch as source
drift, name the exact retained source identity/revision and avoid unrelated
identity or funding advice when those checks already pass. Current
downstream need is input to B9's later 0.110 retriage: retain stateful
application retirement first and defer reserve Fleets, cross-Fleet transfer,
broad funding automation and 1,000-canister qualification unless fresh demand
earns them. The retriage decision itself still occurs only after B9 evidence.

`CANIC-026` is deliberately not part of B9. B10 owns a small published
managed-App qualification harness after simplification passes, and may not add
runtime authority or expose private control-plane ownership.

## Blocking Application Evidence

Toko Miner has hard-removed Core. Its singleton managed IcyDB App owns browser
login and five caller-owned User/Robot methods. Before adoption, those direct
`#[icydb::request_execution]` exports rejected only anonymous callers before
application-owned `Principal -> UserPrincipal -> UserId -> Robot` resolution;
Canister topology and controllership could not intercept the call.

The adopted 0.109 outcome gives that App one exact managed projection. Its
browser-login Canic endpoint uses `caller::is_fleet_admitted()`, and each
protected IcyDB endpoint invokes `require_caller()` before its application work
while retaining Toko's separate membership, administrator and resource rules.
The exact built-Wasm qualification proves public/admitted/denied/anonymous/
fenced behavior and same-release recovery.

## Next Authorized Action

B1 is accepted and B2-B7 compile generation-one authority through the plan,
Registry, sole Coordinator mutation record, exact managed local projection,
one synchronous composed-framework caller guard and replay-safe runtime
convergence. `CANIC-109-GOLIVE-003` is published; `v0.109.2` closes the
executable-path gaps in `001` and `002` with targeted recovery evidence,
and `v0.109.1` publishes `CANIC-027`. The 0.109.2 correction closes the
funding/journal side of `CANIC-029`, and the exact-source read-only recovery
plan passes. Continue B8 by validating and publishing the `CANIC-033`
correction, then use the exact downstream `v0.2.0` recovery checkout to
reproduce the unchanged digest, zero debit and
`fleet_component_provisioning` next before separately authorizing one resume.
Accept B8 only after deployed-state/admission evidence confirms convergence;
then
continue with B9's evidence-first simplification and immutable superseding
audit, then B10's separate managed-App qualification support. Do not begin
0.110 until
those 0.109 gates pass, the maintainer accepts closeout and then explicitly
promotes the revised 0.110 B1. Do not treat this in-repository gate as external
mutation or deployment authority.
