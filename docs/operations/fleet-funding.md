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
