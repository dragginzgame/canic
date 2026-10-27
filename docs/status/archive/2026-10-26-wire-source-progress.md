# Current handoff — 2026-10-26

## Scope and authority
- Continue the accepted 0.112 Blob/Wire hard cut. The maintainer requests a large
  coordinated implementation pass, not one prompt per endpoint. **Not push-ready.**
- Compilation, tests and generated artifact builds remain deferred at the
  maintainer's request. Formatting/static review is not runtime qualification.
- Preserve dirty Backup/Auth and unrelated work. No staging, commits, publication,
  release or deployment. Siblings are read-only; no broad validation or delegation.
- Check shared target ownership before source/lock edits or a focused build.
- Selected graph: Blob/Contracts 0.47.1, Wire 0.7.12 in all three workspaces,
  Platform 0.6.16, Memory/Storage 0.51.2, Auth 0.16.1, Metrics 0.16.2, Timers 0.31.2,
  Tooling/Testkit 0.68.4, Query 0.73.0, Backup 0.44.2; Shared snapshot 0.26.16.
- [Earlier source milestones](archive/2026-10-25-wire-source-progress.md) and
  [historical handoff](archive/2026-10-24-blob-wire-precompact.md) preserve dated
  evidence. Do not treat earlier passing checks as qualification of current source.

## Candid reporting subsystem removal
- Removed Host candid_endpoints, its recursive reporting DTOs, renderer and payload
  projection. The only remaining consumer was the external-Candid admission fixture.
  It now uses a small testing-internal compiled-metadata reader and existing parser
  for return-shape assertions; actual ingress boundary calls remain unchanged.
- Added bounded/single/complete metadata record test source. Formatting/static checks
  only; builds/tests remain deferred. Evidence: `/tmp/canic-reporting-cut/`.
- Profile generation and installed acceptance still require completion; not push-ready.

## Retained inspection cleanup
- Removed unused built/installed Candid loaders, BuiltCandid, ProtocolBindingError
  and byte-verifier machinery. Retained declarations now have a single Wire load
  path; removed DeclarationKind and the redundant intermediate record.
- Ported unfinished/finalization/child-manifest/hash/containment rejection sources
  to Wire. Corrected a missing test import exposed by the prior fixture extraction.
- Scoped formatting/static reference checks only; tests/builds remain deferred.
  Evidence: `/tmp/canic-inspection-cut/`. Complete batch remains not push-ready:
  compiled profile generation is still Candid-derived and installed acceptance remains.

## Endpoint inspection Wire cut
- `canic info endpoints` now selects retained Wire schemas for both explicit builds
  and installed Fleet targets. Removed its Candid parser/types/signature renderer.
  JSON includes type definitions, native method IDs/modes/shapes and codec budgets;
  plain output shows native shapes and bytes/allocation/nodes/depth tuples.
- Source regression covers exact JSON projection, rendered budgets, duplicate
  identities and finalized build selection. Formatting/static checks only; tests
  remain deferred. Evidence: `/tmp/canic-endpoints-wire/`.
- This removes the last production caller of retained owned-Candid inspection;
  its now-unused Host loaders can be removed next. Generated profile generation
  still uses Candid-derived identity and needs a coordinated cut. Not push-ready.

## Inventory identity and build delivery audit
- Consolidated ProtocolEntry on raw_module_hash; removed its duplicate installed
  hash and funding observation's dual-hash acceptance. Updated the stale inventory
  test that previously asserted gzip payload hashes were installed module hashes.
- Verified single/configured/full builders already qualify native schemas and
  provenance includes Wire hashes. Single-role build JSON now exposes wire_schema.
- Scoped formatting/static review only; no build/test execution. Evidence:
  `/tmp/canic-inventory-identity/`. Complete batch remains not push-ready; remaining
  profile generation audit and installed acceptance are not yet qualified.

## Frontend release admission convergence
- Frontend export now calls the common release schema resolver, deleting its
  weaker module-only admission. Public handoff creation is bound to the terminal
  profile/release/capabilities as well as raw module and retained schema hashes.
- Shared application fixture metadata now has a valid derived profile. Added
  regression sources for accepted raw module identity and refusal of gzip identity,
  changed release/capabilities and absent binding metadata. Tests remain deferred.
- Scoped formatting/static checks only. Evidence: `/tmp/canic-frontend-admission/`.
  Complete batch remains not push-ready: generated-profile audit and installed
  acceptance remain; #637 separately awaits npm supply.

## Module identity and installed schema convergence
- Corrected retained application schema/inventory module identity: use raw Wasm
  hash, not gzip payload hash. Delete the duplicate installed_module_sha256 field;
  frontend, observatory and funding consumers use wasm_sha256. Upload payload
  checks keep their independent gzip hashes.
- Installed schema lookup delegates to the common finalized release resolver,
  preserving role/profile/capability/module checks and removing the weaker separate
  module-only helper/test. Shared release tests own mismatch coverage.
- Scoped formatting/static review only; builds/tests remain deferred. Evidence:
  `/tmp/canic-module-identity/`. Complete batch remains not push-ready: generated
  profile delivery and installed acceptance remain.

## Browser Platform adoption — #637
- On 2026-10-26, npm registry lookup for `@dragginzgame/ic-platform` returned E404;
  upstream ic-platform#57 still describes local unpublished 0.1.0. Keep the working
  SDK Core 5.4.0 declaration/imports until registry supply exists. No local alias,
  source-copy dependency or lockfile fabrication was introduced.
- Made the existing Agent defaults explicit: retryTimes 3, query signatures enabled,
  construction-time synchronization disabled. Root-key fetching remains disabled.
- Remaining #637 acceptance: published archive/integrity and SDK graph verification,
  Platform import migration, first-party JS dependency/import CI enforcement and
  actual generated-client/authenticated installed query/update acceptance. Cargo
  dependency gates do not enforce this JavaScript boundary. Issue remains open.
- Syntax/diff checks only; no installed acceptance or build/test qualification.
  Evidence: `/tmp/canic-637/`.

## Retired release binding removal
- Deleted the unused Candid release resolvers, ResolvedProtocolBinding wrapper,
  duplicate sidecar reader and their obsolete errors. Native schema selection and
  retained Candid inspection keep their separate owners.
- Ported release-binding rejection sources to Wire: exact profile/module authority,
  altered finalization/child manifests, missing/changed schema and symlink rejection.
  Removed the obsolete prepared-DID selection test. Builds/tests remain deferred;
  scoped formatting/static review only. Evidence: `/tmp/canic-release-binding-cut/`.
- Complete batch remains not push-ready: generated/profile delivery audit and
  installed acceptance remain.

## Native transport cleanup and metrics error projection
- Deleted unused Host owned-Candid invocation/query helpers and their private
  mode/encoder/response pipeline. External IC/provider transport is unchanged.
- Metrics now recognizes missing-method errors nested in native Wire query/update
  failures. Its previous Invocation match could never catch the current relay.
- Ported cleanup-failure and private argument-file regression sources to Wire.
  Scoped formatting/static checks only; builds/tests remain deferred. Evidence:
  `/tmp/canic-native-transport/`. Complete batch remains not push-ready; retained
  release binding cleanup, generated profiles and installed acceptance remain.

## Unused local binding cleanup
- Removed unused local Registry/infrastructure DID resolvers, their disconnected
  tests, typed binding call wrappers and public aliases. Repository Rust reference
  search found no consumers after the previous bootstrap/capacity migration.
- Retained release/declaration inspection and independent external Candid handling
  remain. This is source cleanup, not completion of generated profile delivery.
- Scoped formatting/static diff review only; builds/tests remain deferred.
  Evidence: `/tmp/canic-dead-bindings/`. The complete batch is not push-ready.

## Bootstrap and capacity native authority
- Infrastructure bootstrap Registry binding and capacity-import provenance consume
  finalized release Wire authority instead of reopening DID sidecars. The shared
  profile selector checks canonical role and recomputed profile identity, then
  admits retained schema/module evidence through the release resolver. Existing
  installed module, topology and certified custody checks remain unchanged.
- Scoped formatting/static checks only; compilation/tests remain deferred.
  Evidence: `/tmp/canic-bootstrap-wire/`. Complete batch is not push-ready:
  generated/profile delivery and installed component-removal acceptance remain.

## List and readiness native authority
- List readiness and framework versions now resolve the terminal release's Wire
  schema for both application and infrastructure participants. Delete the CLI DID
  resolver; metadata/readiness APIs accept profile metadata without a Candid path.
- Application admission checks retained module/schema/profile hashes, recomputing
  the existing profile digest from release, role, capabilities and retained Candid
  hash. The digest algorithm is unchanged; no Candid bytes are treated as Wire.
  Cycle-balance Root inspection also stops requiring an unused DID sidecar.
- Scoped formatting/static diff review only. Builds/tests remain deferred; expanded
  metadata rejection test source is unexecuted. Evidence: `/tmp/canic-list-wire/`.
  Remaining: generated/profile delivery audit, installed component-removal acceptance
  and deferred qualification. Complete batch remains not push-ready.

## CLI native funding authority
- Funding diagnostics and cycle conversion now resolve exact retained release Wire
  schemas. Remove their unused Candid binding arguments and local ICP DID lookup.
  Conversion checks release artifacts before allocating a pending operation; existing
  operation recovery, paid requests and response validation remain unchanged.
- Scoped rustfmt and diff checks only; compilation and tests remain deferred.
  Evidence: `/tmp/canic-cli-schema/`. This closes two native caller sidecar gaps,
  not the full migration: metadata/list binding, generated profile delivery and
  installed acceptance remain under audit. The complete batch is not push-ready.

## Audit Root delivery and remaining Host authority cut
- The bespoke audit Root now builds its actual `root_probe` native producer with
  the runtime package, role features and environment. It reuses Host's input-guarded
  build, final-Wasm admission and retained schema publication. Public explicit
  recipes require one locked package and matching manifest/workspace; ordinary
  configured builds reuse the same implementation. No builds were executed.
- Terminal/funding inventory checks immutable release Wire hashes, not Candid
  sidecars. Delete unused inspection path arguments and fix the dangling descendant
  path reference exposed by that cleanup. Installed release-profile metadata stays
  truthful and separate; this does not rename Candid hashes into Wire hashes.
- Reinstall history witnesses retain native schema authority and check the exact
  history update contract before transport. The nested IC management history
  payload remains Candid. Delete obsolete Candid paths/hashes from typed Fleet
  intent and successor authority. Remove the unused generator DID lookup and its
  path plumbing; actual initialization encoding is unchanged.
- Individual Component reviews bind Root Wire schema hashes. Resolve their exact
  release schema and admit allocation queries/provision updates before transport;
  retain original controller, module, Registry and operation correlation checks.
- Update native sidecar/hash/symlink, selected-recipe and Component authority
  regression sources. Scoped formatting/static reference/diff review only;
  compilation, generated builds and installed tests remain deferred. Evidence:
  `/tmp/canic-audit-schema/`. No publication or release was performed.
- The earlier audit Root source gap and Host journal/inventory sidecar migration
  are now implemented. Remaining release work is the complete direct-method and
  generated-delivery audit, installed component-removal acceptance and deferred
  qualification. Candid-derived installed profile metadata still needs its own
  coordinated audit; external/provider and storage encodings must remain intact.
  **The complete batch is not yet push-ready.**

## Journalled native mutation authority and retained schema delivery
- Fleet actions and source recovery records now bind Wire schema paths/hashes;
  DesiredFleetProtocol carries independent native paths. Generation checks release
  schema digests, and successor authority binds all three schema hashes alongside
  the still-separate initializer/inventory Candid identities.
- Paid mutation dispatch checks exact method/ID/update mode/request and response
  budgets against the same bounded no-follow bytes already checked by digest.
  Replay keeps original journal, target, action and spending authority. Stored
  response encodings and external Candid are unchanged.
- Independent uploads/imports group by Wire authority. Remove unused DID arguments
  and path maps from native pool inspection/reset checks; controller, module,
  duplicate-asset and conservation validation remain in their existing owners.
- Retain Wire schemas in canonical Coordinator and complete release fixture caches;
  child-origin recovery consumes the installed Coordinator's cached schema.
- Added source regressions for schema drift, missing sidecars, exact method/budget
  admission and refusal before transport. Scoped rustfmt/static review passes;
  compilation/tests/generated builds remain deferred. Evidence:
  `/tmp/canic-journal-wire/`. No Git publication or release was performed.
- Concrete remaining delivery gap: the bespoke audit Root builder in
  `pic/fleet_registry/baseline/tests/release_artifacts/audit_root` bypasses normal
  schema production. Its selected native producer must be built from `root_probe`,
  not the canonical Root recipe; do not supply a mismatched schema to satisfy cache
  completeness. Complete that path before running its installed journey.
- Remaining whole-batch blockers: inventory/profile Candid authority and direct
  method audit, bespoke audit Root schema production, generated delivery and
  installed component-removal acceptance. Deferred native/Wasm/strict-Clippy and
  installed recovery qualification are still unproven. **Not push-ready.**

## Final-Wasm admission at release assembly
- Verified normal artifact builds already acquire native schemas and release records
  already carry independent Wire hashes. Do not reimplement that delivered pipeline.
- Fixed a separate admission gap: application and infrastructure assembly previously
  hashed any nonempty schema bytes. Both now re-admit schema/export/method/budget
  agreement against final Wasm and schema type references using the existing artifact/Wire validator before sealing.
- Replace placeholder schema/malformed-module fixtures with one valid declared pair.
  Add malformed-schema and method-ID drift rejection to both persistence test sources;
  retain empty/missing schema and exact-byte immutable-conflict coverage. Whitespace
  changes remain a different retained identity even when semantically valid.
- Scoped formatting/static diff checks pass. Compilation/tests remain deferred;
  fixture source changes are not runtime qualification.
  Evidence: `/tmp/canic-release-schema-admission/`.
- Next: Host mutation journal/schema authority and remaining direct-method/generated
  delivery audit. Installed component-removal acceptance and deferred qualification
  remain. The complete release batch and publication surfaces are **not ready**.

## Final Root operation-selector source cut
- `canic_root_removal` carries complete draining, inventory, Registry publication,
  Store reclamation/finalization/deletion, readiness and Root preparation evidence.
  Reuse retirement DTOs; preserve original full-width cycle and ledger authority.
- Coordinator polling and installed Root-removal recovery now use native framing,
  exact operation correlation and full native/canonical parity. Keep original
  Coordinator/controller admission, Prepared fencing and duplicate-ID detection.
- Delete the old endpoint/constant, nested Root unions, the entire obsolete
  `dto/wire` projection tree, three unused fixture decoders and their checks.
  Simplify shared lookup to ownership/authorization only; stored records are unchanged.
- Add source checks for all four new Store/preparation records, full-width values,
  distinct identities, exact method framing and complete schema. Existing zero-ID
  and duplicate-owner tests now exercise the maintained read/ownership surface.
- Scoped rustfmt, macro syntax, method-ID uniqueness and static diff/reference checks
  pass. Compilation/tests/generated artifacts remain deferred.
  Evidence: `/tmp/canic-root-removal-cut/`.
- **No Root operation selector branches remain in Rust source.** This is not whole
  Wire completion: release/schema/journal integration, generated delivery and deferred
  qualification remain, including the component-removal installed-acceptance gap.
  The complete release batch and publication surfaces are **not ready**.

## Component-removal observation cut
- Add `canic_root_component_removal`, retaining full draining, final-inventory,
  quiescence, deletion and membership-accounting evidence. Reuse principal-free
  canonical draining/final-inventory DTOs with host-only schema metadata; stored
  encodings and destructive effects remain unchanged.
- Delete the old selector; preserve its controller/Prepared and duplicate-ID fences.
  Remove the now-stale large-enum lint expectation from the remaining Root union.
- Add source round trips for unstarted deletion and all three deletion boundaries,
  full schema, exact method framing and zero-ID checks. No remaining Host/installed
  component-removal status caller was found in searched Rust. Installed endpoint
  acceptance is a coverage gap, not proven by the codec sources.
- Scoped rustfmt, macro syntax, method-ID uniqueness and static diff/reference checks
  pass; compilation/tests/generated artifacts remain deferred.
  Evidence: `/tmp/canic-component-removal-read/`.
- **One Root selector branch remains: Root removal.** Release/schema/journal
  integration and generated delivery remain. **Not push-ready.**

## Subtree-removal observation cut
- Add `canic_root_subtree_removal` with the complete native phase/receipt graph:
  traversal, stop/delete intents and receipts, membership transitions, surviving
  member convergence and terminal Registry/Directory authority. Storage is unchanged.
- Migrate the installed Store-interruption recovery reader; delete the old selector
  and retain subtree ownership in duplicate-ID detection. Reads preserve controller
  and Prepared fences and never advance destructive effects.
- Add source regressions for every phase, optional quiescent owners, exact method
  framing, complete schema and zero IDs. Installed reads assert full conversion
  parity and unrelated-caller denial. These tests have not been executed.
- Scoped rustfmt, macro syntax, method-ID uniqueness and static diff/reference checks
  pass. Compilation/tests/generated builds remain deferred.
  Evidence: `/tmp/canic-subtree-read/`.
- **Two Root branches remain:** component removal and Root removal. Release/schema/
  journal integration and generated delivery also remain. **Not push-ready.**

## Complete component-allocation selector cut
- Ordinary Host inventory, operator progress and both remaining installed allocation
  callers use the native component-allocation endpoint. Remove the old selector,
  ordinary-inventory and component-operation Candid projections and their checks.
- Keep exact claim/partition/plan correlation, operator artifact admission, typed
  absence and complete native allocation evidence. Retain allocation ownership in
  the collision resolver independently of the deleted transport payload.
- Add narrow native query sources for exact request identity, typed absent evidence
  and foreign-frame rejection. Correct the preceding funding-parent process fixture
  to emit the raw hex output required by the actual Wire CLI transport.
- Scoped rustfmt and static diff/reference checks pass; builds/tests/Clippy and
  generated artifacts remain deferred. Evidence: `/tmp/canic-component-selector-cut/`.
- **Three Root branches remain:** component removal, Root removal and subtree
  removal. Release/schema/journal integration and generated delivery also remain.
  The complete batch and publication surfaces are **not ready**.

## Native component allocation and Host funding-parent cut
- Add `canic_root_component_allocation` with full native allocation/completion
  evidence, reusing initialization, registry-origin and component-binding owners.
  Preserve original caller authority, Prepared admission and duplicate-operation checks.
- Host funding-parent observation queries top-level and child allocation owners and
  requires exactly one. Only typed absence is absence; malformed frames, rejected
  authority and other failures remain failed observations. Preserve exact allocation,
  component/child, release and immediate-parent checks. Remove Candid path arguments.
- Delete the child selector/wrapper and unused startup-binding Candid projection.
  Migrate installed application-initialization/recovery and denial reads to the new
  endpoint, with full native/canonical parity checks. Host regression sources cover
  both owners, ambiguity, absence, wrong frames and non-workload query suppression;
  add schema/frame and zero-ID sources. These tests have not been executed.
- Scoped rustfmt, macro-body syntax, duplicate-method-ID and static diff/reference
  checks pass. Compilation/tests/generated builds remain deferred.
  Evidence: `/tmp/canic-allocation-reads/`.
- **Four Root branches remain:** component allocation, component removal, Root
  removal and subtree removal. Remaining component-allocation callers must move
  before deleting its selector. Release/schema/journal and generated delivery remain.
  The complete release batch and publication surfaces are **not ready**.

## Installed child-allocation readers
- Migrate all five remaining installed child-selector readers to
  `canic_root_child_allocation`, including startup, reserve, autonomous funding and
  diagnostics. Return canonical allocation data directly; remove obsolete envelope
  accesses and manual Candid transport from those journeys.
- Share the operation collision resolver in the native child workflow while retaining
  registered-parent authorization. Add installed controller/parent equivalence,
  child/unrelated-caller denial, absent-operation and effect-free read assertions.
  The shared fixture reader checks full native/canonical conversion parity.
- Also migrate the remaining batch-provisioning denial check to the native endpoint.
- Formatting/static diff/reference checks pass; compilation/tests/generated artifacts
  remain deferred. Evidence: `/tmp/canic-child-read-callers/`.
- Five Root selector branches still remain. Host funding-parent resolution retains
  the child branch until top-level component allocation has a native read; do not
  replace the branch with codec fallback or guess logical parents from pool ownership.
  Release/schema/journal and generated delivery remain. **Not push-ready.**

## Root batch-provisioning observation source cut
- Coordinator orchestration, Host reset review and eight installed-test reads now
  use `canic_root_component_provisioning`; remove the duplicate response selector,
  Coordinator projection branch and unused Host Candid query helper/imports.
- Native reads preserve exact operation/plan identities, caller authorization,
  current claim capacity and full provisioning receipts. The native workflow now
  shares the existing collision resolver; the retired selector remains represented
  only as an ownership marker, with no transport payload.
- Installed helper checks whole native/canonical conversion parity. Scoped rustfmt
  and static diff/reference checks pass; compilation/tests/generated builds remain
  deferred. Evidence: `/tmp/canic-root-provisioning-cut/`.
- **Five Root operation results remain:** child provisioning, component provisioning,
  component removal, Root removal and subtree removal. Release/schema/journal
  integration and generated delivery also remain. **Not push-ready.**

## Root Registry synchronization observation source cut
- `canic_root_registry_synchronization` returns complete synchronization and
  mirror-activation receipts using native registry, acknowledgement and Directory
  owners. Preserve old/new versions, full Directory provenance/members and caller,
  Prepared and duplicate-operation fences. Persisted encodings remain unchanged.
- Migrate both Host installation observations and all five installed synchronization
  callers; remove the old response branch. Observation does not advance either phase.
- Add Host process tests for both phases, wrong frames and absent operations, plus
  complete-schema, method-identity and zero-ID regression sources. Installed queries
  assert native/canonical parity. Formatting/static checks pass; builds/tests/Clippy
  and generated artifacts remain deferred. Evidence: `/tmp/canic-root-registry-read/`.
- The remaining-selector inventory is superseded by the batch-provisioning cut above.

## Root activation and refill observation source cut
- `canic_root_activation_status` and `canic_root_cycle_refill_status` reuse complete
  native reporting/funding DTOs. Preserve exact operation identities, credentials,
  refill outcomes, arbitrary-width cycle evidence, caller/Prepared checks and
  duplicate-ID detection. No storage, issuer, activation or paid-effect change.
- Migrate Host installation/reset reads and installed activation journeys. CLI
  refill completion now uses typed Wire responses; delete its handwritten Candid
  request builder, old projection and decoder. Existing installed-binding checks
  remain pending release/schema authority integration.
- Add frame/schema, zero-ID, typed-refusal and full-width-cycle regression sources;
  retain resumability/mismatch tests and installed credential round trips. Scoped
  formatting/static checks pass; compilation/tests/Clippy/generated artifacts
  remain deferred. Evidence: `/tmp/canic-root-activation-refill/`.
- **Root operation has 7 remaining results.** Release/schema/journal integration
  and generated delivery remain. Complete batch/publication are **not ready**.

## Root Store observation source cut
- `canic_root_store_adoption` and `canic_root_store_bootstrap` replace the two old
  Store operation results across Host installation/recovery and installed fixtures.
  Retain original controllers, receipt identity, complete catalog/fixture metadata,
  Prepared admission and bounded native responses. Stored authority is unchanged.
- The shared ownership resolver still counts migrated Store domains when rejecting
  duplicate IDs; only their old transport payloads are removed. Domain getters
  return retained evidence without retrying effects. Complete the same ownership/
  transport separation as the remaining Root result variants are removed.
- Add Host process success/refusal/wrong-frame, exact-ID/schema, zero-ID, controller
  and collision regression sources. Installed queries assert conversion parity. Scoped
  formatting/static checks pass; builds/tests/Clippy/generated artifacts remain
  deferred. Evidence: `/tmp/canic-root-store-reads/`.
- **Root operation has 9 remaining results.** Release/schema/journal integration
  and generated delivery remain. Full batch/publication are **not ready**.

## Coordinator provisioning observation source cut
- `canic_coordinator_component_provisioning` completes the Coordinator selector
  cut across native DTOs, endpoint/API, Root orchestration, Host and installed
  provisioning journeys. Preserve full progress, failure origins, funding pauses,
  authority checks and original retry deadlines. Stored records are unchanged.
- Delete all three Coordinator selector types, the emitted union macro, Candid
  provisioning projection, generic fixture decoder and exhausted Candid checks.
  Root callers previously borrowing the Coordinator request now use the Root type.
- Operation-ID retention explicitly checks admission, provisioning, removal,
  rotation and retirement owners. It no longer loses domains as transport switches.
- Add native Host success/wrong-method/absent-operation regression sources, schema,
  exact-ID and original-deadline coverage. Installed journeys assert conversion
  parity. Formatting/static checks pass; builds/tests/Clippy/generated delivery
  remain deferred. Evidence: `/tmp/canic-coordinator-provisioning-cut/`.
- **One old selector family remains: Root operation (11 results).** Release/schema/
  journal integration and generated artifacts also remain. Full batch and
  publication are **not ready**.

## Coordinator Root-removal observation source cut
- `canic_coordinator_root_removal` now carries the complete native draining,
  final-inventory, readiness-intent, Ledger, execution and completion receipts.
  Reuse existing registry, reservation and deletion types; records are unchanged.
- Migrate the live Root recovery client and installed removal observations; delete
  its Candid projection and old union branch. Keep caller authorization, exact-ID
  correlation and active/retired/cross-domain collision checks. Simplify now-
  irrefutable provisioning matches without changing their retained behavior.
- Add method-identity, complete-schema, full-width Ledger evidence and unrelated-
  caller regression sources. Formatting/static checks pass; builds/tests/Clippy
  and generated delivery remain deferred. Evidence: `/tmp/canic-root-removal-read/`.
- Two old selector families remain: Root operation (11 results) and Coordinator
  operation (Component provisioning only). The latter still needs its complete
  native DTO graph, Host callers and installed provisioning journeys migrated.
  Release/schema/journal integration and generated artifacts also remain;
  **the complete batch and publication are not ready**.

## Coordinator operation observations source cut
- Admission, funding-policy rotation and retirement now have separate typed Wire
  queries using the existing native evidence DTOs and domain engines. Caller
  authorization precedes exact operation-ID validation and domain reads.
- Remove the three old Coordinator union branches and their generic dispatch;
  migrate admission/restart/rotation fixtures. Retirement journey source observes
  both uncertain and reconciled transfers without issuing another Ledger effect.
- Add exact-ID/wrong-method, complete-schema and zero-ID regression sources.
  Formatting and static reference/syntax checks pass; compilation, tests, Clippy
  and generated artifacts remain **deferred**, not qualified.
- **Two old selector families remain:** Root operation (11 result variants) and
  Coordinator operation (2: Component provisioning and Root removal). Release/
  schema/journal integration and generated delivery remain unfinished. Full batch
  and publication remain **not ready**. Evidence:
  `/tmp/canic-coordinator-operation-reads/`.

## Component provisioning observation source cut
- `canic_root_component_provisioning` carries complete native Root provisioning
  observations. Preserve operation-owner/controller authorization and Prepared
  admission; workflow, persisted receipts and scheduler decisions remain unchanged.
- Reuse existing native registry/member bindings and provisioning failure evidence.
  Keep full placement members, publication/activation receipts, source/current
  authority, funding pauses and original retry times through lossless conversions.
- Migrate Host terminal inventory to native transport; delete its exhausted Candid
  projection, selector and unused sidecar parameter. Its installed artifact checks
  remain intact pending release/schema authority integration.
- Add actual Host process-query/full-evidence, wrong-method and schema regression
  sources. Scoped formatting, emitted Root syntax, whitespace/reference, field-parity
  and unique method-ID checks pass. Builds/tests/Clippy/generated artifacts remain
  **deferred**. Source evidence: `/tmp/canic-provisioning-read-wire/`.
- **Two old selector families remain:** Root operation (11 result variants) and
  Coordinator operation (5 result variants), including their nested receipt graphs.
  Release/schema/journal integration and generated delivery remain unfinished.
  Full batch/publication remains **not ready**.

## Child operation observation source cut
- Child allocation now has one typed `canic_root_child_allocation` endpoint. Keep
  exact operation ID, existing parent/controller authorization and Prepared admission.
  Reuse native child bindings and canonical principal-free policy/identity values;
  transport conversions preserve creation/install evidence and retry deadlines.
- Move terminal descendant inventory reads and their bounded parallel fixtures to
  native frames. Remove the old selector/projection arms and unused Candid argument,
  fixing the stale renamed-path reference in the previous Host call. No new Host
  adapter or lifecycle engine; existing transport and workflow owners remain canonical.
- Add frame-identity/full-schema, retry/uncertain-effect evidence and actual Host
  fixture round-trip regression sources. Scoped formatting, emitted Root syntax,
  whitespace/reference and unique method-ID checks pass. Builds/tests/Clippy and
  generated delivery remain **deferred**; evidence: `/tmp/canic-child-operation-wire/`.
- **Three old read selectors remain:** Root Component provisioning, Root operation
  and Coordinator operation. Release/schema/journal integration and generated artifacts
  are also unfinished. Full batch/publication remains **not ready**.

## Coordinator funding source cut
- Complete `canic_coordinator_funding` replaces the last Coordinator observability
  selector across CLI, startup accounting, signed release census, Observatory and
  installed fixtures. Native responses retain all rotation phases and terminal
  receipts; CLI JSON no longer discards completion evidence. Reuse canonical window
  metadata and native registry/funding authority; persisted records are unchanged.
- Delete exhausted observability/funding unions and projections. Observatory now
  unconditionally verifies installed module/schema/method/budget authority; its
  Candid transport branch and decoder are removed. Root and Coordinator release
  collection share one bounded native decoder, retaining aggregate-byte accounting.
- Add four-phase/full-receipt and budget/identity regression sources, native schema
  coverage and complete installed caller round trips; update process and PocketIC
  fixtures. Formatting, emitted Coordinator syntax, whitespace, reference and unique
  identity checks pass. Builds/tests/Clippy/generated artifacts remain **deferred**.
- **Four old operation-read selectors remain:** three Root reads and Coordinator
  operation. Host release/schema/journal integration and generated delivery also
  remain unfinished. Full batch/publication is **not ready**. Source evidence:
  `/tmp/canic-coordinator-funding-wire/`.

## Root funding status source cut
- `canic_root_funding` now carries the complete native funding response. CLI,
  Observatory and installed funding journey callers migrated together; no second
  funding engine or replacement policy was introduced. Keep exact authority checks,
  controller-only/Active admission, transport budgets and full refill cycle magnitudes.
- Remove the exhausted Root status endpoint/union and Root-only partial projections.
  CLI JSON now retains rotation receipts previously projected away. Shared Root query
  fences use exact method identities; add the missing funding-release replay entry.
- Add complete evidence round-trip, above-u128/zero/missing/nonminimal cycles,
  method identity and native schema regression sources. Installed journey reads also
  compare the complete canonical/native round trip. These tests remain **unrun**.
- Scoped formatting, emitted Root syntax, whitespace, stale-reference and method-ID
  checks pass. Compilation, Clippy, generated artifacts and runtime qualification
  remain deferred. Source evidence: `/tmp/canic-funding-status-wire/`.
- **Five old read selectors remain:** three Root operation reads and Coordinator
  funding/operation. Host build/release/schema/journal authority and generated delivery
  also remain unfinished. The full batch and publication surfaces are **not ready**.

## Funding-release read source cut
- Retained Root funding evidence now uses `canic_root_funding_release`. Six top-level
  read selectors remain: Root funding, three Root operation reads and Coordinator
  funding/operation reads. The complete release remains **not push-ready**.
- Migrate the signed collector, release-assessment fixtures and installed controller
  census callers. Keep 256 KiB pages, the aggregate census ceiling, pagination/custody
  checks, full source-account/uncertain-effect evidence and canonical reconciliation.
- Reuse existing native funding authority and minimal unsigned magnitudes; external
  refill cycle values are never narrowed to u128. Invalid leading zeroes fail decoding.
  Remove the old selector and exhausted funding-release Candid projections.
- Add above-u128, missing/zero, nonminimal magnitude, frame identity and full-schema
  regression sources. Scoped formatting, macro syntax, whitespace/reference and
  unique identity checks pass. Builds, Clippy, generated delivery and installed tests
  remain deferred; source evidence: `/tmp/canic-funding-release-wire/`.

## Pool inventory read source cut
- Root physical inventory now uses `canic_root_pool`, with complete native page and
  asset evidence. Seven top-level read selectors remain: Root funding/funding release,
  three Root operation reads and Coordinator funding/operation reads.
- Runtime, Host readiness/recovery, signed capacity-import/release collection,
  Observatory, component operations and installed fixture callers moved together.
  Pagination, policy/accounting comparisons, custody and canonical stored evidence
  remain at their existing owners; reuse native creation/handoff census types.
- Remove exhausted capacity/estate Candid projections and the separate test-only pool
  reader. Process fixtures now exercise the production query path. Correct the stale
  boxed-authority access found in estate generation after its previous native cut.
- Add native cursor/method and full-asset-state/receipt round-trip regression sources;
  update malformed/bounded reply checks and installed fixture export. Scoped formatting,
  emitted macro syntax, whitespace and stale-reference/identity checks pass. Compilation,
  Clippy, generated delivery and installed tests remain deferred. Evidence:
  `/tmp/canic-pool-read-wire/`. The complete release batch remains **not push-ready**.

## Registry read source cut
- Three more reads now use native Wire: registry preparation status, current Component
  partition and activation partition evidence. Eight top-level read selectors remain.
- Runtime/Host/installed callers moved together. Reuse the preparation DTO; native
  partition observations retain full administrator, Component, remote Fleet-service
  and Component-group origins, immutable prepared/activation partitions and current
  partition state. Canonical storage, hashing and inventory decisions are unchanged.
- Remove the three old selectors, startup-inventory projection and unused Candid path
  arguments from its reads. Existing publication/journal artifact authority is still
  separate unfinished work; this does not claim native schema delivery completion.
- Add exact request identity, malformed-frame and full metadata regression sources;
  installed partition helpers check native/canonical round trips. Correct malformed
  Prepared-policy test tuples found during review. Targeted formatting, emitted macro
  syntax, stale-reference and identity checks pass. Builds, tests, generated artifacts
  and installed acceptance remain deferred. Evidence: `/tmp/canic-registry-reads-wire/`.
- The complete release batch remains **not push-ready**.

## Root authority read source cut
- Root authority is now the typed `canic_root_authority` query. Deployment recovery,
  component operations, estate generation, local installation and installed fixture
  callers use its exact native identity and complete canonical authority conversion.
- Reuse native Fleet bindings and Store authority; storage/initialization encoding is
  unchanged. Retain controller admission, Prepared access, paired Root/Store equality,
  concurrent-read ordering/draining and existing artifact validation boundaries.
- Remove the old authority selector and exhausted local-installation projection.
  Eleven top-level read selectors remain. Exact native Host schema/journal authority
  and generated delivery still need completion; the complete batch is not push-ready.
- Add Root method-confusion/malformed-frame regression sources and move concurrent
  authority fixtures to native frames. Scoped formatting, macro syntax and static
  review only; compilation, Clippy, generated builds and tests remain deferred.
  Task evidence: `/tmp/canic-root-authority-wire/`.

## Final inspection command source cut
- All **38/38 original command variants** now have native source paths. Root inspection
  and its signed Host, concurrent pool, funding and installed fixture callers use
  the exact typed endpoint. Full status and arbitrary unsigned evidence are retained.
- Delete the exhausted Root command DTO/dispatcher, replay manifest and narrow Candid
  inspection projections. Retain controller guards, reserve evidence, preflight,
  snapshot-only inspection recovery and canonical internal observations.
- Endpoint budgets are four MiB reply bytes, 32 MiB allocation, 1,048,576 nodes and
  depth 64; qualification is deferred. Exact Root update identities now appear in
  Prepared refusals. No storage migration or external provider codec change.
- Targeted formatting, macro syntax and static reference review only. Builds, tests,
  generated artifacts and installed acceptance remain deferred. Evidence:
  `/tmp/canic-final-inspection-cut-20261025/`. **Not push-ready.**

## Previous Wire 0.7.12 unsigned evidence preparation
- Adopt published Wire/derive 0.7.12 in the root and both Blob consumer workspaces.
  Its documented minimal unsigned big-endian `Bytes<N>` convention resolves the
  representation question in Wire #26; the installed Canic cutover is still pending.
- Prepare complete native inspection DTOs and lossless external Nat conversions,
  including settings, memory/query metrics, viewer principals and typed reserve
  refusal. Zero is empty bytes; received leading zeroes reject before BigUint
  allocation. Outbound size checks precede buffer allocation; no u128 narrowing.
- Reuse the existing arbitrary-integer package selected by external Candid through
  an explicit `num-bigint` dependency; no arithmetic engine or codec is added locally.
  A field is bounded by the existing four-MiB Host reply ceiling; endpoint-wide
  budgets and installed admission still need to be selected and qualified at cutover.
- Add whole-observation canonical-byte round trips, overflow/nonminimal/field-bound
  regression sources, complete native metadata, and generated-browser exact-frame
  vectors distinguishing missing, zero, u128 maximum and larger values. Tests and
  generated-client execution remain deferred; these are sources, not passing proofs.
- Root inspection was prepared at this stage; its coordinated native cut is now
  recorded above, including dispatcher/projection deletion and recovery policy.
- Locked/offline dependency admission and selected runtime identities pass in all
  three workspaces; scoped formatting and JS syntax checks pass. No build, Clippy,
  installed-IC, native macOS or TEE qualification is claimed. Evidence:
  `/tmp/canic-wire0712-adoption/`. The complete batch remains not push-ready.

## Previous published dependency adoption
- Registry metadata confirms Tooling/Host/Testkit 0.68.4, Memory/Storage 0.51.2 and
  Wire/derive 0.7.11. Root and both Blob consumer catalogs/locks are aligned; the
  root lock already selected these releases before this catalog reconciliation.
  Auth, Backup, Blob, Platform, Query, Metrics and Timers were already current.
- Refresh Shared Tooling to clean published main revision
  `2077c934efb82b181bea58fe50078dbad289c1aa` (0.26.16); upstream has no matching
  GitHub release/tag, so this is an exact revision snapshot, not a tagged release.
- Update all six native TypeScript generator calls for Wire's required bulk-advice
  diagnostics. Reports go to stderr; no blanket acknowledgements or codec changes.
- Three-workspace locked/offline dependency admission, selected deployed runtime
  identity checks, Shared Tooling integrity and scoped formatting pass. Compilation,
  Clippy, generated clients and installed execution remain deferred. Shared RPC-budget
  policy is adopted; cross-runtime boundary/mutation qualification remains unfinished.
- Wire 0.7.11 does not settle arbitrary unsigned evidence. Root inspection consumers
  are recorded on [Wire #26](https://github.com/dragginzgame/ic-wire/issues/26#issuecomment-6399381891).
  Keep its current transport until a lossless supported representation is settled.
- Evidence: `/tmp/canic-upstream-adoption/`. The complete 0.112 batch remains not
  push-ready; this adoption does not finish the twelve read cuts or Host schema authority.

## Timers — implementation complete
- [x] Timer extraction and local integration. No further timer-specific
  implementation is planned; continue the other accepted extraction work.
- Production shared-runtime lifecycle and Wire runtime/metric reporting were already
  connected. Timer fixture controls, intent reservations, sampling and history-cost
  probes now use bounded native Wire with their actual PocketIC clients, including
  the standalone retained-intent restoration caller. Existing guards are retained.
- Add malformed ingress coverage for wrong method/direction, trailing, truncated and
  oversized frames, checking unchanged timer identities before handler effects.
  Update native registration examples to TimerStart with initial schedule admission.
- Formatting and static contract/caller review only; compilation, schema generation
  and installed timer/lifecycle qualification remain deferred. Source completion is
  not executed acceptance; the complete 0.112 batch remains not push-ready.
- Evidence: `/tmp/canic-timer-integration/`. No new timer service or endpoint adapter.

## Latest provisioning and pool source cut
- **37 of the original 38 command variants now have native source paths**. Migrate
  Coordinator/Root batch preparation, directory synchronization, management history,
  reviewed capacity import (reserve/advance/settle/release) and physical pool import
  together with their runtime, Host and installed fixture callers.
- Delete the exhausted Coordinator command dispatcher and batch payload inspector,
  old Root arms and exhausted fixture command projections. Retain canonical plan,
  reservation, receipt and storage hashing. External management history stays exact
  upstream Candid bytes inside its native response; it is not a fallback codec.
- Preserve Coordinator and controller authorization, Root Prepared gating, original
  replay labels, paid quotas and the Host durable submission boundary. Coordinator
  restoration fencing is independent of the deleted dispatcher; native history is
  admitted only for Root snapshot recovery, never release sealing.
- Migrate concurrent physical reset, failed-reset repair and lost-response callers.
  Update exact fault-injection hooks for native registry synchronization and batch
  preparation so installed proofs still interrupt the intended accepted effect.
- Add native identity/metadata, canonical receipt, import phase, reset outcome and
  authority-fence regression sources. Scoped rustfmt and static reference/identity
  checks only; compilation, Clippy, generated builds and tests remain deferred.
  Evidence: `/tmp/canic-batch-wire/`. **The full release batch is not push-ready.**
- The final inspection command cut is now recorded above.
  Twelve read selectors, exact native Host schema/journal authority, generated delivery
  and qualification remain. Do not narrow external numbers or relabel Candid hashes.

## Previous removal and retirement source cut
- Migrate Coordinator removal, Root removal and terminal Fleet retirement together:
  **31 of the original 38 command variants had native source paths at this stage**.
- Keep canonical draining reservations/hashes, controller versus Coordinator authority,
  restoration/Prepared gates, unbounded Coordinator-to-Root wait, exact receipt identity,
  original Ledger memo/time/fee ceilings and lost-reply reconciliation unchanged.
- Installed removal refusal/replay and retirement lost-reply/fee-limit/effect-free replay
  cases now call the actual native methods. Add native metadata and canonical receipt
  round-trip/frame-identity sources; retain the exact replay labels and paid quotas.
- Retire the old union arms and unreachable mismatch branches; no fallback codec,
  new transfer policy, storage migration or replacement issuance. Host has no direct
  caller of these three mutations in the searched current source.
- Scoped rustfmt, whitespace and removed-reference/method-identity checks pass. Builds,
  Clippy, generated artifacts and installed execution remain deferred. Evidence:
  `/tmp/canic-retirement-wire/`. The complete batch remains **not push-ready**.
- The newer source cut above supersedes this stage’s remaining-command inventory.

## Previous coordinated command-family source cut
- Migrate 28 of the previous 38 command variants together: funding grants and policy
  rotation, funding switch, Root acknowledgement, refill preview/execution, child/peer
  allocation, component/subtree removal, deletion intent/completion, registry join/
  activation/synchronization, pool maintenance/handoff and Root Store adoption/bootstrap/fixture/release
  publication. Separate native methods replace those union arms; no fallback codec.
- Update runtime calls, Host registry/Store submissions, installed caller/denial/replay
  cases, exact reply receipts and response projections. Preserve caller authority,
  Prepared/restoration gates, bounded/unbounded wait choice, attached cycles, original
  deadlines, canonical registry/storage hashes and lifecycle workflow owners.
- CLI cycle conversion also uses native refill submission/receipt validation; dry-run
  reports method and exact frame hex. Retained Nat-bearing status remains Candid.
- Migrate replay entries with their exact labels, cost/quota/reserve policy; add native
  complete-graph/schema, maximum funding-envelope, receipt identity and admission
  regression sources. Delete exhausted peer/child fixture projections and common
  acknowledgement/fixture command helpers. Add CLI/probe direct Wire dependencies
  and matching lock entries without upgrading the graph.
- Scoped rustfmt, whitespace/reference and method-identity review only. Builds,
  schema generation, strict Clippy and installed execution remain deferred. Generated
  Candid snapshots are intentionally stale pending the coordinated artifact build.
  Evidence: `/tmp/canic-command-families/`. Whole batch remains **not push-ready**.
- Remaining command transport: 7 Root variants and 3 Coordinator variants below.
  Continue pool import, inspection, batch component provisioning/
  directory synchronization, Root removal and retirement as connected families.
- Pool maintenance and registry synchronization now use native methods throughout
  Host and installed callers, including uncertain-creation replay and paused-Fleet
  rejection. Inspection retains arbitrary-size external management evidence and
  can now use the Wire 0.7.12 magnitude convention; its caller cut remains.
- Top-level Component reservation and registry preparation now use native methods,
  exact Host receipt/status correlation and installed replay/fencing cases. Retire
  the exhausted Candid submission helper and state-cascade command projection.
- Native Host calls still pass through the existing retained Candid artifact check.
  This check is not Wire schema authority: finish the exact schema/hash journal
  cut before release. No old digest was renamed or reinterpreted and no retained
  spending authority was rewritten. The remaining 12 read selectors, generated
  schemas/clients, artifacts and qualification remain part of this batch.

## Previous Root activation source cut
- Preparation/resume now use separate native controller-owned updates. Retain
  restoration/Prepared admission, exact credential generation and operation receipt,
  original lifecycle orchestration and response-idempotent command labels.
- Installed activation callers submit native requests; existing Prepared credential
  discovery and completion polling remain unchanged. Delete the old command variants
  and fixture selectors; add identity/schema and update-only admission sources.
- Scoped formatting/static review only; builds/tests/artifacts remain deferred.
  Evidence: `/tmp/canic-root-activation-wire/`. Whole batch is not push-ready.

## Previous pool-import observation source cut
- Separate native context and progress queries replace Root's two import selectors.
  Reuse pool-census import evidence and native Fleet binding; lossless context and
  bootstrap projections retain every reviewed source and identity.
- Migrate signed Host review/recovery, HTTP failure fixture, installed progress/
  context/unregistered-Root cases and timing correlation. Preserve controller/
  Prepared gates, exact authority/plan/sequence checks, deadlines, typed refusals,
  and the unchanged durable update-submission boundary. No paid/storage changes.
- Delete exhausted Candid observation projections/comparisons. Added identity,
  bootstrap and native-schema sources. Scoped formatting/static checks passed;
  execution/artifacts deferred. Evidence: `/tmp/canic-import-reads-wire/`.
- Funding census migration can proceed using the Wire 0.7.12 convention from
  [Wire #26](https://github.com/dragginzgame/ic-wire/issues/26); it remains local work.
  New completions are bounded to u128, but release assessment deliberately preserves
  retained arbitrary-size `Nat` overflow evidence. Do not narrow or refuse the whole
  census. Published Wire 0.7.10 needs a documented lossless representation and native
  metadata path; existing supported primitives plus guidance would suffice.
- Other Root/Coordinator flows, Host journal/schema authority and qualification
  remain unfinished. Whole batch remains not push-ready.

## Previous pool-release census source cut
- Root pool-release discovery now has a native controller-owned query. Migrate signed
  Host collection, installed capacity-import census and the opaque signed-query
  fixture; delete the old selector/projection and Candid-only decoding regression.
- Preserve controller/Prepared admission, signed identity, before/after custody and
  Registry verification, deadlines, response/aggregate byte bounds, exact root/subnet
  checks and whole-result refusal. No settlement, spending or storage policy changes.
- Complete native evidence retains all controllers, reservation/call/debit authority,
  import stages and receipts, cycle balances, uncertain creation, funding deadlines
  and pending handoffs. Added all-stage/schema sources; migrated two remaining
  control-plane pool/provisioning census codec regressions from Candid to Wire.
- Scoped formatting/static checks only; builds/tests/generated artifacts deferred.
  Evidence: `/tmp/canic-pool-census-wire/`. Whole batch remains not push-ready.
- Continue funding observations and other Root/Coordinator workflows, coordinated
  Host journal/schema authority and generated/installed qualification.

## Previous provisioning-release census source cut
- Root provisioning discovery now uses a native query with controller/Prepared
  admission. Migrate signed Host collection, installed Root discovery and the raw
  signed-query fixture; remove the exhausted Candid selector/projection and obsolete
  Candid type-table tests. Canonical journal and stored failure encodings are intact.
- Preserve before/after Registry/custody verification, signed caller identity,
  deadlines, per-page/aggregate byte bounds, operation cap, exact Root/header checks,
  advancing owner-aware cursors and whole-result refusal. Native projections retain
  both active owners, delivery target, full dependency failures and retry deadlines.
- Added native frame/conversion/schema and query-kind sources; existing installed
  authorization/no-effect replay and Host assessment cases now target Wire.
  Formatting/static checks passed; builds/tests/artifacts remain deferred. Evidence:
  `/tmp/canic-provisioning-census-wire/`. Whole batch remains not push-ready.
- Continue other Root/Coordinator workflows and Host journal/schema authority.
  This cut removes one remaining status selector, not the other lifecycle flows.

## Previous Root Store reporting source cut
- Root Store overview now has its own controller-owned native query; its composite
  fixture-status query also uses Wire. Migrate Host source-preparation observation,
  installed recovery and topology callers, then delete the exhausted overview
  selector/projection and Candid-only derives/qualification.
- Preserve Prepared query access, composite-query kind, exact content/prefix checks,
  absent-source uncertainty and authority errors. Native overview conversion retains
  all principal, publication, timestamp, GC, capacity and template fields; storage
  and existing journal hash encodings remain unchanged.
- Added complete projection/frame/schema, query-kind and Host observation regression
  sources. Scoped formatting/static checks only; execution/artifacts remain deferred.
  Evidence: `/tmp/canic-root-store-reporting/`. Whole batch remains not push-ready.
- Continue Root/Coordinator workflows and Host release/journal schema admission;
  this removes one union selector and one directly exported Candid query.

## Previous template publication and reply-barrier source cut
- Template publication now uses bounded native Wire across its endpoint, runtime
  client, Host executor and installed concurrent/discarded-reply callers. Exact
  preparation, hashes, borrowed bytes, publication authority, bounded waits and
  receipts are retained. Remove the borrowed Candid request and generic runtime
  Candid call helper; canonical storage/action encodings remain unchanged.
- Internal fixture arm/release/waiting controls now use separate native frames.
  Preserve their controller-only guard and fixture-only emission without introducing
  startup/Fleet fences that could prevent releasing a reply during recovery.
- Added full-size/oversized, owned/borrowed equivalence, method/trailing-frame and
  barrier-shape regression sources. Scoped formatting/static review only; no builds,
  tests or artifact regeneration. Evidence: `/tmp/canic-template-bytes-wire/`.
- Store production endpoint source transport is native. Remaining coordinated work:
  Host release/journal schema admission, other role flows below, generated schema/
  artifact propagation and deferred installed qualification. Not push-ready.

## Previous Store status retirement source cut
- Store authority, activation-operation and GC-operation reads now use separate
  controller-owned native queries. Migrate unbounded Root recovery calls, Host
  authority comparisons and inactive-activation review; remove status/operation
  unions, the Store wire-emission macro and its Candid activation projection.
- Reuse the complete native activation projection; retain every authority field,
  exact operation checks, GC selection precedence, Prepared controller admission,
  bounded Host observation scheduling, fresh retries and lost-response reconciliation.
  Retained Candid source/mutation records remain truthful evidence, not Wire hashes.
- Added native authority/frame/schema, zero-ID, replay and Host wrong-method sources.
  Existing Root/Store mismatch and observation-draining fixtures now use native Store
  replies. Scoped formatting/static checks only; builds/tests/artifacts deferred.
  Evidence: `/tmp/canic-store-status-wire/`. Whole release batch is not push-ready.
- Store's owned command/catalog/status dispatchers are now gone. Remaining Store work:
  template byte publication, internal reply-barrier test controls and coordinated
  Host release/journal schema admission, followed by deferred installed qualification.
  Other Root/Coordinator flows remain in the inventory below.

## Previous fixture byte transport source cut
- Fixture upload and authenticated pull now use bounded native Wire frames and byte
  strings. Migrate Host uploads, managed importer, installed publication and the real
  commit/discard-reply recovery caller. Preserve exact grant caller/revision checks,
  source hashes, sequential upload/replay, all post-await importer fences and distinct
  codec/transport/public/Store failures. No stored encoding or chunk-size changes.
- Fix the internal grant endpoint's missing reply-barrier wrapper: fixture builds
  may await the retained grant/revoke response checkpoint; ordinary builds stay sync.
- Add full-size/oversized and typed-error/schema regression sources, exact read-grant
  conversion and Host native-upload/progress coverage. Scoped formatting/static checks
  only; compilation/tests/generated artifacts deferred. Evidence:
  `/tmp/canic-fixture-bytes-wire/`. Whole release batch is still not push-ready.
- Remaining Store work: authority/operation reads, template publication, internal
  reply-barrier test controls and coordinated Host release/journal schema admission.
  Mutation journals still retain their real Candid authority; no hash relabelling.

## Previous Store catalog retirement source cut
- Separate fixture progress, current grant and template staging Wire queries now
  replace the catalog dispatcher. Migrate bounded runtime clients, Host publication
  observations and twelve installed recovery/fixture query sites; remove the catalog
  union/constant and Candid qualification. Existing native grant identities are reused.
- Preserve Root/controller authority, Prepared query admission, nested fixture
  refusals, revoked revisions, all manifest/chunk evidence and exact template/version
  correlation. Host retains ICP CLI identity, one-query semantics and fresh-pass
  caching; native observation errors retain typed remote versus framing failures.
- Host mutation actions still bind their real Candid path/hash. This read cut does
  not claim native release/journal authority or modify those retained records.
- Native payload/schema, query-policy and Host error regression sources added.
  Scoped formatting/static checks only; compilation/tests/generated artifacts deferred.
  Evidence: `/tmp/canic-store-catalog-wire/`. Whole batch remains not push-ready.
- Next: Store authority/operation reads and byte publication/pull lanes, coordinated
  Host journal/schema admission, other role flows and deferred qualification.

## Previous Store command retirement source cut
- All Store command selectors are migrated. Native grant and deletion-cycle
  reclamation updates now replace the final two variants and bounded production
  callers. Delete command/response unions, dispatcher, variant replay inventory
  and special Candid ingress inspection; ordinary payload inspection owns ingress.
- Reuse native grant binding projections and preserve stored encodings, exact CAS,
  revoke tombstones, lost-reply checkpoints, complete-GC checks, controlled cycle
  destination and quota/reserve authority. Updated installed grant cases and a
  remaining state-cascade GC caller; dependency diagnostics name actual methods.
- Added transport/identity/CAS, cycle evidence, schema/frame and lifecycle/replay
  regression sources. Formatting/static checks only; builds/tests/artifacts deferred.
  Evidence: `/tmp/canic-store-final-controls/`. No new upstream Wire gap found.
- Store catalog/status reads, publication byte lanes and Host journal/schema
  integration remain; other role flows and qualification are unfinished. Generated
  DID/artifact files have not been regenerated. The whole batch is not push-ready.

## Previous Store activation source cut
- Native credential preparation and activation now replace two Store selectors and
  their command projection. Production Root calls retain unbounded waiting, exact
  receipts and status reconciliation after lost replies; initialization still runs
  only on the first successful activation transition. Root guards, Prepared fences,
  original replay policy labels and typed failure stages are retained.
- Added frame/evidence/receipt and lifecycle regression sources. Formatting/static
  checks only; builds/tests/artifacts deferred. `/tmp/canic-store-activation-wire/`.
- Activation status still uses the existing read projection; Host publication journal/schema,
  other role flows and full qualification remain unfinished. Not push-ready.

## Previous coordinated Store controls source cut
- Five controls now use native Wire: template inspection, chunk-set preparation,
  manifest staging, fixture-source preparation and GC initiation. Migrate bounded
  runtime callers and installed prepare/stage callers; remove exhausted selectors.
- Preserve Root/publication-caller guards, internal endpoint selection, Prepared
  Store update access, monotonic policies, exact hashes/manifests and typed nested
  Store failures. GC keeps target-state intent and exact operation receipt checks.
- Fixture descriptor admission measures the actual native frame. Content hashes,
  stored encodings, grant CAS and existing publication/GC engines are unchanged.
  Remaining Store command dispatcher is uniformly Root-only and no longer needs
  variant-dependent authentication. Chunk publication and Host journal identity
  migration remain outstanding; no fallback transport or hash relabelling added.
- Native graph/payload/error, lifecycle and replay regression sources added.
  Formatting/static checks only; builds/tests/artifacts deferred. Evidence:
  `/tmp/canic-store-controls-wire/`. No confirmed upstream Wire gap to file.

## Previous Store inventory source cut
- Native Store inventory/storage methods replace Catalog/Storage selectors. Runtime
  bounded callers, observatory and retirement diagnostics now use exact native frames.
  Preserve Root/controller authentication, Prepared reads, all storage/GC fields,
  typed refusals and dependency-step diagnostics. No storage encoding changes.
- Observatory's native observations admit exact installed schema/module/method/mode/
  limits without a DID prerequisite. Candid observations retain their existing path;
  deadlines, bounded output and attempted-query accounting remain in force.
- Remove exhausted observatory projections. Native graph/frame/error, exact-role
  lifecycle and missing-installed-schema regression sources added. Formatting/static
  review only; execution/artifact generation deferred. No confirmed upstream issue.
- Evidence: `/tmp/canic-store-reads-wire/`. Whole release remains not push-ready.

## Previous application initialization source cut
- Native `canic_root_bind_component_initialization` replaces the Root selector.
  Host emits exact Wire bytes; installed application/Blob journeys and lost-reply
  submissions use the same method. Delete two exhausted Candid projections.
- Keep controller authentication, Prepared access, restoration fence, idempotence,
  operation/target binding and the previous ingress ceiling. Inner application
  bytes, stored hashes, atomic retention and installation effects are unchanged.
- Add maximum-size/schema and malformed-frame regression sources; both installed
  journeys check unchanged allocation state on invalid ingress. No tests/builds or
  artifacts run. Formatting/static checks only: `/tmp/canic-initialization-wire/`.
- This does not complete the Host Fleet mutation-journal/schema authority switch;
  that remaining flow must migrate with its actual effects and retained identities.

## Previous Root auth/proof source cut
- Six native methods cover issuer configuration, public-key discovery, delegation
  proof retrieval, renewal reporting, and role-attestation preparation/retrieval.
  Preserve controller/component guards, capability selection, update/restoration/
  Prepared fences, signing policy, quotas, canonical payload bytes and deadlines.
- Auth CLI now admits the exact installed Root schema and uses typed ICP Wire;
  remove its DID lookup/parser/text arguments and report installed-release metadata.
  Preserve issuer drift diagnostics and distinct malformed/remote/transport errors.
- Migrate PocketIC public-key/attestation and instruction-audit callers. Remove five
  exhausted Candid projections and Root auth-status union. Remaining Root unions
  no longer vary with capability selection, so delete their emission macros and
  share the canonical declarations directly with endpoints.
- Add native method graph, canonical signed-byte/policy, error/frame and lifecycle
  regression sources. Formatting/static checks only; builds/tests/artifacts remain
  deferred. Evidence: `/tmp/canic-renewal-wire/`. No confirmed upstream Wire gap.

## Previous Root capability source cut
- Five native methods now cover acknowledgement, allocation, creation, recycling
  and funding. Production RPC, funding/recovery fixtures, allocation peer and
  instruction-audit callers use the same exact frames and response correlation.
- Preserve structural caller authority, update/restore/Prepared fences, original
  unbounded waits, operation/replay metadata and existing quota/reserve policies.
  Remove the capability command selector, exhausted projection, generic dispatch
  policy and internal envelope Candid derives; stored request encodings remain.
- Regression sources cover all routing identities, native graph, payload/principal
  preservation, replay metadata, response mismatch and exact lifecycle roles.
- Evidence: `/tmp/canic-root-rpc-wire/`. Formatting/static checks only; no builds,
  tests or generated artifacts. No confirmed upstream Wire gap to file.

## Previous issuer status / CLI source cut
- Native public `canic_active_delegation_proof` replaces managed auth-status
  selectors. Preserve issuer profile gating, Prepared reads, native identities,
  all four status states, certificate hash and original proof/refresh deadlines.
- Auth CLI uses typed ICP Wire reads after installed release/schema/module admission.
  Host's existing schema contract validator now also admits one exact retained
  method (name, ID, query mode, request/response budgets) before a read.
- Delete exhausted managed auth types/projection and issuer Candid parser/argument/
  target plumbing. Root renewal status is migrated in the newer source cut above.
- Keep drift/unavailable reporting and distinct codec/transport failures. Native
  state/frame/schema and retained-method mismatch regression sources added.
- Evidence: `/tmp/canic-auth-status-wire/`. Formatting/static checks only; builds,
  tests and generated artifacts deferred. No confirmed IC Wire defect to file.

## Previous non-root capability source cut
- Shared native `canic_request_cycles` serves managed child-provisioning profiles
  and Store. Production non-root RPC selects that exact method. Root RPC is migrated in
  the newer capability source cut above.
- Keep structural caller checks, update guard, Prepared-state access, original
  metadata, replay-first child funding, unbounded wait and typed funding refusals.
  Request bytes remain capped at 16,384; codec errors retain CODEC_FAILED/INVALID.
- Remove exhausted managed command union/projection/replay table, unused generic
  role-command selector and managed-only ingress dispatcher. Shared native endpoint
  has the exact existing RequestCycles replay/quota/reserve classification.
- Regression sources cover full u128 amounts, original replay metadata, typed
  refusals, wrong identities/trailing bytes, public errors, schema and lifecycle.
- Evidence: `/tmp/canic-cycles-wire/`. Formatting/static review only; builds/tests/
  generated artifacts deferred. No confirmed upstream Wire gap identified.

## Previous issuer proof installation source cut
- Native `canic_install_delegation_proof` and Root's provisioning/renewal caller
  replace the old install selector/projection. Preserve controller/update guards,
  issuer profile selection, Prepared admission and original unbounded waits.
- Full proof projections preserve canonical signed/stored fields and original
  deadlines. No new auth mechanism, issuer custody or storage contract. Request
  byte ceiling remains 16,384; issuer rejection stays distinct from codec failure.
- Add native graph, canonical-byte round-trip, response identity/trailing rejection
  and lifecycle regression sources. Tests/builds/artifacts remain deferred.
- Evidence: `/tmp/canic-proof-wire/`. No confirmed upstream Wire defect identified.
  Auth status and its CLI caller are migrated in the newer source cut above.

## Previous runtime reporting source cut
- `canic_runtime_operation` replaces the managed control-status unions. Preserve
  Root authentication, Prepared-state reads, original operation correlation and
  complete fixture/deployment/activation/directory evidence. Storage unchanged.
- Production bounded reads and managed App/group, installed baseline and lifecycle
  callers use native frames. Delete exhausted projection files and old method name.
- Add native metadata and maximum-manifest/frame regression sources; keep original
  credential deadlines and canonical stored authority bytes. No executed proof yet.
- Fix missing public protocol facade exports for pool retries, Fleet controls,
  cascade and runtime configuration/reporting discovered during static review.
- Evidence: `/tmp/canic-runtime-reporting/`. Targeted rustfmt/reference/whitespace
  review only; compilation/tests/generated artifacts deferred. No confirmed IC Wire
  limitation to file yet; maintainer authorizes relevant upstream feedback issues.

## Previous coordinated source batch
- Shared native `canic_synchronize_state` now serves managed and Store recipients.
  Store topology uses `canic_synchronize_topology`. Preserve parent authentication,
  stopped-state recovery for state only, Store topology update gating, request
  limits, original unbounded waits, typed errors and target-bound receipt checks.
- Delete role-specific cascade transport unions, dispatch and fixture selectors.
  Remove transport-only StateCascadeEndpoint routing. Root inventory duplicate,
  anonymous/self checks and downstream reconciliation remain unchanged.
- `canic_configure_runtime` takes complete native directory/group authority and
  returns the original operation receipt. Preserve Root-only authorization,
  update/lifecycle gating, automatic-topup selection, fresh admission opening and
  bootstrap scheduling. Production bounded calls and managed App/group/lifecycle
  fixtures use the same method. Stored authority hashes and JSON bytes unchanged.
- Supply native graph projections and validated deployment-label metadata/client
  checks. Remove exhausted configuration selectors and fixture projection. Roles
  without either remaining optional command provider no longer emit an empty
  Candid command union. Runtime operation status is now cut in the newer slice above.
- Added regression sources for frame identity, canonical authority bytes, schema
  completeness, labels and exact role/kind admission. Targeted rustfmt and static
  reference/whitespace checks only. Evidence: `/tmp/canic-coordinated-wire/`.

## Current continuation boundary
The owned role-union selector cut is complete in Rust source. Normal builds already
produce schemas; release assembly now re-admits their final-Wasm contract. Generated
artifacts and runtime qualification remain deferred; neither source completion nor
formatting proves installed behavior.

## Ordered continuation work
1. Audit remaining direct methods and generated delivery across managed packages,
   schema producers and independent consumers. Audit Candid-derived installed
   profile metadata separately from native transport; preserve external/provider,
   initializer and stored encodings.
2. Complete installed component-removal acceptance and regenerate schema/client/
   artifact delivery when builds resume. Audit Root source, canonical fixture
   schemas and Fleet/Component journal authority are now connected.
3. Run focused native/schema, Wasm/feature, strict Clippy and installed recovery
   qualification when tests resume. Keep #613 open until default-branch delivery
   is substantiated. No native macOS or deployed-IC qualification is implied.
