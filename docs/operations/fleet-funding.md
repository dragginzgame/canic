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

Automatic funding is the normal path. Do not start a competing recovery action
while a Root reports `pending=true` or a refill reports `recovery_required`.
First reconcile the existing operation under the
[recovery and retry runbooks](recovery-retry-runbooks.md).

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
