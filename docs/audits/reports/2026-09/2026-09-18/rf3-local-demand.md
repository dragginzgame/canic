# RF3 observed child shortfalls

## Scope

Continue CANIC-156/174 in the existing .25 draft, based on published .24
`933a35b66403f6a94f99803252cc52c0aec32958`. Preserve the earlier retirement
recovery and speed changes. The latest read-only Toko scan still ends with the
CANIC-166/172 embedded retirement evidence blocker, already corrected locally;
no newer confirmed blocker was found. Publication and exact staging recovery
remain downstream work.

This slice joins a seeded Root-funded Workload's existing native balance sample
to its policy-bound parent ledger. The CLI reports its own threshold deficit,
the part exceeding remaining lifetime allowance and the next configured request
under current request/cooldown limits. Runtime allowance policy remains the
owner of charges, lifetime headroom and cooldown. No observation calls, state,
funding authority, desired fields or persisted plan schema are added.

Threshold equality requires one more cycle of headroom. The next request uses
the configured increment, rather than truncating it to this deficit. A pending
grant leaves local demand unknown even if the sampled balance is high or its
reservation has settled. Missing balances and policy/accounting failures also
remain unknown. Disabled top-ups are explicit. Reservations are not subtracted
from charged allowance or observed balances again.

## Qualification

- 21 selected host startup-funding tests pass, including five new local-demand
  regressions: `/tmp/canic-rf3-local-demand-tests.log`.
- The existing generated retained-estate planning/application/replay native
  fixture passes with Workload observation failure and non-Workload demand
  assertions: `/tmp/canic-rf3-generation-test.log`. Initial added assertions
  incorrectly assumed one unavailable reason for both assets; corrected to
  assert each actual fixture identity and its distinct typed result.
- Both focused CLI preview tests pass, covering exact shortfalls and unknown
  demand: `/tmp/canic-rf3-cli-test.log`.
- Scoped host/CLI library/test all-feature warning-denied Clippy passes:
  `/tmp/canic-rf3-clippy.log`. Its function-length findings were resolved by
  separating local-demand rendering and extracting test setup; no suppression
  was added. Changed-source formatting and whitespace checks pass.

No broad suite or PocketIC run is needed to claim the above host-only diagnostic
behavior; no runtime or IC effect path changed. This is not IC qualification of
complete RF3. No version bump, commit, push, deployment or sibling mutation ran.

## Remaining RF3 work

Balance and ledger are separate observations, not an atomic snapshot. Local
demand excludes descendant transfers, execution burn, parent liquidity,
funding enablement and window admission. It cannot replace the conservative
fresh-child scenario or authorize a recovery payment.

The accepted complete RF3 batch still needs budgeted descendant collection,
exact live placement coverage, recursive demand and complete recovery reserve
integration, followed by transport and IC evidence. Use the existing protected
relay under reviewed recovery observation bounds; do not introduce paid calls
into generation. Preserve minimum native Root recovery before descendant
telemetry becomes affordable. The complete RF3 batch is not push-ready; the
previously qualified recovery/speed changes remain independently ready.
