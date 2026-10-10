# Historical contraction plans — captured 2026-10-01

These excerpts preserve the prior active design at maintainer checkpoint
02af7277664c1664a58bfaa5e6fde997474c9802. They are historical evidence,
not implementation instructions or release authority. B3 is stopped; remaining
B4 is deferred. Completed VS1/CR1 sequencing and the old .43 next action are
superseded by the current design, tracker and handoff. Current obligations remain
in [the maintained design](../../../design/0.110-fleet-runtime-contraction/0.110-design.md).

## Prior status checkpoint

## Status

- Status: B1 accepted, B2 complete, B3 stopped and remaining B4 deferred under
  the 2026-09-25 scope amendment. B5 final-source qualification remains open.
- Purpose: create durable absolute Wasm code-section and replica-validator
  function headroom by removing unused storage, codec and whole generated-
  surface endpoint/provider/recovery reachability. This line adds no runtime
  capability.
- Predecessor gate: complete. Published `v0.109.35` retains B8-B10 and the
  passing immutable superseding complexity audit; the human maintainer
  accepted the minor closeout and explicitly promoted B1 on 2026-09-01.
- Successor: [0.111 blob service extraction](../../../design/0.112-standalone-blob-service-extraction/0.112-design.md)
  follows accepted human closeout. Multi-Fleet estates are deferred.
- Deployment boundary: 0.110 is reinstall-only and cannot delay or gate the
  already-published 0.109 Toko Miner unblock.
- Runtime impact: none from this design amendment.

Implementation status: [status.md](../../../design/0.110-fleet-runtime-contraction/status.md)

## Prior FI1 release-table entry

| FI1 | Reviewed infrastructure bootstrap and capacity import with automatic controller handoff; host/CLI, control-plane and testing owners | Exact-ID import, destructive review, retirement/authority checks, bounded recovery, inventory propagation, docs and changelog | Native authority/journal tests, PocketIC controller/reset/import/replay journey, affected Candid/artifact checks | In progress for .43: production observation/CLI review/apply, durable surveys, signed handoff, Root reset/accounting, publication/archive and certified-rejection recovery are connected. Native recovery checks and HTTP PocketIC import across distinct Coordinator/Root subnets pass. Typed bootstrap allocation fencing and the registered Prepared Root import/provisioning journey also pass. Reviewed Ensure initialization now retains the hold; shared infrastructure compilation and held-source reconciliation exclusion have native evidence. Production supplied-infrastructure initialization now passes its empty-Root/old-code/lost-install-response PocketIC journey with terminal local replay. Durable original survey and the Store/Registry successor also pass, including lost registration response recovery with the pool hold retained. The connected bootstrap CLI owner, durable estate publication, exact initial pool import and ordinary workload convergence now pass the production PocketIC journey, including interrupted publication and later local replay. Native generation/projection/recovery checks, CLI/help checks and scoped warning-denied Clippy pass. The implementation batch is ready for the maintainer release gate; final B5 and human minor closeout remain separate |

## B3: Capability-Owned Records And Conditional Codecs

Stopped for this line on 2026-09-25. The technical direction below is retained
for reconsideration only; it is not an active implementation requirement.

After B2 remeasurement, B3 may split the measured whole-world record families.
It must not replace an aggregate with an enum whose variants still instantiate
every codec.

Activation persistence becomes capability-owned concrete records, such as:

- ordinary activation;
- Root activation; and
- Wasm Store activation.

Bootstrap validates only the selected record and projects its compact phase
into transient runtime state. The projection is a cache reconstructed after
restart, never a second authority. A transition updates that cache only after
the corresponding durable transition commits successfully.

Authorization persistence separates at least:

- local application authorization and session/replay state;
- delegated-token issuer state;
- verifier/cache state; and
- Root issuer, renewal and chain-key state.

An ordinary application role must not instantiate Root renewal, chain-key
batch or issuer-policy codecs merely because another role supports them.

### Positional Codec Hard Cut

Only a record family that remains materially attributable after the preceding
remeasurement may replace named-map CBOR. Each selected stable wire type is
separate from its domain record and public DTO and has:

- a distinct magic/prefix that legacy bytes cannot match accidentally;
- codec version 1 separate from logical record-schema version 1;
- positional fields and numeric enum discriminants;
- fixed endianness for multibyte values;
- one canonical optional-field representation;
- exact maximum lengths checked before allocation;
- an explicit maximum encoded size;
- rejection of trailing bytes; and
- independently retained golden vectors not generated by the tested encoder.

No predecessor decoder, compatibility alias or fallback remains. The host and
actor lifecycle reject an attempted predecessor upgrade before interpreting
stable state or performing another effect. Direct qualification proves that
the rejected upgrade leaves code, stable state, authority and workflow effects
unchanged, and that only the authorized reinstall path can cross the hard cut.

B1 must inventory every state domain deliberately destroyed by the hard cut
and name how fresh Root, authentication, topology and application authority is
reconstructed after reinstall. Missing reconstruction authority blocks the
codec change.

### Reinstall-Only Release Enforcement

The hard cut is executable release policy, not narrative guidance. Governed
0.110 release metadata records a machine-readable reinstall-only transition
mode. Host planning and release qualification must reject an ordinary upgrade,
adoption or mixed-version transition for an already installed stateful Fleet
before issuing any install, stop, controller, funding or other platform effect.

Only the separately authorized reinstall path may cross the release boundary.
It must first prove the reviewed destroyed-state inventory, reconstruction
authority, controller authority and cycle-conservation plan. Direct negative
qualification records the typed pre-effect rejection and unchanged module,
stable state, controller, cycle and workflow observations. No generic
`upgrade` command, mode inference or fallback may silently turn the hard cut
into an upgrade attempt.

## B4: Endpoint, Recovery And Role-Capability Pruning

Remaining implementation deferred on 2026-09-25. The direction below is
retained for future prioritization; B5 records residuals rather than asserting
that all role-inapplicable machinery has been removed.

The endpoint audit maps every export to transport decoding, authentication,
admission, workflow delegation, reply encoding and diagnostic conversion.
Repeated work moves behind shared non-generic functions only when residual
evidence justifies it.

A Canic-owned wire method name has one top-level request/response type family
and one call mode everywhere it is emitted. Capability pruning may remove
variants from the ordinary managed `CanisterCommand` and `CanisterStatus`
families, but it must not reuse their method names for an infrastructure-role
DTO. Fleet Coordinator, Fleet Subnet Root and Wasm Store therefore retain
explicit role-owned command and status names; generic tooling selects them
from the verified role binding rather than probing or falling back.

Source extraction is not proof. Acceptance requires the optimized artifact to
show:

- fewer duplicate machine-code bodies;
- lower replica-limited and optimizer-defined function counts;
- no unexpected table or indirect-dispatch increase;
- instruction deltas inside the frozen allowance; and
- retained absolute byte and function reserves.

Selective `#[inline(never)]` may be tested, but it remains only when the final
optimized Wasm proves the desired size/function/instruction result.

Recovery dispatch is generated as closed direct role code. Ordinary,
automatic-top-up, delegated-token issuer and Root recovery paths do not share
one unconditional dispatcher. Only Root may reference Root issuer renewal,
chain-key batch preparation and Root installation recovery.

B4 also removes role-inapplicable metrics, commands, full configuration,
provisioning and diagnostics. Smaller generic cuts—async adapters, access
expression construction, Candid type documentation or transparent newtype
serialization—proceed only when fresh residual evidence justifies them.

Existing typed status contracts and Fleet Ensure ownership remain unchanged.

### Pool Ledger Recovery Hard Cut

The maintainer retired `pool_ledger_recovery` from the current contract in
published `v0.110.3`. This supersedes the earlier conditional B4 deletion plan;
it does not claim that the former incident-specific gate became a general Fleet
capability or that old operation evidence remains supported.

The complete feature is deleted in one current-contract cut:

- helper source, generated artifact and release-build role;
- Wasm Store template/publication entry;
- Root workflow, durable progress/receipt state, status and command endpoint;
- boundary DTOs and Candid reachability;
- host observation, planning, policy and apply support; and
- feature-specific tests, fixtures and CI ownership.

No migration, compatibility reader, alias, fallback or product protocol v2 is
permitted. This is a Canic deletion only. Current Fleet Ensure funding targets
native canister balances and never creates the accidental Ledger-account state
that motivated the helper. An externally created Ledger-account credit remains
outside the desired-state contract; 0.110 neither changes downstream code nor
treats the helper's reported roughly 195 KiB compressed artifact as part of an
unrelated frontend upload or as predicted recoverable code-section bytes.

## Macro And Generated-Surface Contraction

B1 inventories and B2 contracts the storage/lifecycle portion of public and
internal macro expansions. Remaining B4 implementation is deferred under the
2026-09-25 amendment. The selection/absence criteria below remain the technical
target for future cuts; B5 records any unimplemented portion as residual work,
not a current requirement to finish every cut. Importing a Rust macro does not
itself add runtime Wasm; the
functions, types, statics, dispatchers and serializers referenced by its
expansion determine the reachable graph. The inventory includes at least:

- `start!` and lifecycle expansion;
- memory registration and stable-storage selection;
- role endpoint generation;
- authentication and admission wrappers;
- status and Candid type-generation roots;
- metrics-provider expansion;
- command, configuration and provisioning expansion;
- timer and watchdog expansion; and
- recovery dispatch for Coordinator, Root, Wasm Store and ordinary roles.

For each macro and consuming actor, the retained inventory names the validated
role/capability input and every generated endpoint, function, type, static,
provider, dispatcher, serializer, timer and recovery root that may contribute
runtime code. Compile-only imports and declarations are distinguished from
expanded references that enter the actor's reachability graph.

Each actor expansion may generate only:

- endpoints exported by that actor;
- authentication and admission paths used by those endpoints;
- selected storage initialization, restoration and same-release recovery;
- selected timer and recovery dispatch;
- selected metrics, command, configuration and provisioning providers; and
- Candid types, construction and serialization reachable from the actor's
  exact public surface.

Generated code must not reference:

- an exhaustive all-role runtime dispatcher;
- an enum, tuple or provider aggregate that instantiates every role;
- an executable registration inventory containing unused callbacks;
- Root, Coordinator or Wasm Store workflows from an ordinary actor;
- automatic-top-up, issuer-renewal or sharding machinery unless selected; or
- Candid type construction, documentation or serialization for an endpoint
  absent from the actor.

The implementation may split macros into role/capability-specific entry points
or retain one macro that expands selectively from an exact validated contract.
Source organization and expanded-source appearance are not completion
evidence. Each canonical role must prove optimized-artifact absence plus
differential byte, replica-limited function, optimizer-defined cross-check and
indirect-dispatch evidence against its frozen baseline.

## Parallel Release-Build Work

Release-build acceleration may run alongside B2-B4 when it preserves
measurement integrity. Eligible work includes non-LTO declaration extraction,
canonical sidecar reuse, bounded per-role build context, compatible runtime
batching and measured LTO/codegen A/B experiments.

This supporting work receives no runtime-contraction credit unless a final
optimized artifact actually shrinks. A build change that alters Wasm rebases
subsequent attribution. The maintainer's 2026-09-10 amendment below schedules
validation throughput before the open patch is pushed; it does not postpone or
replace the byte/function acceptance decision with a speed claim.

## Validation Throughput Amendment (VS1)

Requested: 2026-09-10. Implementation and focused qualification are complete. The maintainer
selected speed work before pushing the open 0.110.14 batch after reporting a
roughly 1h40m wait. This overrides the earlier recommendation to publish the
bounded BF2-BF4 changes first. It stays within Canic and the current minor;
B1 acceptance and the human minor closeout remain separate.

### Evidence and objective

The retained 2026-09-09 full test run accounts for 6,322 seconds in its runner
and 6,323 seconds in the outer validation timer. The log reports package
version 0.110.12; its exact source commit and initial cache state are unknown.
It is historical routing evidence, not a benchmark of the current .14 source.
The [structured baseline](../0.110-validation-throughput/baseline.json)
retains the original log hash, timing rows and verbatim timing excerpts.

| Measured part | Elapsed | Interpretation |
| --- | --- | --- |
| Ordinary library/binary tests | 237s | Includes Cargo compilation |
| Ordinary integration tests | 148s | Includes Cargo compilation |
| Internal ordered PocketIC suite | 5,564s | 88% of the full test runner |
| Other PocketIC suites combined | 371s | Host, runtime, blob and payload proofs |
| Mixed topology, recovery and repeated same-release reset case | 26m18s | Nested within the internal suite |
| Retained-estate reinstall case | 17m08s | Nested within the internal suite |

The two named cases account for about 41% of the full runner. Their elapsed
times include artifact construction, replica execution, transport, pacing and
assertions; these are not yet separately attributed. Internal harness execution
is 5,501.66s, with approximately 62s compiling its Rust test binary. Rust/Wasm
builds also occur inside that execution, so neither "all compilation" nor "all
simulation" is established. Adjacent retained check/Clippy timers are 45s/56s.

The objective is a materially shorter complete release feedback loop with the
same required correctness evidence. Under one hour remains a planning target:
relative to this historical runner it would require saving about 2,722 seconds
(43%). Even deleting both largest cases would not achieve that target, and
deleting their required proofs is unacceptable. This is not a new CI timeout,
product limit or fixed performance rejection threshold. Report measured gains
and remaining cost; revise the plan if the evidence cannot support the target.
BF4's 1.835s extraction saving cannot close this gap.

### Execution design

Keep one serial Cargo owner and one governed PocketIC lane. Make the expensive
work explicit through the existing registered catalogue and artifact owners,
then optimize the measured repeated work inside that lane.

1. **Attribute the selected journeys.** Extend the internal runner and fixture
   reporting with structured, monotonic phase timings for artifact resolution,
   compilation/finalization, fixture creation/restore, initial convergence,
   each reviewed reset, injected failure/reconciliation, terminal inventory,
   immediate replay and cleanup. Record parent spans so nested timings are not
   added twice. Separate transport counts/time and observation pacing from
   simulator progress where the existing boundary exposes them. Retain failure
   and interrupted-phase records as incomplete; a missing end event is never
   a success or a zero-duration sample. Keep existing human progress output.
2. **Resolve each required artifact recipe once.** Registered cases declare
   their exact configuration, role set, network, profile, features, protocol
   and release-identity requirements. The selected catalogue derives their
   union; an exact-case run resolves only its own requirements. Reuse the
   existing verified sealed-artifact cache and Cargo input resolver, rather
   than introduce another fingerprint authority. The existing two distinct
   reinstall identities stay distinct. Validate the complete output set before
   a case starts and expose hits, misses and rebuild causes. Cold cache fill
   remains inside the reported command time. Recipes intentionally exercising
   a build failure or invalidation own that build inside their case.
   Resolve native host producer inputs separately from canister inputs with the
   same Cargo resolver. Omit standalone `cfg(test)` Rust modules only when they
   are not embedded producer inputs; retain production sources, manifests,
   locks and directory membership. Ambiguous includes or module paths retain
   the complete host input set. The full producer snapshot still guards source
   changes during cache acquisition and publication, including omitted tests.
   Inline tests remain bound. Test results are never cached.
3. **Separate setup from the invariant under test.** Use the existing baseline
   pool and prepared-funding approach for repeated prerequisites only. Register
   the mixed-topology fresh-start proof, same-release reset/retry proof and
   retained-estate capacity proof with explicit coverage ownership. The current
   mixed-topology case performs fresh convergence followed by two deliberate
   wipes; expose those phases before deciding which setup can be reused. A
   fresh-start test must still execute creation and activation. A reset test
   starts from a real working Fleet, wipes real application rows, and proves
   retry cannot wipe newly inserted rows. A second deliberate reset retains
   its distinct operation identity. The maintainer-authorized 2026-09-27 hard
   cut replaces both nineteen-Workload/five-Ready reset fixtures with two
   Workloads and one Ready asset. Real deployment, exact import completeness,
   controller rejection, funding interruptions, lost reinstall responses,
   successor convergence, conservation and replay stay in those journeys.
   Underfund both Workloads so the one Ready asset cannot mask recovery funding.
   Native policy checks own exact-fit, insufficient-reserve and overflow capacity
   arithmetic, including 19 + 5. Audit-size Ledger/reset cohorts and their
   experiment-budget arithmetic are outside the default product gate. Maintained
   Ledger/CMC replay, pool funding, import/reset and controller-routing proofs
   retain their real-canister coverage. The removed 27-canister deployment size is no longer a
   recovery acceptance requirement. The five-role mixed topology and both of
   its deliberate wipes remain unchanged.
4. **Remove measured redundant observations within one decision.** If the
   traces show repeated status/transport work, pass one typed observation to
   consumers of that same decision. Existing four-wide terminal reads are
   already implemented and do not count as new savings. Invalidate observations
   after every effect, uncertain response, retry or new decision; terminal and
   immediate-replay checks must obtain fresh authority and cycle evidence.
   Do not replace production transport or pacing with a mock to make the
   production-adapter proof faster. Keep any proven host fix within its owning
   package and qualify transport failure, authority drift and error precedence.
5. **Avoid measured launcher overhead.** The installed npm ICP launcher and
   native executable contain the same pinned binary, but repeated Node startup
   materially increases production-adapter test time. Select an installed
   native ICP executable inside the governed invocation's private scratch.
   Preserve explicit native selection, reject missing/mismatched tools and
   retain every command's version and transport checks. Keep ordinary and
   plan-only lanes independent of this selection. No global installation or
   production CLI behavior changes.

Artifact reuse crosses invocations only for immutable, completely verified
build products. Mutable PocketIC state, plans, journals, identities and fault
controls remain invocation-local. A reusable baseline includes coherent replica
and host fixture state at a named checkpoint, with exclusive ownership during
destructive work. If that coherence cannot be established, rebuild the fixture.
Never restore only the canisters while retaining a later host journal. Dirty or
failed leases are discarded. Tests must not manufacture completed production
journals or bypass the admission/effect being qualified.

### Coverage and failure contract

The registered catalogue is the membership authority. Give each required
distributed invariant an exact registered owner and validate non-emptiness,
unique identifiers, required coverage and prerequisite ordering. Do not encode
an aggregate test count or parse narrative descriptions as a release contract.
Splitting or consolidating cases must preserve these properties:

- canonical infrastructure, mixed-role startup and runtime activation;
- large retained inventory, exact controllers and module/release bindings;
- funding review, paid-effect bounds and cycle conservation;
- interruption before effects and lost replies after effects;
- reconstruction of the production adapter and same-operation reconciliation;
- stable-row wipe, preservation of newly written rows on retry, and a new
  deliberate wipe with a distinct operation identity;
- Store outage and the exact originating Root failure at Coordinator, followed
  by recovery and cleared terminal failure; and
- fresh terminal verification and immediate effect-free replay.

Pure decision permutations may move to native tests. Creation, installation,
inter-canister behavior and the production transport boundary remain PocketIC
proofs. A coverage change needs a before/after invariant map, not a claim that
fewer tests are automatically equivalent.

Cache qualification includes changed source/config/tool/release identity,
missing or corrupt output, an interrupted fill and a valid retry. Fixture
qualification includes a mutated baseline, failed lease, mismatched host state
and running the selected case alone as well as through its declared sequence.
No test success receipt is reused merely because an artifact cache hits.

### One batch, ordered slices and closeout

VS1 is one pre-push throughput batch; these steps are not separate patches:

| Slice | Owner | Delivery and focused evidence |
| --- | --- | --- |
| A: attribution | Internal test harness and runner | Structured phase records; exact selected-case coverage; success, failure and interrupted-record checks; current-source baseline for the two dominant journeys |
| B: artifact/setup reuse | Canic testing artifact and fixture owners | Deduplicated exact recipes; cache invalidation/retry; coherent fixture isolation; cold and warm selected-journey comparisons |
| C: dominant residual | Testing owners; host observation owner only where measurements justify it | Remove repeated prerequisites or observations identified in A/B; retain invariant mapping, production-adapter failures, conservation and replay; repeat the affected journeys and package lint |
| Closeout | Same owners and active documentation | Final-source timing/coverage receipt, exact command/cache/machine context, resource observations, draft changelog and removal of superseded fixture paths |

First run the governed exact-case path for the mixed-topology and reinstall
cases with attribution, then select the largest demonstrated removable cost.
Do not implement a speculative host or identity redesign before this evidence.
Compare old/new source on the same host with the same selected cases, fixture
sizes, tools and build profile, separating empty private artifact-cache runs
from warm runs and recording the Cargo target's initial state. Use owned
scratch; do not clear a shared target to manufacture a cold result. Record
source/input hashes, raw elapsed values and memory/process high-water marks.
Where host load or timing variation obscures improvement, repeat only the
affected comparison and retain the variation.

Focused checks must show retained coverage and a reduction in the measured
dominant work before this expanded batch is called ready to push. A completed
instrumentation slice alone does not satisfy the requested speed outcome.
Publication notes describe implemented changes only. A complete new gate time
remains unclaimed until the maintainer-directed release validation measures it;
agents do not pre-run a broad suite during implementation. That unrun gate is
not by itself a focused-closeout blocker.

Do not introduce concurrent PocketIC lanes, alternate runtime release identity,
weaker optimization profiles, skipped recovery cases or cross-commit test-result
reuse in VS1. Concurrent lanes would require a separately measured resource and
global-state isolation design: current fixtures change process working
directory and share scratch conventions, so extra test threads are unsafe.
Existing serial governance stays in force. No Toko Miner, IcyDB or ic-testkit
repository changes are included.

The [final qualification](../0.110-validation-throughput/report.md)
records both exact journeys on unchanged source, native authority/failure
regressions and scoped warning-denied lint. Existing verified recipe resolution
already reuses exact artifacts across invocation roots; sub-second replica setup
does not justify additional mutable fixture pooling or catalogue splitting.
Native CLI selection and same-observation inspection reuse address the measured
dominant removable work. This completes VS1's bounded outcome. CANIC-162 was
briefly deferred while published memory dependencies differed. IcyDB 0.257.4
now shares ic-memory 0.13.2, and the maintainer requested adoption; its restored
source receives fresh composition qualification in the
[assessment](../../reports/2026-09/2026-09-10/canic162-memory.md).

## Next Authorized Action

With `.42` published, continue implementing the accepted FI1
batch targeting `.43`. Preserve accepted B1/B2 and `.42` B5 records and qualify
the final import delta, residual disposition and confirmed in-scope defects. Do not
resume stopped B3 or deferred B4 automatically. Broad validation and release
commands retain their explicit maintainer boundary. Once B5 is ready, the
next gate is the human-requested 0.110 closeout audit and acceptance; 0.111
implementation remains blocked until that gate is satisfied.


## Canonical Fleet Subnet Root Batch (CR1)

The maintainer selected this next in-repository batch on 2026-09-07 after
pushing 0.110.9. It stays in the current 0.110 line and does not cross the
human minor closeout boundary or promote unrelated contraction work.

Outcome: Canic owns all three Fleet infrastructure entrypoints:
`canic-fleet-coordinator`, `canic-fleet-root` and `canic-fleet-wasm-store`.
Root source and lifecycle are canonical; App configuration and selected
capabilities remain exact compiled inputs. Runtime orchestration stays in
`canic-control-plane`. The 2026-09-08 accepted refinement generates all three
unpublished entrypoint packages through one host owner and removes the separate
published Fleet packages and source-discovery alternatives. Canonical
Coordinator/Store Candid ships with `canic`; the package proof covers all three
artifacts from one isolated source set.

CR1 includes automatic workspace/packaged Root building, rejection of custom
Root package selection, removal of Root application lifecycle hooks, canonical
artifact and release-set authority, publication inventory, application-neutral
fixtures, CLI/help and active documentation. Application canister hooks remain
owned by their existing lifecycle. Tests needing instrumentation use explicitly
unpublished fixtures; deployment proofs use the canonical Root artifact.

Required focused evidence: configuration/package/capability selection and
invalid authority; exact Candid and sealed artifact bindings; a packaged
consumer without a Root package; fresh Fleet convergence; explicit reinstall;
same-operation interruption recovery, cycle conservation and effect-free replay.
Reuse the existing focused journeys. No new runtime recovery mode, compatibility
surface or custody machinery is included. All acceptance and propagation work
forms one batch; release/version/publication actions remain separately owned.
