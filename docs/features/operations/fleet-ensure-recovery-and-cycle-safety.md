# Fleet Recovery And Cycle Safety

Use this guide when an operation is interrupted, evidence is unreadable, a
canister must be retired, or you need the exact cycle-conservation and hard-cut
boundaries.

[Back to the Fleet Ensure overview](fleet-ensure.md).

<img src="../../../assets/256x256/mechanic-attention.png" align="left" width="96" alt="The Canic mechanic raising a hand beside recovery safety rules" />

**Recovery outcome:** interrupted or exceptional work reaches a reviewed,
cycle-accounted terminal state without duplicating effects or reviving obsolete
pre-1.0 state.

<br clear="left" />

## At A Glance

| Situation | Read |
| --- | --- |
| Account for controlled cycles | [Cycle Conservation](#cycle-conservation) |
| Remove or replace a canister | [Retirement Boundary](#retirement-boundary) |
| Move between pre-1.0 releases | [Hard-Cut Boundary](#hard-cut-boundary) |
| Retained evidence cannot be decoded | [Unreadable Retained Plan](#unreadable-retained-plan) |
| Repeat or review completed recovery | [Retained Growth And Dependent Recovery Review](#retained-growth-and-dependent-recovery-review) |

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

## Unreadable Retained Plan

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
state. Follow [clean reinstall from physical inventory](fleet-ensure-clean-reinstall.md#clean-reinstall-from-physical-inventory)
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

## Deliberate Selected-Build Database Wipe

Follow [the current clean-reinstall sequence](fleet-ensure-clean-reinstall.md#clean-reinstall-from-physical-inventory)
for both changed-build and identical-build resets, including its distinct operation
identity, reviewed phase digests, artifact retention and same-digest interruption
recovery. Infrastructure or pool clearing alone is not Fleet convergence.
Logical workload assignments may change within the reviewed Root/subnet inventory.
Ordinary Ensure does not request another wipe; a later explicit `--reinstall`
does. Do not combine `--reinstall` and `--apply`.

## Retained Growth And Dependent Recovery Review

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

### Completed Replay After Operator Account Activity

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

## Continue From Here

- [Review and apply a Fleet plan](fleet-ensure-plan-and-apply.md)
- [Follow the clean-reinstall procedure](fleet-ensure-clean-reinstall.md)
- [Return to Fleet Ensure](fleet-ensure.md)
- [Browse Fleet operations](README.md)
- [Browse all documentation](../../README.md)
- [Back to the main README](../../../README.md)
