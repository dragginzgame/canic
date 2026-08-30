# Canic 0.109 Implementation Status

Date: 2026-08-23
Last updated: 2026-08-26
Roadmap reconciled: 2026-08-30

> Historical implementation record: retained-Root repair, install preflight,
> and recovery-bundle claims below describe immutable bridge releases through
> `v0.109.12`. The maintained source after that tag removes those exceptional
> surfaces. Current development truth is in
> [`docs/status/current.md`](../../status/current.md); CANIC-059 owns the
> separately gated generic Fleet convergence direction.

## Status

- Current addendum (2026-08-30): published `v0.109.25` closes `CANIC-091` with
  the management-only stopped-Root Start prerequisite. The open 0.109.26
  batch closes `CANIC-092` by binding live predecessor A to newly requested
  finalized successor C independently of an older retained desired artifact B,
  without rewriting that desired document. `CANIC-034` is already
  closed by direct reviewed Cycles Ledger creation funding for fresh pool
  assets. Current release and validation truth remains in
  [`docs/status/current.md`](../../status/current.md); the bridge chronology
  below is historical evidence, not the maintained operator contract.
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
  `Planned`. Published `v0.109.4` retains every exact monotonic nonterminal
  status, maps complete terminal evidence directly to the terminal host phase
  and extends the explicit 0.109.1 recovery successor set through 0.109.4.
  The retained operation crossed that correction, then exposed `CANIC-034`
  Root-ledger funding and `CANIC-035` undersized pool policy. Authorized
  funding/import work retained Ready assets at 2T and 4.5T for one unclaimed
  5T App. Published `v0.109.5` rejects that policy before fresh-plan
  effects, loads only the exact admitted 0.109.1 retained decision under its
  historical rule, matches exact eligible assets at acceptance, claims the
  smallest sufficient Ready asset, retains typed capacity evidence and lets
  the sole Root pool owner re-inspect the topped-up imported asset without
  replacing the operation or journal. Recovery remediation explicitly rejects
  irrelevant additional funding advice. The separately authorized live Root
  repair preserved the retained canister and journals and refreshed the import
  above 5T. It exposed `CANIC-039`: compact teracycle formatting rendered the
  actual and required amounts identically, and `CANIC-040`: the retained host
  accepted only the predecessor Root module after the authorized repair.
  Published 0.109.6 adds the raw actual, required and deficit cycle integers
  to that actionable diagnostic. It also
  replaces product-version recovery pairs with bounded current/historical-pool
  `v1` contracts, admits one exact already-applied state-preserving Root repair
  through a separate immutable artifact/Candid/authority receipt, and closes
  fresh-install recovery at terminal catalog publication. `CANIC-042` removes
  an exact-step runtime-activation observation assumption: the Coordinator now
  accepts any strictly monotonic, bounded Root progress under the existing
  operation/receipt/timestamp authority, including a terminal first
  observation after every intermediate Component state was missed.
  The accepted post-implementation complexity audit has a
  `fail` closeout verdict; B9 must
  supersede it with a pass before B10, 0.109 minor closeout or 0.110 promotion.
  Direct managed-application ingress architecture is positively qualified.
  Published 0.109.6, the authorized Root upgrade and asset refresh close the
  `CANIC-035` work. The final 0.109.6 source passed the complete unmodified
  maintainer gate, including `CANIC-042` control-plane, warning-denied and real
  five-Component PocketIC proof, before immutable publication. Published
  0.109.7 completes the exceptional retained Root/pool procedure, non-mutating
  monotonic Component Registry proof, receipt/operation interruption recovery,
  recovery-aware Medic and Cargo diagnostics, actionable named-environment
  artifact guidance and release governance. It also advances the generic
  composed-framework fixture to exact published IcyDB 0.240.1. Focused checks
  and the production-boundary retained-repair PocketIC journey pass. The
  complete unmodified maintainer gate also passes on the final source
  candidate, and the complete matching package set is published. The next
  fresh five-Component install exposed `CANIC-047`: a Root can publish every
  Component Directory before the Coordinator records its first passive
  observation, but Directory confirmation still accepts only unchanged or
  exact `+1` publication counts. Published 0.109.8 makes publication
  observation monotonic and coalescing-safe under the existing exact authority
  and receipt checks. The next retained-session resume exposed CANIC-050/051:
  the repair receipt simultaneously authorized the successor Root and required
  the terminal sequence-28 Component Registry proof, so a valid sequence-15
  checkpoint could not admit the successor early enough for normal workflows
  to re-observe live state. Published 0.109.9 separates typed provisional
  module authority from terminal proof, retains exact Wasm/Candid bytes by
  content digest and checkpoints complete install evidence into a
  path-confined operator-state bundle. Its final correction checkpoints every
  repair write-before-effect transition, derives required bundle contents from
  the exact Root journal phase and drives sequence 15 through the production
  Store, Registry and Component Registry phase owners. The final source passed
  the complete maintainer gate before immutable tag and package publication.
  The 0.109.10 source batch closes CANIC-053 by deriving every normal finalized
  Wasm and Candid sidecar from the infrastructure/application manifests. It
  also closes blocking CANIC-054: historical retained Root repair consumes the
  exact manifest-bound `.did` without requiring a debug export, retains both
  transition sidecars and checkpoints a non-operational candidate before
  provisional authority. CANIC-055 adds an effect-equivalent retained-install
  preflight through that verified candidate/bundle checkpoint before any
  operational authority or IC update. Terminal release-narrative checks are
  hardened after CANIC-014 recurred. The targeted retained-Root PocketIC
  journey passes with CANIC-055. B8 remains open on the frozen-candidate
  downstream/full-gate sequence, immutable publication, exact-session resume
  and deployed-state/admission evidence.
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
- Successors: 0.110 is reoriented to Fleet runtime contraction plus stateful
  retirement, 0.111 retains the exact stateful-adoption exception and 0.112
  now owns bounded multi-Fleet estates. All remain blocked on accepted 0.109
  closeout and their own explicit promotion.
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
| B8 | Release and downstream go-live support | Maintain the current hard-cut Fleet Ensure operator loop, consume separately authorized downstream evidence and correct every Canic-owned blocker without restoring historical install/recovery owners | Fresh/reused estate, interruption, replay, cycle-conservation, exact artifact/authority, immutable package-pair, read-only downstream adoption and terminal/effect-free rerun evidence | Active: published 0.109.26 closes CANIC-091 through CANIC-093; open 0.109.27 gives CANIC-094 an exact sealed-C/requested-D diagnostic and CANIC-095 an effective-ID-routed typed install-version transport. Resumed ordinary C apply, terminal convergence, D regeneration and effect-free replay remain |
| B9 | Post-adoption complexity contraction | Canonical complexity/change-friction/structure/duplication evidence, localized admission decisions, finding-backed decomposition of the three gravity wells, dependency-light pure recovery-plan validation separated from IC/PocketIC drivers, active handoff below 250 lines, bounded PocketIC resource envelope, retained-decision source-drift diagnostics and 0.110 scope retriage | Immutable method reruns, targeted changed-package checks and accepted superseding verdict | Accepted; blocked on B8 |
| B10 | Published managed-App qualification support | One bounded downstream test-support surface for exact managed init/activation, admission fencing, fresh install and same-release recovery without private `canic-core`/`ic-testkit` reconstruction | Public-package consumer build plus managed/standalone lifecycle qualification and downstream adapter removal | Published surface and downstream qualification available; final closeout reconciliation remains |

Ten batches fit the normal minor-line guideline. They are not preassigned
patch releases.

## Release And Downstream-Support Gate

B8 must close every Canic-owned prerequisite exposed by the current Toko Miner
go-live review before B9 simplification begins. Known defects must be corrected
before the package publication step:

1. the three accepted fresh-deployment blockers below are reproduced against
   the current 0.109 candidate, corrected and covered by direct recovery
   evidence;
2. one clean source commit is frozen, its exact binary passes the retained
   downstream `canic install ... --preflight` without operational authority or
   IC updates, and the maintainer-owned release flow then runs the complete
   gate once and publishes matching `canic` and `canic-cli` 0.109 packages
   from that unchanged source;
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

### `CANIC-047`: coalesced Component Directory publication

A fresh single-Root 0.109.6 installation provisioned and published all five
top-level Components, but the Coordinator retained its first Directory
confirmation predecessor at zero. Its passive Root status query then observed
the complete terminal `Published` response at five. Directory response and
stale-request replay validation accepted only an unchanged count or exact
`+1`, so the valid zero-to-five observation returned E132 indefinitely while
the Root remained publication-terminal and runtime activation stayed fenced.

Published 0.109.8 accepts every bounded non-regressing publication snapshot while
preserving exact operation, plan, configuration, Registry, Root, Component
count, ordered Component evidence, Component Group Directory hash, terminal
receipt and timing validation. The same rule owns fresh and selected-Root
scale-out publication. Replay recognizes any valid first observation and any
strictly advancing successor, so a response lost after Coordinator commitment
does not turn coalescing into a stale-request conflict. Focused pure and
Coordinator workflow proof covers zero-to-five, zero-to-three-to-five,
regression, overflow, authority/receipt corruption, retained E132 projection
clearing, exact replay, runtime continuation and scale-out.

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

The retained-install host allowance admits only a 0.109.2, 0.109.3, 0.109.4 or
0.109.5 host recovery of one validated 0.109.1 install session using its exact
installed 0.109.1 artifacts. Separately, the maintainer authorized one exact
Root-only upgrade to the 0.109.5 repair after publication so the already-paid
operation can preserve its stable pool/provisioning journals and cycle-bearing
canisters. Neither exception is a Fleet adoption, generic migration, rollback
or mixed-version runtime contract. The Store-bootstrap verification retries only typed
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
publication. The explicit successor set now includes 0.109.5 and still rejects
later or different predecessor pairs. Published 0.109.4 crossed the first-
status race, after which the retained operation exposed the separately funded
but undersized full pool recorded as `CANIC-035`. The open hard-cut correction
uses the existing Root pool and provisioning owners: fresh plans reject the
policy mismatch before effects, the exact supported predecessor plan retains
its historical pool-cycle validation, exact batch acceptance matches eligible
asset amounts, claims preserve heterogeneous capacity, protected status
retains typed `CAPACITY_INSUFFICIENT`, and a repeated protected import/reset
may re-inspect the topped-up 4.5T asset against the remaining 5T claim. No
stable schema or second recovery owner is added. The governed targeted
PocketIC journey proves same-release Root upgrade, retained identities, exact
live-balance refresh, 5T claim/install and no-debit handling on the same
principal. B8 closes only
after the maintainer release flow passes, the retained Root is upgraded rather
than reinstalled, its refreshed
asset is claimed, and deployed-state/admission checks pass;
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
The preceding B8 `CANIC-028` correction preserves exact selected-root
observation but tells a named-environment fresh plan that initial install owns
build/finalization; it adds no fallback lookup, manual copying or new build
capability. B9 must still classify a retained-decision mismatch as source
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
plan passes. Published `v0.109.4` corrects `CANIC-033`, and published
`v0.109.6` closes the subsequent pool, repair-adoption, terminal-completion and
coalesced-activation defects. Published `v0.109.7` completes the exact repair
procedure, non-mutating Registry proof, diagnostic, governance and
IcyDB-fixture corrections. Published 0.109.8 corrects `CANIC-047` coalesced
Directory publication under the retained authority and receipt checks, and
published 0.109.9 closes CANIC-050/051/052. The current 0.109.10 source closes
CANIC-053/054/055 and passes its focused and targeted PocketIC checks. Freeze
one immutable source candidate; the downstream owner must run that exact
candidate's no-effect installer preflight before the human-owned full gate and
publication. Only after publication may the downstream owner separately
authorize the exact retained-session resume; Canic does not authorize or
perform that effect here.
Accept B8 only after deployed-state/admission evidence confirms convergence;
then
continue with B9's evidence-first simplification and immutable superseding
audit, then B10's separate managed-App qualification support. Do not begin
0.110 until
those 0.109 gates pass, the maintainer accepts closeout and then explicitly
promotes the revised 0.110 B1. Do not treat this in-repository gate as external
mutation or deployment authority.
