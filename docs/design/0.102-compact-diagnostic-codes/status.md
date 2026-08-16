# Canic 0.102 Implementation Status

Date: 2026-08-16

## Status

- State: B1 is closed and maintainer-approved. B2 has materialized its 991
  codes as distinct registered runtime identities plus one permanent host
  ledger, rich typed catalogue, language-neutral current registry and CLI
  lookup. No public error shape, internal string-first propagation or
  diagnostic-owned stable record has changed. The maintainer has explicitly
  excluded unrelated timer/recovery and Saltz work from the 0.102.2
  diagnostic release decision.
- Baseline: clean `main` tag `v0.101.53` at
  `23c0328f78b215580d734ef01b52b35fa3e38ade`; current-candidate
  control-plane/core source is pinned at
  `0750c309104b111fa6f5a1b3355c04fcb38faf71`.
- Release boundary: 0.102 is reinstall-only. Every Canic-owned Fleet canister
  must use one admitted release set before activation. Same-release retry,
  backup, restore and interruption recovery remain required.
- Published checkpoints: `v0.102.0` at
  `e6dfd7d2d212f9fce4b1b16caba33d8062e3461d` and `v0.102.1` at
  `86763c5f16478e2e548e2059e5efaa963bf9a966`.
- Review work is the complete B2 allocation-authority batch in the existing
  untagged `0.102.2` changelog draft. Diagnostic B2 changes runtime scaffolding
  and host lookup but no Candid or diagnostic state. Package-version mutation
  remains maintainer-owned.
- Mutation gate: the maintainer approved the complete B1 register on
  2026-08-16 and authorized B2 mutation. The B3 public hard cut remains a
  separate review boundary. A four-digit initial allocation remains rejected.

Detailed source-by-source working evidence is kept outside this design
directory in the
[0.102 diagnostic inventory](../../audits/working/0.102-diagnostic-inventory/index.md).
The design directory retains only the normative design, this tracker, the
complete allocation-review proposal and the permanent allocation-ledger
contract.

## Release-Batch Plan

| Batch | Outcome | Required evidence and cleanup | Status |
| --- | --- | --- | --- |
| B1 | Freeze current diagnostic authority and Wasm baseline | Complete producer, dynamic-context and durable-string inventories; many-to-one handling-identity compression map; compressed allocation proposal; host catalogue; projection owners; representative Wasm baseline | Accepted: complete guarded `1..=991` register approved on 2026-08-16 |
| B2 | Add prose-free runtime identity and host catalogue | Distinct raw/registered types, approved allocations, permanent current/retired ledger, exhaustive host metadata, lookup and Wasm absence proof | Complete in the `.2` draft: implementation and direct evidence pass; public wire unchanged |
| B3 | Hard-cut the Fleet-atomic public diagnostic contract | Replace enum-plus-message with `nat16`, update every owned endpoint and generated surface, reject mixed release sets | Pending |
| B4 | Make internal propagation code-first | Remove owned prose concatenation, map typed causes, preserve explicit projections and masked-code observability | Pending |
| B5 | Bound durable diagnostic ownership | Remove redundant prose, preserve recovery-significant state and prove changed lifecycle journeys | Pending |
| B6 | Whole-program cleanup and measured closeout | Remove residue and temporary inventory tooling, regenerate bindings, remeasure Wasm and publish downstream source guidance | Pending |

## B1 Completion Contract

B1 is complete only when all of these are reviewable together:

1. one exact current producer and consumer manifest;
2. complete dynamic public-value ownership and durable-string classifications;
3. one reviewed many-to-one map from every producer observation to a shared
   canonical-condition and handling identity or explicit non-diagnostic
   disposition;
4. one canonical condition, class, semantic origin, host disposition,
   producer set and public projection for each compressed code;
5. operation-correlated numeric observability for every masked diagnostic;
6. nonzero unique allocation rows across permanent current and retired history,
   with no four-digit initial allocation;
7. a host catalogue and generated current registry bijective with active rows;
8. proof that the coverage frontier and mapping table are absent from Wasm;
9. a reproducible representative-Wasm baseline; and
10. explicit maintainer approval recorded before B2 created runtime authority.

## Closed Coverage Frontier

- 2,208 mechanical `InternalError::*` references are classified as 2,514
  effective helper/call-site dispositions by 98 bounded source passes.
- Source-to-ledger reconciliation corrected a nineteen-observation omission in the
  maintained delegated-session helper and one earlier arithmetic undercount of
  the already materialized coverage tables. The qualified frontier now
  contains 2,864 exact provisional identities and 31 safe-projection
  identities, for 2,895 host-only labels. Qualifying the exact identities by
  their symbolic producer anchors yields 3,898 entries and 3,929 total review
  observations with projections. Neither count is an allocation count or a
  proposed runtime identity set.
- All 656 dynamic public-message values are classified: 287 caller-derivable,
  67 sensitive operator-only, 234 authoritatively typed and 68 requiring
  narrow request/status owners.
- All 31 projection observations and eight exact producer observations reused
  as projections have a proposed operation-correlated observability owner.
  Every projection-only row
  also has a proposed host summary, disposition, action and exposure rationale.
- The nineteen corrected delegated-session rows have concrete producer owners
  and complete proposed host/projection/observation metadata; their numeric
  allocation remains unset with the rest of the producer frontier.
- Every exact provisional identity is now a structured row with at least one
  typed producer, source, decision, dependency-boundary, site or call evidence
  cell and a symbolic producer-function anchor. This closes both typed-owner
  materialization and the exhaustive current producer-function manifest.
- The maintained production-consumer manifest is exact at the pinned
  current-candidate source: twelve machine-decision consumers and six
  transparent decode/render consumers have complete structured rows. The guard
  pins their full row set separately from diagnostic-identity arithmetic.
- A conservative producer-anchor pass finds all 2,864 exact provisional
  identities name a symbolic source anchor in a structured owner cell. Its
  empty debt set and fingerprint are pinned; a backticked symbol remains a
  coverage anchor, not proof of production reachability by itself. All twenty
  access-family identities now have exact function/branch anchors and a
  family-specific completeness check. All twelve authority-restore identities
  likewise have exact typed/function branch anchors; eleven were newly closed
  and the endpoint-policy fence was already typed. The twenty direct-prose
  authentication identities now have exact attestation, proof, retention,
  verifier and signer/configuration anchors plus their own completeness check.
  The remaining verifier/signer test-key and compiled-crypto sites are also
  exact, so all 151 authentication identities pass a family-level guard. The
  thirteen runtime-auth renewal/admission and eleven RPC/runtime-crypto
  identities now have exact function/branch evidence and separate guards too.
  The twenty-one prepare-replay and eight prepare/provisioning identities also
  have complete source-addressed family guards. All eleven core chain-key
  batch/approval identities now bind their exact validation, canonicalization,
  planning and installation branches under another family guard. The first
  core Canister-creation funding-overflow helper is also exact. The ten pool
  initialization, reset and claim identities name their exact transition
  functions under a range-completeness guard. All twenty-one
  autonomous creation identities independently bind intent, paid-attempt,
  terminal-evidence, adoption, commit, retry, cancel and rollover branches.
  All ten exclusive-handoff identities bind their begin/completion authority,
  asset-state and terminal-receipt branches under a separate guard.
  Store deletion, configuration, cost/adoption helpers and recycling settlement
  are also exact: all 56 distinct meanings across the pool ops file's 69
  constructor sites are now symbol-addressed without treating repeated sites as
  new codes. The pool workflow is exact too: its seventeen constructor sites
  reference eighteen identities, with three ops reuses and fifteen net-new
  maintenance, import, handoff and recoverable-refill meanings. All twenty-two
  root/Store bootstrap meanings bind exact manifest-envelope, protected-
  topology, artifact-capacity, staged-authority and live-catalog branches; its
  two topology adapters remain transparent typed-cause propagation.
  The root-Subnet/adoption-state ledger also anchors all nine referenced exact
  identities: one access reuse plus eight net-new discovery and sibling-Store
  state meanings; its Registry discovery failure stays a transparent cause.
  All twenty-one Store-lifecycle coverage labels and all fifty Fleet Mirror
  labels now bind their exact producer functions too; transparent Store-client
  and Coordinator-result adapters remain typed cause propagation rather than
  wrapper allocations.
  The Component Directory/Fleet-service peer boundary now source-addresses all
  forty-two exact meanings as well; its typed Component-binding adapter remains
  transparent and the exact count-overflow projection is reused rather than
  allocated again.
  The adjacent durable Component Directory synchronization journal now binds
  all fifty-five coverage meanings to its exact status, acceptance, retry,
  intent, terminal and stable-commit functions; unreachable placement commit
  variants remain non-diagnostic dispositions.
  Fleet-activation/scaling and the five small core ops owners now bind their
  thirteen referenced coverage labels too; typed configuration edges and the
  two impossible scaling shapes remain non-allocating dispositions.
  The five adjacent small workflow owners bind their six referenced topology
  and deadline labels; cost-guard and capability adapters remain transparent.
  The final small-adapter family now binds all twenty-seven candidate/reuse
  identities to exact producers; its twenty-eight materialized coverage labels
  include the exact `FLEET_ACTIVATION_STATE_INVALID` identity reused as a public
  projection. Five typed adapters remain transparent and two impossible states
  remain deliberately code-free, so only twenty-three previously unanchored
  observations leave the global debt set.
  The Component runtime family now binds all seventy-four exact identities to
  its preparation, synchronization, activation, Directory-validation and
  canonical-hash producers. Sixty-four are net-new meanings, ten are semantic
  reuses and seventy-two were previously absent from the global anchor set;
  three storage adapters remain transparent typed propagation.
  The root-issuer/delegation-batch family also closes: its sole materialized
  identity is the already anchored certificate-TTL policy code, while five
  Fleet/storage/policy adapters preserve their typed causes. It therefore adds
  no duplicate identity and does not change global debt.
  Root/non-root lifecycle orchestration also closes without new codes: its two
  materialized access/environment identities are already anchored, while nine
  memory, environment, configuration and runtime-startup edges preserve their
  typed causes across exact lifecycle functions.
  Runtime coordination, restore and activation also bind all twelve referenced
  identities. Existing access, restore and Fleet-activation codes remain
  shared; only the resumable-refill upgrade fence and credential-bundle
  capability fence were previously absent from global producer anchors. Three
  memory/storage edges remain transparent.
  The final Component Registry families now close too: all 239 workflow, 449
  direct Registry-ops and 230 grouped-provisioning labels have exact function
  or typed-predicate anchors. This includes the direct-child and arbitrary-depth
  lifecycle, top-level commit/activation, Directory refresh, root/Store removal,
  canonical accounting/hash and grouped cursor/result frontiers. The global
  producer-function debt is therefore zero.
- Seventeen IC-call families map to their durable operation or guarded status.
  Store publication includes its previously missing attempt owner.
- The 105-row authentication formatter, the native configuration zero-row
  exclusion and the current Canister durable-string census are closed.
- Publication binding/release authority, all 56 GC/reclamation/deletion
  constructions and both management transports are fully expanded.
- No decision parses retained failure text. Four current Cashier coverage
  conditions remain in 0.102 scope; any resulting compressed codes retire
  without reuse if the standalone blob-service extraction is promoted.

These are exhaustive repository-evidence results. The 3,898 qualified exact
entries map exactly once onto 960 shared
exact-condition contracts; 31 safe public projections produce the dense
`1..=991` register. Maintainer approval made those shared rows B2 allocation
authority; their 503 singleton rows each retain an explicit handling/exposure
rationale.
The coverage labels and mapping table remain repository-only and do not enter
Wasm. The maintained public error remains
`ErrorCode + message`, `InternalError` remains string-first and the host still
consumes typed enum variants.

## Current Decisions

- Use dense, monotonic, nonzero numbers with compact unpadded `E<decimal>`
  rendering and no semantic bands.
- Allocate codes for composable canonical conditions and handling contracts,
  not source observations; physical origin, module, role, endpoint and prose
  do not force another code.
- Give each code one canonical semantic declaration path; declaration files
  may be grouped by semantic domain, but producer-local aliases and copies are
  forbidden.
- Reject any four-digit initial allocation as failed semantic compression.
- Keep lossless raw decoded identities distinct from registered producer
  identities.
- Keep class, origin, disposition, labels, summaries and remediation in
  host-only code.
- Retain every allocated number permanently as current or retired; never reuse
  a retired number.
- Install all Canic-owned Fleet canisters from one admitted release set before
  activating the new public contract.
- Do not add a dual protocol, compatibility decoder, diagnostic generation
  name, message fallback or string-based classification.

The dense allocation proposal is in
[allocation-proposal.md](allocation-proposal.md). The permanent allocation
authority contract is in
[code-allocation-ledger.md](code-allocation-ledger.md).

## Validation Evidence

- `CANIC-WASM-001/v3` passes at immutable tag `v0.101.53` over six Components,
  Fleet Subnet Root, Fleet Coordinator and Wasm Store in release and debug
  profiles at risk `5/10`.
- Source-count, semantic-expansion, dynamic-category and projection arithmetic
  reconcile in the working evidence.
- A targeted repository guard derives 2,864 exact provisional identities and 31
  additional projection observations exclusively from explicit coverage
  columns, then pins the complete 2,895-label frontier with a deterministic
  sorted-set fingerprint. No arbitrary prose scan or manual exclusion
  participates in that coverage evidence. It does not authorize 2,895 codes. A
  second check requires structured owner/source evidence for all 2,864 exact
  identities, and a third pins all eighteen maintained production consumers.
  A fourth pins the empty producer symbol-anchor debt set so later source drift
  cannot silently reopen it. The semantic guard expands the identities to
  3,898 producer-qualified entries, rejects every unclassified action and any
  per-producer exposure, action or machine-class conflict, then requires all
  3,929 observations to map exactly once. The 433 entries without one
  unambiguous explicit exposure fail closed to their aggregate projection or
  internal-only handling; the 424 without one producer-qualified action use
  the narrow condition-derived conservative remediation. Both sets remain
  visible review inputs. All 991 proposal rows name producers, all projections
  have mapped exact inputs, the dense allocation stays below four digits and
  the checked-in register equals its deterministic derivation byte for byte.
  The same guard pins the maintainer-approved review checklist for semantic
  grouping, handling contracts, host dispositions, projections, masked
  observability, singleton rationale and non-producer-local reuse, and rejects
  production source references to the review proposal, working evidence and
  planned host allocation assets.
- B2 deterministically derives 991 canonical registered constants, 991 current
  and zero retired permanent ledger rows, the typed host catalogue and a
  byte-identical current JSON registry from the approved register. Direct
  numeric construction and host-asset imports stay outside canister producers.
- Focused host tests prove the complete ledger/runtime/catalogue/JSON bijection,
  anti-reuse and current/retired/unknown behavior. Focused CLI tests cover
  strict numeric parsing, lookup rendering and recursive lexicographic help.
- Warning-denied Clippy passes for the touched core, host, CLI and allocation-
  generator targets. Changelog governance, diagnostic ownership, layering and
  diff hygiene pass for the B2 review boundary. The repository-wide optional-
  idea collection check observes unrelated maintainer work and is not used as
  diagnostic B2 evidence.
- A fresh complete `make validate` passes on 2026-08-16, including repository
  release guards, full workspace tests and every serial PocketIC suite. That
  repository-wide result does not promote unrelated maintainer work into this
  scoped diagnostic release outcome.
- Canonical role-scoped release builds pass for a representative Component,
  Wasm Store and Fleet Subnet Root at 2,879,695, 2,773,920 and 7,497,834 raw
  bytes. Bounded scans find no permanent-ledger header, host owner, catalogue
  prose, symbolic label, working-audit path or projection-map marker in any of
  the three artifacts. This is B2 absence evidence, not a new
  `CANIC-WASM-001/v3` comparison run.
## Next Action

Diagnostic B2 is implemented and its 0.102.2 source, host, CLI, evidence and
release-note surfaces have passed scoped and complete repository validation.
The approved register materializes the permanent ledger, registered
declarations and host catalogue; the public `ErrorCode + message` contract
remains intentionally unchanged. Rerun the stable failure-string census
immediately before B5 mutation. B3's Fleet-atomic public hard cut remains a
separate later batch and is not part of the current handoff.
