# Fleet Funding Operations

This runbook covers the maintained funding paths for a terminal current Canic
Fleet. The Fleet Coordinator normally funds current Fleet Subnet Roots. Direct
cycle top-up and Root-owned ICP conversion are explicit recovery actions.

## Observe Before Acting

Read the Coordinator and every current Root through the terminal ensure
inventory, its exact protocol bindings, and the protected live Registry:

~~~text
canic cycles funding <fleet>
canic cycles funding <fleet> --json
~~~

The report identifies the exact Coordinator and Root Principals and shows:

- Coordinator balance, funding enablement, protected profile, minimum reserve,
  rolling spend plus outstanding reservations, and non-renewing automatic caps;
- each Root lifecycle state, live balance, funding eligibility, protected
  request/target policy, current operation and last Coordinator result;
- manual/automatic ICP policy, cumulative ICP use and the latest refill result;
  and
- each exact Root placement Subnet from the protected Registry.

Automatic Root funding is the normal path. Do not start a competing recovery action
while a Root reports `pending=true` or a refill reports `recovery_required`.
First reconcile the existing operation under the
[recovery and retry runbooks](recovery-retry-runbooks.md).

## Deployment Reserve Failures

`E163 DEPLOYMENT_CYCLE_RESERVE_REQUIRED` identifies the Root's deployment
reserve guard. Available pool canisters do not provide this native execution
headroom. The existing guard requires 1T available after outstanding cost
reservations; this is a minimum guard, not a recommended operating balance.

Inspect the retained child operation's `allocation.last_failure` for its exact
diagnostic, failure time, consecutive failure count and retry deadline. These
fields are diagnostic observations, not lifecycle advancement. Persistent
failures back off up to 60 seconds, while the existing claim and effect
receipts remain. A successful lifecycle step clears the retained failure.

For an issued Component provisioning operation, repeat Fleet Ensure without
`--apply`. Its retained configuration and provisioning scope can produce a
`funding_review` with `pause.kind = native` and a separately approved digest.
The text report labels its destination `root_native_balance`; `estate` reviews
credit the Root's Ledger account instead. Review the exact Root, Ledger, amount
and fee, then apply that funding review's digest through the same Ensure command.
The original plan, issued operation, effects and starting balances remain fixed.

The native amount restores the greatest of the retained configured Root minimum,
its reviewed bootstrap request threshold plus one cycle, and the runtime's 1T
deployment reserve, plus its configured observation/update margin. The threshold
comes from the exact selected Root; missing or duplicate bootstrap authority and
overflow reject before effects. Without bootstrap authority, the configured
minimum and deployment reserve still apply. This is not a forecast of the initial grant wave
or a claim that low balance explains every provisioning failure. Only a Root
selected by the issued provisioning action is eligible, once per action.

An unapproved quote can be refreshed by repeating review if the Root balance
moves beyond its margin; the old digest then stops authorizing funding. If the
Root is already funded, that unapproved quote is removed. Once an intent exists,
the exact amount and timestamp are retained through every retry.

The operator must already have sufficient cycles. New operator mint credits
remain unsupported in this retained-operation path and unexplained balance
changes reject. Do not edit starting balances or withdraw out of band. A lost
withdrawal response reuses the same timestamp; a retained receipt resumes with
observations. A review cannot issue another credit for a persistently failing
Root under the same provisioning action.

The host review/accounting passes focused native, CLI and composed IC withdrawal
qualification, including lost replies, receipt recovery, conservation and replay.
A combined local IC case also recovers the same E163 initial-child claim through
one reviewed withdrawal and two lost responses. It uses the existing Ledger stub
with zero fees; it does not establish mainnet Ledger or Toko workload qualification.
Operator mint receipts and live child-grant/recovery-quote accounting remain
CANIC-172/174 work. Generation now observes Coordinator accounting as described below. Disposable fixture balances are not sizing recommendations.

## Inspection reserve failures

Before an uncached protected status inspection, the host queries the selected
Root's controller-only `canic_observability(InspectionReserve(target))` surface
when that selector is declared in the exact bound Candid contract.
It reports the Root's native/liquid balances and the exact encoded management
call's current reserve requirement. This query is also available while the Root
is prepared. An invalid response or observed shortfall stops before the update;
a retry queries afresh. It adds one query per uncached inspection, while bounded
pool concurrency and successful observation reuse remain in place.

This is an indicative sample. The update reserves execution cycles that the
query does not reserve, and concurrent work can change balances. A sufficient
sample therefore cannot guarantee admission. No host-defined minimum call fee,
automatic transfer or reserve-policy reduction is introduced. A Root that cannot
serve the query returns its transport failure, not an invented zero balance.

Source-bound recovery may inspect a retained Root whose reviewed contract has no
reserve selector. The host preserves that contract's protected inspection without
requesting a newer selector. This supplies no ahead-of-call reserve estimate.
The current generated Root contract declares the selector and requires preflight;
invalid contracts or failed declared queries never permit bypassing it. Source
artifact, controller and conservation checks still govern recovery admission.

`E162 PLATFORM_INSUFFICIENT_LIQUID_CYCLES` identifies an SDK outbound-call
admission failure, separate from the deployment guard above. The protected Root
`InspectCanister` command returns `InspectionReserveRequired` with the calling
Root, requested target, native balance sampled before the attempt, and the SDK's
available and required liquid-cycle amounts at admission. The host validates
these identities and numbers, then reports a typed error with the raw cycle
amounts. A positive native balance alone does not establish usable call headroom.

The SDK rejects this call before issuing the outbound management request.
Other transport errors are not relabelled as funding failures, and a failed
inspection supplies no target status for later reuse. Review the caller's funding
and freezing reserve before retrying; the evidence neither authorizes a transfer
nor changes any configured threshold. The successful retry observes fresh state.

These details require the Root to admit the inspection update. A frozen Root
that cannot execute it cannot return this evidence. The required amount describes
one outbound call, not total recovery demand or the debit of an approved top-up.
The earlier host query reports known outbound headroom; a complete recovery-funding
preview, including later funding and execution needs, remains work.

## Funding deadlines after child grants

When a child-grant management call returns, the parent resamples its native
balance and updates the existing funding timer. A reserve-crossing debit brings
its safety check forward; a transfer cannot postpone an earlier check. The
sample starts a new burn-estimation interval so internal funding is not counted
as computation burn. Receipt replay does not issue another deposit.

Existing eligibility, enabled state, cooldowns and budgets still govern grants.
A timely Root request can receive a protected no-grant response when the
Coordinator lacks spendable reserves. Timely demand does not pre-fund the
initial runtime grant wave.

## Startup funding estimate

Desired-state generation prints a startup funding scenario alongside its
output path. It reuses the generation observations and the exact configured
initial placements. Infrastructure balances are marked as observed or
configured creation amounts. Workloads use the Root pool readiness floor;
the scenario assumes fresh per-child grant ledgers and zero execution burn.
It is not a quote for the currently running estate or an approved funding plan.

The calculation follows grants from leaves back to their parent. A workload
at 4.9T with a 10T threshold and 5T increments requests twice. Equality also
triggers a request: reaching exactly 10T does not end the grant wave. Each
parent's demand includes the grants it sends to its children. The Root total
contains only its direct transfers, so the same cycles are not added again as
they move further down the hierarchy.

Per-request and lifetime limits clamp the scenario; disabled top-ups and
unfunded per-instance requirements remain visible. Window-budget comparisons
and cooldowns describe timing constraints, not permission to exceed them or
a promise that the wave fits one window. Root native requirements retain its
greater of its request threshold plus one cycle and the 1T deployment guard
after the projected direct grants. Exact raw
shortfalls accompany the rounded display. Coordinator spendable headroom is
shown separately, before its grant limits and outstanding reservations.

For an installed Coordinator matching the selected artifact, generation also
queries the existing protected funding status using the selected operator.
It reports current-window spent and reserved cycles, remaining window allowance,
successful automatic grants/cycles and their caps, funding enablement and pending
Root operations. The Coordinator, configured Root set and policy must match;
active policy rotation or unavailable/invalid responses are labelled unavailable.
A missing observation never means an unused budget. The preview reads fresh
status on each invocation and does not persist it as plan authority.

Window allowance is not native balance or permission to grant: Root limits,
pending operations and grant eligibility still apply. Successful automatic totals
exclude pending operations. Reservations may already represent cycles sent from
the Coordinator, so do not subtract them again from its observed native balance.
These observations do not yet replace the fresh-child assumptions below or feed
supplementary recovery quotes.

For an installed Root matching the selected artifact, generation also queries
the allocation behind each seeded pool Workload claim. A committed allocation
must match the operation, Component, Root and child; its installed binding
supplies the immediate funding parent and role. Only a Root-funded Workload
uses `canic_observability(ChildFunding(child))` on Root. Pool custody alone is
not a funding relationship: nested shards may be funded by their hubs.
Ready assets are marked `NotWorkload`; unresolved or conflicting allocation
evidence remains unavailable. Descendants show their exact parent with
`ParentRelayRequired`. Generation adds no update relay; that observation belongs
in a budgeted recovery flow. Each current Workload adds one allocation query.
The controller-only query binds parent and child and reports charged cycles,
last charge time, unresolved operation count and retained transfer reservations.
It is available while Root is Prepared; it creates no state or paid effect.
The child key identifies ledger usage, not proof of current registry membership
or grant eligibility. Generation obtains child identities from its seeded inventory.

Charged usage includes a grant before its transfer completes; a failed transfer
restores that charge. Consequently it is not always a settled-success total.
Unknown, missing or expired reservation evidence is displayed as `unknown`,
never zero. An unresolved response may remain pending after its cost reservation
settles. Do not subtract reservations again from observed native balances or
assume all charged cycles have arrived at the child. Unavailable observations
remain explicitly unavailable. Current ledger coverage is limited to seeded
Root-funded Workloads. Nested parent attribution is available, while descendant
ledger collection and complete live placement matching remain open.

For observed usage matching the selected release build and Component Spec hash,
the preview also reports `child_grant_allowance`: the lifetime limit, remaining
allowance after charges, remaining cooldown and next-request policy cap. It uses
the runtime's own policy. For example, 95T charged against a 100T limit leaves
5T after charges; a reservation already represented by those charges is not
deducted again. Any pending operation makes the next-request cap `unknown`,
including response recovery whose cost reservation has already settled.
At the exact end of cooldown the policy cap becomes available; exhaustion or
an active cooldown gives zero. Changed release/Spec evidence remains unavailable.

This cap describes only per-child request policy. Parent reserves, funding
enablement, window limits and other runtime admission checks still apply.
It is not a remaining-demand estimate or permission to top up. Minimum native
Root recovery remains available independently of descendant telemetry; a Root
may need that recovery before it can afford a relay observation.

Do not add these internal transfers to computation burn or new operator
funding. The generation estimate leaves creation amounts, desired state,
journal balances and reviewed debit bounds unchanged.

When planning pending provisioning on an observed Root, the planner separately
resolves startup demand from the reviewed application configuration. It adds
the selected protocol execution reserve and the existing target-local margin
to the Root's native requirement, then includes any shortfall in the ordinary
reviewed `Fund` action. An existing top-up grows without another Ledger fee.
The plan digest and maximum operator debit include that funding. Provisioning
deferred by the conservation bound loses its startup increment; a completed
provisioning plan does not refill the startup allowance on replay.

Fresh creation/reinstall continuation includes startup funding in its initial
review. Each participating Root receives an execution allowance derived from
its bounded artifact catalogue, imports, shared orchestration and permitted
fixture retries. The allowance includes Store publication conservatively but
does not include another Root's catalogue. The existing configured per-step
execution/observation bounds determine the amount. Creation includes it in the
one creation transfer; reinstall uses the existing native top-up. Larger
configured creation amounts are preserved, and successors cannot add spending.

These bounds still assume fresh per-child grant ledgers and configured initial
placements. They are not live remaining-demand quotes. Integrating the observed
child usage/reservations, recursive descendants and upstream eligibility remains open; do not treat
this partial integration as a complete activation funding guarantee.

## Funding Event Counts

Automatic grants count funding events, not Components, Shards or installed
canisters. Adding an application role does not require adding a grant.
The current protected policy permits at most four automatic grants per Root.
Coordinator's lifetime grant and cycle allowances must fit the combined
allowances of its Roots; it does not have a universal four-grant limit.
Each Root and the Coordinator must also satisfy their independent reserve,
target, cooldown, window and cycle limits. A larger combined allowance does
not guarantee a particular request will succeed.

Generation, Fleet Ensure admission and Medic use the same protected funding
validation authority as the runtime. Invalid funding policy must be corrected
before paid effects; do not infer validity from application canister count.

## Workload Funding And Application Failures

Workload replenishment is separately opt-in. `initial_cycles` funds creation;
it does not enable later requests. A Component selects replenishment through
`component_specs.<spec>.topup`; each child selects it independently through
`component_specs.<spec>.children.<role>.topup`. Without that exact role's
`topup` policy, Canic does not schedule its automatic replenishment. Funding a
Root or enabling a parent's policy does not enable a child's policy.

The policy's `threshold` selects when to request and `amount` selects the
requested cycles. Choose them from measured application consumption, expected
bursts and replenishment delay, within the parent's funding limits and reserves.
Configuration validation requires `amount` to be at most half `threshold`.
Requests remain subject to funding policy, cooldown and retry timing. Automatic
replenishment is not a guarantee against an application rapidly consuming its
balance, and supplying cycles does not repair an instruction-limit loop.

For an exhausted Workload:

1. Retain the exact environment, Fleet, canister Principal, installed release,
   failing method and platform rejection. Public `PLATFORM_UNAVAILABLE` alone
   does not identify cycle exhaustion or an instruction-limit failure.
2. Use the current Fleet's controller-authorized observation route:
   `canic info cycles <fleet> --subtree <workload-principal> --verbose --json`.
   This uses Root's protected relay for descendants. Inspect sample timestamps,
   coverage, balance and top-up outcomes; missing history is not evidence of
   zero burn. If the observation is unavailable or unauthorized, retain that
   result and involve the existing authorized operator. Do not change access
   policy to obtain diagnostics. `canic cycles funding` reports infrastructure
   headroom and does not establish that Workload replenishment is enabled.
3. Compare the installed release's exact configuration with its role policy.
   The current checkout alone cannot prove what was installed. With no `topup`
   policy, continued operation needs explicitly supplied cycles. With a policy,
   inspect the parent's limits, reserve and request outcomes before diagnosing
   a scheduling defect.
4. Correct the application path responsible for excessive work before resuming
   that traffic. If immediate balance recovery is needed, review the exact
   canister, network and a bounded deposit with the authorized operator's ICP
   CLI canister top-up facility. The Canic `cycles topup` command accepts only
   Coordinator or exact current Root targets; `cycles transfer` credits a
   ledger recipient and is not a Workload canister deposit.
5. After recovery, repeat the failed application call and observe balance and
   replenishment over representative traffic. Record the deposit and outcomes.
   A successful retry proves restored availability, not that the loop is fixed
   or that long-running funding is qualified. Keep manually supplied cycles
   explicit until an application funding policy is reviewed and installed.

## Bounded Two-Subnet Staging Profile

Use `preview_multi_subnet` for a restrained Fleet whose Coordinator and Root
occupy different physical Subnets. The recommended one-Root protected input
materializes these standard 13-node values:

| Purpose | Value |
| --- | ---: |
| Coordinator creation funding | 140 Tcycles |
| Coordinator protected reserve | 80 Tcycles |
| Root creation funding | 30 Tcycles |
| Wasm Store creation funding | 10 Tcycles |
| Root request threshold / target | 10 / 30 Tcycles |
| Grant cooldown / accounting window | 30 / 90 days |
| Window allowance | 30 Tcycles |
| Non-renewing automatic allowance | 2 grants / 60 Tcycles |
| Automatic ICP spending | Disabled |

The initial debit is 180 Tcycles. The existing 5 Tcycle Core allocation comes
from the Root's 30 Tcycles and does not add to that total. A deliberately lean
one-grant policy uses a 30 Tcycle lifetime allowance, a 110 Tcycle Coordinator
creation amount and a 150 Tcycle total, but permits only one automatic recovery
event. Use `multi_subnet` only for the retained high-reserve professional
profile; choosing a profile never substitutes for the exact required
Fiduciary cost acknowledgement.

## Direct Cycle Top-Up

Direct top-up is the break-glass path when the Coordinator or a Root needs
cycles immediately. Preview the exact authenticated target before the live
call:

~~~text
canic cycles topup <fleet> coordinator <amount> --dry-run
canic cycles topup <fleet> <root-principal> <amount> --dry-run
~~~

Then remove `--dry-run` to execute the reviewed command. There is no `root`
alias: a Root target must be one explicit current, non-Removed Fleet Subnet Root
Principal from `canic cycles funding`.

A direct top-up changes only the canister balance. It does not reset rolling
windows, cooldowns, reserved spend, or non-renewing grant/refill caps. Re-run
`canic cycles funding <fleet>` afterward and allow the current operation, if
any, to reach a terminal state before initiating other funding work.

## Manual Root ICP Conversion

Use manual conversion only when the selected Root has protected ICP-refill
authority and sufficient ICP for the requested amount, the ledger fee and its
configured minimum retained balance. Preview first:

~~~text
canic cycles convert <fleet> <root-principal> --icp-e8s <amount> --dry-run
~~~

Then remove `--dry-run` to execute it. The Root Principal is both the protected
source owner and the cycles recipient. Optional `--from-subaccount <hex64>` and
`--operation-id <hex64>` arguments remain bound to that exact request.

The CLI writes a generated live operation identity to
`.canic/operations/pending.json` before sending. If the command is interrupted
or reports a resumable result, repeat the exact same command from the same ICP
project root, environment, Fleet, Root, subaccount and amount. The pending log
reuses the original identity. Do not delete or edit it, choose a fresh identity,
or start an automatic/manual competitor while the ledger transfer or CMC notify
outcome is uncertain. Follow the
[pending-refill and recovery-required procedures](recovery-retry-runbooks.md#icp-project-root-pending-icp-refill).

Each Root retains at most 4,096 lifetime ICP-refill operation identities so a
delayed replay can never become a new transfer. Terminal identities are not
evicted. If status reports the capacity limit, exact replay of retained work
remains valid but a new conversion fails closed; use direct cycle top-up and
plan a fresh reinstall rather than deleting replay evidence.

## Refusals And Lifecycle Fences

- Funding-disabled Coordinator or Root state is an intentional kill switch.
  Direct top-up may restore balance, but it does not re-enable policy.
- Removed Roots are never valid targets. A Draining Root is eligible only
  before the one lifecycle-owned funding fence and only while exact unfinished
  funded teardown work remains. At or after that fence no new automatic work
  may begin; retained same-operation recovery must settle before removal.
- Rolling-window, cooldown and non-renewing cap exhaustion is fail-closed. Wait
  only for a renewable window/cooldown. Use an explicit direct top-up for
  immediate recovery; it does not renew policy authority. The hard-cut CLI does
  not expose the deleted install-plan-owned policy-rotation flags.
- Automatic ICP refill is a terminal emergency fallback, not a parallel funding
  source. It can start only after Coordinator funding cannot restore a Root at
  or below its protected emergency threshold.
- Per-call/cumulative ICP caps, minimum retained ICP, ledger fee, conversion
  rate floor and exact-target checks refuse the refill before an unsafe value
  transfer. Use direct cycle top-up for immediate recovery or correct policy in
  a newly reviewed current desired-state plan. Pre-1.0 releases remain
  reinstall-only.
- A Fiduciary placement warning is retained evidence of higher-cost authority,
  not a runtime override. Confirm that its exact acknowledgement matches the
  installed plan before funding the Fleet.

After any recovery, run both text and JSON status as needed, confirm there is no
unresolved current operation, and retain the terminal receipt for the incident
record.
