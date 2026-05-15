# Audit Summary - 2026-05

## Included Run Days

| Day | Summary | Status |
| --- | --- | --- |
| `2026-05-01` | `docs/audits/reports/2026-05/2026-05-01/summary.md` | complete |
| `2026-05-07` | `docs/audits/reports/2026-05/2026-05-07/summary.md` | complete |
| `2026-05-09` | `docs/audits/reports/2026-05/2026-05-09/summary.md` | complete |
| `2026-05-10` | `docs/audits/reports/2026-05/2026-05-10/summary.md` | complete |
| `2026-05-11` | `docs/audits/reports/2026-05/2026-05-11/summary.md` | complete |
| `2026-05-12` | `docs/audits/reports/2026-05/2026-05-12/summary.md` | complete |
| `2026-05-13` | `docs/audits/reports/2026-05/2026-05-13/summary.md` | complete |
| `2026-05-14` | `docs/audits/reports/2026-05/2026-05-14/summary.md` | complete |

## Month-Level Status

Status: **complete**.

May has day summaries for the currently recorded audit days.

## Carry-Forward Follow-up List

1. `canic-core` auth/capability maintainers: keep role-attestation issuance and
   verification checks centralized; review again during the next
   `audience-target-binding` recurring run.
2. Audit maintenance: keep future `audience-target-binding` runbooks aligned
   with current test names and the required role-attestation audience DTO shape.
3. Auth maintainers: keep `dto::auth` passive and rerun `token-trust-chain`
   with `audience-target-binding` after delegated audience, role-attestation,
   or endpoint auth macro changes.
4. `canic-backup` and `canic-cli`: move duplicated backup/restore fixture
   builders into crate-local `test_support` modules after functional backup
   testing identifies the stable fixtures.
5. Audit maintenance: keep recurring audit templates aligned with the ICP CLI
   artifact vocabulary in `docs/architecture/build-artifacts.md`.
6. Docs maintenance: keep non-fleet test/audit/sandbox canister placement
   guidance centralized in `TESTING.md`.
7. Audit maintenance: keep lifecycle and freshness invariant templates direct
   and evidence-based; avoid embedding review prose in runnable audit
   definitions.
8. Auth maintainers: keep macro access generation, `AccessContext`, and
   delegated-token verifier ordering aligned when changing authenticated
   endpoint syntax, delegated sessions, or replay semantics.
9. Auth API watchpoint: keep private `AuthApi::verify_token_material(...)`
   private unless a future public helper performs the full endpoint-auth
   boundary, including subject binding and update replay.
10. Capability maintainers: keep `CapabilityProof`, `CapabilityService`, and
    capability envelope DTO changes coordinated across API, ops, workflow,
    metrics, and tests.
11. Replay maintainers: keep capability replay metadata, root replay records,
    delegated-token use markers, and session-bootstrap replay policy aligned on
    the same exclusive `now >= expires_at` boundary.
12. Auth maintainers: keep transport caller and authenticated subject lane
    semantics explicit when editing `AccessContext`, endpoint macro generation,
    delegated-session resolution, or delegated-token verification.
13. Lifecycle maintainers: keep optional macro `init = { ... }` support behind
    zero-delay lifecycle timers so generated IC hooks stay restore/schedule-only.
14. Layering maintainers: keep pure cross-layer identifiers in `ids`, with
    storage-specific persistence implementations kept in storage modules.
15. Workflow maintainers: keep test-only replay harness storage imports from
    expanding into production workflow code.
16. Surface-governance maintainers: keep generated DID surface scans pointed at
    refreshed `.icp` artifacts and exclude internal `test` canisters from
    consumer-facing counts.
17. Facade maintainers: keep default-on `canic_metrics` documented as
    intentional surface whenever endpoint bundle defaults change.
18. Complexity maintainers: keep the remediated metrics, directory placement,
    config schema, and intent storage modules decomposed; new metric families,
    placement states, and schema validation cases should land in focused
    support/test modules rather than re-growing production hubs.
19. Complexity maintainers: watch remaining large config/IC facade files only
    when they become active edit centers.
20. Control-plane maintainers: keep release publication behavior in the focused
    `publication/release/*` modules, and split `publication/fleet.rs` or
    `publication/lifecycle.rs` before adding more phase branches there.
21. Core/runtime maintainers: keep new IC management and provisioning behavior
    in the focused `infra/ic/mgmt/*` and `workflow/ic/provision/*` modules.
22. Facade/build maintainers: keep metrics/config build helpers behind hidden
    `__build`.
23. Operator maintainers: preserve CLI/host/backup ownership boundaries as the
    ICP CLI flow continues.
24. Auth maintainers: keep delegated-session cleanup side effects isolated to
    endpoint access-boundary code.
25. Operator maintainers: keep `canic-cli`, `canic-host`, and `canic-backup`
    dependency direction one-way and facade-free as the operator package
    surface grows.
26. Host maintainers: keep host features on `canic-core`/data dependencies
    unless a future facade dependency is deliberately justified.
27. Package maintainers: keep all fleets and test/audit/sandbox canisters
    explicitly unpublished.
28. Operator maintainers: keep routine post-hard-cut command changes narrower
    than the broad 0.33 release sweeps by deciding early whether the behavior
    belongs to CLI UX, host mechanics, or backup domain logic.
29. CLI maintainers: split or isolate `list` responsibilities before adding
    more live projection columns, fallback logic, or rendering modes.
30. CLI/host maintainers: continue routing fleet-scoped live commands through
    the shared installed-fleet resolver. `list`, `cycles`, `metrics`, and
    `endpoints` use it; `snapshot download`, `backup`, and `status` remain
    candidates.
31. Host/CLI maintainers: move shared ICP response parsing primitives needed by
    both host and CLI into `canic-host`, starting with cycle-balance parsing.
    Initial parser ownership is complete in `canic-host::response_parse`; keep
    future ICP response normalization there instead of adding CLI-local parsers.
32. CLI maintainers: continue splitting large command modules into options,
    transport, parse, and render modules before adding more behavior.
    `endpoints`, `cycles`, `metrics`, and top-level CLI help/global-option
    dispatch are split; backup remains the largest command module but should
    wait for the backup/restore flow to stabilize further.
33. Backup/CLI maintainers: after 0.34 backup/restore functionality stabilizes,
    consolidate repeated fixture builders into crate-local test support modules.
34. Host/backup maintainers: before promoting installed-fleet resolution into
    `canic-host`, move the live registry DTO/parser out of
    `canic-backup::discovery` so host can own environment-aware registry
    resolution without depending on backup-domain APIs.
35. Host/backup maintainers: add receipt convention guidance before receipt
    models grow further: naming, timestamps, truncation, provider metadata, and
    serialization shape should be normalized.
36. Runtime/performance maintainers: rerun `instruction-footprint` after the
    next concrete performance change so the 0.35 baseline gains comparable
    drift deltas.
37. Auth docs maintainers: keep active config docs and examples focused on
    `[auth.delegated_tokens]`, `[auth.role_attestation]`, and per-canister
    `auth` flags.
