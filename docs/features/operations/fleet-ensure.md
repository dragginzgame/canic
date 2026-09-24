# Fleet Ensure

`canic fleet ensure <fleet>` is the sole maintained Fleet installation and
convergence workflow. It reads one current desired-state document, observes the
configured controlled estate, and either writes a reviewed plan or applies the
exact retained plan digest.

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

Normal `fleet ensure` review/apply and `fleet generate` commands print a timing
receipt path under `.canic/diagnostics/fleet/` before measured work. `--json`
Ensure output emits a `fleet_ensure_timing_receipt` event with that path. These
private JSONL diagnostics supplement the retained plan and journal; they never
prove deployment completion or authorize continuation. Funding-observation and
operator-mint subcommands retain their existing output owners.

Each line has UTC Unix milliseconds and monotonic elapsed microseconds. Existing
progress DTOs bind operation, plan and phase; stage and request identifiers link
start/end pairs and inclusive parents. An observation's `succeeded: null` is a
start, `true` is successful completion of that boundary and `false` is failure.
A successful submission is not verified remote convergence. Do not sum a parent
with its children, or concurrent request durations as critical-path wall time.
Request times include local startup/IPC and remote response/confirmation; pure
CPU, internal IC calls and remote-wait components remain unavailable.

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

Non-converged retained work blocks a new operation. Preserve its plan, journal
and selected build and use the existing recovery flow. A supported completed
source whose current journal cannot decode reports `retained_terminal_review`
and requires the separate `fleet ensure --reinstall` review without `--apply`.
Other unreadable or inconsistent evidence fails closed. Do not patch missing
journal fields or delete retained evidence.

Downstream orchestration should call readiness before `canic build`. Offline
artifact-only builds do not acquire a Fleet identity or query Ledgers
automatically. Readiness does not predict full lifecycle convergence, validate
application hooks, authorize payment or approve reset scope. Exact plan and
funding admission still run immediately before effects.

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
current-estate identity seed in `deployments/<fleet>.estate.toml`:

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
Root-only ownership or the exact Root-plus-operator set before installation.
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

CANIC-157's single-Root partial-activation recovery is qualified against the
affected 0.110.12 runtime. Preserve the source plan, journal, paid
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

After a completed fresh installation, the original symbolic fresh seed may be
used again. Ensure resolves its Root name through the existing Fleet state
before management prerequisites. A configured Principal that conflicts with
that retained identity rejects; observed identity, Subnet, controller and module
checks still apply. A changed release still requires reviewed reinstall.

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
starting balances, source seals and withdrawal identity remain unchanged.

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

Completed retirement accounting embedded at
`reinstall.source.terminal_retirement.conservation` is immutable evidence. Its
recorded field names and canonical encoding remain part of the original plan
digest, including recorded execution/settlement observations. They are not
converted into current net debit/credit observations or used to admit a new
payment. Active plans, actions, journals and fresh conservation still require
their complete current contracts.

For CANIC-166/172's retained withdrawal, resume ordinary Ensure without
`--reinstall` to obtain its funding review, then follow the conversion and
original-plan apply sequence above. The original desired input and source seals
remain bound throughout recovery. A request for a new reinstall while that
operation is in progress returns `RetainedOperationRecoveryRequired`, naming
the original operation and plan digest. After original convergence and
effect-free replay, request a separate `--reinstall` review selecting the new
release. This does not establish that a live withdrawal is unpaid, that its
Ledger retry window remains open, or that live source authority still matches;
the existing receipt, retry, seal and conservation checks retain those decisions.

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
their readiness. This supports a full pool with four Workloads and four Failed
reserve assets without funding the Root account or creating another canister.
If the protected inventory cannot satisfy the required capacity through those
repairs, planning returns the typed capacity failure before funding.

Planning rejects a fresh pool whose creation amount is below its readiness
floor plus those margins. The typed failure reports the requested creation
funding, floor, admissible burn, required funding and exact shortfall. After a
Create is applied, its first exact balance must still cover the readiness floor
plus the remaining controller-finalization margin; otherwise resume stops
before any controller or protocol effect and reports both readiness and
pre-finalization shortfalls.

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
observable and resumable. Root-owned pool assets remain solely under their
Root and are observed through its protected bounded inventory. A Store retains
its exact owning Root and protected operator; when a retained Store is still
Root-only, the Root durably prepares that exact controller set before the host
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
If a verified current-schema in-progress plan still uses the former inline
Store-chunk projection, apply first publishes those exact bytes to the
content-addressed object store and atomically rewrites `plan.json` to hashes
and bounded sizes. This local compaction preserves the plan digest, operation
identity and journal bytes and completes before any platform observation or
remote effect.

A schema-`v1` plan created before reviewed-input retention normally requires
its exact original desired document. The bounded no-debit terminal case is
recoverable without inventing that input: all canisters must be reused under
the same exact names and Principals, every earlier action must already be
applied, and the final issued action must be typed Component provisioning.
Canic may only observe that action, never reissue it, and must validate the
protected terminal inventory and conservation equation before closure.

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
pool membership, the reviewed lifecycle and sole-Root controllers. Retry keeps
the original Ledger withdrawal identity and receipt; it does not repeat an
already completed credit. This is the current schema-1 hard cut.

Fleet Ensure no longer installs a temporary recovery canister. Direct pool
creation and ordinary top-up target native canister balances, while
`FundEstate` alone transfers the forecast shortfall to a Root's Cycles Ledger
account. An externally created Ledger-account credit remains outside the
reviewed operation and cannot be silently consumed or substituted for its
durable transfer receipt.

## Retirement Boundary

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

The reconciler does not read or migrate former install plans, role journals,
repair receipts, recovery bundles, installed-Fleet caches, or version-pair
contracts. Historical release notes remain evidence only. Current desired
state, current `v1` ensure state, and current live observations are the only
host authorities.

A release boundary discards the predecessor's installation state and current
Fleet authority. The new host does not resume an old journal with substituted
desired input or silently fill omitted durable fields. Cycle conservation must
be established before controlled infrastructure is erased. Any retirement work
runs under its exact original authority; its receipts account for cycles and
do not admit old identities, topology, stable bytes or protocols into the new
Fleet. The replacement Fleet uses a separately reviewed current plan. Exact
controlled canister identities and their cycle accounts may remain in place;
identity reuse is not promised. Same-release interruption recovery retains its exact current plan,
journal, installed artifact and paid-effect receipts.

## Unreadable retained plan

An unreadable plan or journal is not permission to replace an unfinished
operation. Missing required fields, such as plan `recovery_review` or journal
effect `publication_attempts`, are rejected even when `schema_version` is 1.
Preserve the complete Fleet directory, referenced
content objects, release artifacts, desired inputs, estate seed and paid-effect
receipts. Do not insert null fields, recalculate the plan digest or delete the
journal. The current decoder cannot determine whether omission reflects a
different source contract or damaged evidence.

The completed-source retirement inspector reads an immutable evidence projection,
not an executable current journal. For a proven converged supported source,
missing `funding_observations` contributes no additional execution allowance.
Present observations must validate; null/malformed observations, unknown paid-work
fields, unfinished effects and inconsistent source identities reject review.
Original document hashes bind the existing terminal inventory, conservation,
archive and handoff checks. This permits a separate review without importing the
source state into a current execution contract or rewriting source bytes.

This boundary also applies to read-only commands such as `canic info env`.
A working frontend does not prove that the retained operation completed, and
an unreadable plan does not authorize reconstructing role bindings from stale
state. Decode errors retain the document path and underlying cause and point
here, including missing continuation bounds such as `maximum_successor_actions`.

For an explicitly disposable **local simulator**, first retain the evidence
above and confirm that discarding its simulated data and balances is authorized.
Stop its owning session, then use that owner's exact-session reset procedure.
For Canic's `LocalFleetSession`, follow
[persistence, recovery and reset](local-development-fleet.md#persistence-recovery-and-reset):
reset the recorded session only after its owner exits, start a fresh session,
use its newly returned environment, and generate/review a new Fleet plan from
the current local release. Reset preserves historical Ensure records and shared
artifacts; deleting just `plan.json` is not this procedure. A downstream-owned
simulator must use its own documented reset owner; the Canic session command
cannot reset arbitrary local infrastructure. A name containing `local` does not
prove disposability. This procedure cannot resolve or discard outstanding live
payments, controlled real cycles or an unresolved real operation.

Ordinary Ensure diagnoses failures in either document and reports
`RetainedActivationReviewRequired` only when the
existing local source inspector finds an exact Applied protocol prefix ending
in Issued provisioning, with an unissued readiness tail. The diagnostic names
the operation, journal's plan reference and hash of the source document. These
are evidence identities; the journal reference is not a verified current plan
digest, and local inspection does not establish live reset authority.

The inspector also verifies the original action hash for an already Applied
Store bootstrap receipt that lacks fixture metadata. This private receipt
projection is limited to the completed prefix; it does not construct a current
bootstrap command, invent empty fixtures or permit an Issued bootstrap row.
The source documents and all paid-attempt evidence remain byte-for-byte intact.

For that source shape, use the existing explicit `fleet ensure <fleet>
--reinstall` review with the selected corrected release's desired input and
the exact environment. Omit `--apply`. This deliberately bypasses ordinary
resume selection, inspects source evidence before requiring current journal
fields, and requests the bounded CANIC-157 partial-activation review
described above. It leaves the active source documents in place and may reject
if source artifacts, controllers, complete physical inventory, Root Ledger
balances, pending paid effects or debit bounds do not satisfy admission. It
requires a changed, corrected Root module and admits only one Root.

An admitted review is a new current operation, not a repaired source plan.
Review its exact digest and conservation evidence before applying it. The
existing sequence archives source evidence, settles Coordinator/Root work,
reviews Root reinstall separately, then completes the dependent Full Ensure
review. Interrupted steps resume their own retained digest; a later build must
not replace their selected input. Completion requires terminal conservation
and immediate effect-free replay. No successful local diagnostic or preview
establishes those completion properties.

A completed operation has a separate bounded assessment. Ordinary Ensure reports
`RetainedTerminalReviewRequired` when the receipt inspector recognizes a
converged full operation with supported native funding/install effects and
immutable protocol successor phases. An explicit `--reinstall` review can then
inspect those completed effects without decoding the source plan as current
executable authority. Informational recovery forecasts remain opaque source
bytes; no missing field or replacement source digest is manufactured.

This assessment requires every action hash and Applied receipt to match, native
payments to retain their original recorded balance evidence, and exact phase
identities. Fresh source-bound inventory must contain the same complete physical
estate and registry. Original operator debit, ledger fees, Root account balances,
pool membership and controlled-cycle burn must reconcile within the source's
bounds. Unresolved effects, creation/funding-review histories, pending creation,
transfers, missing assets and unrelated operator balance changes reject this
bounded path. Receipt inspection verifies the retained completion evidence; it
does not independently fetch historical Ledger blocks.

A completed source with no operator payments may explicitly review one external
Cycles Ledger withdrawal using `--reinstall --retirement-debit-block <BLOCK>`.
This is limited to a default operator account and a currently operator-controlled
destination outside the source Fleet. The block must postdate the source review;
its exact amount plus fee must explain the entire independent balance change.
No source document or bound changes. Sources with operator payments, unexplained
movement, refunds or a destination inside the source estate reject this lane.

The host fetches that single block through a replicated query and verifies the
destination with controller-only management status. The record binds the Ledger,
operator, destination, network root key, block hash/index, timestamp, amount and
fee. A burn receipt proves the debit only: it neither proves successful delivery
nor authorises retrying a withdrawal. The receipt is retained separately from
source conservation in the new reviewed plan. Apply takes only that plan's digest,
re-reads the receipt and live balances, and rejects changes before source adoption
or authority sealing. Response size, decoding work and observation time are bounded;
archived or unsupported receipt shapes fail closed. Existing source archive,
interruption and effect-free replay owners remain unchanged.

The separate preparation review exposes those measured values under
`reinstall.source.terminal_retirement.conservation`, together with exact raw
source document hashes. Review the full reset scope and selected target artifacts.
Apply repeats source and live conservation checks before adoption. The existing
local handoff archives the source plan, journal, state and every successor phase
before committing replacement intent. Preserve the referenced content objects
and source artifacts too. Interrupted handoff selects the same replacement pair;
a completed handoff never rolls back subsequent progress. Subsequent preparation
and full reset use the existing journaled effect owner and their selected digests.

CANIC-166's September 17 staging report is this completed-source case: the missing
`maximum_successor_actions` is in its informational forecast. Its two phase files
and 61 Applied effects pass local source inspection. This host correction does
not change canister runtime contracts or inherently require rebuilding the
already-qualified game release. Actual source artifacts, Candid contracts, live
authority and conservation must still pass review. Local inspection and handoff
regressions do not establish staging admission, successful deployment or terminal
effect-free replay after deployment.

If neither bounded inspection succeeds, `RetainedPlanUnreadable` preserves the
underlying error. There is no general force-reset, predecessor executable decoder
or journal-supersession command. Resolve issued effects under their exact original
authority before changing release contracts. The earlier
[partial-activation recovery evidence](../../audits/reports/2026-09/2026-09-08/activation-feedback.md#final-installed-source-proof)
remains evidence for that distinct source shape.

## Deliberate selected-build database wipe

`canic fleet ensure <fleet> --reinstall` requests a new wipe of a fully converged
Fleet using the selected desired build. Build the current workspace and generate
its desired input before making a fresh request. The completed source operation
supplies installed authority and source protocol contracts; the new review binds
the selected target artifact hashes separately. An identical rebuild is also a
new deliberate wipe. The request retains its own operation identity; it is not
a persistent desired-state flag.

1. Review the `reinstall_preparation` plan and apply its `plan_sha256`. This
   seals Root and Coordinator allocation and maintenance.
2. Run ordinary `fleet ensure` again. Review the `full` reset plan, including
   every physical pool asset captured after sealing, then apply its digest.
3. If interrupted, apply the retained digest again through ordinary ensure.
   Retained reviewed input selects the original target even when workspace input
   changes again. The journal reconciles completed effects and continues that
   same wipe. A new `--reinstall` request cannot replace it.

After sealing, the journal records `prepared`; current-Fleet reads reject it
until the full reset converges. Preparation completion applies only to the
`reinstall_preparation` scope. Reinstall recovery checks management deployment
history through a reviewed Root witness against the issued effect, installed
hash, operator and observed version. The controller-only call uses replicated
management history, so ordinary inspection or timer version advances permit
retry when no newer deployment exists; conflicting history fails closed. Root's
own intended replacement can use the exact selected Root module and Candid to
verify its lost response. Other modules or changed controllers are rejected.
Keep both source and selected build artifacts until the operation completes.
The current durable reinstall record changes through a pre-1.0 hard cut; this
extension does not import unfinished plans from another Canic schema.

Preparation reviews each Root and Coordinator against its own conservative
`update + 8 × observation` burn allowance. Another authority's cycles cannot
cover a shortfall: preparation has no funding transfer. A rejection reports the
exact authority, available cycles, required allowance and shortfall. This is a
maximum execution bound, not predicted spend or a cheaper substitute for reset
headroom.

Reviewing a preparation plan does not pause grants. Applying and confirming its
seal fences ordinary child funding and suspends the Root maintenance timers,
including after a subsequent native-cycle top-up. Explicit live resume opens
that authority again; restoring a snapshot keeps it fenced. This protection is
the current-source, exact-authority preparation contract. It is not automatic
pre-top-up protection for the management-only recovery prerequisite, and does
not authorize calling a predecessor release's protected protocol.

Before each reviewed native or estate credit to a Root or Coordinator that is
about to be reinstalled, the host rechecks its source module, controllers,
Principal, subnet and Running status, then queries the seal for the exact reset
operation. A changed or removed seal rejects before withdrawal, including on
an interrupted retry. The review reserves the additional observation allowance.
Later successor funding after replacement uses the current runtime's ordinary
funding checks; it never queries the replaced source's seal. This does not pause
an unverified installed runtime or protect externally issued top-ups.

The disposable five-Workload/one-Ready PocketIC journey exercises this boundary
with an actual Root native credit: a retained payment intent refuses to proceed
after explicit seal removal; resealing the same operation permits one withdrawal,
whose lost reply is reconciled without another payment. Child grants remain
fenced, and interrupted reset recovery reaches full readiness and effect-free
replay. This does not qualify Coordinator or estate credits on a live network.
Management-only reset funding uses the separately described Stopped-state guard,
without querying the old runtime's seal.

Completion requires full Fleet readiness and conservation of the complete
physical estate. Application stable data is discarded; authored installation
fixtures and current framework authority are rebuilt. Physical identities,
cycle balances and Root-owned Ledger account identities are retained, with
observed execution charges accounted for by the reviewed bounds. Logical pool
role assignments may change. No partial reset is reported as full convergence.

After completion, ordinary ensure is a no-op. A new `--reinstall` request creates
a new intentional wipe. A conflicting new request cannot replace an unfinished
wipe, and `--reinstall` cannot be combined with `--apply`.

## Retained growth and dependent recovery review

Before a changed-release Root reinstall, Ensure compares retained descendant
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
