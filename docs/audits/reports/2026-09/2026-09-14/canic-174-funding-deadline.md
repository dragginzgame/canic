# CANIC-174 child-grant funding deadline

Working-tree correction on published Canic 0.110.16, in the open 0.110.17
batch. Toko's latest read-only source review still ends at CANIC-175 and now
confirms the grant/deadline defect. This report qualifies the runtime deadline
owner, generation estimate and observed-Root planning boundary. Complete
bootstrap funding admission remains unfinished.

## Reproduction

The existing managed-component IC fixture prepares its audit Root at 11T,
records a stable sample through the maintained state command, and observes a
funding deadline more than 3,000 simulated seconds away. Its configured Root
threshold is 10T. A real registered User Hub requests cycles through the Root
capability command; the current policy caps the grant at 2T.

On unchanged runtime code, the grant succeeds and the child's balance grows,
but the Root timer retains exactly the same deadline, generation and zero
execution count. The focused case fails at that observable boundary. This
confirms the previously Candidate owner-path defect; internal transfer, rather
than computation burn, crosses the threshold.

The audit-only balance setter prepares the disposable fixture. Its 11T balance,
2T request cap and 3T child allowance are test inputs, not recommended settings.
No production endpoint, configuration or funding limit changes.

## Correction

The existing child-grant workflow invokes the cycle owner after the management
call returns, for both successful and rejected calls. The owner reads current
configuration, any retained Root request and the actual balance. It records a
fresh sample through the existing cycle tracker and retention policy, then
reconciles the same native top-up registration.

The transfer boundary does not feed its discontinuous debit into the estimated
computation burn rate. Scheduling uses the existing conservative policy floor
until the next ordinary observation can compare against the fresh sample.
Transfer reconciliation uses the provider's earlier-deadline scheduling:
it can advance a wake-up but cannot postpone an existing safety check.
Lifecycle/state reconciliation retains its exact-schedule behavior. Disabled
new funding still disarms when no current request needs recovery.

No second timer, new stable store or replacement grant protocol is introduced.
A timer/configuration failure is logged without replacing the paid transfer's
result. Original grant receipts, replay, per-request/per-child limits, Root
reserves, automatic funding budgets and recovery ownership remain intact.
The existing deployment 1T reserve guard is unchanged.

## Qualification

The registered case is
`pic::fleet_registry::baseline::tests::funding_deadline::child_grant_refreshes_root_funding_deadline_without_repeating_credit`.
It uses PocketIC 16.0.0 and the existing audit Root, real Coordinator and
managed component fixture. It covers:

- the real 2T grant crossing the Root threshold and advancing its timer;
- an actual upstream Root grant within 30 one-second simulated-time steps;
- replay of the original child receipt without another deposit;
- a later 1T grant exhausting the same 3T child allowance, preserving the
  earlier safety deadline and avoiding a spurious high burn-rate estimate;
- typed rejection of a further grant, with no second upstream grant.

The unchanged-runtime failure is retained in
`/tmp/canic-174-deadline-before.log`. The first corrected runtime passed the
threshold, upstream-demand, replay and budget sequence in
`/tmp/canic-174-deadline-after.log`. Review then added the earlier-deadline
requirement before treating the correction as complete. Final scoped Clippy
passes in `/tmp/canic-174-deadline-clippy.log`. The finished IC case passes in
`/tmp/canic-174-deadline-final.log`, including the earlier-deadline assertion.
Its 470.67-second case duration includes fixture Wasm builds; the targeted
runner segment took 653 seconds including native compilation. These are
qualification durations, not wall-clock performance benchmarks.

Scoped formatting, layering, diff and the pure governed inventory-order case
also pass; the latter log is `/tmp/canic-174-deadline-inventory.log`. The final
read-only Toko refresh remains unchanged through CANIC-175. Protected
Coordinator reserves and funding budgets may still produce a legitimate
no-grant response; timely demand does not create missing cycles.

Failure-path scheduling and provider/configuration failure logging are source
reviewed, not separately fault-injected in this IC case. The test does not
qualify Toko's installed workload, operator mint receipts, bootstrap funding
forecasts or the combined E163 retained-claim/withdrawal journey. Those remain
in the accepted funding batch. No broad suite, version transaction, Git
publication, deployment, live spending or sibling mutation ran.

## Startup generation estimate

Generation now returns a read-only funding view and the CLI renders it beside
the generated desired-state path. It uses the already loaded application
configuration, compiled initial topology, Root placements and native balance
observations. It adds no IC query, paid action, journal record or desired-state
field. Native balances distinguish observed values from configured creation;
missing observations reject instead of silently becoming zero.

The scenario uses the pool readiness floor for each initial Workload, fresh
per-child grant ledgers and zero burn. It works from leaves to parents, rounds
all required increments, includes equality at the threshold, respects
per-request/lifetime clamping and exposes disabled or unfunded roles. A parent's
required incoming grants account for its outgoing grants. Only direct child
transfers enter Root demand; descendant transfers are not added to it again.
Configured Component/Root window budgets and cooldowns remain visible. The
Root requirement is its request threshold plus one cycle plus direct grants;
Coordinator balance above its protected reserve is shown separately.

Review also found the shared initial-role counter recorded only the first
incoming parent for a role. The counter now accumulates every initial path,
excludes zero-initial dynamic edges and rejects nonterminating initial graphs
within a bound derived from the declared graph. Both pool sizing and the new
forecast use this counter. In the controlled graph, three direct Shards plus
two Branches each starting two Shards produce seven Shards, not three. Its
Root grants total 100T, while intermediate transfers total 130T; those are
distinct accounting views, not 230T of new funding.

Qualification passes:

- Four pure startup cases cover repeated/equal-threshold grants, lifetime caps,
  disabled-parent shortfalls, arithmetic overflow, hierarchical demand,
  multi-parent counts and cyclic initial demand. The log is
  `/tmp/canic-174-startup-tests.log` (its filter also ran one existing cache case).
- The generated retained apply/replay journey verifies the new projection,
  low native balances, exhausted Coordinator headroom, missing observations
  and configured fresh balances. Three existing initial-workload/pool-supply
  regressions also pass in `/tmp/canic-174-startup-generation-tests.log`.
- The CLI case preserves evidence labels and a nonzero raw shortfall smaller
  than the rounded display unit, in `/tmp/canic-174-startup-cli-test.log`.
- Host/CLI all-target, all-feature Clippy passes in
  `/tmp/canic-174-startup-clippy.log`; scoped formatting, layering and diff
  checks pass.

This is a synthetic configuration scenario, not a measurement of Toko's live
nine-Workload estate. It excludes install/call burn, grant history, outstanding
reservations, upstream target overshoot and timing-dependent admission. A
window comparison is not proof of immediate funding eligibility. No IC behavior
changed in this follow-up, so its checks are native host/CLI checks; the earlier
IC deadline proof remains the separate runtime evidence. Binding demand into
approved funding before activation, exact operator mint receipts and the
combined retained-claim/withdrawal journey remain open. The Toko ledger is
unchanged through CANIC-175; the .17 batch is not push-ready.

## Reviewed startup funding for observed Roots

Artifact resolution now recomputes the configured startup requirement and
requires the compiled application deployment configuration to equal the
reviewed desired configuration. The requirement is transient planning input;
no new persisted schema or runtime funding owner is introduced.

For a pending `ProvisionComponents` action, only an observed Root selected by
a nonempty provisioning batch receives startup funding. Its native requirement
is the greater of the ordinary configured minimum and the zero-burn startup
requirement plus reviewed Root/provisioning execution bounds. The existing
target-local observation/update margin is retained. Internal grants are not
also counted as execution burn. An unsatisfiable configured per-role demand
fails with a typed policy error before effects.

The planner inserts or extends the existing `Fund` action ahead of protocol
work. Amount, recipient, Ledger, fixed timestamp, plan digest and conservation
remain under the existing review and receipt owner. Extending an ordinary Fund
adds only its additional cycles, not another Ledger fee. When conservation
tranching defers provisioning, the planner restores ordinary funding before
rechecking the surviving prefix. Terminal plans and unrelated Root batches
receive no startup increment.

The existing generated retained-estate native regression now also qualifies
configuration mismatch, a new Fund, extension of an existing Fund, fee/debit
accounting, deterministic plan identity, funding-before-provisioning ordering,
no pending provisioning, unrelated batches, missing demand, unsatisfiable role
demand and deferred-provisioning rollback. It deliberately sets a synthetic
50T planner requirement to exercise both funding paths; the real fixture's
configuration-derived requirement is separately asserted as 10T plus one cycle.
It does not claim that this fixture or Toko needs a 50T balance.

The completed generated-estate regression passes in
`/tmp/canic-174-plan-binding-tests.log`. Scoped host/CLI
all-target, all-feature Clippy passes in
`/tmp/canic-174-plan-binding-clippy.log`. The shared startup policy and existing
native funding-review regressions pass (four and seven cases respectively) in
`/tmp/canic-174-plan-binding-regressions.log`. Scoped formatting, layering and
diff checks also pass. These are host planning/receipt
checks, not new IC withdrawal or lifecycle proof.

Remaining boundaries are explicit:

- Fresh creation/reinstall continuation currently admits protocol effects only.
  Its startup funding must be prepaid in the initial reviewed plan; this slice
  does not add permission for a successor to spend operator cycles. Additional
  successor funding continues to require review.
- Demand uses the configured full initial placements, fresh per-child ledgers
  and readiness-floor balances, not live remaining grants or outstanding
  reservations. Window/cooldown admission and deployment reserve headroom need
  complete qualification before claiming activation sufficiency.
- The combined E163 retained-claim/host-withdrawal journey and exact operator
  mint credits remain separate accepted work. The prior IC deadline proof and
  issued-operation withdrawal proof retain their narrower evidence scopes.

The read-only Toko follow-up agrees with these remaining boundaries, adds no
issue ID and confirms CANIC-174. Its ledger SHA-256 is
`614cfd58b9b612ca162279b2496a13c42cc98226d6760399558d79142063e9c6`.
The full .17 batch is not push-ready. No broad suite, package version change,
commit, push, deployment, live spending or sibling edit is part of this work.

## Initial continuation prepayment

The next correction prepays configured startup demand when the initial plan
admits fresh creation or infrastructure reinstall continuation. The existing
artifact resolver supplies each Root's finite continuation catalogue: its
artifact/publication bound, imports, two shared orchestration steps and bounded
fixture retries. The calculation includes Store publication conservatively,
but does not charge a Root for another Root's catalogue. Multiplying by the
configured update bound plus three observation bounds gives the execution
allowance; the initial target's own action/observation margin is separate.
This is a conservative review bound, not measured application burn.

Participating fresh Roots increase their existing Create amount. Reinstalled
Roots use the existing Fund before installation. Larger configured creation
amounts remain unchanged, Ledger fees are charged once, and plan identity and
operator debit include the additional cycles. Empty placement scopes get no
startup increment. Missing or overflowing continuation bounds reject before
effects. The successor contract remains protocol-only and cannot add spending.

The generated-estate regression passes fresh creation, already-funded creation,
reinstall prepayment, deterministic review identity, exact fees/debits and
missing-bound rejection. Existing already-funded reinstall/replay paths use a
500T synthetic Root fixture balance; their assertions compare actual starting
and final balances instead of retaining the former illustrative total. This
balance is test setup, not a product default. The log is
`/tmp/canic-174-continuation-tests.log`. All ten continuation authority,
affordable-prefix, durable-phase and additional-effect rejection regressions
pass in `/tmp/canic-174-continuation-regressions.log`. Scoped host/CLI/internal
all-target/all-feature Clippy passes in
`/tmp/canic-174-continuation-clippy.log`.

The IC fixture's precreated Ledger result now matches the reviewed initial
Root amount, and its retained operator debit uses that amount. Its deliberate
native underforecast is measured as current Root balance plus 20T, rather than
assuming the initial Root still has its old fixed balance. These are fixture
accounting updates; the separate withdrawal proof still does not establish the
combined E163 child-claim journey. The focused IC rerun passes, including lost
withdrawal replies, lost post-receipt observations, reconstructed adapters,
terminal conservation and effect-free replay. Its retained log is
`/tmp/canic-174-continuation-native-ic.log`.

Fresh prepayment closes the earlier missing initial-plan path. Live grant
usage/reservations, operator mint receipts
and combined child recovery remain open. No runtime budget, successor effect
permission or live funding authority changed.

The shared deployment floor now also feeds startup generation, reviewed
prepayment and supplementary native funding. Startup retains
`max(request_threshold + 1, deployment_reserve) + initial_child_grants` before
adding execution margins. Supplementary funding uses
`max(configured_minimum, deployment_reserve)` plus its existing margin. The
runtime floor remains exactly 1T; moving its constant to the pure deployment
policy gives these consumers one authority. Five startup arithmetic cases,
the generated-estate planner journey and all seven native funding cases pass
in `/tmp/canic-startup-floor-tests.log`, `/tmp/canic-startup-floor-planning.log`
and `/tmp/canic-native-floor-tests.log`. The native cases retain their exact
fee/burn deltas around the real floor, with initial and final balances shifted
together; no production test behavior was added.


## Live Coordinator usage projection — 2026-09-15

Generation now supplements the fresh-child scenario with one protected current
Coordinator funding query. It requires the selected artifact to match the
observed installed module, binds the selected operator, and checks the exact
Coordinator, unique configured Root set and current policy. Active policy
rotation, failed/malformed responses and inconsistent accounting remain explicit
unavailable observations; none means an unused budget. The existing canonical
DTO and Candid sidecar are reused, without another endpoint or stable store.

The report shows current-window spent/reserved/remaining cycles, successful
automatic totals and caps, funding enablement and pending Root operations.
Pending operations remain separate from successful counts. Window allowance
is not native balance, a lifetime allowance or approval: reservations may
already represent outgoing cycles, and Root eligibility/caps still apply.
The response is queried afresh on each generation and is not persisted as plan
authority. Native balances and this response are separate observations, not
an atomic snapshot.

The shared pure window calculation also fixes an admission defect: overflow
of spent plus reserved cycles previously fell through to zero usage. Invalid
current-window accounting now denies the grant. A missing or stale runtime
ledger still means no spend in the current window; a missing host observation
remains unavailable. Ordinary grant limits and reserve policy do not change.

Qualification on the dirty 0.110.16 base, for the open 0.110.17 draft:

- Four host cases cover reservations, successful/pending separation, authority
  and rotation rejection, canonical Candid decoding, repeated fresh queries and
  malformed responses: `/tmp/canic-startup-usage-tests.log`.
- Six core policy cases cover actual admission, overflow, window/reserve/cooldown
  boundaries and non-renewing caps: `/tmp/canic-startup-usage-policy.log`.
- The retained-estate generator/planner/replay regression passes and checks that
  a different installed Coordinator build cannot supply usage authority:
  `/tmp/canic-startup-usage-generation.log`.
- CLI evidence labels and exact cycle amounts pass:
  `/tmp/canic-startup-usage-cli.log`. Core, host and CLI scoped all-target,
  all-feature warning-denied Clippy passes: `/tmp/canic-startup-usage-clippy.log`.

These are native arithmetic and synthetic transport checks. No new PocketIC,
mainnet or Toko workload run is claimed. The separately qualified combined
same-child/native-withdrawal recovery is recorded in the
[native funding report](canic-172-native-funding.md). Live Root/Component child
ledgers and their in-flight reservations, integration into supplementary
recovery quotes and exact operator mint credits/fees/receipts remain open.
The initial-demand calculation still assumes fresh child ledgers and excludes
execution burn. The complete accepted release batch is not push-ready.


### Next child-ledger observation boundary

The current parent ledger charges a child grant before awaiting its deposit,
and restores the previous snapshot after a rejected transfer. Its grant total
therefore includes reservations; it is not solely a settled-transfer total.
The replay receipts own pending/recovery state. A live forecast must not add the
same in-flight transfer again or treat uncertain child receipt state as unused
allowance. Prefer bounded parent observations over repeating a full replay-store
scan for every child; no second durable grant ledger is needed. This is a design
constraint for the remaining work, not implemented live child forecasting.
