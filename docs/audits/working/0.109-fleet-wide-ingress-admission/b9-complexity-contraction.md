# Canic 0.109 B9 Complexity Contraction

Date: 2026-08-31

```text
evidence_status: superseded
baseline_tag: v0.109.31
baseline_commit: 43f0d80d884053720fed253d52b3603a3a439ca2
candidate: v0.109.32 source 641f843ac5bc1ddb823bef6b3c32427a5cca70dc
immutable_superseding_verdict: pass; accepted 2026-08-31
external_effects: none
```

This evidence records the accepted simplification boundary. The
[immutable superseding audit](../../reports/2026-08/2026-08-31/0.109-b9-superseding-complexity-audit.md)
now owns the canonical rerun and accepted passing B9 verdict. The later minor
closeout remains a separate human-owned gate.

## Gravity-Well Ownership

| Measured owner | Starting lines | Current parent lines | Extracted responsibility owners |
| --- | ---: | ---: | --- |
| `ops::component_registry` | 17,452 | 6,303 | allocation/activation, Directory refresh, initial inventory, subtree and Root/Store retirement, tests |
| `workflow::component_registry` | 8,751 | 5,838 | lifecycle scheduling, pool/install reconciliation, authority validation, passive Registry/allocation and retirement response projection |
| `ops::fleet_coordinator` | 8,997 | 2,688 | admission, Registry history, Root join/snapshot/draining lifecycle, Root/Directory/runtime progress and replay classification, Directory call/response authority, retry projection, retained/observed validation, projection and Fleet-service publication |

The extracted modules use the same stable stores, records, ops types and
workflow entry points. No split adds a policy, journal, timer, endpoint,
transport, Candid variant or platform-effect owner.

## Admission Authority And Transition Map

```text
protected Fleet source
  -> host config validation and selector resolution
  -> canonical policy template plus participant catalog
  -> immutable desired/Registry/install authority
  -> Coordinator canonical policy and sole Fleet transition journal
  -> exact Root distribution operation for that Root's participants
  -> exact managed-target local projection and retained receipt
  -> synchronous endpoint guard reads local projection plus msg_caller()
  -> application independently resolves membership/resource ownership
```

| Decision | Sole owner | Passive copies retained at boundaries |
| --- | --- | --- |
| Authored selector validity and widening rejection | host configuration policy | source DTO and immutable desired document |
| Canonical Fleet policy, generation and mutation | Coordinator admission model/ops | Registry policy and protected status DTO |
| Participant membership | protected topology and Coordinator catalog | per-Root participant projection |
| Per-target effective Principal set | Coordinator policy compilation, narrowed by exact rule intersection | Root command and target stable projection |
| Transition operation and successor generation | Coordinator transition journal | Root and target operation/receipt identity |
| Local ingress allow/deny | managed target local projection | no remote cache or caller argument |
| Application membership and resource ownership | application | no Canic admission substitution |

The repeated Fleet, generation, policy digest, participant digest, target and
operation fields are passive transport or persistence bindings. They permit a
receiver to reject substitution without becoming a competing decision owner.

## Retained-Decision Source Identity

Current Fleet Ensure retains the complete normalized reviewed desired input in
the immutable in-progress plan. The CLI resolves that retained authority before
reading newer working bytes, including when the original desired TOML is absent.
Apply binds platform observation and effects to the retained input and considers
newer desired state only after terminal closure.

The bounded current-schema plan that predates reviewed-input retention instead
returns `RetainedDesiredUnavailable { actual, expected }`, where both values are
exact desired-source SHA-256 identities. The diagnostic asks for the reviewed
source document and does not recommend changing the ICP identity or adding
funding. Existing typed regressions cover retained-input resume and the
pre-retention fail-closed path.

## Current-Plan Policy Boundary

`fleet_ensure::policy` is synchronous and imports model data, Principal,
Cycles, hashing and collections only. It owns current desired/live validation,
cycle arithmetic and immutable plan compilation. It does not import
`fleet_ensure::ops`, ICP CLI, PocketIC, clocks, journals or platform effects.

`fleet_ensure::workflow` owns observe/plan/apply ordering and durable intent.
`fleet_ensure::ops::platform` owns mechanical ICP observation and one approved
effect. PocketIC remains test infrastructure. The removed historical install,
recovery-bundle and retained-Root-repair contracts are not compatibility
inputs to this current-plan boundary.

Focused policy evidence:

```text
cargo test -p canic-host \
  conservation_equation_accounts_for_funding_fees_and_burn_separately \
  --lib --locked
# PASS: 1 passed; 432 filtered out; no PocketIC or network driver
```

## Validation Inventory And Provisional Envelope

The repository runner keeps ordinary tests before one serial PocketIC lane.
The current governed inventory contains:

- 32 explicitly ordered `canic-testing-internal` PocketIC cases;
- 2 ignored governed `canic-host` PocketIC journeys;
- 9 classified `canic-tests` PocketIC targets containing 29 test cases; and
- serial capacity of one until a separate stability proof changes it.

The resulting current inventory is 63 PocketIC cases. Source counts are an
inventory statement, not a claim that the dirty B9 candidate passed them.

The provisional release-line ceiling for the final immutable run is:

| Resource | Ceiling |
| --- | ---: |
| Workspace test wall time | 2,100 seconds |
| Shared PocketIC high-water RSS | 6 GiB |
| Shared PocketIC thread high-water | 300 |
| Concurrent PocketIC capacity | 1 |
| Governed PocketIC case count | 63; increases require explicit invariant ownership |

The ceiling is deliberately above the retained complete-run evidence of about
1,718 seconds, 5,037,288 kB and 257 threads. The immutable candidate must emit
fresh elapsed/RSS/thread/case evidence. Exceeding a ceiling requires reducing
duplicate setup or cases, or an explicit maintainer reassessment; it does not
authorize parallel PocketIC execution.

## Targeted Slice Evidence

The current behavior-preserving extractions pass:

```text
cargo check -p canic-control-plane --lib --locked
cargo test -p canic-control-plane \
  active_pool_scale_out_and_restore_preserve_cross_document_authority \
  --lib --locked
cargo test -p canic-control-plane \
  initial_service_publication_commits_registry_receipt_and_phase_atomically \
  --lib --locked
cargo test -p canic-control-plane \
  coordinator_journals_each_root_acceptance_and_reconciles_lost_responses \
  --lib --locked
cargo test -p canic-control-plane \
  coordinator_advances_each_accepted_root_and_freezes_terminal_receipts \
  --lib --locked
cargo test -p canic-control-plane \
  coordinator_accepts_partial_coalesced_directory_publication_and_rejects_regression \
  --lib --locked
cargo test -p canic-control-plane \
  coordinator_accepts_coalesced_terminal_runtime_activation_and_publishes_catalog \
  --lib --locked
cargo test -p canic-control-plane \
  progress_is_a_strict_monotonic_successor \
  --lib --locked
cargo test -p canic-control-plane \
  invalid_or_regressing \
  --lib --locked
cargo test -p canic-control-plane \
  root_join_compare_and_commit_retains_exact_response_receipts \
  --lib --locked
cargo test -p canic-control-plane \
  grouped_root_lifecycle_fence_is_exact_to_referenced_root \
  --lib --locked
cargo test -p canic-control-plane \
  root_draining_reservation_is_durable_hash_bound_and_target_readable \
  --lib --locked
cargo test -p canic-control-plane \
  grouped_allocation_cannot_advance_through_ordinary_lifecycle \
  --lib --locked
cargo test -p canic-control-plane \
  grouped_install_status_requires_exact_context_and_empty_prepared_fence \
  --lib --locked
cargo test -p canic-control-plane \
  peer_requester_access_requires_exact_active_top_level_component \
  --lib --locked
cargo test -p canic-control-plane \
  component_deletion_is_prepared_durable_and_absence_idempotent \
  --lib --locked
cargo test -p canic-control-plane \
  subtree_removal_target_finalization_is_terminal_and_releases_the_live_fence \
  --lib --locked
cargo test -p canic-control-plane \
  grouped_allocation_cannot_advance_through_ordinary_lifecycle \
  --lib --locked
cargo test -p canic-host \
  in_progress_operation_resumes_reviewed_desired_before_newer_input \
  --lib --locked
cargo test -p canic-host \
  pre_snapshot_zero_debit_final_observation_resumes_without_reissuing \
  --lib --locked
cargo clippy -p canic-control-plane --lib --tests --locked -- -D warnings
bash scripts/ci/run-layering-guards.sh
cargo fmt --all -- --check
git diff --check
```

Earlier focused Component Registry preparation and Root-retirement tests are
retained in the detailed changelog. No broad workspace or PocketIC gate was
run during coding.

## Method Disposition

The working candidate now supplies the manual ownership evidence required by
the canonical methods:

- the three measured parents contract by 64%, 33% and 70%, with every extracted
  owner named in the gravity-well table;
- the authority map identifies one admission decision owner and classifies
  repeated identity/hash fields as passive substitution guards;
- the current-plan policy boundary has no transport, journal, clock or
  PocketIC dependency, while workflow and platform effects remain separate;
- no extraction adds a store, journal, timer, endpoint, policy decision or
  platform-effect owner; and
- all moved public/internal behavior retains the same module surface and is
  exercised through the focused regressions above.

The immutable audit now grades the canonical complexity, change-friction,
structure, duplication and module-surface methods. This working document
remains the extraction and ownership ledger; it does not compete with that
release evidence.

## Remaining Closeout Evidence

The human maintainer accepted the passing superseding verdict on 2026-08-31.
Canic-owned B10 implementation and direct proof are complete; immutable
publication plus downstream private-adapter removal remain before the separate
human-owned 0.109 closeout audit.

No item here authorizes versioning, publication, deployment, downstream
mutation or 0.110 implementation.
