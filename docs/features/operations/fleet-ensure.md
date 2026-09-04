# Fleet Ensure

`canic fleet ensure <fleet>` is the sole maintained Fleet installation and
convergence workflow. It reads one current desired-state document, observes the
configured controlled estate, and either writes a reviewed plan or applies the
exact retained plan digest.

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
contracts. The seed supplies identities only. Canic then verifies the active
operator, controller and role relationships, Registry-backed placement,
protected Root pool inventory and exact cycle balances before publishing
`fleets/<fleet>.toml`. A Root-owned Store or pool asset is resolved through the
Root's protected inventory; a retained Store controller handoff accepts only
Root-only ownership or the exact Root-plus-operator set before installation.
The live Root's identity authority must match exactly. Init-only policy fields
may differ only when the reviewed current Root artifact will be reinstalled to
converge them; an already-current Root with policy drift fails closed. A seeded
pool identity remains in the conservation set as it moves from idle bootstrap
capacity through claimed state to a Component workload, without receiving pool
minimum top-ups or being counted twice.

A stopped retained Root requires a separately reviewed prerequisite rather
than a desired-document mutation. Generation first verifies the exact
management-observed Principal, Subnet, controller set and installed module
hash, then atomically seals the live predecessor to the newly requested
finalized release and Root artifact. It returns a deterministic same-ID Start
diagnostic before calling any protected Root endpoint or changing
`fleets/<fleet>.toml`.

Fleet Ensure uses the retained desired document only for stable Fleet and Root
identity. It independently loads the sealed authority's finalized release-set
and infrastructure manifests and re-reads the exact raw successor Wasm; it does
not require the authority's release or successor to equal the older retained
desired release, and does not decode or regenerate that old release manifest.
This boundary works even when current schema-1 state has
retained cycle balances but no Principal or topology projection. The explicitly
scoped plan contains only the same-ID Root `Start`; it has no create,
replacement, reinstall, funding, transfer, fee or operator-debit authority and
cannot publish or backfill Fleet topology. Text and JSON output expose the
`root_start_prerequisite` scope. Apply that exact digest, then rerun generation
to publish and review the newly requested desired release. A stopping Root,
missing module, changed identity, foreign controller, wrong Subnet, unfinalized
or changed successor artifact, or malformed management observation fails
closed. Once the exact Root is running, generation performs the complete
protected Fleet-authority and pool-inventory verification; management evidence
never substitutes for those proofs.

The predecessor seal also fixes the successor order. If live predecessor A is
sealed to successor C, a later build D cannot retarget that authority while A
is still live. Generation returns a typed
`SealedSuccessorConvergenceRequired` diagnostic containing the sealed C and
requested D release-build IDs and Root artifact hashes. First plan and apply
the unchanged retained desired C with `canic fleet ensure <fleet>` until its
terminal re-observation succeeds. Then reuse the finalized D build, rerun
`canic fleet generate <fleet>` with D's release-build ID, explicitly replace
the converged C desired document by its reviewed SHA-256, and review a fresh D
plan. C apply, C terminal proof, D generation and D planning are four distinct
review boundaries. Neither the diagnostic nor either planning pass changes the
sealed authority or grants D an effect through C's predecessor bridge.

Retained-estate treasury policy is explicit adoption, not discovery: it must
name an already-present, non-replaceable controlled canister. Omitting
`treasury` selects the exact seeded Coordinator. Canic does not silently invent
or globally search for a retained identity. Missing, foreign, duplicate,
unseeded or conflicting identities and unexpected co-controllers fail closed.
If an exact retained identity is no longer observable, planning rejects instead
of creating a substitute. The generator queries the configured Cycles Ledger's
current fee and binds it into the desired document. Retained generation has
zero management creation-fee authority; fresh generation uses only its explicit
seeded fee. Observation and update burn values are distinct conservative
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
logical treasury for the fresh operation. Each configured Root's
`canister_pool.minimum_size` becomes its initial set of root-controlled pool
assets; retained `imports` are forbidden in a fresh seed.

Fresh convergence can cross two reviewed-plan boundaries. The first plan
allocates and installs roles whose Principals did not exist at generation time.
When those retained results make the typed control-plane graph compilable, the
terminal check closes that journal as `ReplanRequired` instead of silently
adding unreviewed effects. Run plan-only again, review and apply the new digest,
and continue until the report is terminal. The immediate plan after terminal
convergence has no mutation action.

The management creation fee is explicit because it is network/Subnet economic
authority and cannot be inferred from release metadata. Zero is appropriate
only where the selected local platform actually charges zero. A wrong value
cannot silently change the reviewed debit or conservation equation.

Initial pool assets are direct Fleet Ensure creation actions. The configured
`canister_pool.canister_cycles` value is their readiness floor, not their fresh
creation amount. Fresh generation adds the exact bounded first-observation and
controller-finalization margins to that floor; retained assets keep the floor
as their target. Creation funding, one exact Cycles Ledger creation fee and one
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
`EstateFundingRequired` pause and issues no protocol command; obtain and review
a new plan rather than funding the account outside Canic. Repeating apply while
the balance is unchanged performs no remote effect and does not rotate the
operation or its creation identities. A lower balance, funding above the
reviewed maximum, changed Root or Ledger, or missing account observation also
requires a newly reviewed plan rather than an inferred correction.

Every autonomous pool creation retains its exact Ledger block, operation,
amount, Ledger fee, management creation fee, readiness floor, execution margin
and first observed native balance. Terminal conservation sums those receipts,
requires unique operation and block identities, and rejects a missing,
below-floor or impossible first observation. Later reinspection cannot replace
the first balance. The plan-time forecast uses the complete bounded protected
Root pool inventory, including dynamically created assets in every lifecycle
and any durable pending creation, so a full pool fails before funding.
This forecast does not reinterpret an already-Failed asset as reusable or
silently top it up. A pool whose failed assets consume all configured capacity
stops with the typed capacity failure and leaves every asset and cycle balance
untouched; repairing those retained assets requires its own reviewed native
funding and Root-reset operation.

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
management_creation_fee_cycles = "0B" # retained; fresh uses its seeded exact fee
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
when necessary, to its exact topology-derived terminal-observation floor: one
base observation, five per Root and three per top-level Component. That
automatic floor is capped at 64; an explicitly reviewed larger configured
limit is still honored. Only passive status queries are paced, using bounded
exponential delays from 250 milliseconds to five seconds. Any durable semantic
progress resets the counter. Silence never authorizes a second command; only
the exact retained typed retryable-failure result may replay the same operation
identity.

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

Before the first effect, changed desired bytes, artifacts, authority-bearing
live state, funding sufficiency or the live Cycles Ledger fee stop apply and
require a new plan. Live controlled balances may move up through refunds or
down through execution burn only within the reviewed per-canister observation
bound and only while the normalized action graph and funding authority remain
identical. The accepted apply-time balances become the journal's truthful
initial conservation evidence; movement outside the bound rejects before any
effect. Once the journal is in progress,
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
`.canic/fleet-ensure/<environment>/<fleet>/` and resumes that action before
opening another. The stall budget counts only consecutive non-progress.
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

If an exact retained Root is stopped, planning uses management status before
calling its protected role endpoint. Exact current state may defer observation
of that Root's already-bound Store and pool children only long enough to review
one same-Principal `Start` action. Child, Root, parent, topology and controller
bindings must all match, and no other mutation is admitted from deferred
evidence. After the Root starts, ordinary protected observation resumes.

The reviewed prerequisite embeds the complete generator authority and still
permits only `Start`; it cannot install either module or authorize a paid
effect. Apply revalidates stopped status, Principal, Subnet, controllers and
predecessor module before issuing that Start. A missing, changed or unrelated
live module fails before a plan or effect. Terminal replay reuses the embedded
authority and cannot issue a second Start.

The resulting corrective graph reinstalls the Coordinator, replays the exact
Root-owned Store-controller adoption, reinstalls the Store, and only then
reinstalls the Root. All later protocol work stays fenced until ordinary
protected observation resumes. Reinstall intent records the management
canister version before the effect; terminal observation requires the requested
module at a strictly newer version. A same-module predecessor therefore cannot
be mistaken for an applied reinstall, including after process restart or a
lost response.

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

Fleet Ensure no longer installs a temporary recovery canister. Direct pool
creation and ordinary top-up target native canister balances, while
`FundEstate` alone transfers the forecast shortfall to a Root's Cycles Ledger
account. An externally created Ledger-account credit remains outside the
reviewed operation and cannot be silently consumed or substituted for its
durable transfer receipt.

## Retirement Boundary

An IC controller cannot pull cycles from an arbitrary canister. A material
source must therefore declare an idempotent, controller-authorized drain
endpoint before replacement or deletion:

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

A successor Canic runtime correction does not upgrade an already-installed
predecessor Root merely because a newer CLI reopens its issued plan. Under the
pre-1.0 reinstall-only contract, the operator must discard the predecessor's
local in-progress ensure evidence and review a new current desired-state plan.
That plan may reuse the same controlled Principals and cycle balances while
reinstalling current infrastructure artifacts; it is not a cross-release
recovery or migration contract.
