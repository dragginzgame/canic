# Fleet Bootstrap And Capacity

Use this guide when starting with supplied infrastructure or when adding
supplied pool canisters to a current Fleet. Both operations establish reviewed
authority before ordinary Fleet convergence continues.

[Back to the Fleet Ensure overview](fleet-ensure.md).

## Supplied Infrastructure Bootstrap

`fleet bootstrap` initializes a new, untracked current-release Fleet using explicit
physical IDs. It supports an empty Root and a Store with disposable old code.
For a tracked completed estate, use the completed-estate reinstall procedure
above, which requires valid current-schema records and receipts. Bootstrap
refuses an existing tracked operation. Neither path supplies missing fields or
decodes a historical contract; see [unreadable retained plan](fleet-ensure-recovery-and-cycle-safety.md#unreadable-retained-plan)
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

## Continue From Here

- [Generate or write desired state](fleet-ensure-desired-state.md)
- [Review and apply a Fleet plan](fleet-ensure-plan-and-apply.md)
- [Read the recovery and cycle-safety rules](fleet-ensure-recovery-and-cycle-safety.md)
- [Return to Fleet Ensure](fleet-ensure.md)

