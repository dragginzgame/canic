# Audit Summary - 2026-05

## Included Run Days

| Day | Summary | Status |
| --- | --- | --- |
| `2026-05-01` | `docs/audits/reports/2026-05/2026-05-01/summary.md` | complete |
| `2026-05-07` | `docs/audits/reports/2026-05/2026-05-07/summary.md` | complete |
| `2026-05-09` | `docs/audits/reports/2026-05/2026-05-09/summary.md` | complete |
| `2026-05-10` | `docs/audits/reports/2026-05/2026-05-10/summary.md` | complete |

## Month-Level Status

Status: **complete**.

May has day summaries for the currently recorded audit days.

## Carry-Forward Follow-up List

1. `canic-core` auth/capability maintainers: keep role-attestation issuance and
   verification checks centralized; review again during the next
   `audience-target-binding` recurring run.
2. Audit maintenance: keep future `audience-target-binding` runbooks aligned
   with current test names and the required role-attestation audience DTO shape.
3. Audit maintenance: keep `token-trust-chain` aligned with the current
   self-contained delegated-token verifier and root-key cascade tests.
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
