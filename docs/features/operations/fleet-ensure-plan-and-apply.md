# Plan And Apply A Fleet

Use this guide for the normal Fleet Ensure workflow: inspect the no-effect plan,
approve its exact digest, apply it, and confirm immediate no-effect replay.

[Back to the Fleet Ensure overview](fleet-ensure.md).


Planning performs observation and local current-state writes but no paid Fleet
mutation:

```bash
canic fleet ensure staging --desired fleets/staging.toml
```

Review the printed canister dispositions and conservation bounds, then apply
the exact digest:

```bash
canic fleet ensure staging \
  --desired fleets/staging.toml \
  --apply <plan_sha256>
```

The `--json` report preserves the complete plan metadata without embedding
Store publication payloads. Each `publish_store_chunk` request contains a
workspace-relative `bytes_path` under
`.canic/fleet-ensure/objects/sha256/`, plus the exact `bytes_sha256` and
`bytes_size`; the referenced object is the same hash-verified content retained
for interruption recovery.

After a payload's metadata and chunk zero are confirmed, apply may upload up to
four remaining independent chunks together. Each retains its own reviewed action
and durable intent. Submitted calls are drained before returning an error; successful
sibling receipts and exact live hash observations are retained. A restart reconciles
the retained individual effects before issuing later work. The bound controls network
pressure, not artifact capacity. Preparation, fixture streams and activation are
ordered, and the reviewed cycle budget and terminal conservation checks still apply.

Apply can also reconcile up to four distinct pool assets together when their
Root and Candid authority match. Initial funding admission still runs before
issuance. Each asset keeps its own intent, controller/module/balance checks,
receipt and interruption recovery. All submitted calls finish before a failure
returns, retaining successful siblings. Duplicate assets or changed authority
end a batch; funding, provisioning and maintenance actions remain ordered.

Before the first effect, changed desired bytes, artifacts, authority-bearing
live state, funding sufficiency or the live Cycles Ledger fee stop apply and
require a new plan. Live native balances may increase through donations or refunds; decreases
must stay within the reviewed per-canister observation bound. The normalized
action graph and funding authority must remain identical. If a donation changes
the required action set, obtain a fresh review before starting. The accepted
apply-time balances become the journal's truthful initial conservation evidence;
a decrease outside the bound rejects before any effect. Once the journal is in progress,
the plan's digest-bound reviewed desired input is authoritative: newer working
bytes cannot alter or supersede it, and an explicit environment lets the CLI
resume even if the working TOML is missing. After terminal closure, rerun the
planner to review the current working desired state as a separate successor.
If an accepted effect produces less live state than reviewed, Canic closes that
completed action journal, refuses to call the Fleet converged, and requires a
new plan from the resulting live estate; it never guesses a compensating debit.
Any newly created Principal remains retained as pending current authority, so
the successor plan reuses that canister instead of issuing another creation.
An interrupted invocation retains one intent per action under
`.canic/fleet-ensure/<environment>/<fleet>/` and reconciles retained actions before
opening another batch. The stall budget counts only consecutive non-progress.
Durable Store publication requests contain exact `bytes_sha256` and `bytes_size`
fields. The content-addressed object store retains the bytes before plan
publication; reopening verifies their hash, size and prepared authority.
Infrastructure bootstrap and unfinished activation preparation also admit native
credits while retaining the original source observations, bindings and debit
limits. A new activation preparation records its current controlled balance and
keeps the original activation source unchanged. Fleet-release custody and
held-capacity checks admit increases in the checked native-plus-reserved total;
the original debit ceiling and minimum native balance still apply at both
boundaries. Credits do not expand funding authority or permit Ledger-account
drift.

Current import reviews always contain `funding_credits`, including an empty
array, and registration recovery fields serialize as explicit `null` until
requested. These fields bind current review and journal hashes.

When a partial current-control-plane reset makes a Root's protected pool status
return `STATE_CONFLICT` or `STATE_UNAVAILABLE`, planning does not invent an
empty pool or configured-capacity balance. For an exact desired Store or pool
identity under an exact live Root/controller binding, it first attempts the
public Canic cycle-balance query and otherwise uses the last exact balance
retained by the current Fleet Ensure state. A zero-valued `PendingReset` row is
treated the same way. Missing exact evidence is a blocker. This narrow
observation cannot create, fund, replace, transfer, drain or delete anything.

Root management prerequisites use management status before protected Root
queries. The Start-only plan embeds its generator authority, and apply revalidates
the exact target and installed module. Its completion reports
`prerequisite_complete`; it does not publish terminal Fleet topology.

Outside this Start-only prerequisite, the reviewed Ensure plan owns explicit
infrastructure reinstall effects. Pool-policy drift and status failures cannot
independently select a reset. The reviewed reinstall records the management canister version before the
effect. Terminal observation requires the requested module at a strictly newer
version, including after response loss or process reconstruction. The generated
changed-release journey now qualifies the complete transition through a current
working Fleet. The Root-only prerequisite remains a separate completion scope.

The normal ICP CLI status projection supplies module, controller, runtime and
cycle evidence. If that projection omits `canister_version`, Canic obtains the
exact install-only pre/post version from the typed management-canister
`canister_status` response. The direct agent calls the management Principal but
routes through the install target as the HTTP effective canister ID. It uses
the selected ICP environment's resolved root key and an in-memory export of the
selected controller identity, rejects a Principal mismatch, and zeroes the PEM
buffer without retaining it in Canic state. The fallback takes module hash and
version from that one response so terminal proof cannot combine different
observations. It never defaults or infers the value. If the identity is not
exportable, or either observation boundary cannot supply the proof,
apply stops before install and directs the operator to restore management
status access and resume the same reviewed plan; an already-retained
empty-effect journal remains the replay authority.

The Store authority retained by a Root describes the only Store that may be
adopted; it is not proof that adoption occurred. Store bootstrap remains
blocked until the exact derived operation ID returns the durable adoption
receipt with the matching authority and final Root-plus-operator controller
set. Missing or conflicting receipts keep the one idempotent adoption action
open.

## Continue From Here

- [Automate Fleet Ensure](fleet-ensure-automation.md)
- [Recover an interrupted operation](fleet-ensure-recovery-and-cycle-safety.md)
- [Return to Fleet Ensure](fleet-ensure.md)
- [Browse Fleet operations](README.md)

