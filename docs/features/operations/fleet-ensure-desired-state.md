# Fleet Desired State

Use this guide to generate a current desired-state document or write one
explicitly. It covers the complete operator-owned deployment contract consumed
by Fleet Ensure.

[Back to the Fleet Ensure overview](fleet-ensure.md).

<img src="../../../assets/256x256/mechanic-notes.png" align="left" width="96" alt="The Canic mechanic holding a desired-state checklist" />

**Operator outcome:** one reviewable document that binds the intended network,
infrastructure, application canisters, placement, funding, artifacts, and
spending limits for a Fleet.

<br clear="left" />

## At A Glance

| Task | Result |
| --- | --- |
| Select current build and estate inputs | Exact authority and artifact sources |
| Generate a fresh or retained Fleet document | No paid Fleet effect |
| Review deployment fields and bounds | Operator-owned desired state for planning |
| Pass the document to Fleet Ensure | A separate no-effect plan to review |

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

### Retained Estate Seed

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

### Generate Without Mutation

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

### Mainnet Catalog And Root Readiness

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

### Funding And Activation Forecasts

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

### Completed Fleets And Reset Selection

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

### Treasury, Fees, And Network Binding

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
#### Progress And Timing

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

#### Estate Funding

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

### Operator ICP Conversion

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

When preparing a new conversion review, an operator balance that already covers
the paused debit needs no conversion. Ensure reports the matching resume digest
before requesting a conversion quote or retaining a conversion review. Repeat
the same Fleet ensure invocation with that `--apply` digest and omit
`--operator-mint`; original withdrawals use the original plan digest, while
supplementary funding uses its reviewed funding digest.

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

#### Pool Funding And Capacity

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

## Continue From Here

- [Bootstrap supplied infrastructure](fleet-ensure-bootstrap-and-capacity.md)
- [Review and apply a Fleet plan](fleet-ensure-plan-and-apply.md)
- [Return to Fleet Ensure](fleet-ensure.md)
- [Browse Fleet operations](README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)
