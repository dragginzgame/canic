# Fleet Ensure Automation

Use this guide when a script or deployment wrapper consumes Fleet Ensure JSON,
continuation commands, timing receipts, or completion evidence.

[Back to the Fleet Ensure overview](fleet-ensure.md).

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

## Continue From Here

- [Review and apply a Fleet plan](fleet-ensure-plan-and-apply.md)
- [Recover an interrupted operation](fleet-ensure-recovery-and-cycle-safety.md)
- [Return to Fleet Ensure](fleet-ensure.md)
- [Browse Fleet operations](README.md)
