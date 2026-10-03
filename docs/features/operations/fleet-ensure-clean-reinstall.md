# Clean Reinstall From Physical Inventory

Use this guide for every pre-1.0 release transition, including replacement of a
malformed or incomplete installation. It covers inventory selection, review,
apply, interruption recovery, and terminal replay.

[Back to the Fleet Ensure overview](fleet-ensure.md).


Run readiness before building, supplying the current policy and intended estate
seed together. This uses the generator's input checks without loading artifacts
or decoding a predecessor's executable contracts. A symbolic fresh seed is valid
for initial creation. Replacing retained state requires explicit physical
inventory, including when bootstrap, activation or import never completed.
Readiness and generation do not require a healthy predecessor plan or journal.

Only owned operation records and nonempty evidence directories establish retained
work. Local metadata, notes, editor backups, temporary writes and empty evidence
directories do not turn a fresh deployment into a reset. Malformed or unreadable
owned records remain evidence; their presence still requires explicit physical
inventory or same-operation recovery.

Explicit reset review qualifies the current artifacts and checks certified
physical custody before archiving the predecessor bytes. An unfinished Root import
with no uncertain host handoffs can be discarded: its Root and child IDs remain
controlled, and the new Root imports their actual cleared, stopped or running
state. The old reservation and budget are not amended or reused. A genuinely
uncertain host payment, creation or controller effect still requires reconciliation;
`ResetUncertainEffect` identifies its file and effect. An unreadable paid-effect
journal cannot prove those outcomes. Preserve it; do not delete evidence or infer
that an Ensure `Intent` means the call was never sent.

```sh
canic --environment staging fleet readiness toko-miner-staging-001 \
  --identity toko-miner-mainnet \
  --operator '<CURRENT_OPERATOR_PRINCIPAL>' \
  --source deployments/toko-miner-staging-001.toml \
  --seed deployments/toko-miner-staging-001.estate.toml
```

Prepare those inputs explicitly: select the estate's physical
Coordinator, Root and Store IDs, plus every Root-owned child, including allocated
workloads and descendants. Terminal identity metadata is an inventory source,
not import approval. Keep its records unchanged; write the selected IDs into a
current seed with `fresh_estate = false` and put each Root's children in its
`roots[].pool_imports`. Match those imports in the current policy's
`fleet_subnet_roots[].canister_pool.imports`, with sufficient pool capacity.
Reject incomplete or conflicting inventory instead of substituting new IDs.
The subsequent review establishes live custody, membership, cycles and exact
wipe/funding effects; only its approved digest authorizes execution.

Successful readiness reports `generation_inputs_checked: true` when both paths
were supplied. Without them, readiness checks retained work, signer/network and
funding only. Input checks do not qualify artifacts or guarantee live admission;
generation and review check their inputs again. `--source` and `--seed` cannot be
combined with `--desired` on readiness. Preserve the selected source, seed and
qualified build throughout interrupted reinstall phases.

For explicit replacement inventory, that readiness command also reports
`funding.clean_reinstall_infrastructure`: current native balances and an upper
funding forecast for each selected Coordinator, Root and Store. It uses the
current generated allowances and a bounded installation window, before loading
replacement artifacts. The sum excludes Ledger fees; `maximum_ledger_transfers`
identifies the possible fee count. A missing balance or controller mismatch keeps
the sum unknown. Import, pool and workload funding remain explicitly unresolved
until their own reviews; this infrastructure forecast is not a whole-Fleet quote.

When no infrastructure forecast is available, readiness reports
`funding.clean_reinstall_infrastructure_unavailable`. Missing paired inputs use
`generation_inputs_not_supplied`; an operation outside the completed-Fleet
forecast boundary uses `fleet_not_completed`. A retained infrastructure plan
without an execution journal uses `retained_infrastructure_review`, including its
exact `operation_id` and `plan_sha256`. Plain output explains the same boundary.
An unavailable quote does not itself block compilation of replacement artifacts
needed for review. Empty blockers still do not establish funding sufficiency;
a null forecast is never a zero quote.

Explicit current inventory also enables the forecast for an unpaid review or
unfinished predecessor. Readiness does not decode its executable authority,
replace the review, approve payment or invent a conversion amount. A new reset
review binds its own current funding; same-operation retry retains its original
authority.
The unavailable reason is null when an infrastructure forecast is present;
individual unavailable observations remain on that forecast's targets.

An infrastructure review selected before approval can be cancelled by exact
digest when a new release or funding policy is required:

```sh
canic --environment staging fleet ensure <fleet> --cancel-reinstall <plan-sha256> --json
```

This local command requires no build, signing or network access. It archives the
original review bytes under `.canic/fleet-ensure/history/<environment>/<fleet>/`
before releasing the selected desired build. JSON reports
`stage: "clean_reinstall_cancelled"`, the cancelled operation/plan and archive
digests, and false payment/deployment authority. Repeating the same cancellation
replays its receipt while no replacement review exists; a stale digest cannot
cancel a new review. Cancellation conflicts with apply, reinstall, funding,
conversion and replacement-input options.

Any execution journal, side-operation evidence, unknown owner file or inspection
record showing execution began refuses cancellation. Issued work must retain its
original reconciliation owner. An interrupted cancellation fences other Fleet
writers; repeat the exact cancellation command to finish archival/removal.
Changed or newly introduced files stop recovery before further removal. Ordinary
review observations are archived, including their bounded attempt records.

After cancellation, generate current desired state from the complete explicit
physical inventory and qualified replacement build, then request a new
`fleet ensure --reinstall` review. That review refreshes custody and funding under
the current policy. Inspect its new digest before separately approving effects.
No historical application schema, manual journal edit or alternative reset driver
is involved.

Infrastructure review funds each owner's installation and observation window.
It adds any shared budget shortfall as an exact Store credit in the same approval;
it does not deposit the whole continuation ceiling on every canister. Fresh
bootstrap review quotes initialization and Store/Registry registration before
admitting initialization. Supplied identities use the actual registration
compiler; Coordinator creation uses the artifact-bound action/retry ceiling until
its ID exists. Required work is never reduced to fit the current balance. A
shortfall is calculated including configured floors and excluding reserved cycles
from spending. If the operator cannot fund the quoted debit, review reports the
required, available and missing amounts before retaining an executable plan.
The operator's free Ledger preflight rejects known shortfalls before consuming a
management-inspection attempt. Fund that account and retry the same survey;
original source balances and custody are preserved. Paid inspection attempts
remain finite, and the protected observation rechecks funding before publication.
Each further native write checks the actual
target's current headroom. A typed headroom or continuation-budget failure stops
further writes; retain the original operation and receipts. Inspection attempts,
lost-response reconciliation and terminal conservation remain bounded. The
generated observation/update allowances are unchanged; do not lower them or edit
generated authority to bypass a funding failure.

Already retained approvals keep their original execution ceiling and effect
records. Repeating their review does not apply fresh admission retroactively.
If registration exceeds that approval, use the separately approved bootstrap
registration recovery review; a new estimate alone grants no extra spending.

Build Toko against the same current Canic release as the CLI, using the normal
`canic build` command. Keep the selected build's artifacts. Generate fresh current
authority from the explicit estate seed and current policy:

```sh
canic --environment staging fleet generate toko-miner-staging-001 \
  --identity toko-miner-mainnet \
  --app-config apps/toko_miner/canic.toml \
  --source deployments/toko-miner-staging-001.toml \
  --seed deployments/toko-miner-staging-001.estate.toml \
  --release-build '<CURRENT_RELEASE_BUILD_ID>' \
  --output fleets/toko-miner-staging-001-current.toml
```

Use a new output path, or the explicit replacement digest when replacing an
existing output. Do not use `--fresh`: the seed supplies the retained Coordinator,
Roots, Stores and every child ID, including allocated workloads. Generation with retained state compiles the replacement without querying its old
runtime. It
does not claim observed cycle balances; those are sampled during current review.

Review the first phase:

```sh
canic --environment staging fleet ensure toko-miner-staging-001 \
  --identity toko-miner-mainnet \
  --desired fleets/toko-miner-staging-001-current.toml \
  --source deployments/toko-miner-staging-001.toml \
  --seed deployments/toko-miner-staging-001.estate.toml \
  --reinstall --json
```

Follow the returned `next_command`. Each phase shows its exact digest and debit
bounds before apply. Infrastructure installs the qualified current Coordinator,
Root and Store under typed initialization. Import observes the held children
through the new Root, stops running application code, clears it and publishes the
exact retained capacity. The final phase provisions the fresh workloads.

```sh
canic --environment staging fleet ensure toko-miner-staging-001 \
  --identity toko-miner-mainnet --apply '<REVIEW_SHA256>' --json
```

After a phase completes, the returned command reviews the next phase. Current
build authority and input paths are retained for retries. Repeat the same apply
digest after interruption; do not regenerate artifacts or edit journals. Every
paid effect retains intent before submission and reconciles uncertain responses.
Additional funding requires its own bounded review. `fleet_completed: true`
marks completion of the whole reset; infrastructure or import completion alone
does not. Immediate completed apply replays the retained receipt locally.

A later explicit `--reinstall` with a different qualified desired selection starts
a new reset after the paid-effect checks, even if the predecessor is unfinished.
A matching unfinished current selection resumes its frozen operation and allowances.
After Fleet completion, explicit `--reinstall` starts another reset. A matching
selection marker does not require repairing malformed current execution files;
those files fall back to the same physical reset and paid-effect checks. Ordinary
`fleet ensure` does not discard a completed Fleet. Historical records remain evidence; they are never repaired into executable authority and
missing controller fields are never filled with guessed defaults.

These commands clear Canic and application state. Supplied IDs and their cycles
remain controlled, subject to the reviewed protocol debit. Application roles may
be reassigned within the selected Root/subnet inventory. Toko's frontend is a
separate ICP asset canister outside that inventory: publish its new assets to its
existing ID to retain the origin. No Canic command here changes Toko's wrapper or
frontend configuration.

Downstream orchestration should call readiness before `canic build`, use the
current release consistently for CLI and runtime, and follow the phase commands
until Fleet completion. Readiness does not authorize payment or predict complete
application initialization.

After convergence, `canic admission plan`, `apply` and `status` use the selected
release retained in the terminal Fleet plan to locate Coordinator and Root Candid
sidecars. Keep that release's finalized manifests and artifact files available.
Each command verifies manifest hashes and the retained role/module/protocol
binding before transport. Missing or changed artifacts reject; these commands
neither rebuild interfaces nor require environment-local sidecar copies. A newer
unapplied Fleet review must first complete its own plan/journal handoff.

Human-readable reports describe a **planning budget**: maximum operator debit,
unavoidable fees, Root-funded creation fees and execution burn are allowances,
not measured expenditure. The conservation equation names each term; observed
conservation appears separately when terminal evidence is available. Cycle
amounts use compact `B`, `T` and `Q` units rounded to three decimals; use JSON for
exact integer amounts.

Native canister balances accept outside donations. A higher observed native
balance does not invalidate a creation receipt, completed funding withdrawal,
retained operation or terminal replay. The original starting balance and exact
payment identity remain unchanged. An outside donation does not authenticate a
Ledger payment, authorize another withdrawal, or increase a reviewed paid-effect
limit. Successor phases retain the highest previously recorded net-debit
watermark; a later surplus cannot replenish that recorded allowance.

Terminal JSON reports `observed_net_cycle_debit_cycles` and
`observed_net_cycle_credit_cycles`. These are mutually exclusive net differences
between final controlled balances and starting balances plus recognized funding,
less exact estate creation fees. They do **not** measure gross execution costs or
total donations: a deposit and consumption between observations can offset one
another. The reviewed execution allowance bounds the observed net deficit;
receipts and independent action/payment bounds remain necessary. Human output
calls this `observed_conservation`.

This treatment covers native canister donations. Operator and Root Cycles Ledger
accounts retain their receipt-bound accounting. Unexpected Ledger credits are
not silently classified as authorized funding. Destructive drain/delete steps
still require their reviewed residual limits: a late donation may require another
drain or review so the additional cycles are not discarded.

Each canister row lists its action kinds in plan order. `host_create_actions`
counts direct initial or replacement creation actions in that plan. Funding
domains report `root_funded_creations` for additional pool capacity created by
the Root; zero does not mean the host will create no canisters. Progress remains
on stderr with the current phase and applied/reviewed effect counts. Preserve
that stream when capturing stdout in a wrapper; these counts do not estimate
remaining time or indicate full-Fleet readiness before terminal verification.

Observation diagnostics also use stderr. JSON emits
`event: "fleet_ensure_observation"`, `schema_version: 1`, and an `observation`
containing `stage`, `parent_stage`, `elapsed_millis`, `remote_call_attempts`,
`identity_lookup_attempts`, `cached_read_hits` and `succeeded`.
Counts represent logical remote-call attempts, including failures, rather than
transport packets or handshake traffic. Independent infrastructure reads may
overlap up to four at a time. Pool reads and error precedence retain configured
order; every issued batch drains before an error returns. An explicit planning transaction shares its read-only snapshot across Root
management, estate observation and protocol preparation. Nested consumers reuse
that snapshot; it expires on success or failure, pacing, changed reviewed inputs
and before an effect. Standalone observations retain their own shorter lifetime.
The counter covers remote attempts issued inside the named stage, not the cost
of evidence it reuses from an earlier stage. A non-zero PoolBalances duration
with zero attempts can include local preparation, validation and use of an
already observed response within that same read-only snapshot. It does not mean
zero work or authorize reusing the balance in a later review or after mutation.
Identity lookup attempts count local ICP Principal resolutions, including
failures. Cache hits count responses actually served to consumers, including
repeated consumers of the same response. Pool-balance preparation validates
shared Root/operator authority once per batch of at most four assets; every
asset retains its controller/module checks. A later batch checks the operator
again. Nested stage durations/counts overlap and must not be summed as disjoint
whole-operation work. `Planning` and `FleetSnapshot` report inclusive totals;
`parent_stage` identifies nested work. Sum only non-overlapping top-level events
when estimating observed-stage totals; this is not a count of every IC message.

Coordinator provisioning-status reads retry only typed transient transport
failures: HTTP 408/429/502/503/504, connection/body transport failures and timeouts.
The authenticated agent binds the signer, network, target, method and arguments
for all attempts. There are at most three logical query attempts, with 250/500 ms
backoff, a ten-second per-attempt limit and a thirty-second overall network-read
budget after identity/network resolution. Agent-internal HTTP retries and
certificate reads share that deadline but are not separate logical attempts.
Response bodies are bounded to 8 MiB. Authentication, signature, application and
Candid decoding failures stop immediately; mutation calls are outside this retry
path. Exhaustion retains the original issued operation for ordinary same-digest
recovery; it never authorizes a replacement effect.

Authenticated queries share connection/runtime setup within a command, while
each logical query resolves and verifies its signer, network and root key
afresh. This reuses transport resources, not authority or query results.

Terminal inventory also overlaps independent Component partition reads and,
within each parent's child set, Root allocation-receipt reads up to four at a
time. Each set must validate before its management inspections begin. Receipt
failures drain the issued batch, stop later batches and prevent partial inventory
publication. Pagination, parent traversal and fresh authority checks retain their
existing boundaries; these observations are not cached across terminal passes.

Within one read-only protocol planning pass, manifest and chunk checks share a
successful template-status read for the exact Store, Candid path/digest, template
and version. Each action still verifies its Candid binding. Independent upload
batches similarly share one catalog response within each reconciliation pass,
testing every chunk's exact hash separately. Pre-submit and post-submit passes
start fresh; no template observation crosses an effect or survives a failed
pass. Standalone checks and later plans/retries query afresh. Cycle observations
remain fresh per effect, and `cached_read_hits` includes shared catalog reads.

The later pool-balance stage refreshes PendingReset and Failed assets in groups
of at most four. Each required inspection retains its target-specific reserve
preflight and Root/controller checks. Issued reads drain before a failure is
returned in inventory order; a failed group publishes no balance changes and
does not start the next group. Other lifecycle states retain their existing
observation path. Balances are not retained across reviews, effects or retries.

For a fresh native top-up, the executor uses one protected funding observation
both for its initial balance and its first completion check. It persists the
exact intent before the withdrawal, and consumes that observation once. The
Root, pool membership, lifecycle, module, controller and reserve checks still
run. An interrupted intent is observed afresh; post-payment completion requires
the exact receipt and a new balance observation. This does not share evidence
between payments or permit concurrent withdrawals.

Protected provisioning status retains one latest failure with stage, target,
operation, diagnostic, retry category and the originating timestamp. Transient
Root retries use delays of 1, 2, 4, 8, 16, 32 and then 60 seconds, capped at 60;
remote execution takes additional time. Durable work progress clears that
backoff. A proved Store activation
binding conflict suspends retries for review; exact acceptance replay does not
clear it. Failure timestamps and attempt counts do not count as work progress
or bypass host stall detection. Issued effects remain in their existing records.

Authorized child-allocation status also includes nullable
`last_failure.platform_rejection`: up to 1,024 UTF-8 bytes of the originating
IC rejection, including its rejection code. This evidence survives same-release
restart and is cleared with the failure after work advances. The public error
remains its bounded diagnostic code (for example E66); rejection text does not
select retry or recovery behavior. Allocation admission reserves space for the
maximum diagnostic before any failure occurs.

> Development status: canister/code/controller/cycle convergence and the typed
> Store, Registry, Root-mirror, local Component Registry and Component action
> graph are implemented. A fresh-estate governed PocketIC journey traverses the
> complete graph through terminal Ensure publication and immediately recompiles
> with no update effect. Authority-bearing operator commands now consume the same
> terminal inventory and exact protocol bindings. That inventory is rebuilt
> from terminal protected control-plane evidence, including protocol-created
> Components, pool assets and bounded descendants. Focused implementation
> qualification, including retained-estate generation through an effect-free
> second ensure and focused fresh-seed/create replay, is complete; broad
> validation remains maintainer-owned.

## Continue From Here

- [Generate or write desired state](fleet-ensure-desired-state.md)
- [Review and apply the resulting Fleet plan](fleet-ensure-plan-and-apply.md)
- [Read the recovery and cycle-safety rules](fleet-ensure-recovery-and-cycle-safety.md)
- [Return to Fleet Ensure](fleet-ensure.md)

