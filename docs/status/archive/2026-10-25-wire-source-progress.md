# Archived source progress — 2026-10-25

Historical milestones only. The [current handoff](../current.md) owns remaining work.

# Current handoff — 2026-10-25

## Scope and authority
- Continue the accepted 0.112 Blob/Wire hard cut; the complete batch is **not push-ready**.
- Latest request prioritizes coordinated coding and defers compilation/tests.
  Current discovery/control source changes are unqualified; earlier dated passing checks
  apply only to their recorded source and dependency graph.
- Preserve pending Backup/Auth and all unrelated work. No Git publication, release,
  deployment or broad validation is authorized by ordinary continuation.
- Check shared target ownership before source/lock edits and each focused build.
  Siblings are read-only. Changelog additions extend the existing 0.112 draft.
- Manifests/locks own versions. Selected graph: Blob/Contracts 0.47.1, Wire 0.7.10 in all three workspaces,
  Platform 0.6.16, Memory/Storage 0.51.1, Auth 0.16.1, Metrics 0.16.2, Timers 0.31.2,
  Tooling/Testkit 0.68.3, Query 0.73.0, Backup 0.44.2; Shared snapshot 0.26.10.
- [Historical handoff](2026-10-24-blob-wire-precompact.md) preserves
  superseded evidence; do not treat old dependency/blocker descriptions as current.

## Fleet state controls — 2026-10-25
- Source cut complete for Root Fleet status and automatic-funding mutation methods.
  Native replies preserve local changes, bounded cascade observations/counts and
  reconciliation errors. Existing state/cascade execution and storage are unchanged.
- Preserve controller-only access while ordinary updates are disabled, plus the
  original restore and active-state gates. Direct replay rows retain response
  idempotence. Remove old union arms and exhausted funding-deadline projection.
- Migrate installed state-cascade recovery/partial-failure, funding-deadline and
  child-reserve callers. Add native frame/schema/conversion/replay sources and
  change maximum receipt sizing coverage to native frames.
- Evidence: `/tmp/canic-fleet-state-control/`; targeted rustfmt and static reference/
  whitespace checks only. Builds, tests and generated artifacts remain deferred.
- Complete batch remains not push-ready: remaining control/status families, Fleet
  journal native schema authority, generated artifacts and focused qualification.

## Root pool retry transport — 2026-10-25
- Cut refill/reset retry commands to `canic_root_pool_retry_refill` and
  `canic_root_pool_retry_reset`. Preserve controller authentication, update guard,
  Prepared Root access, restore fence and original pool workflow effects.
- Move existing snapshot-convergent policy labels and spending quota/reserve
  records to direct endpoints. Remove old union arms and unused Candid responses.
  No Rust callers of those retired variants were found; no storage change.
- Add exact native identity/frame/schema and role-policy regression sources.
  Evidence: `/tmp/canic-pool-retry-wire/`; targeted rustfmt and static checks only.
  Compilation, tests and artifact generation remain deferred.
- Complete batch remains not push-ready: remaining control/status families, Fleet
  journal native schema authority, generated artifacts and focused qualification.

## Coordinator-to-Root admission transitions — 2026-10-25
- Source cut complete for prepare/activate/open: three exact native methods retain
  Reserve/Fence/Release, Coordinator caller checks, restore/activation fences and
  the 16 KiB request cap. Ops owns bounded transport; existing workflow owns
  operation checks, retained receipts and retry progress. No stored-state change.
- Remove old Root command/projection arms and the Candid response dispatcher.
  Update direct replay manifests, maximum-envelope and policy regression sources;
  add lossless stage/authority, schema and strict frame identity coverage.
- Evidence: `/tmp/canic-root-admission-transitions/`. Targeted rustfmt and static
  whitespace/reference inspection only; compilation/tests/artifacts deferred.
- Complete batch remains **not push-ready**: remaining control/status families and
  Fleet mutation journal native schema/hash authority need source work, followed
  by generated artifacts and focused qualification. No Git publication performed.

## Managed admission transitions — 2026-10-25
- Cut target prepare/activate/open to separate native methods with exact method
  IDs and the existing 16 KiB request cap. Preserve the compiled capability,
  AllowsUpdates gate, Root-only authentication, Prepared-state infrastructure
  admission, operation IDs and retained receipts. Storage encodings unchanged.
- Root's new ops transport uses bounded waits and native frames, validates the
  returned phase, and passes receipts back to unchanged workflow recording/retry
  logic. Remove the exhausted target Candid projection and managed command arms.
  Coordinator-to-Root transitions are now cut in the newer slice above.
- Migrate actual managed App/group helpers, wrong-authority/changed-input tests,
  replay/interruption cases and state-cascade refusal callers. Update direct
  endpoint replay manifests, capability checks, native schemas and maximum
  prepare-envelope round-trip coverage. These regression sources are unrun.
- Evidence: `/tmp/canic-admission-transitions/`. Targeted rustfmt syntax and
  whitespace/reference review only. Compilation, tests and generated artifacts
  remain deferred; nothing committed or published.
- Complete batch is not push-ready: other remaining
  control/status families and Fleet mutation journal schema authority still need
  source work, followed by generated artifacts and focused qualification.

## Managed admission native reporting — 2026-10-25
- `canic_admission_status` now accepts a native PageRequest and returns a complete
  native ManagedStatus. Retain the same method name, controller-or-Root guard,
  compiled capability selection and current lifecycle policy allowlists.
- Contracts ops performs lossless stored/native identity conversion. No projection
  storage, membership, prepared-successor or transition semantics changed.
  Remove the old managed admission request/response unions and exhausted fixture
  projection; remove its unused status-macro capability parameter.
- Managed App/group helpers and installed lifecycle, interruption and native-agent
  callers use the production native frames. Adapt maximum-page and identity
  coverage; add schema and cross-method/trailing-byte cases as source only.
- Evidence: `/tmp/canic-managed-admission-wire/`. Targeted rustfmt syntax,
  whitespace and reference inspection passed; compilation, tests and generated
  artifact qualification remain deferred. Nothing committed or published.
- Complete 0.112 is still not push-ready: admission mutation transitions, other
  control/status families and Fleet journal exact schema authority need source
  work before deferred generation and qualification.

## Root admission native reporting — 2026-10-25
- Cut Root admission status to `canic_root_admission` with controller admission
  and the original active-state gate. Preserve original participant catalog,
  progress, receipt/hash and current/successor evidence in native projections.
  Stored bindings, receipts and state encodings are unchanged.
- Host now admits both Coordinator and Root schemas from the retained release.
  Keep pagination progress, same-head and current-policy checks. Installed
  admission catalog/interruption callers use the production native method.
- Delete the exhausted admission Candid projection. Managed-only fixtures that
  borrowed its request were first aligned with their canonical owner; the newer
  managed reporting cut above now migrates their transport as well.
- Added page/retained-progress round-trip and complete native schema regression
  sources. Targeted rustfmt and static reference/whitespace review only; all
  compilation, tests and generated artifacts remain deferred.
- Evidence: `/tmp/canic-root-admission-wire/`. Complete batch remains not
  push-ready: other status/control families and Fleet journal schema authority
  still need source work, followed by generated artifacts and qualification.

## Registry, admission and snapshot control source cut — 2026-10-25
- Full registry snapshots and Root physical inventory now use native projections
  end to end: Root intercanister synchronization, signed capacity observers,
  Host/Fleet funding observations, subnet CLI and installed fixtures. Canonical
  stored registry bytes and digests retain their original types. Remove old
  Coordinator registry selector and exhausted subnet-information projections.
- Native Coordinator admission status/mutation methods replace old union arms.
  Controller admission, 16 KiB update cap, restore fence, exact plan/operation
  identity and retained-result semantics remain in their original owners. Host
  selects the retained release's native Coordinator schema. Root admission reads are now cut above; participant transitions remain
  part of the unfinished control-family work.
- Native snapshot prepare/resume methods are shared by Root and Coordinator.
  Preserve Root activation admission, Coordinator funding readiness and original
  live-history checks. Remove old command variants, dispatch and fixture paths.
- Validated deployment names/member paths have explicit host-only schemas and
  client predicates; do not replace custom Deserialize validation with unchecked
  transparent metadata. Added frame/schema and canonical-byte regression sources.
- Evidence: scoped before-images/scripts in `/tmp/canic-full-wire-cut/`.
  Targeted rustfmt syntax and whitespace/reference inspection only. No Cargo,
  Clippy, runtime execution, generated schema/client build or DID refresh.
- Remaining **source** blockers: other Root/Coordinator/Store/managed control and
  status families; native mutation transport and Fleet journal exact schema/hash
  authority together. Do not rename Candid journal fields without changing the
  actual admitted transport/schema, or treat native observations as paid-effect
  authority. Generated metadata/declarations and focused qualification are also
  deferred. Complete 0.112 is **not push-ready**, independently of deferred tests.

## Release evidence coordinated source cut — 2026-10-25
- Root and Coordinator now expose shared native `canic_release_intents` and
  `canic_release_receipts` query bodies. Preserve controller authentication and
  Root's original prepared-state access. Remove old selectors and exhausted
  Candid projections. Read-only evidence DTOs use native principals; conversion
  stays in runtime ops. Stable records, hashes and settlement logic are unchanged.
- Signed Host census and installed callers use the same fixed methods on both
  roles. Preserve 256 KiB replies, aggregate/row budgets, deadlines, monotone
  cursors, original owner evidence and before/after custody/registry verification.
  Fixture reply channels now separate census evidence from registry replies.
- Adapted existing regression sources and added native cursor identity coverage.
  Only targeted formatting, whitespace and reference inspection performed.
  Compilation, strict Clippy, runtime tests, generated schema/client builds and
  canonical Root/Coordinator DID refresh remain deferred by the maintainer.
- Scoped before-images: `/tmp/canic-release-evidence-cut/`. No Git publication.
  Complete 0.112 remains not push-ready: remaining control transport and Fleet mutation journal Wire authority still
  require source work (full registry/inventory are now cut above).

## Registry discovery coordinated source cut — 2026-10-25
- Implemented controller-protected native manifest/version queries and moved the
  acknowledgement's native registry version DTO to the shared discovery owner.
  Removed corresponding old observability selectors and projection variants.
  Persisted registry types, canonical hashing and mutation receipts are unchanged.
- Migrated all located admission, subnet CLI, startup preview, funding observation
  and installed fixture callers. Native reads retain exact release schema/module
  authority. The CLI's remaining full-registry/Root inventory contracts now come
  from the selected retained release, fixing the local-preparation sidecar assumption.
- Wrote lossless conversion/framing and installed manifest/outsider coverage;
  adapted the query-only changed/unavailable funding-head regression. Ran only
  targeted rustfmt and whitespace/reference checks. No Cargo compile, Clippy,
  tests, managed artifact build or canonical DID refresh for this source slice.
  The generated Coordinator declaration still needs its owning builder refresh.
- Evidence and scoped before-images: `/tmp/canic-discovery-cut/`.
- This completes the compact registry query source family, not the entire Wire
  hard cut. Remaining source families are the other control/status contracts, their shared
  transport DTOs and Fleet mutation journal schema authority; see the newer cut above.
  Complete 0.112 batch remains not push-ready; changelog draft is updated.

## Published Host and private-consumer alignment — 2026-10-25
- Adopt Host/Build/Testkit 0.68.3 and align declared IC dependency minimums
  with their published selections. Both private Blob graphs now use
  Blob/Contracts 0.47.1, Auth/Protocol 0.16.1, Platform 0.6.16,
  Metrics 0.16.2 and Host artifacts 0.68.3. Root already selected Host 0.68.3;
  its lock update only removes unused itertools 0.14.0.
- Verified published versions against crates.io and its sparse index. Query
  0.73.0 and Wire 0.7.10 remain selected; newer sibling-only revisions are
  not consumer dependencies. No path override or compatibility adapter added.
- Passed: locked dependency-boundary/runtime-identity admission for all three
  workspaces; strict Host/CLI/Blob adapter and both private-consumer Clippy;
  ten Host build/cache regressions; both normal managed Blob Wasm/schema/client
  builds; strict TypeScript compilation of both generated clients.
- Evidence: `/tmp/canic-registry-wire/`. No installed PocketIC, live-IC, native
  macOS, MSRV execution or TEE qualification was run for this adoption.
  Earlier installed evidence remains tied to its recorded dependency graph.
- Complete batch remains not push-ready. Next: cut Coordinator registry
  manifest/version queries and all admission, CLI and funding callers together;
  retain exact installed-release schema/module authority. Full registry/mutation
  transport and Fleet journal Wire authority remain unfinished. No runtime or
  transport source changed in this dependency slice.

## Protected authority-state Wire cut — 2026-10-25
- Root Fleet state and shared Root/Coordinator restore-fence reads now use
  separate fixed Wire queries. Installed restore and state-cascade callers use
  those exact contracts; old status selectors, fixture query enums/projection
  registrations and the read-only FleetStateResponse Candid derive are removed.
- Native fence DTOs convert at Contracts' ops owner. Stored fence records and
  snapshot mutation contracts remain unchanged. Root endpoint admission retains
  the original active-state check; both roles retain controller authorization.
  Maximum-value regression corrected the initial traversal allowance while
  retaining the 1 KiB reply frame cap.
- Passed: strict Contracts/Core/facade/internal-fixture Clippy; 238 focused
  frame/schema/projection/admission/replay tests; 35 facade protocol checks;
  normal Root and Coordinator managed Wasm/schema/client builds and strict
  generated TypeScript compilation. Canonical Coordinator DID regenerated by
  the owning builder for its remaining Candid endpoints.
- Installed Coordinator restore/replay acceptance passes (153s proof / 323s
  runner); installed Root restore acceptance passes (219s / 234s), preserving
  sealed operation evidence, physical inventory, suspended timers and refusal
  of unsafe resume/allocation. New queries also reject outsiders. Evidence:
  `/tmp/canic-state-wire/`. No full-suite, live-IC, macOS or TEE result implied.
- Complete batch remains not push-ready. Next coordinated family: Coordinator
  registry discovery/version reads and their Host admission/Fleet callers;
  mutation journal authority still needs its retained Wire schema cut.

## Published dependency adoption — 2026-10-25
- Adopted crates.io Query 0.73.0, Host/Build/Testkit 0.68.2, Storage 0.51.1,
  Timers 0.31.2 and Wire/derive 0.7.10. Sibling unreleased versions were not used.
  Root Host/Storage were already selected; their manifest minimums now match.
  Both private Blob locks also advance Memory to 0.51.1 as required by Storage.
  At that checkpoint private Blob/Auth/Platform selected 0.47.0/0.16.0/0.6.15;
  the later alignment above supersedes those selections.
- Passed: three-workspace locked dependency-boundary/runtime-identity admission;
  strict affected-package and both private-consumer Clippy; 192 focused native
  tests for Query catalog/certificates, timers, storage and Wire contracts;
  normal managed builds for dedicated and embedded Blob, with retained native
  schemas/clients and exact artifact admission. Generated clients pass strict
  TypeScript compilation. No Rust source adaptation was required.
- A direct consumer Wasm check was refused by the managed-build policy; both
  canonical managed builds subsequently passed. No bypass was added.
- Evidence: `/tmp/canic-deps-1025/`. No installed PocketIC, live IC, macOS, MSRV
  execution or embedded-fixture refresh was run for this dependency-only slice.
  Earlier installed results remain tied to their recorded graph. Complete batch
  remains not push-ready for the transport/journal work listed below.

## Current qualification pass — 2026-10-25
- Caller-authority prepare/commit/complete and startup release now use separate
  typed Wire updates alongside status. Production Root orchestration and shared
  fixtures retain exact authority, receipt hashes, 16 KiB ingress, lost-reply
  reconciliation and replay behavior. Exhausted Candid commands/projections removed.
- Fixed native schema metadata for fourteen explicitly renamed enums without
  changing Serde labels, Postcard tags or storage. Every variant has Rust and
  generated TypeScript byte-vector coverage.
- Passed: affected Contracts/Core/control-plane/facade/internal-fixture strict
  Clippy; host/CLI/fixture compilation; 33 replay tests, 7 activation tests,
  21 protocol tests, the added status-budget regression, 26 schema tests and
  31 facade protocol-surface tests; strict TypeScript compilation and Node
  conformance. Final strict Clippy passes on the finished source.
- Installed managed-Component lifecycle passes (345s), covering publication,
  lost replies, foreign authority, restart, fencing and replay. Installed Root/Store
  recovery and effect-free replay passes (204s), including Root-config access checks.
  All six managed Wasm profiles compile. Final embedded refresh and verification pass.
- Store, Coordinator and user-hub normal artifact builds retain native schemas and
  generated clients; strict TypeScript compilation passes for all three actual
  clients. Infrastructure DIDs were regenerated by removing migrated methods.
- Installed acceptance exposed an undersized status-request traversal budget.
  Valid operation IDs now fit 4 KiB allocation/256 nodes while the frame remains
  bounded to 128 bytes; wrong method, trailing bytes and oversized frames reject.
  Removed the redundant stale Store presence test; its exact current Candid method
  inventory remains checked. Evidence: `/tmp/canic-wire-completion/`.
- Complete batch remains not push-ready: remaining owned transport families and
  Fleet mutation journal authority are listed below. No full-suite, deployed IC,
  macOS, real-provider or TEE evidence is implied.

## Root inspection-reserve Wire completion — 2026-10-25
- `canic_root_inspection_reserve` now serves a bounded native-principal quote.
  Controller authentication and Prepared-state availability remain. ICP and direct
  Agent preflight callers require exact fresh evidence before paid inspection;
  missing or malformed responses never bypass preflight. Existing management
  admission, paid-command binding, custody and numeric liquidity checks remain.
- Removed Root’s exhausted observability union, reserve Candid projections and
  capability parser/bypass. Migrated Host fake transports and installed callers.
- Passed: two codec/schema tests, 32 Host inspection tests, seven prepared-state
  policy tests, 196 retained projection checks and the retained-estate generation/
  apply/replay regression. Corrected that regression’s stale omission assertion:
  the maintained journal explicitly permits omitted `release` as no authority;
  other required fields still reject omission. Production persistence is unchanged.
- Strict Clippy passes for Contracts/Core/Host/facade and the installed test target;
  scoped formatting and diff checks pass.
- Installed Root reserve acceptance passes (152s test / 270s targeted runner),
  including unauthorized callers, liquidity shortfall and recovery. Root Wasm
  and supporting normal Store artifact builds pass. Evidence:
  `/tmp/canic-inspection-wire/`. No live-IC/provider or complete-release result
  is implied; the remaining owned transports below still prevent push readiness.

## Coordinator acknowledgement Wire completion — 2026-10-25
- Controller query `canic_coordinator_root_acknowledgements` now returns bounded
  native Root/registry evidence. Reuse native directory authority and preserve all
  Fleet, Coordinator, recovery-controller, epoch, revision and content-hash fields.
  Stored receipts, acknowledgement mutation and replay are unchanged. Remove the
  old observability request/reply variants and migrate both installed consumers.
- Two codec/schema regressions and Contracts/facade strict Clippy pass. Governed
  fixture strict Clippy passes. Installed Coordinator joining-root replay passes
  (19s test / 21s targeted proof), including exact non-controller refusal. Canic's
  established controller-only refusal is `AUTHORITY_UNAVAILABLE`, distinct from
  the registered-root predicate's `AUTHORITY_UNAUTHORIZED`.
- All 35 facade protocol-surface regressions and scoped formatting/diff checks pass.
- Normal Coordinator Wasm/schema/client build passes; regenerate residual Candid
  through its owner, removing exactly the two old acknowledgement variants. Native
  schema and TypeScript retain the new method. Evidence: `/tmp/canic-ack-wire/`.
- This pass used the maintainer-updated root lockfile (Wire 0.7.7, Auth 0.16.1,
  Blob/Contracts 0.47.1, Backup 0.44.2); it does not requalify every integration.
  Prior Blob acceptance above/below remains explicitly tied to its tested graph.
- The later inner-hop cut below supersedes the remaining descendant relay
  implementation described at this point; the outer operator transport remains.
  Whole-batch readiness and the remaining transport/journal work below are unchanged.

## Descendant observation inner-hop Wire cut — 2026-10-25
- Root now calls six separate replicated `canic_observe_*` methods for child
  funding, balance, history, top-ups, memory allocations and metrics. Each has a
  fixed native response shape and exact identity/budgets. Managed/local endpoints
  retain controller authorization, prepared-state recovery access and typed
  `REQUEST_INVALID` refusal without invoking disabled providers. Keep the existing
  unbounded-wait semantics, invalid-target rejection and remote application errors.
- Removed the managed Observe command/reply, capability-pruned Candid relay union,
  projection and dispatcher. Reverse top-up projection lives at its existing
  Contracts conversion owner. Core now declares Wire directly; all three lockfiles
  retain their selected versions and include that dependency edge. Stored records
  and the outer operator-to-Root Candid command remain unchanged.
- Passed: strict Core/Contracts/facade/Host Clippy, 35 facade protocol tests,
  eight activation-policy tests, the full top-up-evidence conversion regression,
  194 retained Candid projection checks, scoped formatting/diff checks and locked
  metadata for root plus both independent Blob consumers.
- Installed protected-memory relay acceptance passes (358s targeted proof),
  exercising Root-to-child history and allocations, outsider refusal, repeated
  observations and unchanged stable bytes. Root, Coordinator, issuer and Store
  Wasm builds pass. Evidence: `/tmp/canic-relay-wire/`. This does not qualify every
  relay operation/capability combination, a live IC deployment or the whole batch.
- Next: cut the outer operator-to-Root observation methods and their callers;
  remaining control/discovery and Fleet journal schema authority still block
  whole-batch push readiness.

## Protected observation release binding — 2026-10-25
- Descendant telemetry and Root-paid balance inspection now resolve the Root
  sidecar from the terminal Fleet’s finalized release, matching admission’s
  existing resolver. Require exact installed module, role, protocol identity and
  manifest authority before transport. Missing selected release refuses; local
  preparation is no longer required. ICP still owns selected network and caller.
- Removed the redundant sidecar environment argument and stored CLI target copy.
  Corrected the guide’s direct-memory-query description to the implemented Wire
  method. This is a release-binding correction, not the outer relay Wire cut.
- Passed: six release-authority regressions, two observability regressions, strict
  Host/CLI all-target Clippy and scoped formatting/diff checks. The new regression
  checks competing local and selected-release sidecars without network effects.
  Evidence: `/tmp/canic-observation-release/`. No installed/live qualification
  or broad validation was run for this host-only correction.
- Next remains the coordinated outer relay cut, including funding-observation
  callers and exact Wire release/journal authority. Whole batch is not push-ready.

## Outer observation Wire contracts — 2026-10-25
- Added a passive native `root_observation::Request<T>` carrying exact target
  principal and method-specific input, and six fixed outer method IDs/names.
  The 256-byte request budget includes the target; reply budgets retain their
  existing operation owners. There is no role-pruned union or Candid projection.
- Contract qualification passes for all request and response schemas, maximum
  principals/page values, wrong operation/inner-hop/direction, trailing bytes
  and oversized frames. Contracts strict all-target Clippy with native schemas
  and default-feature Wasm compilation pass. Scoped formatting/diff checks pass.
  This does not qualify deployed endpoints, generated clients or runtime callers.
  Evidence: `/tmp/canic-outer-contract/`.
- These are contract declarations, not installed endpoints. Outer Root dispatch,
  Host telemetry, funding reconciliation and installed fixtures still use the
  existing Candid call. Switch them together only with exact retained Wire
  authority; do not bypass Fleet mutation journal authority. No storage or
  lifecycle change. Complete batch remains not push-ready.

## Coordinated outer observation switch — 2026-10-25
- Root now exposes all six typed outer `canic_root_observe_*` updates. Host
  telemetry and funding reconciliation use exact release-bound native schemas;
  no Candid sidecar is read for the relay. Identity checks retain the release,
  role, capabilities, module and protocol-profile authority. Existing Candid
  identities in unrelated Fleet mutation journals are not repurposed as Wire.
- Removed the old Root Observe command/reply, projections, obsolete outer DTO
  and Candid derives on internal routing enums. The internal routing result is
  projected at Contracts’ conversion owner into each fixed native reply.
  Controller admission, Prepared recovery access, invalid-target checks and
  unbounded inner calls remain. Paid balance inspection keeps its distinct
  reserve/intent authority and external management contract.
- Installed history, memory and funding callers migrated; also corrected the
  remaining direct top-up-history fixture. Added installed checks for all six
  outer operations, outsider rejection, invalid targets and optional-provider
  refusal, plus native schema authority and one-shot ICP transport regressions.
- The maintainer updated the root graph during this slice (Wire 0.7.9,
  Tooling/Testkit 0.68.2, Memory/Storage 0.51.1, Metrics 0.16.2, Timers 0.31.1).
  Preserve private Blob consumer locks; those still select Blob 0.47.0/Wire 0.7.5.
  Platform advanced to 0.6.16 during the first installed attempt; the input guard
  refused that changed graph. Retried against the unchanged current lock hash.
- Passed: 255 focused Contracts/Core/Host native checks; strict affected-package
  Clippy; installed all-six-operation relay acceptance (296s proof / 459s runner),
  including outsider/target refusals and unchanged stable memory. Normal Root
  artifact/schema/client generation passes with exact Wasm export/identity/budget
  parity (29 unique method identities); strict generated TypeScript compilation
  passes. Final checkpoint Contracts strict Clippy and scoped formatting/diff
  checks pass. Earlier facade protocol-surface checks passed (35 tests).
- Normal Root finalization exposed checkpoint IDs overlapping cycle/health/log
  identities. Moved the shared checkpoint family into its own range; endpoints
  and callers share those constants. Added a collision regression and rebuilt
  normal Root successfully. The installed observation result precedes this
  checkpoint-only ID correction; checkpoint lifecycle acceptance was not rerun.
  Evidence: `/tmp/canic-outer-switch/`.
  Earlier sections describe the prior slices; this section supersedes their
  statements that the outer observation transport is still Candid.
- Complete batch remains not push-ready: remaining discovery/control families
  and Fleet mutation journal schema authority still need their coordinated cuts.

## Blob ownership and current qualification
- Blob owns storage, service policy, funding and contracts. Canic composes endpoint
  authentication, memory grants, synchronous lifecycle, metrics and optional funding.
  Keep these adapters and the dedicated/embedded consumers; no second Blob engine
  was found. [Guide](../../features/blob-storage/README.md) and
  [coverage audit](../../audits/working/blob-0411-adoption.md) own detailed acceptance.
- Blob 0.47 removes default provider-json; Canic does not use those helpers.
  All three locks select 0.47. Three-workspace admission, strict adapter/consumer
  Clippy and adapter Wasm compilation pass: /tmp/canic-blob047/.
- Current-graph generated-client acceptance passes in both installed compositions
  (536s), including retained schema/TS delivery, tenant query/update, caller and
  service binding refusal, malformed frames, replay and stable-memory preservation.
  All five embedded funding/recovery profiles pass (255s), including provider
  errors, callback traps, liquidity and budget boundaries.
  Evidence: /tmp/canic-blob047-installed/{browser,embedded}.log.
- These are Linux/PocketIC and Node checks, not browser UI, live IC, macOS or
  real-provider acceptance. Method coverage is not exhaustive branch coverage.
  [#622](https://github.com/dragginzgame/canic/issues/622) remains open for delivery.
- [#494](https://github.com/dragginzgame/canic/issues/494) still lacks positive
  independently authenticated provider-credit evidence and immutable receipt replay.
  Acknowledged payments do not establish independent credit. Do not invent a receipt
  from operation names, local balances, ingress assertions or successful replies.
- No configured live Blob provider/payer or sample-pool Fleet was found in this
  checkout. No exact paid-provider budget is established. Provisioning #493,
  Toko Miner and other application qualification are outside this framework batch.

## Wire: delivered locally
- Follow the [accepted hard-cut contract](../../design/0.112-standalone-blob-service-extraction/wire-hard-cut.md).
  External IC/provider Candid remains; storage/lifecycle encoding is outside the cut.
- Health, Metrics, History, Overview, Children and Root-to-Store chunk reads have typed native Wire
  contracts, explicit identities and bounded frames. Chunk replies use byte buffers.
- Endpoint declarations drive final-Wasm admission and native schema/TS generation.
  Normal builds acquire/retain producer outputs under the owning build transaction.
  Exact Wire schema hashes bind release artifacts and installed/frontend selection.
  Owned frontend DID factories are removed; pinned compilation produces retained JS.
- Store chunk cut passes three codec/schema tests, all 33 protocol-surface tests,
  strict Contracts/control-plane/facade Clippy, managed Store build/schema generation,
  and installed Root/Store publication/recovery/effect-free replay (177s).
  Canonical Store Candid was regenerated. Evidence: /tmp/canic-chunk-finish/.
- The stale embedded allocation peer was refreshed through its owner; final-source
  verification passes in /tmp/canic-chunk-finish/embedded-final.log.
- Wire 0.7.5 resolves Cow<str> metadata for actual TemplateId/TemplateVersion.
  Schema/text vectors, scoped Clippy, Contracts Wasm and Rust 1.91 schema checks
  pass: /tmp/canic-wire075/. Do not replace those identifiers with local wrappers.

- Overview now uses `canic_public_overview` across Root, Coordinator, Store and
  managed roles. Readiness, metadata, Observatory and fixture callers use bounded
  Wire. Exhausted Overview projections and unused Candid derives are removed;
  role/release/profile checks and prepared-state allowances remain. Raw ICP metadata
  responses are decoded as hex, matching the command's requested output.
- Overview evidence: four codec/schema tests, 228 retained projection checks,
  three metadata tests, 33 protocol-surface tests, six prepared-state policy
  tests and affected-package strict Clippy pass. Isolated facade qualification
  explicitly selects Testkit Wire on the optional native-only testing dependency.
  Managed Store/Coordinator builds regenerate DIDs by removing the
  migrated surface and produce native Wire schemas. Installed Root/Store recovery
  (158s), Coordinator replay (40s) and dedicated Blob composition (393s) pass.
  Embedded fixture refreshed and verified. Logs: /tmp/canic-overview-cut/.
  Prior embedded/browser Blob evidence predates this Overview runtime change.

- Children now uses `canic_public_children` on Root and child-provisioning roles.
  `CanisterInfo` is a native-Principal transport DTO; stored child records and
  lifecycle semantics are unchanged. Host discovery reuses authenticated bounded
  reads. Old Children projections, managed/local public-status unions and the
  child-discovery Candid path are removed. The directory cut below removes Root public status too.
- Children evidence: three codec/schema tests (including the existing 1,000-entry
  page), 226 remaining projection checks, 17 inventory regressions, six transport
  tests, six prepared-state tests and scoped strict Clippy pass. Installed initial
  Shard provisioning validates Root/Hub native child queries, exact identities,
  pagination and replay (392s). Fixture refresh/verification and codec/installed
  proof pass on root Wire 0.7.6; its published codec/derive sources match 0.7.5.
  Logs: /tmp/canic-children-cut/. Private Blob locks were preserved at 0.7.5.

- Directory authority identifier prerequisites now have native Wire schemas for
  Fleet/Component IDs, bounded Component Spec names, App IDs and Fleet bindings.
  The shared `ids::wire_schema` validator bundle is retained by schema production,
  Host artifact admission/frontend compilation and Blob composition. Existing
  Serde text, Candid and storage contracts are unchanged. Rust/TypeScript vectors
  refuse malformed IDs and validator overrides; 33 identifier tests, five facade
  schema-delivery tests, six Host artifact-admission tests, strict scoped Clippy,
  Contracts Wasm and Rust 1.91 pass. Composed Canic/Blob generated codecs compile
  under strict TypeScript and preserve both owners' scalar checks in Node.
  Evidence: /tmp/canic-directory-identities/. The subsequent implementation below
  uses explicit transport DTOs around the shared stored authority bindings.

## Additional Wire families in the current qualification pass
- Directory head/member/controller pages now use separate typed Wire endpoints:
  `canic_component_directory_page`, `canic_root_directory_head` and
  `canic_root_directory_page`. Host inventory and fixture callers moved together.
  The full native-principal authority graph lives in Contracts `dto/component_directory`;
  lossless conversions live in `ops/component_directory`. Stored bindings,
  cursor encoding and synchronization/recovery records are untouched. Root's
  exhausted public-status union, selectors, projections and unused derives are removed.
- Direct cycle reads now use `canic_cycle_balance` / `canic_cycle_history` on
  Root, managed/local and Store roles. Root prepared-state fences, controller checks,
  optional application history and the Root management-inspection/relay paths remain.
  Store's exhausted observability union and old selectors/projections are removed.
- ICP CLI Wire queries share frame decoding and codec-neutral private binary argument
  scratch with existing metadata/Candid callers; pool balance reads retain their
  selected CLI identity and one-query semantics. Overview still validates its
  preselected binding and preserves typed codec/application errors.
- These cuts now compile under the affected-package checks and generated artifact
  builds above. The two installed cases exercise their relevant lifecycle and
  Root/Store paths; this is not exhaustive per-method or all-profile acceptance.
  Original implementation snapshots: /tmp/canic-directory-cut/.

- Direct automatic-topup history now uses `canic_cycle_topups` on managed/local
  roles with both history and automatic-topup capabilities. Native transport
  projections preserve parent operation IDs, failures, totals and event ordering;
  stored records and the Root relay remain unchanged. Removed the old direct
  selectors and now-unused role-union capability parameters. The installed
  authorization fixture uses the new endpoint for controller/outsider checks.
  Snapshots: /tmp/canic-topups-cut/.

- Protected health diagnostics now use typed `canic_health` on Root/managed/local
  roles. Controller authentication and Root prepared-state refusal remain distinct
  from public responsiveness. Old Health union selectors are removed; native schema
  metadata covers the existing health/check payload. The installed authorization
  fixture checks controller response fields and outsider denial.
  Snapshots: /tmp/canic-health-cut/.

- Protected logs, metrics, memory allocations and child-funding reads now use
  separate typed Wire methods. Root and managed/local capability selection and
  controller admission are retained; Root child funding remains readable while
  Prepared, whereas logs/metrics/memory remain fenced. Native metric and funding
  DTOs preserve principal dimensions, participants, ordering and uncertain reserves.
  Host direct Root reads no longer need the protected-observability Candid
  projection; startup funding and direct fixture callers use the new methods.
  Removed exhausted projections/selectors, unused macro parameters and log Candid
  derives. Ported timer/instruction/attestation metrics, funding/memory diagnostics,
  authorization checks and the log-filter codec regression. The outer Root observation
  transport remains Candid; the inner hop is superseded by the cut above. Snapshots and scoped diff:
  /tmp/canic-diagnostics-cut/.

- Readiness, runtime and managed binding now use `canic_readiness`, `canic_runtime`
  and `canic_binding`. Native projections preserve all runtime/topology fields and
  exact fixture installation/completion evidence, reusing directory authority DTOs.
  Stored records and lifecycle semantics are unchanged. The canonical release-build
  ID schema joins the shared identity validator bundle. Control-plane binding and
  readiness checks use bounded Wire calls with retained dependency-failure attribution.
  CLI inspection keeps its selected ICP identity and now reports Wire payloads.
  Fixture polling, managed-component helpers and installed diagnostic callers moved
  together. Managed/local observability dispatchers/unions, exhausted projections,
  obsolete parameters and top-level diagnostic Candid derives are removed.
  Snapshots and exact scoped diff: /tmp/canic-runtime-cut/.

## Remaining Wire implementation
1. Cut remaining owned endpoint families and all callers together. Protected
   discovery and control families retain Candid; the descendant observation
   relay now uses Wire on both hops. Use separate typed operations, not capability-pruned
   Serde union layouts or Candid structural projection over Wire.
2. Migrate Fleet mutation authority with its transport. DesiredFleetProtocol and
   EnsureAction::FleetProtocol still bind Candid paths/hashes; current_protocol
   resolves those hashes before effects. Introduce exact Wire schema authority
   from retained release evidence before moving those calls. Do not reuse a Candid
   hash as a Wire hash or bypass intent-before-effect/reconciliation.
3. Resolve actual transport types at their owner. Shared fixture/Fleet bindings
   still contain ExternalPrincipal and some also serve stored state. Preserve
   storage bytes and external Candid; no blanket serializer/type substitution.
4. Qualify remaining generated clients and installed callers on the selected
   graph, then remove exhausted Candid projections/adapters. Keep
   [#613](https://github.com/dragginzgame/canic/issues/613) open until complete
   default-branch delivery. A partly migrated batch is not releasable.
- Normal production schema-feature isolation and Account acceptance remain tracked
  by Wire #17 / Canic #628. Inspect current feature graphs; native-only derives
  alone do not prove JSON is absent when schema features are enabled for Wasm.

## Other retained safety boundaries
- Auth preserves portable credentials, local sessions and #506 renewal semantics.
  No production clock-skew bound is qualified: activation stays closed. TEE
  hardware evidence is unavailable. #491/#507 and the
  [Auth guide](../../features/authentication/README.md) own unfinished integration.
- Backup live capture remains unavailable. Restore refuses replay/rearming of
  unsettled snapshot loads; preserve journal and source backup until qualified
  settlement. Root checkpoint storage/dispatch exists, but installed multi-party
  interruption proof and paid snapshot settlement still precede the engine switch.
  Preserve #513/#550/#394 and the [Backup guide](../../features/backup-and-restore/README.md).
  Existing evidence: /tmp/canic-backup-load-settlement/, /tmp/canic-backup-ordering/,
  /tmp/canic-backup-cleanup/ and /tmp/canic-root-checkpoint/.
- Prior scoped fixes for discovery #266, exact absence #261, Auth diagnostics
  #279/#280 and bounded replica transport #260 passed; evidence is retained in
  the historical handoff. Do not replay those fixes from old descriptions.
- No complete-release, native macOS, deployed-IC or TEE qualification is implied.

## Root configuration and caller-authority follow-through
- Root-only caller-authority status uses `0x43414e490000010b`; publication and
  startup transitions use `0x43414e4900000400` through `0x43414e4900000403`.
  Hashing, stored authority and generation semantics remain with the existing engine.
  Production orchestration, shared fixtures and generated schema delivery are migrated.
  Original status snapshot: `/tmp/canic-caller-status-cut/`.
- `canic_root_config` (`0x43414e490000010c`) exports original compiled TOML via
  bounded Wire. Controller authentication and Root prepared-state refusal remain;
  Config is removed from the Root Candid union. No runtime parser was introduced.
  Codec/schema tests and installed controller/outsider checks pass in this run.
  Original implementation snapshot: `/tmp/canic-config-cut/`.
