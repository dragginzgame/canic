# Fleet Ensure

`canic fleet ensure <fleet>` owns desired-state Fleet installation and
convergence. It reads one current desired-state document, observes the
configured controlled estate, and either writes a reviewed plan or applies the
exact retained plan digest.

For other starting points, see [supplied infrastructure bootstrap](#supplied-infrastructure-bootstrap)
and [capacity import](#add-supplied-capacity-to-a-current-fleet). These explicitly
reviewed setup operations publish authority for subsequent Ensure convergence.

Every pre-1.0 release transition requires hard cut plus clean reinstall, including
malformed or incomplete installations. Select the complete physical inventory
and a qualified current build; `fleet ensure --reinstall`
reviews replacement infrastructure, child clearing and fresh workload convergence.
Completed operations become immutable history before current execution authority
is created. Their old desired documents, application interfaces and executable
schemas do not participate in the replacement decision. Predecessor state is
cleared rather than repaired to continue it. Unfinished paid effects require
cycle-safe disposition before destructive reset. Current same-release recovery
still prevents duplicated effects and spending.

Current certified controllers and subnet placement authorize the reset. The
operator must control the supplied Coordinator, Root and Store. Root-controlled
children remain held while current infrastructure is installed, then the current
Root observes their cycles and clears their code/state through reviewed import.
The archive under `.canic/fleet-ensure/history/<environment>/<fleet>/` retains
original operation bytes and available referenced objects. Retirement is local,
journaled and recoverable; it grants no live effect authority.

The `.45` focused qualification covers successive resets, interruption recovery
and offline terminal replay; see [current status](../../status/current.md) for
evidence and release status.

Use `--identity <name>` on `fleet generate`, `fleet readiness` and `fleet ensure`
to select the ICP signing identity without reading or changing ICP's global
default. For example:

```bash
canic --environment staging fleet ensure staging --identity staging-operator
```

`--environment` and `--icp` are top-level options; `--identity` belongs to the
Fleet subcommand. The selected identity is carried through observations,
generation forecasts, apply, funding-observation collection and operator-mint
recovery. Every existing expected-operator Principal check remains required;
an explicit name does not override the plan's operator or authorize spending.
An unknown identity fails through ICP rather than selecting another identity.
Password-file handling is unchanged. Pass the same selection when reviewing or
resuming an operation; generated successor-review commands retain it. Without
the option, the existing default-selection behavior applies. This option does
not select an identity for unrelated Canic command groups.

Normal `fleet ensure` review/apply, clean-reinstall review/apply/replay and
`fleet generate` commands print a timing receipt path under
`.canic/diagnostics/fleet/` before measured work. `--json` Ensure output emits a
`fleet_ensure_timing_receipt` event with that path. These private JSONL diagnostics
supplement the retained plan and journal; they never prove deployment completion
or authorize continuation. Funding-observation and operator-mint subcommands
retain their existing output owners.

## Automation results

Use `--json` for deployment wrappers. Successful Ensure, clean-reinstall,
bootstrap and import responses include `automation` with `phase`, nullable
`operation_id`, `plan_sha256` and `review_sha256`, `phase_completed`,
`fleet_completed` and nullable `next_action`. Only `fleet_completed: true` means
the complete Fleet has converged. Infrastructure and import completion do not
imply workload completion.

`next_action.kind` distinguishes `review`, `apply`, `resume` and `review_funding`.
An `apply` or `review_funding` action requires explicit operator approval;
`resume` retains the exact already approved digest. A completed clean-reinstall
phase returns a `review` for the next phase, which produces a new approval to
inspect. The fields `executable` and `arguments` describe a direct process call
in the same workspace. Pass the array as arguments without shell splitting or
`eval`; paths and identities may contain spaces or quotes. A null executable
requires an operator decision, such as the separately reported funding review.
Ensure continuations carry the selected `--desired`, `--source`, `--seed` and
identity through review, apply, resume and successor review. Source and seed
flags are accepted with `--apply`; they do not request a new reset. If omitted
on a retained reset, their paths come from the frozen selection. An explicitly
changed publication path is refused before effects. Human `next_command` uses
the same selected arguments.
Explicit bootstrap/import completion may have no next action because selecting
the next capacity or desired document is a separate decision.

A typed successor-budget or post-creation balance pause exits nonzero with JSON
on stderr, `code: "successor_review_required"` and a `next_action` for review.
Other failures report `code: "operation_failed"` and no automatic continuation.
Wrappers should follow these decisions and explicit approvals, and preserve
unhandled failures. They should not infer completion from private journals or
use a fixed number of repeated commands. Human `next_command` and
`apply_command` renderings are for display, not process execution.

Clean reinstall selects its local owner and current input before opening the
receipt; local selection is outside the measured interval. Remote review and
apply use the shared receipt owner with `invocation_started.command: ensure`.
`applied_plan_sha256` identifies an infrastructure or Fleet execution plan;
`applied_review_sha256` identifies a pool import review. A
`clean_reinstall_authority` event identifies the phase and its current digest,
with explicit `phase_completed` and `fleet_completed` fields. Completing
infrastructure or import does not complete the Fleet. Infrastructure execution
retains the real effect count and conservation report through local publication.

Failed invocations close as failed; abrupt interruption retains incomplete
evidence. Same-digest continuation and terminal replay each create a separate
receipt. Keep all of them. A terminal execution replay reports zero effects and
issues no IC calls.

An exact completed approval returns the conservation accounting retained at
completion. Ordinary Ensure and clean reinstall share this local receipt, which
binds the plan, final journal and state, plus the reset selection when applicable.
An interrupted final journal write recovers from that receipt. Missing or altered
completion evidence rejects locally. Run a new review to observe subsequent
balance or topology changes; replay is historical completion, not a live health check.

Each line has UTC Unix milliseconds and monotonic elapsed microseconds. Existing
progress DTOs bind operation, plan and phase; stage and request identifiers link
start/end pairs and inclusive parents. An observation's `succeeded: null` is a
start, `true` is successful completion of that boundary and `false` is failure.
A successful submission is not verified remote convergence. Do not sum a parent
with its children, or concurrent request durations as critical-path wall time.
Request times include local startup/IPC and remote response/confirmation; pure
CPU, internal IC calls and remote-wait components remain unavailable.

Pool import retains direct HTTP boundaries in the same `icp_request_timing`
events: `agent_query` covers Root status/context and Coordinator registry,
`agent_update` covers signed controller handoff and Root commands, and
`agent_request_status` covers certified ingress reconciliation through
`read_state`. Root advances and signed handoffs retain the imported canister
as `subject`, distinct from the carrying endpoint. An update's duration includes
Agent response polling; it is not one raw HTTP request or an IC call count.
Capacity-limit diagnostic reads have separate spans after the failed update.

Async spans have independent request IDs and no implicit thread-local parents.
Their `in_flight` counts describe overlapping diagnostic lifetimes. Cancellation
leaves an unmatched start and marks the receipt's timing evidence incomplete.
Successful reconciliation means the existing owner obtained a validated outcome;
it does not grant another attempt or establish full-Fleet completion. Pairing
completeness does not establish coverage of every infrastructure/inventory read.

Protected read timings distinguish the transport endpoint (`request.target`,
usually the Root) from the inspected child (`request.subject`, a Principal or
`null` when unavailable). An inclusive `canister_inspection` request encloses the
fresh reserve query, protected status request and local validation. Its nested
requests inherit the child and link back through `parent_request_id`; a
`canister_history` boundary similarly attributes protected install-history reads.
These logical boundaries add no IC calls and do not count as remote attempts.
Their elapsed time includes their nested transports; do not add both, and do not
sum concurrent child durations as wall time. Failures retain the child binding;
an unmatched start remains incomplete evidence.

For example, list completed per-child inspections from an existing receipt:

```bash
jq -c 'select(.event == "icp_request_timing") | .data |
  select(.request.kind == "canister_inspection" and .request.succeeded != null) |
  {parent_span_id, request: (.request |
    {request_id, target, subject, elapsed_micros, succeeded})}' receipt.jsonl
```

The observation span ties each inspection to inventory, planning or final
authority verification. Changing or missing subjects never grants authority to
reuse an observation; all existing freshness and reserve checks still run.

Receipts distinguish confirmed increases in applied receipts or provisioning
counts from repeated polls and local activity. They retain the exact next
no-effect review command and available originating retry owner/cause. A missing
origin or runtime retry deadline stays unknown. Protected failure origins carry
`retry_at_ns` as Unix nanoseconds or `null` in progress JSON and timing receipts.
Human output shows a reported deadline as an observed UTC timestamp. It reflects
the owner's last reported schedule, not a promised completion time or a live
countdown; expired and stale observations keep their original timestamp. Deadline
changes alone do not count as remote advancement or reset the last-change age.
The final diagnostic workflow outcome
includes the plan scope and `terminal` flag; the plan/journal remain the execution
authority. An 8 MiB per-invocation cap reserves room for an outcome and an omitted
count. A partial final line, absent outcome, omitted events or diagnostic I/O
error means incomplete evidence. Interrupted files are retained; continuation
creates a new file. Keep both when reporting a deployment issue. No automatic
cross-invocation pruning is performed.

Provisioning progress includes `components`: each entry names the component spec,
deployment, placement ordinal, member path and exact Root Principal. Repeated
instances remain separate. States come from the Coordinator's exact sequential
member cursors (`reserved`, `claimed`, `installed`, `registered`,
`published`, `runtime_pending`, `active`); absent or inconsistent member evidence stays
`unknown`, even when aggregate Root counts have advanced. `current` identifies
the observed member cursor, not proof of failure. Root retry reasons and deadlines
remain separately attributed to their reported owner. The live panel prioritizes
current members and shows up to four occurrences; plain/JSON milestones and
receipts include the full list. Terminal clipping and observation age apply to
these rows. Animation adds no reads.

Fleet apply's Root status reads (authority, registry and pool observations) use
the authenticated query transport with at most three logical attempts, ten
seconds per attempt and thirty seconds total after local identity/network
resolution. Retries keep the same signer, network, target, method and argument.
Only typed transient timeouts, connection/body failures and HTTP
408/429/502/503/504 qualify. Authentication, certificate, rejection and decode
failures propagate immediately. Mutation retries remain governed by retained
intent and receipt reconciliation. Exhausted read retries leave the same reviewed
operation resumable; they do not replace its authority or reset paid-effect limits.

Human output ends with an invocation summary of completed outer observation
costs, remote attempt counts and the latest persisted effect count. Nested
observations are excluded from these aggregates; the phase costs are not an
end-to-end breakdown. Failed invocations and incomplete timing evidence are
identified explicitly. Command completion alone does not establish Fleet
convergence: the summary requires a terminal result for a full plan, otherwise it
prints the retained no-effect review command. JSON output keeps the existing
event schema. Batch reconciliation refreshes progress after persisting effects
already observed as complete, without an additional remote poll.
`root_management` also encloses final reinstall authority and retained-asset
verification, including their protected inspection/reserve pairs. Nested Root
status timings remain child observations and must not be added to that total.

Generation retains catalog progress and endpoint collection durations. Endpoint
collection includes certification; final acquisition completion includes the
upstream agreement, cache validation and publication boundary. Existing validated
cache reuse and freshness/assurance rules remain unchanged. See the
[qualification report](../../audits/reports/2026-09/2026-09-21/deployment-timing.md)
for measured costs and coverage limits.

Before compiling or qualifying a release, or attempting recovery with a newly
installed CLI, run `canic medic --ci` from the application workspace. Its locked,
offline package checks compare each resolved application Canic dependency with
the running CLI's exact version. A mismatch requires a matching CLI or a jointly
updated and requalified application; a host-only update does not bypass the
runtime contract. If Cargo evidence is unavailable, resolve that finding before
treating the preflight as passed. This check does not rebuild artifacts or prove
live deployment readiness.

Then run the early Fleet check from the workspace:

```sh
canic --environment staging fleet readiness staging --identity staging-operator \
  --operator <principal> --desired fleets/staging.toml \
  --estimated-cycles 90T --quote-conversion --json
```

This read-only command verifies the selected signer and enrolled network trust,
reads the operator's Cycles Ledger balance and reports retained Fleet work.
`--desired` additionally reads the application configuration and observes selected
Root native balances under exact Principal/controller bindings. It does not load
Wasm artifacts, open an operation lock, compile a release or create payment state.
Use `--cycles-ledger`, `--icp-ledger` and `--cmc` to select exact quote providers;
the latter two are queried only with `--quote-conversion`.

The report keeps these amounts separate:

- Operator Ledger availability, optional caller-estimated debit and shortfall.
- Each Root's native balance, configured minimum and configuration-derived startup
  floor. The effective floor is their maximum; a positive observed shortfall or
  an unfunded startup role blocks the early check. An unavailable balance is
  `null` with a typed reason, never zero or a claim of sufficiency.
- Configured execution allowance per step. The total execution reserve depends
  on artifact-bound planning and remains unknown before that work exists.
- Optional advisory ICP conversion for the caller's operator-Ledger shortfall,
  with mint amount, transfer fee, estimated deposit fee, total ICP debit and rate
  timestamp. This is not a Root native top-up quotation. Missing estimates or
  failed observations leave the amount unknown; no ICP payment is issued.

`--estimated-cycles` is a caller estimate, never selected-plan spending authority.
The JSON includes the observation start/end times, selected desired/configuration
hashes and explicit unresolved inputs, including pool/current grant usage,
artifact execution reserve, actual plan debit, operator ICP balance and fresh
admission. Configuration and retained identity changes during Root collection
reject the snapshot. Observations are sequential and have no retained freshness
lease; every fact can change immediately afterward. Success means the known early
checks passed, not that deployment is affordable or approved.

## Clean reinstall from physical inventory

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

## Generate Current Desired State

Do not hand-author the low-level Coordinator/Root/Store authority document.
After a complete `canic build`, generate it from the protected high-level Fleet
policy, the finalized release-build ID printed by that build, and an explicit
current-estate identity seed in `deployments/<fleet>.estate.toml`.

For a new Fleet, put direct recovery controllers at the top level of the
high-level Fleet policy, before any TOML table. The maintained default is
explicitly empty until the developers' Principal IDs are supplied:

```toml
recovery_controllers = []
```

Set this to the three developers' distinct, non-anonymous Principal IDs before
installing the Fleet to give each of them direct IC management authority over
the Coordinator, Roots, Stores, pool reserves, Components, descendants and
controlled treasury. An empty list adds no recovery controller. Each listed
Principal can independently reinstall code, change controllers or delete any
of those canisters; this is not threshold approval. Generation rejects
duplicates, the operator Principal and more than eight entries. The exact set
is carried through controller creation, observation and retry. This hard-cut
contract is for new installations; it does not update an existing live Fleet.

A retained estate seed has this shape:

```toml
schema_version = 1
fleet_id = "<retained-live-fleet-id>"
coordinator = "<retained-coordinator-principal>"
cycles_ledger = "um5iw-rqaaa-aaaaq-qaaba-cai"
management_creation_fee_cycles = "500B" # illustrative; use the exact fee for the creation subnet

# Optional; omit to adopt the Coordinator as treasury.
[treasury]
principal = "<retained-controlled-treasury-principal>"
subnet = "<treasury-subnet-principal>"

[[roots]]
placement_subnet = "<subnet-principal>"
root = "<retained-root-principal>"
store = "<retained-store-principal>"
pool_imports = ["<retained-root-owned-canister-principal>"]
```

`fleet_id` is the exact live Fleet identity. It is explicit retained authority,
not a value derived from the environment name or operator, so operator rotation
cannot rename the Fleet. Every paid canister controlled by each Root must be
listed. In particular, `pool_imports` must contain every retained pool asset,
including idle, claimed and workload assets; omitting one fails closed rather
than leaving its cycles outside the reviewed estate.

Generate the current document without a Fleet mutation:

```bash
canic fleet generate staging \
  --app-config apps/demo/canic.toml \
  --release-build <release-build-id>
```

The generator does not infer Principals from release metadata, project
mappings, removed install plans, or canister ancestry. Release authority
supplies exact Wasm, Candid, artifact identities and typed infrastructure init
contracts. The seed supplies retained identities and the exact fee for future
canister creation. Canic then verifies the active
operator, controller and role relationships, Registry-backed placement,
protected Root pool inventory and exact cycle balances before publishing
`fleets/<fleet>.toml`. A Root-owned Store or pool asset is resolved through the
Root's protected inventory; a retained Store controller handoff accepts only
the exact Root-plus-recovery set or Root-plus-operator-plus-recovery set before
installation.
The live Root's identity authority and installed policy must match the current
configuration exactly; policy drift fails closed. A seeded
pool identity remains in the conservation set as it moves from idle bootstrap
capacity through claimed state to a Component workload, without receiving pool
minimum top-ups or being counted twice.

Mainnet generation reports catalog acquisition progress on stderr, including the
active endpoints, elapsed time and completed endpoint collections, with a heartbeat
every ten seconds. Both endpoints collect concurrently. Heartbeats include each
endpoint's latest Registry pin, history watermark, record read or retry and query
attempt count. Acquisition has a shared ten-minute deadline. A timeout stops
collection, preserves the previous cache and releases the refresh lock so
generation can be retried. Completed endpoint collection is not yet validated
agreement; generation proceeds only after the final validated result.

Host callers may retain `MainnetCatalogClient` across attempts to reuse upstream
validated history prefixes in memory. Each acquisition still checks current
snapshot agreement and freshness. A new CLI process starts without those prefixes.

Mainnet generation accepts catalogs at most one hour old. Missing, invalid,
lower-assurance or expired caches are refreshed by comparing Registry version
and canonical payload through `https://ic0.app` and `https://icp-api.io`.
Both endpoint results must agree; an outage or disagreement stops generation
and leaves the previous cache intact. There is no single-endpoint fallback.
This is endpoint agreement, not certification or a guarantee of independent
providers. Cached evidence must meet at least the same assurance level; its
actual contributing endpoints are shown in the report.

The `subnet_catalog` generation summary reports cache disposition/path,
collection time, observation time, age/maximum age, Registry version, catalog
digest, assurance and source endpoints. Local generation reports that the
catalog is not applicable. These observations accompany the generated result;
they are not fields of `fleets/<fleet>.toml` and do not alter its review digest.
Root-management and reinstall observations use the validated cache without
refreshing it. They do not impose the new-generation age limit or silently
replace reviewed placement; a missing or insufficient-assurance cache fails
closed. Rerun generation to acquire acceptable evidence, then review any changed
desired state before proceeding.

A retained Root that cannot serve the current protected endpoint requires a
management observation before any protected query. For a stopped Root,
`root_start_prerequisite` seals the exact Principal, Subnet, controller set and
installed module. Start planning, execution and replay do
not require a successor artifact or application release manifest. The issued
plan retains its exact observed authority and permits only the transition from
Stopped to Running with that module. Reinstall is a separate, explicit reviewed
effect: it discards application state while retaining exact cycle and asset
control. Canister identities and their cycle accounts may remain in place.
When the installed Root module differs, generation returns the replacement
input without calling the old Root's protected interface. Ensure planning
produces `root_reinstall_prerequisite`, with exact installed module, Principal,
Subnet and controller bindings in `root_reinstall_bindings`. Review and apply
that plan to stop the Root, reinstall its sealed current initializer and start
it again. The same journal records intent and the pre-install canister version;
a lost response resumes observation instead of repeating an already completed
reset.

Each Root must hold its own conservative stop-and-observation allowance. If it
cannot cover that first effect, the typed headroom error names the Root,
Principal, available balance, required allowance, shortfall and effect count.
Another Root's balance cannot cover that deficit without a transfer.

When a Root can stop but lacks the complete reset allowance, its reviewed action
sequence is Stop → Fund → Reinstall → Start. The Fund action includes the deficit
and an additional effect margin; the plan discloses its exact amount, recipient,
Ledger, withdrawal timestamp, fee and maximum operator debit. Apply verifies the
operator account and fee before stopping any Root. Payment requires exact source
module, controllers, Principal and subnet with the Root observed Stopped. A
restarted source, changed fee or exhausted margin blocks payment. Same-operation
retries keep the original withdrawal identity and recover its Ledger receipt;
they do not start the old runtime to fund it. Fully funded Roots retain the
three-effect sequence. Review arithmetic and the retained pre-payment balance
must permit receipt reconciliation before withdrawal; unexpected credit cannot
silently enlarge the approved payment. A separately reviewed source-bound
activation reset retains its own funding and fee budgets. Its preceding
preparation remains unfunded and requires its existing native headroom.

These allowances are neither predicted burn nor complete successor deployment
quotes. Planning does not pause the runtime; the applied Stop protects the
reviewed credit from old-runtime child grants. Outside native top-ups are accepted
as balance observations, without an invented payment receipt or larger allowance.
Terminal verification retains the completed funding evidence and checks net
accounting after the new runtime starts.

Recovery review also reports configuration-bound startup funding for each Root
before a reinstall prerequisite. The required native balance is the greater of
the configured minimum and startup minimum plus continuation allowance. The
allowance uses the Root's bounded continuation steps multiplied by one update
burn bound plus three observation burn bounds. Ordinary startup prepayment uses
the same calculation. The review exposes the Fleet successor and fixture-retry
counts separately; Root allowances overlap that ceiling and must not be added
to it again.

The forecast assumes fresh children and full publication, with no runtime reuse
credit. It identifies any role lacking cycle policy. It does not include all
install/funding margins, fees or dependent top-ups, freeze balances, pause child
grants, or authorize successor funding. Prerequisite funding covers only the
reviewed reset effects; subsequent funding requires its own fresh review.

Root prerequisite activation authority remains bound to its desired input until
the dependent Fleet finishes. A changed input after that prerequisite is refused
before Store effects. Reusing a Root module alone does not prove that its
activation operation matches a new Store install.

Single-Root partial-activation recovery is qualified with current artifacts.
Preserve the source plan, journal, paid
receipts, artifacts and complete estate seed. An explicit `--reinstall` review
inspects the issued source prefix without executing the old plan. It requires
exact source modules/controllers, complete physical assets, unchanged Root
Ledger balances, bounded prior debit and a corrected Root module. Preview keeps
the active source files intact; applying its digest archives them before
adopting the new journal. The preparation stops the Coordinator, stops and
restarts the unchanged Root, then checks fresh inventory. A separate review
reinstalls the Root; the next Full Ensure review resets remaining infrastructure
and completes readiness. The Coordinator remains stopped between preparation
and Full Ensure. Multi-Root partial activation is not admitted by this bounded
path. Converged-Fleet selected-build reinstall remains separate.
The installed-release proof covers controller-drift rejection, lost install
response recovery, retained assets, conservation and effect-free replay. Exact
evidence and the separate live-adoption boundary are in the
[activation feedback report](../../audits/reports/2026-09/2026-09-08/activation-feedback.md).

For ordinary startup of a completed Fleet on the same build, reuse its retained
current desired document with `fleet ensure <fleet> --desired <path>`, without
`--reinstall` or a new `fleet generate`. Ensure resolves symbolic names in that
already reviewed desired document through the existing Fleet state. Conflicting
Principals reject; observed identity, Subnet, controller and module checks still
apply. This does not permit reusing the original symbolic fresh seed for new
generation after completion. New generation selects clean reinstall and needs
explicit physical inventory as described above. A changed release always requires
reviewed reinstall.

A completed prerequisite is not a ready Fleet. Run Ensure planning again and
review its full plan for the remaining infrastructure and current protocol
convergence. Repeat `canic fleet ensure <fleet> --desired <path>` to plan, then
apply the reviewed digest with `--apply <plan_sha256>` using the same environment
and identity. JSON reports expose `plan.scope`; text reports expose `plan_scope`.
`terminal: true` means completion of that scope. Fleet readiness requires a
terminal `full` plan; `prerequisite_complete` is an intermediate result.
The reset request is not a persistent desired-state flag. A
completed Fleet therefore plans no additional reinstall. The
[changed-release PocketIC journey](../../audits/reports/2026-09/2026-09-05/fleet-reinstall-journey.md)
qualifies generation, reviewed reset, lost-response recovery, working Fleet
reconstruction, conservation and both effect-free replay paths.

PendingReset and Failed pool assets use the Root's reconciliation funding
authority. They do not receive ordinary Ready-pool top-ups. If a completed
infrastructure reset discovers funding outside its reviewed authority, Ensure
pauses for another review. Retrying the paused reset preserves its plan and
completed installation receipts and reports that a review is required. Run
ordinary Ensure planning with the same selected input to review the additional
effects under the same operation; apply the new digest. The completed installs
are verified through their retained action hashes, canister versions and live
authority, and are not repeated. Replay of that new funding plan uses its own
reviewed input and digest.

Whole-Fleet evacuation applies only to deletion and is an optional follow-up to
this reinstall correction. Existing Root deletion returns native cycles and
transfers its Ledger balance with an exact receipt; the Coordinator can then
transfer its own Ledger balance to the reviewed operator. That Ledger receipt
alone does not authorize deleting a Coordinator with native cycles remaining.
Complete Coordinator native evacuation/deletion remains outside the qualified
flow. Explicit reinstall requires no current endpoints on old code or temporary
recovery artifact. The Root-start
prerequisite itself authorizes no reinstall. Current source changes do not add
endpoints to an installed release; protected queries require current authority.

Retained-estate treasury policy requires an explicit identity: it must
name an already-present, non-replaceable controlled canister. Omitting
`treasury` selects the exact seeded Coordinator. Canic does not silently invent
or globally search for a retained identity. Missing, foreign, duplicate,
unseeded or conflicting identities and unexpected co-controllers fail closed.
If an exact retained identity is no longer observable, planning rejects instead
of creating a substitute. The generator queries the configured Cycles Ledger's
current fee and binds it into the desired document. Every seed must explicitly
declare `management_creation_fee_cycles` in compact `B`, `T` or `Q` units for
the reviewed creation subnet, including retained estates that need more capacity.
There is no implicit zero: use `0B` only when it is the exact applicable fee.
The fee applies to future creations, not already-paid retained assets. Planning
adds the readiness floor, execution margin and management fee per new asset,
then accounts for the separate Ledger fee. A changed fee changes the generated
authority and requires reviewing the new plan; do not edit generated desired
state or compensate with an unreviewed Ledger credit.
The scalar fee currently supports creation on only one exact subnet per
operation. `MixedSubnetCreationFees` rejects a fresh mixed-subnet topology,
mixed-Root growth, or direct creation combined with Root growth on another
subnet before creation or funding effects. It checks pending Root creations too
and applies to retained plans. Reuse, reinstall and fully supplied Roots on
other subnets do not consume a creation fee and do not trigger this limitation.
The host cannot infer that two different subnets charge the same amount and
does not substitute a global maximum. Preserve retained evidence on rejection;
do not split an issued operation or erase its journal to bypass the guard.
Observation and update burn values are distinct conservative
ceilings checked against measured terminal conservation, not assumed fees.
On IC mainnet, every Fiduciary placement must carry an exact
`acknowledge_fiduciary_cost = true`; non-Fiduciary placements must not claim
that acknowledgement.

The complete build is network-bound. Select the same named environment that
the generated Fleet will use:

```bash
canic build <app> --environment staging --profile release
```

The finalized release-build and release-set manifests retain `local` or `ic`
as immutable authority. Generation rejects a local-network infrastructure set
for an IC environment, and rejects an IC set for a local environment, before
publishing desired state. Reusing artifact hashes alone cannot bypass that
network check.

Generated output is content-exact. Repeating generation with the same bytes
succeeds without rewriting the file. A changed document is never overwritten
implicitly; replace it only by supplying the SHA-256 of the file already on
disk:

```bash
canic fleet generate staging \
  --app-config apps/demo/canic.toml \
  --release-build <release-build-id> \
  --replace <current-fleets-staging-toml-sha256>
```

The guarded replacement rejects a missing output, a changed current digest, or
an invalid digest before writing. Publication uses the same atomic durable-file
boundary as current Fleet operator state.

### Bootstrap A Literally Empty Estate

When no estate seed or live canister exists, explicitly create a fresh seed and
generate the same current desired-state contract:

```bash
canic fleet generate staging \
  --app-config apps/demo/canic.toml \
  --release-build <release-build-id> \
  --fresh \
  --management-creation-fee-cycles 500B
```

`--fresh` durably creates `deployments/<fleet>.estate.toml` before generating
the desired document. The seed contains a cryptographically generated Fleet
ID, the exact Cycles Ledger, exact management creation fee and logical
Coordinator/Root/Store/pool roles. Repeating the command reuses those exact
bytes; a changed fee, Ledger or topology rejects rather than replacing the
seed. Use `--cycles-ledger <principal>` only when the selected network does not
use the default Cycles Ledger Principal.

Generation still performs no paid effect. The resulting ordinary `fleet
ensure` plan shows every new Principal as unallocated and includes the exact
maximum operator debit, Ledger fees, management creation fees, funding and
burn. Only `fleet ensure --apply <plan_sha256>` may create canisters. Each
creation intent is durable before the Cycles Ledger call; a duplicate response
recovers the same Principal, and later role creation, typed initialization and
protocol work resolve only those retained identities. The Coordinator is the
logical treasury for the fresh operation. Each Root receives enough host-created
pool assets for every initial top-level Component and recursive initial child,
plus its independent `canister_pool.minimum_size` Ready reserve. Generation
rejects a total above `canister_pool.maximum_size` before any effect. Retained
`imports` are forbidden in a fresh seed. Local readiness observes this supply;
autonomous Root creation remains restricted to IC-mainnet builds.

A fresh plan binds the App config, complete artifact union and role Candid
identities and reserves a finite successor-action and execution-burn budget.
Apply installs infrastructure, reconciles imported assets, and advances the
resulting canonical control-plane and provisioning actions under that original
reviewed plan. Each successor plan is retained by digest before its first
effect intent. Response loss and process restart resume the same journal and
cumulative conservation baseline. Immediate replay of the completed original
plan has no effect.

A changed input, additional effect or debit, exhausted phase limit, or
insufficient remaining burn budget returns a typed new-review requirement.
Review the resulting current plan before authorizing that additional work.
Progress events distinguish advancing, awaiting progress, prerequisite
completion, funding required, new review required and complete convergence.
Waiting events report elapsed seconds for the current effect or terminal check
within this invocation, measured with a monotonic clock. The timer includes
issuing and observing that effect, continues across provisioning stages, and
starts anew when an invocation resumes it; it is not the operation's durable age.
When the existing Coordinator observation is available, provisioning detail names
the phase and accepted/provisioned, directory and runtime Root counts, plus the
Component count. For example, `ActivatingRuntimes` with `runtime Roots 0/1`
identifies the pending activation while the reviewed effect count stays unchanged.
JSON carries `state.elapsed_seconds` and nullable `state.provisioning` on
`awaiting_progress` events; provisioning phases retain Coordinator enum names.
These bounded informational fields add no polls, do not publish the internal
progress identity, and cannot replace fresh funding or completion evidence.
Interactive stderr uses a dedicated screen with named work, a Root stage table,
reviewed-effect accounting and the age of the latest progress observation and
transition. A local spinner indicates an observed wait; at thirty seconds without
a new progress event it becomes a stale-observation marker. Neither animation nor
an all-applied counter proves backend health, time remaining or final success.
The displayed effect wait is the host's last reported duration, not a fabricated
live measurement. No extra IC reads or changes to polling, retries or effects
are made for display.

The host's nullable `next_action` projects the first unfinished reviewed action
from its journal position, with a typed kind and bounded target name. It is not
proof that remote execution has started. Provisioning's nullable
`pending_root_failure` preserves an already observed retry diagnostic. Individual
Component readiness and per-stage durations remain unavailable. Root counts are
never presented as Component counts or a whole-deployment percentage.

Redirected stderr, limited terminals and `NO_COLOR` use plain milestone output
with thirty-second local heartbeats, including explicit observation age when a
remote call stops returning new events. Advancing counters alone do not print a line per
effect; phase, authority, denominator and provisioning changes remain visible.
The interactive screen keeps prerequisite transitions, placement collection and
observation failures in place. It does not insert scrolling log lines between
updates or switch to milestone logs when resized. Narrow/short windows display a
bounded portion of the same screen. Reviewed effects are a count, without a
deployment percentage bar. Completion restores the ordinary terminal before the
receipt summary, final report or recovery error is printed; the repaint clock
cannot overwrite that output. Ctrl-C, SIGTERM and panic restore the terminal
before retaining their normal termination/error behavior. Raw input mode and
cursor hiding are not used.

Successful per-query timings and cache/identity counts are available through the
existing explicit `--json` mode. Plain logs print one inclusive planning summary;
the interactive screen retains detailed timing in the receipt and displays failed
observations in place. JSON emits every typed host event on stderr and
the complete final report on stdout, without animation or milestone suppression.
Receipt write failures stay inside the interactive screen and mark final timing
evidence partial; they do not interrupt the display with a separate log line.
The schema-1 receipt's `invocation_finished.data.timing_evidence_complete` is
false when events were omitted, a timing start has no completion, or timing
events contain duplicate starts or unmatched completions. A failed request with
a retained completion still supplies complete
timing evidence. The human timing summary uses the same check. A missing final
record, truncated JSON line or receipt write failure also means the evidence is
incomplete. This diagnostic completeness flag does not establish Fleet
convergence, cycle conservation or a measured performance improvement. Compare
timings only with matched inputs and conditions; nested and concurrent request
durations cannot be summed into elapsed deployment time.
Errors after ensure-option parsing use the `fleet_ensure_error` JSON event;
invalid CLI syntax still follows the ordinary argument-parser error path.
For detailed evidence, select `--json` on the intended invocation and redirect
stdout and stderr to separate files; Canic does not create an implicit log file.
Full successful apply prints a concise outcome and operation/plan identities;
prerequisite and review reports retain their detailed output. Dated
[reinstall evidence](../../audits/reports/2026-09/2026-09-05/fleet-reinstall-journey.md)
and the [combined paid-growth proof](../../audits/reports/2026-09/2026-09-07/canic-140-retained-creation-fee.md)
record the completed focused qualification and its downstream acceptance limits.

The management creation fee is explicit because it is network/Subnet economic
authority and cannot be inferred from release metadata. Zero is appropriate
only where the selected local platform actually charges zero. A wrong value
cannot silently change the reviewed debit or conservation equation.

Initial pool assets are direct Fleet Ensure creation actions. The configured
`canister_pool.canister_cycles` value is their readiness floor, not their fresh
creation amount. Fresh generation adds 1T for Create execution, 1T for its first
observation and 1T for controller finalization. These are bounded reservations,
not assumed fees or measured per-call costs. Retained assets keep the floor as
their target. Creation funding, one exact Cycles Ledger creation fee and one
exact management creation fee are included in the reviewed maximum operator
debit before any effect. Fresh convergence does not fund a Root's default
Ledger account implicitly and does not let Root pool maintenance discover an
unreviewed payer. When the selected current protocol will provision Components,
the plan instead exposes one separate funding domain for every Root. The
forecast includes top-level Components, recursive configured initial children
and the post-provision Ready-pool floor, less eligible retained Ready assets and
already-completed workloads. Each row reports the raw Ledger balance, Root and
Ledger identities, creation count and amount, Ledger and management fees,
maximum plan-owned funding and exact shortfall independently from ordinary
managed-canister funding.

When the observed account is short, the reviewed plan contains one exact
`FundEstate` action before the first Fleet protocol effect. Canic persists the
transfer intent before debit, uses one stable Cycles Ledger duplicate identity,
adopts an exact duplicate receipt after a lost response and re-observes both
the source debit and Root-account credit. Apply then queries the account again
before protocol work. An unexpected remaining deficit persists a typed
`EstateFundingRequired` pause and issues no new protocol command. Repeat the same
`canic fleet ensure` command without `--apply`. Its report retains the original
plan and adds `funding_review`: review the exact Root, Ledger, shortfall, fee and
pending creation identity, then repeat the command with
`--apply <funding_review.review_sha256>`. Text output labels that digest
`funding_review_sha256`. The original plan digest alone cannot approve another
debit. Do not credit the account outside Canic or discard the journal.

The additional transfer uses the existing Ledger adapter and remains inside the
same operation. Once its intent is persisted, interruption recovery retains its
timestamp, account, amount and duplicate receipt, including when the Root has
already consumed the credit for its pending creation. Original protocol effects
and creation receipts remain authoritative. Planning performs no remote mutation.
Changed account or fee authority, uncertain pending creation, unexplained Ledger
loss or funding above the reviewed bounds fails before a new debit. Terminal
verification still reconciles every credit and exact creation debit; the funding
review does not increase creation limits or excuse an unexplained balance change.

### Operator ICP conversion

Fresh apply checks the complete reviewed operator debit before retaining a new
execution journal. When it reports a shortfall, `canic fleet ensure <fleet>
--operator-mint` reads the selected plan's operator balance, CMC rate and Ledger
fees and reports an advisory ICP amount. It creates no journal or payment.
This requires a selected plan; a staged reinstall review takes precedence over
its predecessor without changing source evidence. It is not a readiness check
before compilation.

For a retained original withdrawal or supplementary funding pause, the same
option retains or displays an exact conversion review. The report binds the
operator, network root, ICP Ledger, CMC, Cycles Ledger, amount, transfer fee and
fixed timestamp. `--mint-icp-ledger` and `--mint-cmc` override their canonical
canister identities when a different selected network requires them. Review
those identities and the maximum ICP debit before applying.

Apply the conversion with `--operator-mint --apply <operator_mint_review_sha256>`.
This authorizes one ICP transfer and its CMC notification, not Fleet withdrawals.
Retries use the identical transfer and notification arguments. Canic authenticates
the ICP transaction and the memo/account-bound Cycles Ledger deposit, then records
gross cycles, actual deposit fee and net credit exactly once. Original Fleet
starting balances, original operation authority and withdrawal identity remain unchanged.

After `credit_admitted: true`, repeat ensure without `--operator-mint`, using
the original plan digest for the original retained withdrawal, or the separately
reviewed funding digest for supplementary funding. The conversion does not
approve an additional Fleet debit. Completed conversion replay is effect-free.
Use `--operator-mint --cancel-mint <operator_mint_review_sha256>` only to cancel
an unapproved review; the original Fleet operation remains retained.

Quotes round up using the observed rate and estimated deposit fee, with the ICP
transfer fee reported separately. A changed rate never authorizes a second
payment. Processing, refunds without complete accounting, expired identities,
and unavailable or over-budget receipt history remain unresolved. Do not reset
the journal, change timestamps, or independently mint into a retained operation.
An Intent without a receipt is not proof that its original withdrawal did not pay.

Completed Fleet records are historical evidence. Clean reinstall uses the
selected current build, explicit inventory and observed controllers, without
requiring old plan schemas, receipt decoding or source interface reconstruction.
Unfinished paid effects retain their reconciliation owner and exact bounds.

For a retained original withdrawal, resume ordinary Ensure without `--reinstall`
to obtain its funding review, then follow the conversion and original-plan apply
sequence above. The original desired input, operation authority and withdrawal
identity remain bound throughout recovery. A new reinstall request during that
operation returns `RetainedOperationRecoveryRequired`, naming the original
operation and plan digest. After original convergence and effect-free replay,
request a separate `--reinstall` review selecting the current build. Existing
receipt, retry, authority and conservation checks determine whether the
withdrawal paid and which recovery effects remain admissible.

Every autonomous pool creation retains its exact Ledger block, operation,
amount, Ledger fee, management creation fee, readiness floor, execution margin
and first observed native balance. Terminal conservation sums those receipts,
requires unique operation and block identities, and rejects a missing,
below-floor or impossible first observation. Later reinspection cannot replace
the first balance. The plan-time forecast uses the complete bounded protected
Root pool inventory, including dynamically created assets in every lifecycle
and any durable pending creation. When retained Failed or PendingReset assets
can satisfy the reserve, planning reviews their exact native funding and Root
reconciliation before forecasting additional creation. Apply repairs only
those reviewed identities; a subsequent protected inventory must establish
their readiness. The focused proof fills the pool with two Workloads and two
Failed reserve assets, then repairs them without funding the Root account or
creating another canister.
If the protected inventory cannot satisfy the required capacity through those
repairs, planning returns the typed capacity failure before funding.

Planning rejects a fresh pool whose creation amount is below its readiness
floor plus those margins. The typed failure reports the requested creation
funding, floor, admissible burn, required funding and exact shortfall. After a
Create is applied, its first exact balance must still cover the readiness floor
plus the remaining controller-finalization margin; otherwise resume stops
before any controller or protocol effect and reports both readiness and
pre-finalization shortfalls.



## Supplied Infrastructure Bootstrap

`fleet bootstrap` initializes a new, untracked current-release Fleet using explicit
physical IDs. It supports an empty Root and a Store with disposable old code.
For a tracked completed estate, use the completed-estate reinstall procedure
above, which requires valid current-schema records and receipts. Bootstrap
refuses an existing tracked operation. Neither path supplies missing fields or
decodes a historical contract; see [unreadable retained plan](#unreadable-retained-plan)
when retained evidence cannot be read. Application state is disposable.

Start with the normal Fleet policy and a retained identity seed (`fresh_estate =
false`). Supply every Root, Store and held pool ID. Each pool must be on its Root's
subnet. The Coordinator may be on a different explicitly configured subnet. Keep
capacity policy large enough for the selected workloads and Ready reserve.

Choose the Coordinator explicitly:

| Selection | Seed Coordinator | Reviewed effect |
| --- | --- | --- |
| `--coordinator initialize` | Exact supplied Principal | Wipe and initialize current Coordinator authority |
| `--coordinator ready` | Exact supplied Principal | Verify current genesis authority before dependent effects |
| `--coordinator create` | Literal `"create"` | Create exactly one Coordinator with reviewed placement, funding and debit |

Canic does not guess an unknown Coordinator. Inspect operator-controlled estate
configuration and its live controller/Registry evidence to identify a candidate.
An arbitrary running Coordinator cannot satisfy `ready`; it must match the exact
current Fleet genesis contract. If no suitable Coordinator exists, select `create`.
After creation, durable publication replaces only that seed slot with the
receipted Principal. Unsuitable supplied IDs reject without replacements.

Use the declaration schema in [Capacity Import](#add-supplied-capacity-to-a-current-fleet). The bootstrap
declaration must cover every supplied infrastructure and pool ID; omit a
Coordinator selected for creation. Stop installed pool sources and remove their
snapshots before the initial review. Bootstrap journals infrastructure stops,
wipes, installs, starts and controller handoffs itself. Declare disposable state,
no outside obligations and no ownership by another Fleet explicitly.

With current CLI/runtime artifacts finalized, the supplied-ID staging flow is:

```sh
canic --environment staging fleet bootstrap toko-staging \
  --identity toko-miner-mainnet \
  --app-config apps/toko_miner/canic.toml \
  --release-build '<CURRENT_RELEASE_BUILD_ID>' \
  --coordinator initialize \
  --declarations deployments/bootstrap-sources.toml \
  --source deployments/toko-staging.toml \
  --seed deployments/toko-staging.estate.toml --json

canic --environment staging fleet bootstrap toko-staging \
  --identity toko-miner-mainnet --apply '<BOOTSTRAP_PLAN_SHA256>' --json
```

Review lists each original subnet, controllers, code, native/reserved balance and
operator disposition, plus exact effects, funding and maximum debit. It preserves
initial survey samples and spent inspection allowances. A failed review retains
its original timestamp, so retrying can use the remaining inspection allowance
without changing the plan identity. Apply uses retained
current typed authority, establishes the Coordinator before Root/Store effects,
registers infrastructure and publishes the estate seed. Pool IDs remain held;
workloads do not run at this point. Repeat the exact apply digest after an
interruption, including an interrupted local seed publication. Do not edit
journals or regenerate a replacement operation. Completed bootstrap replay reads
its local receipt before resolving ICP, even after ordinary Ensure has begun.

If initialization effects are all applied but registration cannot fit its balance
or approved execution allowance, preserve the operation directory and selected
release build. A top-up does not enlarge the original plan's execution budget.
Review a supplementary registration allowance within that same operation:

```sh
canic --environment staging fleet bootstrap toko-staging \
  --recover '<ORIGINAL_PLAN_SHA256>' --json
```

Use the original operator identity. The review reports the exact Store/Registry
actions, remaining registration work,
additional inspection allowance, target-specific Ledger deposits and fees. It
preserves original cycle baselines and applied receipts. Check these amounts and
run the returned command, which repeats `--recover <ORIGINAL_PLAN_SHA256>` and
adds `--approve-recovery <REVIEW_SHA256>`. Approval rechecks installed code,
controllers, identities, held pool bindings and the selected release inputs.
Existing operator Ledger funds must cover the reviewed deposits and fees.

The ordinary effect journal executes and reconciles those deposits, then resumes
registration. Retry the same approved command after interruption; deposits and
initialization effects are not repeated. Review and approval observations each
have two retained attempts, and approval adds two registration and terminal
inspection rounds without clearing previously consumed attempts. This recovery
is available after initialization and before a registration successor has begun;
it does not authorize replacement artifacts or pool import. Import the held pool
canisters only after bootstrap reports `completed: true`.

Next import **every held ID for one Root together**, using pool-only declarations
with the original reviewed bindings. Repeat separately for each Root. For the
Root supplied in the staging request, the command shape is:

```sh
canic --environment staging fleet import toko-staging \
  --identity toko-miner-mainnet \
  --root 5lnwm-ziaaa-aaaae-agtqa-cai \
  --canister '<POOL_ID_1>' --canister '<POOL_ID_2>' \
  --canister '<POOL_ID_3>' --canister '<POOL_ID_4>' \
  --canister '<POOL_ID_5>' --canister '<POOL_ID_6>' \
  --canister '<POOL_ID_7>' --canister '<POOL_ID_8>' \
  --declarations deployments/bootstrap-pools.toml \
  --source deployments/toko-staging.toml \
  --seed deployments/toko-staging.estate.toml \
  --maximum-source-debit 1T --maximum-root-debit 4T \
  --maximum-root-paid-calls 72 --json

canic --environment staging fleet import toko-staging \
  --identity toko-miner-mainnet --apply '<IMPORT_REVIEW_SHA256>' --json
```

These sample debit limits are review inputs, not funding estimates; select bounds
that cover the actual eight-canister operation. Initial import preserves the
bootstrap survey's original source balances and the setup receipt's Root balance.
It clears capacity through Root, publishes both generator inputs without duplicating
seeded IDs, and releases the allocation hold only after publication completes.

Finally generate and review ordinary workload convergence from the same current
configuration and release build:

```sh
canic --environment staging fleet generate toko-staging \
  --identity toko-miner-mainnet \
  --app-config apps/toko_miner/canic.toml \
  --release-build '<CURRENT_RELEASE_BUILD_ID>' \
  --source deployments/toko-staging.toml \
  --seed deployments/toko-staging.estate.toml \
  --output fleets/toko-staging.toml

canic --environment staging fleet ensure toko-staging \
  --identity toko-miner-mainnet --desired fleets/toko-staging.toml --json

canic --environment staging fleet ensure toko-staging \
  --identity toko-miner-mainnet --apply '<ENSURE_PLAN_SHA256>' --json
```

The first workload plan requires completed setup and every exact held import.
It archives original setup evidence unchanged and continues its operation identity
through provisioning. It does not allocate replacement infrastructure. Completed
bootstrap/import receipts remain available for local replay. Ordinary Ensure
keeps its existing live convergence verification. Retain `.canic` receipts,
referenced immutable phase/content files and the published policy/seed together.


## Add supplied capacity to a current Fleet

`fleet import` reviews and clears explicitly supplied canisters after a completed
current-release Fleet or receipted infrastructure bootstrap setup. The Coordinator,
Root and Store must already be initialized with current typed authority. Sources must be on the destination
Root's subnet; the Coordinator may be on another subnet. This command does not
bootstrap an empty Root or change an earlier release into a current Fleet. Finish
the completed-estate hard cut above before importing additional capacity into an
old completed estate. Use `fleet bootstrap` above for the empty-Root starting point.

Retain the current build's infrastructure manifest and Candid sidecars. The review
checks the completed Fleet or setup plan/journal, exact live installed modules,
Registry, controllers and complete pool membership. An absent or unsuitable supplied ID
rejects; the command never allocates a replacement. Each source must be controlled
by the selected operator or already by the destination Root. Operator-held
installed sources must be stopped before review. Root-held sources may be running:
the reviewed Root operation journals and reconciles a stop, normalizes controllers,
then clears their code and state. It retains the observed stopped version for the
later effects instead of assuming a fixed version throughout application activity.
It does not invent an operator handoff receipt.
Remove snapshots before reviewing import. Clearing code and application state is
destructive.

Create `deployments/capacity-import.toml` with the original physical identities and
an explicit disposition for every `--canister`. All fields below are required;
repeat `[[canisters]]` for additional sources. Use `module_sha256 = "empty"`
only for a canister with no module. `retired` is the other disposition value.
Neither value is an application-state import or migration.

For a running Root-controlled source, `canister_version` is an observed lower
bound: application activity may advance it before the stop. Subnet, controllers,
module and snapshot checks remain exact. After stopping, Root retains the actual
version and requires exact subsequent controller and uninstall history.

```toml
schema_version = 1
operator = "<operator-principal>"
network_root_key_sha256 = "<sha256-of-selected-network-DER-root-key>"

[[canisters]]
canister = "<supplied-canister-principal>"
subnet = "<observed-subnet-principal>"
controllers = ["<operator-principal>", "<previous-controller-principal>"]
module_sha256 = "<observed-module-sha256>"
canister_version = 7 # Replace with the actual observed version.
disposition = "absence"
no_external_obligations = true
no_other_fleet_ownership = true
evidence = "<basis for asserting no remaining obligations or foreign Fleet ownership>"
```

The operator assertions cover outside debts and ownership that IC custody queries
cannot establish. Canic independently verifies the declared physical binding and
absence from this Fleet's infrastructure and complete pool inventory. Unknown or
incomplete declarations reject.

For Gabriel, after the Toko hard cut has completed, an additional-capacity review
has this form. Substitute exact IDs and choose debit limits appropriate to the
observed balances; these example limits are not funding estimates or payments.

```bash
canic --environment staging fleet import toko-miner-staging-001 \
  --identity toko-miner-mainnet \
  --canister <first-supplied-id> --canister <second-supplied-id> \
  --root <destination-root-id> \
  --declarations deployments/capacity-import.toml \
  --source deployments/toko-miner-staging-001.toml \
  --seed deployments/toko-miner-staging-001.estate.toml \
  --maximum-source-debit 1T --maximum-root-debit 4T \
  --maximum-root-paid-calls 72

canic --environment staging fleet import toko-miner-staging-001 \
  --identity toko-miner-mainnet --apply <review-sha256>
```

Without `--root`, selection requires one unique registered Root on the verified
source subnet. Without `--source` or `--seed`, paths default to
`deployments/<fleet>.toml` and `deployments/<fleet>.estate.toml`. The command prints
an exact apply command retaining the environment, identity and ICP executable.
`--json` exposes the complete journal/review and the same next command.

Review shows original, transitional and final controllers; code/state clearing;
separate source and Root balances, floors and maximum debits; paid-call limits;
and the exact before/after hashes of both generator inputs. Operator payment is
zero and the import does not transfer funding. Initial review may issue at most
two management status requests per ID, persisted before each request. A successful
sample remains the original balance baseline on retry. Source balances must retain
the Ready floor plus their debit allowance; Root must retain its threshold plus
its separate allowance.

If a retained sample lacks that headroom, fund the exact source or Root first,
then repeat the unapproved review with `--funding-credit '<CANISTER_ID>=1T'`
(substitute the exact total added amount). Repeat the option for each funded ID.
This option records a received credit; it does not send a payment. Keep the
original declarations, paths, source set and debit limits. Review the new digest
before applying it. An ordinary non-bootstrap source needs its original successful
review observation on disk; bootstrap-held operator sources use their original
bootstrap sample and Root uses its terminal setup sample.

The supplementary `funding_credits` record binds the original sample, its retained
owner, the declared amount and a fresh bounded observation. The effective balance
is original native cycles plus that amount; reserved cycles stay non-liquid, and
all consumption since the original observation still counts against the original
debit ceiling. Custody must still match. A new observation does not replace the
bootstrap plan, journal or prior survey. Uncredited reviews include an empty
`funding_credits` array in their exact current-contract digest. A credit requires a new import
approval and cannot alter an approved unfinished import. Completed replay remains
effect-free.

Before handoff, Host and Root require at least `13 × source_count + 1` paid calls
and a debit ceiling covering every allowed call at Root's largest current import
quote. The quote is effect-free; it is a conservative allowance, not an expected
payment. Clean reinstall derives `26 × source_count + 16` calls and the matching
cycle allowance automatically. Explicit import reviews report the required bound
when their supplied limits are insufficient. Successful callbacks settle unused
call allowance; failed or unresolved callbacks retain theirs and retries never
reset consumed call counts. A capacity-limit error reports the protected phase,
reserved and observed debits, and paid-call bounds; extra Root funding does not
increase an existing reviewed allowance.

Stop and controller confirmation can issue the next mutation from the same fresh
status sample, with no await between validation and durable intent. Running
sources take four advances; each advance still refreshes mainnet placement and
issued controller/uninstall effects still require exact history evidence. Eight
running sources require 105 calls before retries; the recommended allowance is
224. Already issued allowances never change to match new workflow estimates.

If a retained Host survey, submission or inspection allowance is exhausted, review
an effect-free continuation with:

```sh
canic --environment staging fleet recover-attempts staging --json
canic --environment staging fleet recover-attempts staging --apply <review-sha256> --json
```

The review lists exact owner files, operation bindings, spent counters and proposed
ceilings. Approval adds two attempts to each reviewed exhausted resource under
the Fleet lock. It preserves consumed attempts, original approvals, signed ingress,
cycle baselines and Root paid-call/debit ceilings. Certified retired handoff
envelopes have a separate reviewed ceiling. Changed owner files refuse before any
grant; interrupted local publication resumes exact grants once, and replay adds
nothing. Repeat the original interrupted command after approval. Further exhaustion
requires a fresh explicit review; recovery performs no IC calls and cannot prove
an uncertain paid effect completed. Exhausted Root authority requires the governed
cycle-safe reset path, rather than a Host continuation.

The policy and seed paths are publication outputs. Use mutable operator copies
when release inputs must stay frozen. A semantic no-op preserves exact seed bytes,
including comments and formatting; resolved physical-ID changes publish the
reviewed replacement bytes through the same recoverable paired write.

Apply approves the saved operation digest. It does not accept new canister IDs,
declarations, paths or debit limits. Repeat that exact apply command after an
interruption: signed ingress, spent attempts, Root reset receipts and paired local
publication remain durable. Completed current or archived receipt replay returns
before ICP resolution and has no remote effects. A new review cannot replace an
approved unfinished operation. Do not edit the journal to bypass a rejection.

Root, signed-handoff and management-inspection allowances are retained after their
local and read-only authority preflight succeeds, immediately before the update.
Approval prepares every source before spending its first inspection allowance.
A failed preflight leaves those allowances and handoff issuance state unchanged.
Root-owned inspection preflight reports an outbound-reserve shortfall with its
observed and required cycle amounts. Unknown submitted outcomes still require
reconciliation with the original request.

Only the destination pool must be quiet. Complete inventories still exclude any
candidate already owned elsewhere, including pending or failed assets. Unrelated
Roots may continue maintenance and allocation; unrelated membership changes do not
invalidate an otherwise matching source observation.

Apply retains certified terminal evidence for the exact controller request before
reconciling custody. A rejected request can renew only after a fresh inspection
proves the source still has its original version, controllers, code and disposition.
A certified `done` response also permits reconciliation after its reply is pruned.
Certified absence permits it only when the certified subnet time is after expiry
and within the IC's specified five-minute absence window. Host wall-clock expiry,
an HTTP refusal and a still-processing request do not establish that boundary.
These rules follow the [IC request lifecycle](https://docs.internetcomputer.org/references/ic-interface-spec/https-interface/#overview-of-canister-calling).

For a retired request, exact transitional controllers and the single expected
version advance complete the handoff without another update. An unchanged source
may use a new signed request within the original budget. Foreign changes remain
blocked. Unissued intent can refresh its envelope without an effect; submission
allowance and issuance are published together before sending any bytes.
The original cycle baseline and debit allowance remain in force. New requests
and resubmissions share the same two-submission limit; paid observations retain
their separate four-per-canister limit. An unknown request observed beyond the
certified absence window or an exhausted allowance remains fenced for recovery.



## Desired State

The default document is `fleets/<fleet>.toml`:

```toml
schema_version = 1
fleet = "staging"
environment = "local"
treasury = "treasury" # logical name of one controlled canister below
operator = "<operator-principal>"
cycles_ledger = "<cycles-ledger-principal>"
ledger_fee_cycles = "0.1B" # generated from the live Ledger
management_creation_fee_cycles = "500B" # exact future creation fee from the seed
material_cycle_threshold = "0.001B"
maximum_observation_burn_cycles = "1T"
maximum_update_burn_cycles = "1T"
maximum_stalled_observations = 8

[protocol]
app_config = "canic.toml"
coordinator_candid = "artifacts/fleet_coordinator.did"
root_candid = "artifacts/fleet_subnet_root.did"
store_candid = "artifacts/wasm_store.did"
[[protocol.component_group_placements]]
deployment = "primary_cells"
ordinal = 0
root = "root"

[[canisters]]
name = "treasury"
kind = "auxiliary"
presence = "present"
principal = "<controlled-treasury-principal>"
replace = false
subnet = "<subnet-principal>"
controllers = ["<operator-principal>"]
initial_cycles = "0B"
minimum_cycles = "0B"

[[canisters]]
name = "coordinator"
kind = "coordinator"
presence = "present"
replace = false
subnet = "<subnet-principal>"
controllers = ["<operator-principal>"]
initial_cycles = "5T"
minimum_cycles = "1T"
wasm = "artifacts/fleet_coordinator.wasm"
```

`maximum_stalled_observations` is the base consecutive-unchanged limit for one
effect. The long-running typed `ProvisionComponents` action raises that limit,
when necessary, to its compiled initial-topology floor: one base observation,
five per Root, three per top-level Component and three per recursively required
initial child. Future descendant capacity does not increase this floor. That
automatic floor is capped at 64; an explicitly reviewed larger configured
limit is still honored. Only passive status queries are paced, using bounded
exponential delays from 250 milliseconds to five seconds. Any durable semantic
progress resets the counter. Silence never authorizes a second command; only
the exact retained typed retryable-failure result may replay the same operation
identity.

For fixture preparation and chunk publication, the configured base
`maximum_stalled_observations` also supplies the total permitted paid attempts.
The reviewed action copies that value into `maximum_attempts`, reserves the
per-update burn for every permitted call and accounts for retry observations.
Its journal consumes `publication_attempts` before issue, even when a response is
lost. Those attempts never reset on progress or process restart. At the limit,
status reconciliation remains available, but an uncommitted action returns
`FixturePublicationBound` before another update. Preserve that operation;
changing local desired input does not enlarge its retained review authority.
Fresh-Fleet review includes fixture preparation/chunk actions in the successor
bound and separately records `fixture_publication_retry_attempts` for their
additional permitted calls. Their configured update/observation allowance enters
the continuation reserve and plan digest. Generation verifies the selected
release's fixture manifest and retained payloads before admitting effects; see
the [fixture contract](../build-and-evidence/fixture-artifacts.md).

The provisioning burn reservation includes this selected wait bound. It does
not turn unused descendant capacity into terminal inventory work.

After protocol completion, a retained Root-owned pool asset may briefly lack an
exact current balance while the Root finishes publishing its lifecycle result.
Fleet Ensure treats that state as passive observation under the same finite
`maximum_stalled_observations` bound. It issues no protocol, installation,
controller, creation or funding command while waiting. The terminal observation
retains the exact balance; exhaustion names the target and last Root-owned
lifecycle and leaves the operation resumable.

Human-authored `canic.toml`, Fleet policy and cycle-valued CLI options require
quoted exact values with a case-sensitive `B`, `T`, or `Q` suffix. Exact
decimals such as `1.5T` and `0.1B` are accepted; bare integers, unsuffixed
strings, lowercase units, exponent notation and sub-cycle precision reject.
Generated operator-reviewable TOML uses the largest exact unit with `B` as its
minimum, including `0B` and fractional billions. Durable plan JSON, Candid,
stable state, hashes and receipts continue to use their exact machine-owned
integer or bounded-decimal representations and must not be hand-edited.

Generated fresh Store and pool entries additionally use
`controller_canisters = ["root-0"]`. These are logical dependencies, not
caller-supplied Principals. Their referenced role must appear earlier in the
desired document; Fleet Ensure resolves the exact Principal from its durable
creation state before issuing the dependent effect.

The generated `[bootstrap]` and `[protocol]` blocks enable Canic-owned
infrastructure initialization and control-plane choreography. They name only the checked-in App configuration, exact
Coordinator/Root/Store Candid contracts, and typed deployment placements.
Operators do not provide Candid methods, argument documents or expected
response bytes, and missing infrastructure init arguments never silently
degrade to `()`. Canic compiles Store artifact staging/bootstrap, deterministic
Registry joins, Root synchronization, Registry and Root-mirror activation, and
exact local Component Registry preparation before Component provisioning in
that order. Every configured initial placement must appear once and every
selected Root must be a declared Root role.

Every admitted Component Spec's release-bound `initial_cycles` must be less
than or equal to the owning Root's exact pool `canister_cycles` target. Canic
checks this while generating desired state and again while planning from a
current desired document. A stale or edited 4.8T pool target therefore cannot
reach live provisioning for a 5T Component: the no-effect diagnostic names the
Root, Component Spec, exact target and required cycles.

All cycle quantities are exact decimal strings. Unknown fields and unknown
schema generations reject. Wasm, binary init-argument, and drain-Candid files
are hashed into the reviewed plan and rechecked immediately before their
effect. Fleet/environment labels are path-safe before Canic accesses operator
state. Authority Principals must be valid and non-anonymous. The configured
treasury names one present desired canister and is always reused, never
replaced. The active ICP identity must equal `operator`, and every
host-controlled canister retains that Principal as a direct controller so
interrupted effects remain
observable and resumable. Root-owned pool assets retain their Root and
configured recovery controllers and are observed through its protected bounded
inventory. A Store retains its exact owning Root, protected operator and
configured recovery controllers; when a retained Store still lacks the
operator, the Root durably prepares that exact controller set before the host
installs the current Store artifact.

## Plan And Apply

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

## Cycle Conservation

The reviewed maximum equation is:

```text
observed controlled cycles
+ maximum operator debit
- maximum unavoidable fees
- maximum observation and update burn
= expected minimum post-operation cycles
```

Terminal evidence uses measured values:

```text
observed starting cycles
+ received new funding
- measured execution and observation burn
= final controlled cycles
```

After protocol convergence, Canic rebuilds the terminal inventory from the
exact active Coordinator Registry, retained Root provisioning result, protected
Component Registry partitions, Root pool pages and bounded sharding-child
pages. Every discovered Principal must retain the exact current authority,
parent, role, Candid profile and module hash before its live balance enters the
conservation equation. This prevents a no-effect successor plan from forgetting
protocol-created Components, descendants or unused pool assets.

Creation funding, Cycles Ledger fees, management creation fees, update burn,
observation burn, and retirement transfers are separate report fields. Apply
cannot issue actions whose planned debit exceeds the reviewed operator bound;
terminal success additionally requires measured burn to remain within its
reviewed ceiling. Fresh-pool creation funding includes its bounded pre-import
margin in both the reviewed debit and terminal conservation equation; the
readiness floor remains a separate invariant.
Each existing-canister funding action also reports the exact observed deficit,
target-local uncertainty margin and expected post-funding native balance. The
margin covers only that target's planned update actions plus one observation;
it is never multiplied by the Fleet-wide observation ceiling.
This action is a Cycles Ledger `withdraw` to the target canister—a native
canister top-up—not a transfer to the Principal's Ledger account. Its Ledger
block/duplicate receipt proves issuance only. Completion requires a fresh
Root-owned or management observation at or above `expected_native_post`.
Ordinary `Fund` actions cannot substitute a Ledger-account transfer for native
canister funding. Root estate funding is the separate, explicitly reviewed
`FundEstate` action described above: it credits the exact protected Root Ledger
account before autonomous creation and is never represented as native pool
capacity.

Native pool funding records `pool_funding.root` and `pool_funding.lifecycle`
in the reviewed action. Ready assets require an empty module. PendingReset and
Failed assets may retain installed modules because funding precedes their
separately journalled Root reset. Before funding, the adapter verifies exact
pool membership, the reviewed lifecycle and exact Root-plus-recovery controllers. Retry keeps
the original Ledger withdrawal identity and receipt; it does not repeat an
already completed credit. This is the current schema-1 hard cut.

Fleet Ensure no longer installs a temporary recovery canister. Direct pool
creation and ordinary top-up target native canister balances, while
`FundEstate` alone transfers the forecast shortfall to a Root's Cycles Ledger
account. External Ledger-account credits do not block admission or completion,
including when no creations are reviewed. They remain separate from transfer
accounting: surplus never substitutes for a durable funding receipt or expands
the reviewed creation count or debit ceiling. Terminal conservation includes
the observed surplus as net credit and still rejects unexplained deficits.

## Retirement Boundary

Root's controller-only `canic_root_status` and Coordinator's controller-only
`canic_observability` accept `ReplayRelease : opt blob`. The cursor is the returned
32-byte stable slot key. Each page reads one retained shared replay receipt and
uses key-only lookahead. Entries preserve the original command, operation, actor,
authentication class, payload hash, exact phase/recovery reason, timestamps,
accounting intent IDs and effect target. Expired uncertainty and completed history
remain visible; the query neither prunes nor resumes them. Cached response bytes
stay in their existing owner. The encoded stable receipt is limited to 32 MiB
before decoding or writing, and projected command/method identities to 1 KiB each.
The record's CBOR layout and stable allocation are unchanged. This census does
not prove settlement of the referenced cost intents or role-specific journals.

Host collects these pages from the reviewed Coordinator and every Registry Root,
bracketed by certified owner custody and unchanged Registry observations. It keeps
the original owner, actor and accounting identities without expiry filtering or
settlement inference. Reads are limited to 256 KiB per reply, 8 MiB overall,
4096 receipts per owner, 512 Candid types and a 16 KiB header, with bounded decode
and skip work, a 15-second query deadline and a 120-second collection deadline.
Owner/cursor mismatches, malformed replies and exhausted bounds refuse the whole
collection, including a failure after earlier owners succeeded. These are
time-local observations, not a producer fence or a destructive-release decision.

Root's controller-only `canic_root_status` accepts
`ProvisioningRelease : opt variant { Provisioning : blob; DirectorySynchronization : blob }`.
Start with `null` and follow `next_after` until it is absent. Each page reads one
retained operation and uses key-only lookahead. It returns the original operation
key, plan hash, exact stage, outstanding Directory/publication delivery and last
provisioning failure, plus the two active-operation pointers. Discovery does not
depend on those pointers, admit new work or resume effects. A missing delivery
intent is not proof that lower-level paid work has settled. These are time-local
observations; production release still needs producer quiescence and reconciliation
before clearing an owner. Completed history alone is not a refusal condition.
The internal Host collector `ops::release::provisioning::collect` retains the
original pages for every reviewed Root, bounded to 4,096 operations per Root,
256 KiB per reply and 8 MiB in total. Each query has a 15-second deadline; the
collection has a 120-second deadline. Candid decoding and skipping each have
2 MiB work quotas, with at most 512 types and a 16 KiB header. Certified custody
and matching Registry observations bracket collection. Changed active pointers,
foreign Root identities, invalid key/phase pairs and broken cursors refuse the
complete result. Equal operation IDs in the two different journal kinds remain
distinct owners. Host collection adds no settlement or destructive authority.

Root's controller-only `canic_root_status` also accepts `PoolRelease`. It reads
the bounded pool singleton independently of admission for new work, preserving
bootstrap hold identities (including Store), retained import reservations and
progress, consumed call/debit allowances, pending creation and pending handoff.
Released import history remains visible. The query neither resumes effects nor
changes their allowance, and a record bound to another Root refuses the result.
It is available before activation once protected Root authority exists.

Host's `ops::release::pool::collect` reads this evidence for every reviewed
Registry Root. It binds the selected signer/network, verifies Coordinator/Root
custody and Registry before and after the queries, and checks retained Root and
subnet identities. Replies are limited to 1 MiB each and 16 MiB total, with bounded
Candid decoding/skipping work, 512 types and a 16 KiB header. Each query has a
15-second deadline and collection a 120-second deadline. Refusal returns no
partial result. Historical operators, issued allowances and released imports are
retained exactly; they are not required to match a new operator or new policy.
This time-local pool observation does not settle effects, establish custody of
every mentioned historical source or replace provision/child-funding evidence.
It cannot by itself establish that a Root is safe to clear.

Root's controller-only `canic_root_status` query accepts
`FundingRelease : opt nat64`. Start with `null`, then pass each returned
`next_after` until it is absent. Each page returns at most 32 retained ICP
refills; stable storage reads at most one additional record for lookahead and
seeks directly past the cursor. The response also preserves the current funding
request, accepted grant, pending policy rotation and configured refill policy.
Refill evidence includes exact historical accounts, transfer identity, ledger
block, refund block and expired CMC notification evidence, including operations
whose notification retry allowance is exhausted.

The Host release funding collector retains those exact pages for every selected
Root. It checks the reviewed operator/network, brackets queries with Coordinator
and Root custody certificates and matching Registry observations, and requires
the current policy binding, stable funding header and strictly advancing cursors.
Its limits are 256 KiB per reply, 8 MiB total replies, 4,096 refills per Root,
15 seconds per funding query and 120 seconds for collection. Candid decoding,
skipping, type count and header size are bounded. A refusal returns no partial
census, and no observation issues a management update or transfers funds.
The same collection retains the Coordinator's existing controller-only
`canic_observability::Funding` response under those byte/work/deadline limits.
It requires every reviewed Root exactly once, with matching policy and lifecycle
bindings; input ordering is immaterial. Coordinator cycle balances, reserved
windows, current grants, terminal results and policy rotation remain intact.
Pending operation IDs from both sides are deduplicated for follow-up. A retained
terminal Coordinator result alone does not create pending work, while a Root
still awaiting that result remains visible. These are time-local reads, not an
atomic cross-role snapshot or permission to settle a grant.

The Host `workflow::release::observe_funding` library boundary combines that
collection with a receipt assessment. Exact conversion and refund receipts remain
historical evidence, not automatic blockers. A refund without a refund block is
marked for explicit residual review; incomplete or exhausted operations retain
their Ledger/CMC reconciliation requirement, and inconsistent terminal receipts
are identified separately. Current Coordinator requests remain visible even when
Root has accepted a grant. Pending policy rotation is reported separately rather
than treated as proof of a paid effect. The report retains original pages and
accounts, including those attached to completed operations.

The retained `transfer_uncertain` fact is persisted before Ledger dispatch. A
first explicit refusal clears it; a refusal after a lost reply preserves it.
A confirmed transfer or duplicate receipt clears uncertainty and retains its
Ledger block. Unissued or definitely refused transfers can therefore report
`NoLedgerTransfer` without unnecessary Ledger reconciliation. An expired window
or a cleared accounting reservation alone does not establish that an earlier
transfer failed. Uncertain expired/rejected transfers retain their reserved
allowance; a fee error following a lost reply preserves the original fee and
transfer identity instead of retrying different bytes. The assessment grants
no new spending/retry authority and does not itself reconcile that effect.
The required current refill record field follows the pre-1.0 reinstall-only
hard cut; there is no predecessor-record conversion.

This census is an observation, not a release seal or settlement receipt. Pages
are not an atomic snapshot while producers remain active. A release executor must
quiesce producers, reconcile unfinished effects, resolve configured default Ledger
identities and observe account balances separately. Neither an empty retry queue
nor a completed latest refill establishes that all historical obligations are
settled. The whole-Fleet release command remains under implementation; this query
does not authorize resetting the observed Root.

An IC controller cannot pull cycles from an arbitrary canister. A material
source selected for deletion must therefore declare an idempotent,
controller-authorized drain endpoint. In-place reinstall retains its cycle
accounts and requires no drain solely because its module changes:

```toml
[canisters.drain]
candid = "interfaces/cycle-drain.did"
method = "canic_cycle_drain"
destination = "treasury" # exact logical name from the desired document
maximum_execution_burn_cycles = "0.1B"
```

Fleet Ensure resolves that logical name through its durable current state. The
endpoint receives the Fleet operation ID, exact destination Principal, and exact
cycle amount and must return either `Accepted` or `Replayed` with that same
amount. A missing, changed, foreign, or unsafe drain returns a typed blocker.
The source response is issuance evidence only. Canic retains the exact source
and treasury balances from before the call, then proves both the bounded source
debit and the exact controlled-treasury credit from fresh live observations.
Canic leaves the canister running and funded if either side is absent,
inconsistent, or ambiguous. Stop and delete occur only after that two-sided
proof and a fresh stopped/balance check.

The same rule applies to Canic control-plane updates: a successful update call
marks the command issued, not applied. The journal advances only after the
exact typed status query proves terminal state; consecutive unchanged status
observations consume the stall budget and genuine progress resets it.

## Hard-Cut Boundary

The finalized `current-release-set-manifest.json` declares
`"transition_mode":"reinstall_only"`. This required field is included in the
canonical release digest. The authority loader rejects omitted or unsupported
policies; there is no default or inferred upgrade mode. The field describes
release policy and does not authorize a reinstall: the exact reviewed plan,
controller authority and cycle-conservation checks still govern effects.
Planning and apply validate this policy before paid platform observations,
including terminal replay with no continuation work. A retained operation uses
its reviewed release authority; rejection does not compact its journal.
The exact Root-start prerequisite can still use retained installed authority
when application build files are unavailable. It does not select or install a
release; full release operations require the current manifest.
That prerequisite's management reads remain paid. A present manifest with an
omitted or unsupported policy is rejected before those reads.

The reconciler does not read or migrate former install plans, role journals,
repair receipts, recovery bundles, installed-Fleet caches, or version-pair
contracts. Historical release notes remain evidence only. Current desired
state, current `v1` ensure state, and current live observations are the only
host authorities.

A release boundary discards the predecessor's application/framework state and
completed execution authority. The new host does not resume an old journal with
substituted desired input or silently fill omitted durable fields. Cycle
conservation must be established before controlled infrastructure is erased.
Unfinished issued effects require exact accounting and cycle-safe disposition
before reset; they do not require repairing predecessor state or restoring an old
Root/client. Historical records are archived without admitting old stable bytes
or protocols into the new Fleet. The replacement uses a separately
reviewed current plan. Selected ID-preserving reset retains those physical
identities and controlled cycle accounts, subject to reviewed protocol debit.
Same-operation interruption recovery retains the exact current plan, journal,
artifact bytes and paid-effect receipts.

## Unreadable retained plan

An unreadable plan or journal cannot authorize continuation or destructive
spending. For active current-release work, missing required fields such as plan
`recovery_review` or journal effect `publication_attempts` are rejected even when
`schema_version` is 1. Preserve the complete Fleet directory, referenced
content objects, release artifacts, desired inputs, estate seed and paid-effect
receipts. Do not insert null fields, recalculate the plan digest or delete the
journal. The current decoder cannot determine whether omission reflects a
different source contract or damaged evidence.

Explicit `--reinstall` selects current-build reset before predecessor executable
plan decoding. It does not require completion identities or a readable application
state. Follow [clean reinstall from physical inventory](#clean-reinstall-from-physical-inventory)
for current-build qualification, custody review and archival. Uncertain paid-effect
evidence is checked separately; malformed application fields alone do not block
replacement. Ordinary continuation still requires its exact current contract.

An unreadable executable payload alone neither proves nor disproves completion.
If completion metadata is damaged, contradictory or genuinely unfinished, retain
the evidence and establish cycle-safe disposition from physical inventory,
current controllers and exact paid-effect evidence. Release replacement remains
a current-build hard cut plus reinstall; do not repair old state or require the
old executable owner to make it work again. Do not fill missing fields or delete
journals to manufacture reset authority. A working frontend is not proof that a
paid operation finished. Read-only commands that need an active role map
also cannot invent it from an unreadable plan.

For an explicitly disposable **local simulator**, use its owner's exact-session
reset procedure after the owner exits. For Canic's `LocalFleetSession`, follow
[persistence, recovery and reset](local-development-fleet.md#persistence-recovery-and-reset).
Deleting just an Ensure plan is not a simulator reset. A simulator reset cannot
resolve outstanding live payments or discard controlled real cycles.

## Deliberate selected-build database wipe

Follow [the current clean-reinstall sequence](#clean-reinstall-from-physical-inventory)
for both changed-build and identical-build resets, including its distinct operation
identity, reviewed phase digests, artifact retention and same-digest interruption
recovery. Infrastructure or pool clearing alone is not Fleet convergence.
Logical workload assignments may change within the reviewed Root/subnet inventory.
Ordinary Ensure does not request another wipe; a later explicit `--reinstall`
does. Do not combine `--reinstall` and `--apply`.

## Retained growth and dependent recovery review

During same-operation dependent recovery, Ensure compares retained descendant
identities with the selected Root pool imports. Known assets missing from that
selection cause typed `IncompleteRootEstate` rejection before Stop or Install.
Refresh the existing operator seed and matching policy imports from terminal
Fleet evidence, regenerate desired state while the current Root is still
observable, and review the exact live controller/subnet bindings. The host never
silently promotes retained identities into import authority. Preserve the active
state and journal until terminal completion; deleting them removes useful
omission evidence and is not a seed-refresh procedure.

Infrastructure reviews expose `recovery_review`: base execution burn, the
reserved continuation allowance, the complete successor-catalogue ceiling and
currently known pool-reset top-ups. The reserve is capped by available cycle
headroom after the base allowance. It is a conservative maximum, not expected
expenditure. A first phase that cannot afford its own bound still rejects.
Automatic protocol successors retain the longest affordable ordered prefix under
the original sealed budget; each immutable phase is durable before its first
intent. Another phase or new debit beyond that authority requires fresh review.

Known reset top-ups use the same calculation as executable pool funding actions,
including the funding margin and exact configured Ledger fee. Their presence in
`recovery_review` grants no debit authority. `pending_current_protocol` explicitly
marks work that can only be resolved after installation and fresh observation.
A zero-funding infrastructure phase is therefore not a complete deployment quote.

Reports also expose `continuation_forecast` outside the immutable plan. It lists
known import names and Principals, distinguishes post-initialization candidates
from already reviewed reconciliation, carries separately reviewed dependent
funding estimates, and names readiness, capacity and publication/provisioning
work that still needs live discovery. The successor-action limit is an authority
ceiling, not an estimate. A terminal Root-reset prerequisite still carries this
forecast; only full terminal completion clears the remaining-work projection.

Each initialization-dependent import includes a `headroom` assessment. Known
bootstrap samples report required, available native and missing cycles before
initialization effects. The forecast assumes the same `0.1T` source debit used
by clean reinstall; an explicit import can review a different allowance. Held
sources whose balances are unavailable until the current Root runs report
`awaiting_current_root_observation`, rather than an invented balance. These
assessments are advisory and do not grant funding or import authority. They appear
in bootstrap and clean-reinstall JSON and text reports; final import admission
still checks the full explicit bounds.

When a freshly observed phase is admitted as an exact bounded successor, its
observation may satisfy the immediately following protocol funding check. The
handoff is bound to the first action digest, stays in this invocation and is
consumed once. Any restart or intervening effect requires fresh observation.
Replanning after a completed phase shares configured infrastructure status with
protocol planning within one decision. Pacing clears that evidence, and the
scope ends before a continuation is appended or any new effect is issued.
Terminal replay first proves inventory, then uses one fresh merged-estate snapshot
for both convergence and conservation; controller, authority and effect-free
replay checks remain. Its read-only replanning decision shares infrastructure
status with protocol planning. That evidence expires before the separate terminal
authority check and never carries into another replay.

Typed `SuccessorReviewRequired` errors and `review_required` progress include the
newly observed target/action list, maximum additional debit including fees and
the next read-only review command. Completed infrastructure receipts and the
operation identity remain available through that review boundary; reviewed
funding still requires fresh authority, fee and balance revalidation before any
debit. The same informative pause also applies after an explicitly reviewed
recovery phase when activation work remains.

### Completed replay after operator account activity

A completed plan still checks its original operator source and reviewed debit
against the current Cycles Ledger balance. Unrelated account activity is outside
that replay contract; completed accounting must not be rewritten to accommodate
it. A balance outside that reviewed range returns `TerminalReplayBalanceChanged`,
naming the operation, plan and balance bounds. In-progress conservation and
recovery remain unchanged.

After separately authorised spending or a deposit changes that balance, preserve
the completed plan, journal and receipts. Run `canic fleet ensure <fleet>` with
the same environment and desired input, without `--apply`, to review a fresh plan.
If the Fleet remains converged, the fresh plan has no actions and no operator
debit. Review its actual actions and debit before applying its new digest; drift
can require additional work. Do not repeat a reinstall or edit the old journal
to make its balance agree. Immediate replay with unchanged accounting remains
effect-free.
