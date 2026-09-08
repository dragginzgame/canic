# Generated Fleet Artifact Packages

Date: 2026-09-08
Scope: the accepted generated-build refinement of the open 0.110.10 batch.

## Result

The host generates the three Fleet entrypoint packages through one shared
writer. Root, Coordinator and Store retain distinct Wasm artifacts, exact role
features and protocol/release authority. Generated packages are unpublished,
use the selected Canic version and exact dependency graph, and live beside the
selected configuration under `.canic/generated/`.

Separate workspace/published Fleet entrypoint crates and the Coordinator/Store
package-discovery alternatives are removed. Runtime implementations remain in
Canic and its control plane. Root selects configured capabilities; Store uses
its existing configuration compilation; Coordinator remains independent of App
configuration. The shared writer preserves unchanged source files and existing
locks, avoiding needless timestamp-triggered compilation.

Canonical Coordinator/Store Candid moves unchanged into `crates/canic/candid`,
which ships with the selected facade package. Ordinary builds use that owner;
explicit refresh retains its existing declaration-build boundary. Store still
compares its compiled declaration with the maintained contract. Root Candid
continues to follow its exact configured capabilities.

Passive Store reports derive the host-owned role contract without package
search or generated files. Publication inventories, package guards, crypto
feature checks, fixture build inputs and active documentation follow the new
ownership. The packaged artifact proof uses one isolated package set to build
all three canisters. The redundant second Store source scenario is removed.

## Qualification

| Targeted check | Result |
| --- | --- |
| Host package generation, role contracts, passive reports and provenance | 92 passed |
| Protocol surface / canonical Candid structure | 42 passed |
| Workspace manifests, package metadata and feature docs | 7 passed |
| Checked-in configuration validation | 1 passed |
| Candid/Serde and stable-memory guards | 3 passed |
| Affected-package all-target/all-feature warning-denied Clippy | PASS |
| Internal fixture library with default features disabled | PASS, warning-denied Clippy |
| Wasm crypto closure | PASS |
| Changed shell scripts | PASS, syntax and ShellCheck |
| Isolated packaged Root, Coordinator and Store artifacts | PASS; exact versions, features, dependency paths, Wasm/gzip/Candid and canonical DID equality |
| Prepared Root initial-Shard runtime activation and replay | PASS; 425.19s including cold artifact builds / 489s runner |

Minimal-feature compilation exposed two fixture visibility mistakes. The Root
cache helper now remains available to its unconditional caller, while the
Coordinator read helper follows its callers' feature gate. Both minimal and
complete affected-target lint selections pass after those corrections.

Logs are `/tmp/canic-generated-*.log`. These are targeted source checks, not an
immutable release-validation receipt. Both canonical Candid files are
byte-for-byte unchanged by relocation. Formatting for all 26 changed Rust files,
diff hygiene and changed guide/report links pass.

The complete accepted canonical Root, retained Fleet, read-surface and generated
Fleet package batch is ready for release approval. Implementation, propagation,
cleanup and targeted qualification are complete; the open 0.110.10 changelog is
ready while package versions remain 0.110.9. No in-scope implementation blocker
remains. The maintainer-selected release gate and publication remain separate.
No full workspace gate, version change, publication, live deployment or
downstream repository edit has been made.

The earlier [canonical Root](../2026-09-07/canonical-fleet-root.md),
[retained Fleet](../2026-09-07/retained-fleet-feedback.md) and
[public observability](../2026-09-07/public-observability.md) reports retain
runtime recovery and interface evidence for their recorded source boundaries.
This refinement changes build/package ownership and fixture construction;
it adds no runtime owner, recovery mode or compatibility surface.

## Adoption

Consumers use matching Canic and CLI versions and rebuild sealed artifacts.
They do not depend on separate Fleet entrypoint packages or supply Root source.
App configuration remains the input for generated infrastructure builds.
The [packaged proof guide](../../../../operations/0.56-packaged-wasm-store.md)
describes artifact-only qualification. Application readiness, dashboard
integration and live performance remain downstream validation responsibilities.
CANIC-141 remains deferred.

## Release Preflight Correction

The maintainer's 2026-09-08 preflight rejected the audit catalog. The parent
`check-invariants` failure is the same child failure, not a separate runtime
failure. Reviewing every active definition found five stale fingerprints from
canonical Root/package changes, including scopes still naming removed Fleet
crates. This is an `audit_method_defect` in the maintained audit inputs.

The corrected methods are Lifecycle 4, Dependency 3, Structure 2, Publish 2
and Module Surface 2.1. Their catalog entries and active SHA-256 identities now
agree; superseded identities remain recorded. Scopes follow the host-generated
entrypoints and shared runtime owners. Package checks select the maintained
published roster. The lifecycle scan includes `start_fleet_root!` and the
actual host generation directories.

No audit result using the changed definitions was established by this batch;
the qualification table above records targeted tests and builds, not scored
runs of these methods. Historical reports retain their exact method identity.
Any result claiming the old fingerprint for a changed definition has
`result_validity: invalid`; this correction supplies no comparable baseline or
current-product audit result. A future method comparison must rerun both
snapshots under the corrected method, or report non-comparability when the
original baseline cannot be reproduced.

The audit catalog and current-document semantics guards and diff hygiene pass.
Runtime/build sources are unchanged, so their prior targeted evidence remains
applicable. The full release gate was not rerun; the batch remains ready for
release approval with the corrected open changelog.

## Deployment Unit-Test Fixture Correction

The next maintainer test run reported 20 failures across four library targets.
Three metrics/allocation fixture definitions still supplied a Root package;
the Store client inventory and replica-query wire bytes named removed read
methods; the infrastructure manifest's pinned hash still described its prior
Store package label. The standalone consumer fixture also inherited the outer
workspace when the deployment runner placed it under repository scratch.

The fixtures now use the maintained Root configuration and read contracts,
retain exact wire-byte and manifest-digest assertions, and give the standalone
consumer its own empty Cargo workspace. A source scan also corrected the same
Root package field in the related Fleet peer fixture. Production behavior is
unchanged.

All 32 selected library regressions pass across Canic, control plane, core and
host, including every reported failure and the related peer/manifest/CBOR
checks. The run uses the deployment scratch wrapper, not only system temporary
storage. Log: `/tmp/canic-hard-cut-fixture-tests.log`. No PocketIC or full
workspace suite was run. The corrections extend the open 0.110.10 batch.

## Protocol Test Feature Qualification Correction

The ordinary integration runner compiled `protocol_surface` without the
optional Canic infrastructure features. Its combined read-contract test imported
Coordinator and Store facade DTOs unconditionally. The earlier all-feature
qualification masked those missing test gates; it did not establish ordinary
integration compile coverage. No public DTO feature gate needed widening.

The shared Candid assertion helper now serves separate Coordinator and Store
tests. Each test uses exactly its facade DTO's feature condition. Package-isolated
runs through the deployment scratch wrapper pass with default features (34 tests),
Coordinator only (35), Store only (35), and Root control plane only (35).
Warning-denied Clippy passes for the changed `protocol_surface` target with all
features. Logs: `/tmp/canic-protocol-{default,coordinator,store,root,clippy}.log`.

CI/deployment governance now explicitly requires the ordinary runner feature
selection and directly affected role selections for changed feature-gated
imports/tests, plus deployment scratch for Cargo consumer fixtures. These are
narrow implementation checks, not another full release suite. Formatting and
document checks pass. Production sources and package versions are unchanged;
the corrected open batch remains ready for release approval, with completion
of the maintainer's release gate still outstanding.
