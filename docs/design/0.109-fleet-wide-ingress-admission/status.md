# Canic 0.109 Implementation Status

Date: 2026-08-23
Last updated: 2026-08-24

## Status

- State: B1-B7 are functionally implementation-complete. B8 now owns the
  0.109 release and downstream go-live support that must precede complexity
  remediation. The inherited pool-bootstrap and Registry-recovery defects plus
  the retry-diagnostic gap are corrected in the current working candidate with
  focused native and PocketIC evidence. The complete maintainer validation
  gate has not run, so publication and downstream adoption remain pending. The
  accepted post-implementation
  complexity audit has a `fail` closeout verdict; B9 must supersede it with a
  pass before 0.109 minor closeout or 0.110 promotion. Direct managed-
  application ingress still blocks Toko Miner staging until publication and
  downstream adoption.
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
- Downstream posture: Toko Miner remains read-only from Canic. Its IcyDB App is
  now a normal managed Component and may adopt the synchronous
  managed-projection adapter in downstream-owned work after publication.

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
| B8 | Release and downstream go-live support | Correct the accepted fresh-deployment blockers, publish an immutable matching `canic`/`canic-cli` 0.109 pair, freeze the current Fleet-input/planning and App-only adoption contracts, consume separately authorized qualification evidence and correct every resulting Canic-owned blocker | Fresh-install/restart/retry-status evidence, targeted checks, maintainer release gate, package-pair equality, read-only downstream review and retained qualified-adoption/plan evidence | Corrections ready; blocked on maintainer release and downstream evidence |
| B9 | Post-adoption complexity contraction | Canonical complexity/change-friction/structure/duplication evidence, finding-backed gravity-well decomposition, active-document contraction, bounded validation and 0.110 scope retriage | Immutable method reruns, targeted changed-package checks and accepted superseding verdict | Accepted; blocked on B8 |

Nine batches fit the normal minor-line guideline. They are not preassigned
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
   with retained IcyDB state and reconstructed timers, and retains a reviewed
   interactive `make staging-plan` digest, placements, fees and maximum debit.
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

The current working candidate corrects all three defects. One IC-profile
PocketIC journey starts with zero imported pool assets, advances the sole
Root-owned Cycles-Ledger creation journal once and provisions the Component
with one paid request while exposing the typed pending Root failure. A
complementary local-profile journey publishes Registry revision 4 and reaches
`RuntimesActivated`. Host recovery accepts that successor only with exact
compiled-plan and protected Coordinator operation evidence; predecessor,
missing-evidence, premature, wrong-plan and later-revision cases retain their
fail-closed outcomes. The detailed commands and results are retained in the B8
correction evidence. The complete maintainer gate, publication and downstream
adoption have not occurred.

The downstream repository, its CI and every deployment effect remain outside
this repository's mutation authority. In particular, this gate does not
authorize `make staging`; the maintainer must authorize that live effect
separately after reviewing the no-effect plan.

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

## Blocking Application Evidence

Current Toko Miner has hard-removed Core. Its singleton managed IcyDB App owns
browser login and five caller-owned User/Robot methods. Those direct
`#[icydb::request_execution]` exports currently reject only anonymous callers
before application-owned `Principal -> UserPrincipal -> UserId -> Robot`
resolution, so any non-anonymous Principal may attempt enrollment directly.
Canister topology and controllership do not intercept the call.

The accepted 0.109 outcome gives that App one exact managed projection. Its
browser-login Canic endpoint may use `caller::is_fleet_admitted()`, and each
protected IcyDB endpoint may make `require_caller()` its first body operation
while retaining Toko's separate membership, administrator and resource rules.

## Next Authorized Action

B1 is accepted and B2-B7 compile generation-one authority through the plan,
Registry, sole Coordinator mutation record, exact managed local projection,
one synchronous composed-framework caller guard and replay-safe runtime
convergence. `CANIC-109-GOLIVE-001` through `003` are corrected with targeted
recovery evidence. Continue B8 with the maintainer-owned complete validation,
version and publication flow. After the exact published pair is separately
adopted and qualified, close B8 and continue with B9's evidence-first
simplification and immutable superseding audit. Do not begin 0.110 until that
audit passes, the maintainer accepts it and then explicitly promotes the
revised 0.110 B1. Do not adopt the dirty candidate in Toko before publication
or treat this in-repository gate as external mutation or deployment authority.
