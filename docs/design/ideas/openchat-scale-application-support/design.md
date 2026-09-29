# Idea: OpenChat-Class Application Support

Date: 2026-09-29

**Product direction and planning status**

The maintainer requires Canic to support an application like OpenChat. Treat that as a product capability objective: a large, persistent, multi-subnet application with heterogeneous canisters, shared and dedicated tenancy, substantial background traffic, and governed operations. Supporting the vocabulary in configuration is insufficient; the operating behavior needs reproducible qualification.

This note turns the [OpenChat source review](../../../forum/openchat-and-canic.md) into proposed work packages. The product direction is maintainer-requested; the package ordering below is a recommendation, not an accepted implementation queue or numbered release allocation. It does not change the open 0.110 batch, the human minor-closeout gate, or the selected 0.111 blob extraction. No OpenChat modification or production deployment is authorized.

**Comparison scope: scaling and infrastructure only**

The maintainer explicitly limits this comparison to the scaling and infrastructure elements inside OpenChat. OpenChat is a chat application, not a competing general-purpose framework. Compare responsibilities, not whole products or repository size. Its local/global indexes can mix application and infrastructure concerns: replacing allocation or funding machinery does not imply deleting the index, its user directory, or its business rules.

In scope: physical canister allocation and pools; subnet placement and capacity admission; infrastructure discovery; artifact distribution and management effects; cycle funding/accounting; infrastructure queues, timers, recovery and throttling; lifecycle composition; operational observation; same-release backup orchestration; and governance authorization of infrastructure effects. Application storage layout and wire protocols are integration constraints, not a mandate for Canic to replace them.

Out of scope: chat functionality, UI, membership/moderation policy, search quality, message semantics, media product features, token economics, and application identity design. Use representative payloads and traffic only to measure infrastructure behavior. Application data migrations remain application-owned; the future infrastructure lifecycle requirement must support the application's chosen process rather than prescribe its schemas.

The qualification application should be the smallest workload that exposes those infrastructure requirements. It need not implement a chat product. Retain only enough application state, traffic, and readiness logic to detect an infrastructure regression.

There are three distinct outcomes:

- **Build:** implement an OpenChat-class application from a fresh installation without maintaining another complete fleet-management framework.
- **Operate:** grow, fund, observe, recover, and eventually release that persistent application without losing its application invariants.
- **Adopt:** move an existing OpenChat deployment onto Canic. This is a separate future undertaking with its own state, identity, authority, governance, and application requirements.

Fresh-install support is a useful first qualification target. It does not establish persistent production readiness. Even a newly built application eventually needs a subsequent release.

**Lifecycle boundary that must remain visible**

Current pre-1.0 releases remain reinstall-only. Proposed implementation work in this note is limited to fresh installation and same-release operation/recovery. State-preserving releases, mixed-version operation, existing-installation adoption, and cross-release authority continuity are unmet future production requirements, not implementation specifications here. Backup/restore and version pinning do not resolve them. The cancelled stateful-adoption proposal remains cancelled.

Before claiming persistent production support, the maintainer must make a separate future lifecycle decision and accept the corresponding new design. That decision needs to define what survives a release, which owner proves application readiness, and which interrupted obligations remain authoritative. It must address the application as well as Root, Coordinator, and Store. This note neither grants a pre-1.0 exception nor prescribes an upgrade mechanism.

**Success means replacing operational work, with evidence**

An application team should retain its user model, routing decisions, membership, message ordering, storage semantics, authorization, and business transactions. Canic should own physical canister allocation, artifact qualification, protected infrastructure identity, admitted funding, bounded management effects, and their recovery. The application must not need a second implementation of those same management operations.

Measure success using application-owned operational code and state machines removed, incremental runtime cost, operator interventions, recovery outcomes, and explicitly supported capacity. A smaller integration diff or successful empty-canister installation does not establish success. No savings percentage or live OpenChat fleet size is inferred from the source review.

**Comparative audit: establish a compelling reason to choose Canic**

The maintainer clarified that the objective is infrastructure strong enough that an OpenChat-class team has a compelling reason to use it instead of maintaining its own. Capability qualification alone does not establish that advantage. Audit each infrastructure responsibility against an incumbent implementation, expose disadvantages, and use the results to prioritize improvements. The verdict must be allowed to favor the incumbent; a predetermined Canic win would make the audit unusable.

“Better in every conceivable way” is an ambition, not a defensible audit conclusion. Safety, latency, memory, flexibility, and operating cost can trade off. The decision standard is: preserve all mandatory application guarantees; meet the selected workload's cost and capacity budgets; demonstrate material reductions in operational work or risk; and disclose every material disadvantage. An accepted tradeoff must remain visible, not be relabeled superiority. No aggregate score may hide a failed safety, lifecycle, or governance requirement.

The comparison should cover:

| Dimension | Evidence needed for an advantage claim |
| --- | --- |
| Safety and recovery | Same effect boundaries and fault schedule; observed authority, retained intent, uncertain-outcome handling, bounded debit, and terminal replay. Fewer documented invariants in the incumbent are not proof of a defect. |
| Performance and capacity | Equivalent application behavior, topology, traffic, payloads, retention, and assurance requirements; latency distributions, throughput, limits, and recovery time across selected sizes. |
| Economics | Steady-state idle/active costs, infrastructure canisters, pool reserves, storage, monitoring, installation, failure/retry costs, and operator work. Treat locked reserves separately from consumption. |
| Integration effort | An executable consumer integration; all adapters, configuration, generated code, dependency constraints, and application rewrites counted. Document which infrastructure implementation can actually be deleted. |
| Operational simplicity | Task-based evidence for setup, growth, diagnosis, funding exhaustion, interruption, and recovery; count required interventions and unresolved manual decisions. |
| Governance and trust | Approved artifact/effect binding, principal/controller requirements, observation access, and no additional unreviewed operator authority. |
| Maintainability | Number and ownership of durable protocols and recovery state machines, dependency/update burden, understandable errors, documented extension boundaries, and realistic support obligations. |
| Persistent application lifecycle | Explicit unmet requirement under current policy; no persistent production recommendation until a separately accepted future contract and implementation qualify it. |

Use two distinct comparators. First compare against the incumbent's actual supported contract to identify missing capability or integration regressions. Then compare like-for-like costs for equivalent guarantees. An additional Canic safety guarantee may justify overhead, but its value and cost must both be reported. Do not weaken one side's behavior to manufacture a performance win.

Freeze source revisions, artifact hashes, dependencies, workloads, and measurement conditions. Set acceptance thresholds before evaluating the candidate result; retain baseline and candidate repetitions, failed runs, variability, and limitations. Separate source inspection, real execution, and synthetic projection. Synthetic records do not prove an equally large running fleet. Missing incumbent benchmark access leaves the result unmeasured; it cannot become an inferred Canic win. Keep OpenChat and other external repositories read-only, and seek separate authority for any external build or modification needed for later comparisons.

Produce one finding per responsibility with these fields: requirement, incumbent owner/source, Canic owner/source, workload and fault boundary, evidence type/location, result (`advantage`, `parity`, `gap`, `tradeoff`, or `unmeasured`), severity, adoption impact, proposed fix, and exact requalification. Record mandatory unsupported capabilities as blocking gaps. Report results separately for fresh installation, same-release operation, and existing-application adoption.

The practical adoption test is that a downstream team can remove its corresponding infrastructure implementation and operate through the supported Canic surface with lower total burden. Counting deleted source while adding equivalent application adapters or manual runbooks fails that test. A greenfield qualification does not prove existing-installation adoption is economical or safe. Existing users also need evidence that expected ongoing benefits justify integration cost and transition risk; that remains a separate future evaluation, not a current migration design.

The first comparative deliverable should be a source-backed gap register, not another feature list. Rank fixes in this order: mandatory correctness/lifecycle/authority gaps, inability to meet the selected capacity or cost budget, integration burden that prevents deleting incumbent infrastructure, then optional advantages. Every accepted fix belongs to a complete batch and returns to the same audit criteria. A later independent review of the evidence should challenge the recommendation before Canic claims an adoption advantage.

**Additional infrastructure questions the audit must answer**

These extend the comparative criteria within OC-1 through OC-6; they do not create another implementation queue. The tests below are proposed acceptance experiments, not claims that either implementation already passes or fails them.

| Concern | Concrete question and experiment | Work-package owner |
| --- | --- | --- |
| Control-plane dependency on ordinary traffic | Trace which ordinary application calls need Root, Coordinator, Store, or Host participation. Make each dependency unavailable independently and record which existing-workload operations continue, which provisioning/administrative operations stop, and why. Do not promise blanket independence for workloads that actually require an unavailable authority. | OC-1, OC-2, OC-3 |
| Failure isolation | Fail one subnet, requester, funding owner, or artifact source while another admitted operation runs. Identify actual shared authority dependencies and the affected population; distinguish authorization loss from a transient outage. Shared infrastructure should not accidentally enlarge the failure beyond those dependencies. | OC-3, OC-4 |
| Stability above capacity | Increase offered allocation/funding/observation load past sustainable throughput, then remove it. Measure admitted throughput, rejection/backoff, pending bytes, retry amplification, fairness, and time to settle. Bound queued debt as well as active concurrency; a bounded worker pool with an unbounded backlog is insufficient. | OC-3, OC-4 |
| Long-running state growth | Run repeated same-release allocate/settle/recycle cycles while holding the live population roughly constant. Track receipts, tombstones, journal history, indexes, diagnostics, and physical stable pages. Explain retained historical growth, reuse, and capacity exhaustion; do not claim physical memory shrinks because logical records were deleted. Never reclaim genuinely unfinished obligations. | OC-3, OC-4, OC-5 |
| Artifact-distribution economics | Install the same qualified artifact into many fresh workers. Measure compilation/finalization count, Store writes, uploaded/downloaded bytes, management calls, peak artifact storage, and time per additional worker. Include multiple role artifacts and interrupted publication; cleanup must retain bytes still required by exact retries. Keep this experiment within fresh-install/same-release policy. | OC-1, OC-3, OC-6 |
| Operator absence | Close the CLI and interrupt host collection after convergence. Prove which admitted runtime allocation, top-up, and recovery work remains autonomous, and which budget/authority boundaries correctly require the next operator review. Report interventions per unit of sustained growth; automation must not gain unlimited spending authority. | OC-4, OC-5 |
| Physical-topology overhead | Measure fixed cost per Fleet, occupied subnet, Component, physical worker, and logical assignment separately. Compare dedicated and shared-worker populations using the same application load; include idle infrastructure, pool reserves, and deployment calls. Attribute the effect of each extra layer rather than assuming it is free or necessarily redundant. | OC-1, OC-2, OC-3 |
| Operational task completion | Give an operator an injected fault and the supported CLI/docs. Record elapsed diagnosis/recovery time, required observations and decisions, wrong turns, and whether private implementation knowledge was necessary. Validate that the indicated next action belongs to the actual recovery owner. | OC-5, OC-6 |

For each concern, distinguish a confirmed correctness defect, a limit of the maintained contract, a performance tradeoff, and missing evidence. An undocumented or untested property is not automatically an implementation bug. Keep claims scoped to the actual measured application profile and fault model.

**OC-1: Establish one representative qualification application**

Outcome: a Canic-owned fixture that can expose infrastructure failures before they reach a downstream app. It models global directories, local indexes on two subnets, dedicated tenants, shared-tenant workers, stateful workers, large-payload workers, and application-owned cross-subnet traffic. OpenChat's user/group/community/media roles motivate these workload shapes; full chat behavior is unnecessary. Logical users and channels are data, not automatically additional canisters. The forum configuration is an illustration; its proposed storage allocator is not a required new Canic abstraction.

Owners: `canic-testing-internal` for infrastructure journeys; `canic-tests` for public consumer integration; public examples only after their surface is qualified.

Deliverables and acceptance:

- A workload manifest describes physical topology, logical tenant population, payload distributions, traffic, fault points, source/artifact hashes, and observation windows. Use current Canic-owned TOML for human-authored parameters and structured records for results.
- Start with a real two-subnet PocketIC estate of approximately 32 workload canisters. Derive test membership and actual totals from the manifest. Include both shared and dedicated tenancy, a hot destination, idle leaves, and a slow or unavailable peer.
- Exercise registration, child allocation, application initialization, a representative message exchange, and media-sized payloads through production adapters. A canister answering a health query is insufficient: application readiness must be observed.
- Keep large-scale storage/algorithm benchmarks separate: proposed population tiers are 100, 1,000, 10,000, and 100,000 physical inventory records, plus up to one million logical assignments. These are experiment inputs, not advertised supported capacities or required PocketIC canister counts.
- Publish p50/p95 operation timings, instructions, stable/heap extents, calls, bytes, queue age, and cycle observations with attribution limits. Record uncertainty instead of converting instructions into cycle savings.
- Freeze numeric cost and latency budgets after the baseline, before optimization work is selected. Until budgets and evidence are accepted, record the capability as unqualified rather than silently treating a benchmark run as a pass.

First slice: a source-level fixture specification and baseline manifest, reusing existing Canic recovery cases where they already cover a requirement. Avoid another expensive end-to-end suite that repeats infrastructure setup for every assertion.

**OC-2: Qualify an affordable application integration boundary**

Outcome: one realistic application role can use Canic without accidental wire-contract changes, competing memory managers, lifecycle owners, or unmeasured timer costs.

Owners: `canic`, `canic-core`, `canic-macros`; shared memory and timer providers retain their own ownership. Changes in external provider repositories require separate authority.

Deliverables and acceptance:

- Inventory the exact dependency identities, memory declarations/geometry, lifecycle exports, timer providers, endpoint codecs, exported names, and payload ceilings before selecting the integration. Explain which facts are statically provable and which require execution evidence; do not promise arbitrary macro inspection from Cargo metadata.
- Use one physical stable-memory owner and one qualified timer composition. Diagnose conflicting owners early. Do not add an implicit fallback or silently reinterpret an existing allocation layout.
- Qualify application-owned custom encoding, a normal small update, representative large application payloads, and exact admitted/excess boundary cases in real Wasm. Check ingress and inter-canister paths independently, including authentication and pre-decode rejection.
- Prove synchronous restoration precedes deferred work, and reject or explicitly restructure consumers whose decoding schedules work before their state is ready. Same-release restoration must preserve business demand without duplicate timer ownership.
- Measure lean-leaf, busy shared-worker, application-index, and infrastructure profiles independently. Optional diagnostic surfaces must not become prerequisites for functional correctness.
- Publish a consumer guide and generated declarations matching the qualified artifact. A configuration file pointing at an unmodified foreign crate is not an integration proof.

First slice: a custom-codec, application-storage, native-timer fixture with an explicit ownership map and one successful/one rejected payload journey. Reuse the existing lifecycle participant and payload-contract machinery before introducing APIs.

**OC-3: Make same-release growth bounded and fair under load**

Outcome: one busy Component cannot monopolize a subnet Root, and increasing Fleet population does not turn each ordinary operation into a complete estate scan.

Owners: `canic-control-plane` for Root/Coordinator workflows and durable registries; `canic-core` for contracts and pure admission policy; `canic-host` for observation and reconciliation.

Deliverables and acceptance:

- Publish capacity separately for top-level Components, descendants, registry bytes, logical assignments, pending operations, retained receipts, and ready pool assets. Report the first limiting resource and its exact owner.
- Keep logical tenant allocation in application policy. Physical capacity admission must distinguish a ready slot from merely available cycles; respect shared-worker application readiness and temporary admission refusal.
- Measure indexed/paged access against the synthetic tiers. Document complexity and bytes touched, and identify any complete-Fleet operations explicitly. Population changes during traversal must be detected or represented by a defined revision/freshness contract.
- Bound concurrent management work per Root and requester, preserve fairness, and expose starvation/backpressure reasons. Dispatch independent work only where existing authority permits it; serialize effects that share an operation or accounting owner.
- Exercise simultaneous allocation, capacity exhaustion, failed initialization, registry-byte exhaustion, duplicate demand, and an unavailable subnet. Unrelated authorized work must progress where its dependencies permit; incomplete global work must remain visibly incomplete.
- Reject requests before paid effects when their known admission limits fail. Do not raise a bound to conceal a scan or storage problem.

First slice: benchmark the existing child registry, publication/directory traversal, and one concurrent allocation workload; fix only demonstrated bottlenecks. Existing bounded paging is the baseline, not a feature to reinvent.

**OC-4: Keep funding and recovery effective during sustained operation**

Outcome: growth and retries have known funding owners, bounded debit, observable pauses, and a recovery path even when application traffic continues.

Owners: control-plane workflows and accounting records; Host/CLI for reviewed funding and operator diagnostics. This extends current safety owners rather than introducing a second funding subsystem.

Deliverables and acceptance:

- Distinguish operator balances, Coordinator operating cycles, Root operating cycles, estate Ledger funds, ready-asset reserves, and child balances. Adequate total cycles do not establish spendable funds at the required owner.
- Forecast setup, workload installation, ongoing reserves, and admitted growth separately. Report unresolved amounts explicitly and identify the next reviewed action when a budget is exhausted.
- In PocketIC, lose replies around funding, allocation, install, and application registration; reconstruct the same-release owner and reconcile the exact operation. Prove bounded paid effects, conserved observed cycles, and immediate effect-free terminal replay.
- Combine a failed owner with continuing application traffic and another requester. Test stale callbacks, expired leases, repeated transient failures, and application pressure that deliberately pauses nonessential work.
- Expose durable pending demand, next admissible retry, dependency, and accounting uncertainty. Scheduler activity and callback counts are not evidence of progress.
- Measure idle maintenance and watchdog costs separately. Any quiescence change must prove re-arming after every demand transition and same-release restoration without weakening recovery liveness.

First slice: add a sustained-growth/funding-pressure scenario to OC-1 using current reviewed funding boundaries. Reconsider autonomous replenishment only if operator-intervention measurements establish a need and an explicit bounded authority is accepted.

Related existing ideas: [demand-driven pool maintenance](../demand-driven-canister-pool-maintenance/design.md), [estate budget replenishment](../estate-budget-replenishment/design.md), and [cross-subnet transport groundwork](../cross-subnet-data-transport-groundwork/design.md). Their promotion remains separate; merge overlapping outcomes into their existing owners rather than implementing duplicate mechanisms.

**OC-5: Make dynamic-fleet operations usable without scanning everything**

Outcome: an operator can identify what is stuck, why, and which current authority can act on it, including children created after initial Fleet convergence.

Owners: `canic-host`, `canic-cli`, `canic-backup`, and the authoritative control-plane inventory APIs.

Deliverables and acceptance:

- Trace current child discovery through inventory, status, observation, and backup selection. Explicitly qualify whether post-convergence autonomous children are covered; a retained initial plan is not proof of complete live inventory.
- Provide bounded filtering/paging and aggregate counts by subnet, role, and operation state. Avoid principal-per-series metric cardinality and unconditional polling of every leaf.
- Connect one operation identifier across host plans, Root/Coordinator records, child readiness, funding, and retained evidence. Expose an actionable current retry owner without inventing new authority from a diagnostic report.
- Qualify partial subnet failure, stale pages, concurrent child creation, and missing bindings. Unknown does not become zero, healthy, or empty.
- Define same-release backup membership and quiescence requirements for a changing estate. Individually valid canister snapshots must not be advertised as an application-consistent distributed checkpoint unless the application's coordination protocol proves that property.
- Ship CLI/JSON surfaces, help, runbooks, fixtures, and propagation with the implementation batch.

First slice: an inventory-coverage audit that follows a newly allocated child through all operator consumers, followed by the narrowest confirmed gap and its regression.

**OC-6: Connect artifacts and governed effects without assuming operator ownership**

Outcome: a governance-controlled application can review exact artifact/effect authority and observe its outcome without relying on an operator key that bypasses governance.

Owners: Host for artifact evidence and proposal preparation; control-plane endpoints/workflows for effect authorization; the actual governance system remains external authority.

Deliverables and acceptance:

- First provide passive inspection of externally built immutable Wasm, with provenance limitations, export/size evidence, and the exact supplied hash. Never modify those bytes while claiming to attest to the input artifact.
- Keep compiled templates, bound/finalized artifacts, approved bytes, and observed installed modules distinct. All selected transforms and release-binding inputs belong in reproducibility evidence.
- Before designing an SNS adapter, freeze the target trust model: who authorizes, who executes, which callers may observe, what targets/effects/debit are approved, and how revocation or changed authority affects genuinely unfinished work.
- Qualify fresh-install and same-release governed effects with altered-artifact/target rejection, unauthorized callers, duplicate delivery, interruption, and exact reconciliation. A UI approval or successful proposal submission is not proof of remote completion.
- Use a real SNS test deployment or its actual supported authority path for an SNS support claim. A test principal standing in for governance qualifies only the generic caller boundary.

First slice: passive artifact comparison tied to one reviewable effect description. Governance execution is a later dependent slice, not an assumed consequence of having a reviewed Host plan.

**Recommended sequence and complete-batch boundaries**

| Candidate batch | Bounded outcome | Dependency / exit evidence |
| --- | --- | --- |
| A: Comparative consumer baseline | Source-backed comparative gap register, OC-1 fixture contract, and OC-2 composition baseline | Explicit incumbent/Canic responsibilities and evidence gaps, fresh-install behavior, invalid integration cases, measured overhead, budgets, docs and bindings |
| B: Sustained same-release operation | Selected OC-3/OC-4 bottlenecks and recovery gaps | Allocation under contention and application pressure, funding interruption, cycle accounting, terminal replay, diagnostics and cleanup |
| C: Operator coverage | OC-5 dynamic inventory and bounded observation | Autonomous child coverage, partial failures, backup boundaries, CLI/JSON/docs propagation |
| D: Governed integration | OC-6 immutable evidence and selected authority adapter | Exact approval binding, unauthorized/altered/replayed requests, effect recovery and real authority-path qualification |

These are outcome groupings, not a four-release commitment. Retain narrow slices inside each accepted batch until its positive, invalid, recovery, propagation, and cleanup obligations are complete. Select performance fixes from measurements rather than scheduling every candidate mechanism. Any new persistent-production lifecycle work needs its own later decision; completing A-D cannot silently claim that requirement has been met.

**Relationship to current priorities**

- The current 0.110 runtime contraction and funding corrections supply baseline behavior and measurements. This idea does not reopen stopped work or change current batch readiness.
- The accepted 0.111 standalone blob extraction can improve the ordinary managed-service boundary. It does not need to implement OpenChat's full media semantics, and this idea does not add an OpenChat dependency to that line.
- Bounded multi-Fleet estates remain a separate deferred idea. Multiple subnets within one Fleet do not by themselves justify introducing a second Fleet or changing controller topology.
- Application durable queues remain application-owned until repeated consumer evidence justifies extracting a narrowly scoped mechanism. Canic must not grow a chat framework or imply generic exactly-once delivery.

**Next planning action**

Select candidate batch A for detailed scope review after the already accepted sequencing permits it. Begin with the source-backed comparative gap register; use it to define the fixture/workload manifest, ownership map, reused-test inventory, and measurement plan. No production API expansion is needed to write those artifacts. Use the findings to decide which OC-2 integration defects and OC-3 capacity costs warrant implementation. Independently keep the future lifecycle decision visible as a prerequisite to persistent production support. Completion means evidence that supports a bounded adoption recommendation, not an assumption that a general framework must outperform a specialized application.
