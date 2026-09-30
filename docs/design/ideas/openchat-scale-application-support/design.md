# Idea: OpenChat-Class Application Support

Date: 2026-09-30

**Product direction and planning status**

The maintainer requires Canic to support an application like OpenChat. Treat that as a product capability objective: a large, persistent, multi-subnet application with heterogeneous canisters, shared and dedicated tenancy, substantial background traffic, and governed operations. Supporting the vocabulary in configuration is insufficient; the operating behavior needs reproducible qualification.

This note turns the [OpenChat source review](../../../forum/openchat-and-canic.md) into proposed work packages. The product direction is maintainer-requested; the package ordering below is a recommendation, not an accepted implementation queue or numbered release allocation. It does not change the open 0.110 batch, the human minor-closeout gate, or the selected 0.111 blob extraction. No OpenChat modification or production deployment is authorized.

**Maintainer direction: make OpenChat adoption easy**

The September 30 direction is to build towards infrastructure that OpenChat has a
compelling reason to adopt, with a small and reversible evaluation commitment.
Canic should aim to outperform the incumbent in operating cost, reliability,
maintenance burden and usability. Those are engineering targets to demonstrate,
not a present superiority claim. OpenChat's confidence in its existing system is
a baseline requirement, not a reason to abandon the objective.

The adoption experience is part of the product: useful tooling before runtime
integration, clearly separable capabilities, familiar infrastructure boundaries,
and an explicit account of the work each capability removes. A mandatory rewrite
of OpenChat's application, controller hierarchy, storage or treasury before the
first useful result would fail this objective. The future runtime-adoption goal
remains subject to the lifecycle boundary below; this note does not design or
authorize adoption of an existing installation under today's pre-1.0 policy.

**Developer feedback received September 30**

Source: the maintainer supplied an OpenChat developer's message in this session.
The statements below are developer-reported operational facts and plans, separate
from the September 29 pinned source review. The message's original date and a
public permalink were not supplied. Its reference to starting migration
“tomorrow” is not evidence that migration has started or completed.

Follow-up developer feedback asks us to ignore the legacy `User` canister
because it will soon be deprecated. Exclude that role from the test port,
synthetic OpenChat fixture, integration adapters and qualification targets.
Use `MultiUser` for user workloads; retain the required user indexes, which
are distinct roles. Group and community canisters remain in scope. This is
direction for our evaluation, not confirmation that deprecation is complete.
Recheck lifecycle, memory, timer and payload assumptions against the selected
MultiUser revision instead of transferring findings from legacy User code.

| Reported OpenChat behaviour | Consequence for the Canic objective |
| --- | --- |
| Roughly 250,000 canisters, with a reported recurring base charge of approximately 0.4T per canister-year introduced in mid-June. | Count physical canisters and fixed overhead explicitly, including infrastructure and spare pools. Do not optimise only instructions or logical tenant capacity. |
| MultiUser canisters are ready to launch, targeting roughly 10,000 users per canister and about 25 user canisters. | Make shared tenancy the primary comparison. User placement, isolation and application readiness remain application-owned. The 25 figure is a user-canister target, not the total future estate including groups, communities and infrastructure. |
| One SNS-controlled CyclesDispenser holds ICP, converts it when its cycle balance falls below a threshold, serves top-level requests and sweeps SNS-registered canisters every 24 hours. | Preserve treasury and governance ownership. Compare against an autonomous funding service, not a manual operator top-up baseline. Distinguish ICP conversion, cycles transferred, consumption and retained reserves. |
| Busy children request funding from their subnet LocalIndex during updates; LocalIndexes sweep children weekly. | Qualify prompt demand-driven funding and bounded idle-canister coverage. Neither permanent fine-grained polling nor traffic-only funding is an adequate default. |
| Wasm is relayed from top-level owners to LocalIndexes, which upgrade their children. Local creation, funding and upgrades stay on the child's subnet. | Preserve subnet locality and hierarchical distribution. Measure cross-subnet calls and bytes; do not add a central round trip to every child operation. |
| Funding and upgrades have worked for years under SNS control; fewer canisters should simplify operations further. | Demonstrate an incremental advantage over that working system. Generic top-up and installation APIs alone offer little reason to switch. |

Using the reported fee, 250,000 × 0.4T = 100,000T per year, or 100 quadrillion
cycles. Applying the same rate to 25 user canisters gives 10T per year for that
population's base charge. This is conditional arithmetic, not an observed saving
or the total future OpenChat bill. It excludes other canisters, storage, execution,
messages, transition costs and Canic overhead. A quoted initial cycle deposit is
retained spendable funding, not itself consumption; keep it separate from creation
fees and later resource charges.

The official [cycle-cost reference](https://docs.internetcomputer.org/references/cycle-costs/)
confirms the listed 0.5T creation charge for a 13-node application subnet and
subnet-size scaling. The references checked on September 30 did not independently
confirm the reported June recurring fee. Retain its provenance and request the
applicable fee schedule or measured debit before freezing a benchmark; do not
hard-code 0.4T as a universal current tariff. Use the platform's
[current cost functions](https://docs.internetcomputer.org/references/management-canister/#cycle-costs)
where available and record the subnet and fee assumptions for other charges.

**Adoption path and concrete next actions**

The initial deliverable should be useful even if OpenChat never installs Canic.
This is an adoption sequence, not a new release allocation or authority to contact
OpenChat, build its repository, change its canisters or execute a migration.

| Priority | Action and owner | Evidence of completion |
| --- | --- | --- |
| First | OC-6/Host: specify a passive report over externally supplied Wasm and proposal inputs, preserving their exact bytes. | A reviewer can compare supplied artifacts, hashes, exports and sizes without a Canic runtime, controller change, new canister or alternate release pipeline. Provenance that cannot be established is marked unknown. |
| First | OC-1/OC-2: write a dependency and integration-cost matrix for each proposed capability. | Every capability lists required crates, state/memory/timer ownership, canisters, caller permissions, configuration, operational obligations and existing code it could replace. Hidden dependencies prevent a claim of independent adoption. |
| Next | OC-1/OC-3: benchmark a shared-user estate against the reported LocalIndex topology. | Separate cost per physical canister, occupied subnet and logical user; record warm-spare cost, idle checks, active-load latency, cross-subnet traffic and failure impact. The same user's workload stays equivalent on both sides. |
| Next | OC-4: qualify reactive funding plus coarse idle coverage, with exact funding owners and finite debit authority. | Busy workers obtain timely funding; idle workers remain funded through the declared sweep/outage window; interrupted conversion or transfer cannot duplicate spending. Operator absence, depleted treasury and stale observations have explicit outcomes. |
| Next | OC-6: document the actual SNS trust boundary and testable approval/effect bindings. | No Canic operator key bypasses SNS. Existing treasury, artifact approval and subnet execution responsibilities remain explicit; support is unclaimed until the real authority path is qualified. |
| Conditional future | OC-2/OC-6: evaluate replacing one infrastructure responsibility at a time after a production lifecycle contract is separately accepted. | Each integration has one effect owner and a demonstrated reduction in total work, not two competing funding, deployment or recovery systems. No live adoption is specified or authorized here. |

For any future runtime integration, prefer preserving the CyclesDispenser and
LocalIndex responsibilities unless measurements establish a benefit from changing
them. Reusing Canic code inside an existing infrastructure owner may be a better
fit than introducing additional controller canisters, but this remains an option
to evaluate, not a claimed supported embedding API. Package boundaries and runtime
requirements must prove which capabilities can be consumed independently.

Define an adoption budget before selecting runtime work: person-days to first
useful result, application files and lines changed, additional physical canisters,
new permissions, extra idle cycles, new operational steps and systems retired.
Use OpenChat's own priorities to set acceptable values. A capability that requires
full migration to deliver any value fails the first-use objective. A generic
framework that merely moves maintenance into adapters has not reduced it.

If OpenChat is willing to provide further input, ask for the post-consolidation
physical topology, shared-worker resource targets, any remaining operational pain,
and the smallest evidence/tooling contribution they would actually use. Treat
these as validation questions, not prerequisites for writing the Canic-owned
fixture specification. Do not send messages on the maintainer's behalf.

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

An application team should retain its user model, routing decisions, membership, message ordering, storage semantics, authorization, and business transactions. For each capability selected for future adoption, Canic should remove the corresponding physical allocation, artifact qualification, infrastructure identity, funding or management-effect recovery work. Adoption must not require selecting every capability together. Each paid effect must still have one authoritative owner; an adapter cannot leave two systems independently managing the same operation.

Measure success using application-owned operational code and state machines removed, incremental runtime cost, operator interventions, recovery outcomes, and explicitly supported capacity. A smaller integration diff or successful empty-canister installation does not establish success. No savings percentage or live OpenChat fleet size is inferred from the source review. Developer-reported population and cost figures above remain explicitly attributed rather than measured evidence.

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
| Physical-topology overhead | Measure fixed cost per Fleet, occupied subnet, Component, physical worker, and logical assignment separately. Compare equivalent MultiUser populations on the upstream and Canic variants; include idle infrastructure, pool reserves, and deployment calls. Attribute the effect of each extra layer rather than assuming it is free or necessarily redundant. | OC-1, OC-2, OC-3 |
| Operational task completion | Give an operator an injected fault and the supported CLI/docs. Record elapsed diagnosis/recovery time, required observations and decisions, wrong turns, and whether private implementation knowledge was necessary. Validate that the indicated next action belongs to the actual recovery owner. | OC-5, OC-6 |

For each concern, distinguish a confirmed correctness defect, a limit of the maintained contract, a performance tradeoff, and missing evidence. An undocumented or untested property is not automatically an implementation bug. Keep claims scoped to the actual measured application profile and fault model.

**OC-1: Establish one representative qualification application**

Outcome: a Canic-owned fixture that can expose infrastructure failures before they reach a downstream app. Its profile models global directories, subnet-local indexes on two subnets, MultiUser workers, stateful workers, large-payload workers, and application-owned cross-subnet traffic. OpenChat's MultiUser/group/community/media roles motivate these workload shapes; full chat behavior is unnecessary. Logical users and channels are data, not automatically additional canisters. The legacy User role is excluded, including as a secondary comparator. The forum configuration is an illustration; its proposed storage allocator is not a required new Canic abstraction.

Owners: `canic-testing-internal` for infrastructure journeys; `canic-tests` for public consumer integration; public examples only after their surface is qualified.

Deliverables and acceptance:

- A workload manifest describes physical topology, logical tenant population, payload distributions, traffic, fault points, source/artifact hashes, and observation windows. Use current Canic-owned TOML for human-authored parameters and structured records for results.
- Start with a real two-subnet PocketIC estate of approximately 32 workload canisters. Derive test membership and totals from the manifest; distinguish user workers from groups, communities and infrastructure. Model the reported 10,000-user packing target with representative per-user state and active fractions, not empty counters. Include a hot shared worker, idle leaves and a slow or unavailable peer. Larger packing claims require measured storage, instruction, latency and failure-isolation evidence; shared tenancy increases the population affected by one worker failure.
- Exercise registration, child allocation, application initialization, a representative message exchange, and media-sized payloads through production adapters. A canister answering a health query is insufficient: application readiness must be observed.
- Keep large-scale storage/algorithm benchmarks separate: proposed population tiers are 100, 1,000, 10,000, and 100,000 physical inventory records, plus up to one million logical assignments. These are experiment inputs, not advertised supported capacities or required PocketIC canister counts.
- Publish p50/p95 operation timings, instructions, stable/heap extents, calls, bytes, queue age, and cycle observations with attribution limits. Record uncertainty instead of converting instructions into cycle savings.
- Freeze numeric cost and latency budgets after the baseline, before optimization work is selected. Until budgets and evidence are accepted, record the capability as unqualified rather than silently treating a benchmark run as a pass.

First slice: a source-level fixture specification and baseline manifest, reusing existing Canic recovery cases where they already cover a requirement. Avoid another expensive end-to-end suite that repeats infrastructure setup for every assertion.

**Real OpenChat qualification companion**

The maintainer proposed keeping a test-only copy of OpenChat and getting it
working on Canic. Add this as a proposed OC-1/OC-2 qualification lane: the
synthetic fixture provides controlled fault experiments, while real application
code exposes integration costs and assumptions that the fixture could miss.
Neither lane alone establishes production adoption readiness.

Keep a pinned upstream revision and a separately reviewable integration patch
series. Record the source URL, commit, license/notice obligations, dependency
locks, toolchain, build inputs and final artifact hashes. Verify that the chosen
revision actually contains the MultiUser path being evaluated. First reproduce
the selected upstream journey without Canic; then run the equivalent journey
with the patches applied. Keep application semantics and assertions equivalent,
and report any excluded behavior explicitly. Do not maintain an independently
evolving chat product or weaken upstream tests to accommodate Canic.

The initial scope is one complete local journey using the real MultiUser role
and its required index/registration dependencies: create synthetic users, route
them to a shared worker, exchange and retrieve messages, and observe application
readiness. Inventory dependencies before promising that this is a small port.
Then integrate one funding responsibility through maintained Canic APIs, with
one effect owner and the former funding path disabled for that responsibility.
Measure low-balance funding under traffic, idle coverage, treasury exhaustion,
interruption and duplicate requests. Existing Canic parent funding, autonomous
balance timers, Coordinator-backed Root funding and protected Root ICP refill
are baseline capabilities to exercise; hierarchical funding is not a missing
feature to recreate.

Use fresh disposable PocketIC deployments and synthetic data for both variants.
Identify memory, lifecycle, timer, codec, controller and subnet ownership before
changing initialization. Repeat at one exact Canic release for same-release
recovery evidence. Cross-release state retention and existing-installation
adoption remain outside this lane under current policy. A governance stand-in
qualifies only its caller boundary; actual SNS support still requires OC-6.

Acceptance evidence includes equivalent application results, the exact patch
footprint and integration effort, operating calls/cycles and retained reserves,
extra canisters, subnet traffic, failure outcomes and maintenance steps removed.
PocketIC comparisons establish behavior under the recorded test conditions;
they do not establish mainnet latency or cost savings. Refresh the upstream pin
deliberately and rerun both variants so patch growth and upstream divergence are
visible. Keep this expensive lane separately selectable from focused Canic tests.

The reproducible harness, source manifest and adaptation patches should live in
Canic; a generated source copy is disposable test input. This planning update
does not create or modify an external repository. If a maintained sibling copy
is selected instead, name its exact path and obtain the repository authority
required by AGENTS.md before creating it. The lane remains unscheduled pending
the same batch selection as OC-1/OC-2; no clone, port or execution is claimed here.

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
- Measure idle maintenance and watchdog costs separately against the reported reactive-request and periodic-sweep baseline. Prove idle-canister discovery, bounded sweep continuation, sufficient runway for the declared sweep/outage interval, and prompt funding under bursts. Any quiescence change must prove re-arming after demand transitions and same-release restoration without weakening recovery liveness. OpenChat's daily/weekly schedules are comparison inputs, not universal Canic constants.

First slice: add a sustained-growth/funding-pressure scenario to OC-1 using current reviewed funding boundaries. Treat autonomous funding as an explicit incumbent capability to match, while identifying which Canic operating-funding paths already provide it and which estate-replenishment or ICP-conversion paths remain deferred. Promote only the demonstrated gap under explicit bounded authority; do not substitute repeated manual funding for the reported operational contract.

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
| A: Low-friction comparative baseline | Source-backed gap register, passive OC-6 artifact-report specification, capability dependency matrix, OC-1 shared-user fixture and OC-2 composition baseline | Useful evaluation without a runtime switch; explicit incumbent/Canic owners, first-use/adoption costs, fresh-install behaviour, invalid integration cases, measured overhead, budgets and docs |
| B: Sustained same-release operation | Selected OC-3/OC-4 bottlenecks and recovery gaps | Allocation under contention and application pressure, funding interruption, cycle accounting, terminal replay, diagnostics and cleanup |
| C: Operator coverage | OC-5 dynamic inventory and bounded observation | Autonomous child coverage, partial failures, backup boundaries, CLI/JSON/docs propagation |
| D: Governed integration | Extend OC-6 passive evidence into a separately admitted authority integration | Exact SNS approval binding, unauthorized/altered/replayed requests, effect recovery and real authority-path qualification; existing-installation adoption remains blocked by the lifecycle boundary |

These are outcome groupings, not a four-release commitment. Retain narrow slices inside each accepted batch until its positive, invalid, recovery, propagation, and cleanup obligations are complete. Select performance fixes from measurements rather than scheduling every candidate mechanism. Any new persistent-production lifecycle work needs its own later decision; completing A-D cannot silently claim that requirement has been met.

**Relationship to current priorities**

- The current 0.110 runtime contraction and funding corrections supply baseline behavior and measurements. This idea does not reopen stopped work or change current batch readiness.
- The accepted 0.111 standalone blob extraction can improve the ordinary managed-service boundary. It does not need to implement OpenChat's full media semantics, and this idea does not add an OpenChat dependency to that line.
- Bounded multi-Fleet estates remain a separate deferred idea. Multiple subnets within one Fleet do not by themselves justify introducing a second Fleet or changing controller topology.
- Application durable queues remain application-owned until repeated consumer evidence justifies extracting a narrowly scoped mechanism. Canic must not grow a chat framework or imply generic exactly-once delivery.

**Next planning action**

Prepare candidate batch A for detailed scope review after the accepted sequencing
permits it. Start with the passive artifact-report specification, capability
dependency/adoption-cost matrix and source-backed gap register. Use the reported
MultiUser topology and existing SNS/local-index funding system to define the
fixture, workload, cost budgets and fault schedule. Specify the pinned real
OpenChat companion's dependencies, first application journey and comparison
protocol alongside the synthetic fixture. No production API expansion,
external build or live operation is needed for these planning artifacts.

The maintainer's requested direction is easy adoption and demonstrably better
infrastructure. Select later fixes by the measurable value of one independently
usable capability, then the total burden removed as more capabilities are chosen.
Keep the separately accepted future lifecycle contract as a hard prerequisite to
persistent runtime adoption. The next deliverable should help OpenChat evaluate
Canic with little commitment; it must not require them to accept a rewrite first.
