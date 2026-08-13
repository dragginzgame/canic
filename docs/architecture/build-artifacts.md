# Build Artifacts

This is the canonical vocabulary for Canic build outputs and generated canister
interfaces after the 0.33 ICP CLI hard cut.

## Project State

- `.icp/` is the local ICP CLI project state root.
- `.canic/` is Canic operator state. Canonical network authorities live under
  `.canic/networks/<canonical-network-id>/`; the friendly Fleet catalog is
  `fleets/catalog.json`, and installed Fleet state is keyed beneath
  `fleets/<fleet-id>/`.

## Environment and Network Vocabulary

Canic uses one name for each distinct target concept:

- `environment` is the target selected by Canic's `--environment <name>`
  option. It may be the implicit `local` or `ic` environment, or a named entry
  such as `staging` declared under `environments` in `icp.yaml`.
- `build_network` is the `local` or `ic` compile-time class baked into Wasm.
- Guarded runtime status projects that same class as typed `build_network`; it
  does not expose a selected environment or a second runtime network label.
- `artifact_environment` is the exact `.icp/<name>/canisters` namespace read
  or written by an artifact operation; it can differ from the deployment
  environment when a named deployment installs artifacts built under `local`.
- An ICP backing network is the entry referenced by an `icp.yaml` environment.
  This upstream configuration term is not another Canic target selector.
- `runtime_variant` is target-specific materialization identity, not a network
  alias.

Canic maps its selected environment to ICP CLI's `-e/--environment` selector.
The upstream `ICP_ENVIRONMENT` variable remains the child-build contract and
carries the resolved `build_network` (`local` or `ic`) into Wasm compilation.
Direct `-n/--network` targeting is reserved for an explicit local replica URL
plus root key and is not a second named-environment path.

## Canister Build Contract

Every Canic-managed canister package declares its runtime role in Cargo
metadata:

```toml
[package.metadata.canic]
app = "demo"
role = "app"
```

The package role is the single source of truth for the build and startup
macros. `canic::build!("<path-to-canic.toml>")` validates that the metadata role
exists in the named App config and emits the compile-time role/config
environment consumed by `canic::start!()`.

`role = "root"` emits the root build cfgs and selects the root lifecycle and
root endpoint bundle. Every other configured role selects the non-root
lifecycle and endpoint bundle. There is no separate public root startup macro.

Ordinary roles may be declared before topology placement so `cargo check` can
run during early development. `canic build <app>` batches the configured Fleet
Subnet Root with every attached Component role, then builds Canic's canonical
`fleet_coordinator` and `wasm_store`. `canic build <app> <role>` selects one
deployable configured role. Both forms reject declared-only roles before Canic
writes deploy artifacts.

Complete App builds classify Component artifacts under Application Wasm and
the Fleet Coordinator, Fleet Subnet Root and Wasm Store under Infrastructure
Wasm. The configured root retains the shared Cargo build for efficiency and
uses `shared` rather than a fabricated per-role duration; Coordinator and Store
row-level `ELAPSED` values cover their dedicated builds. Both tables show each
resolved Cargo package version as the second column.

One complete configured build issues one Cargo command per Cargo workspace and
selected profile. Artifact copying, shrinking, Candid extraction, metadata
embedding and gzip materialization run concurrently after that shared Cargo
command succeeds. A release-build identity is compiled only into each leaf
canister; changing it does not invalidate `canic-core` and the rest of the
shared dependency graph.

A successful install finalizes its immutable release build beneath
`.canic/release-builds/<release-build-id>/`. An exact interrupted install
automatically reuses those finalized artifacts. A separate fresh Fleet in the
same ICP root may select them explicitly:

```text
canic install <app> <fleet> --fleet-input <path> --release-build <id>
```

Reuse skips Cargo and artifact transforms. It remains fail-closed: the stored
Canic builder version and profile, current App topology and role/package
identities, canonical artifact manifests and exact artifact bytes must still
validate before activation. If `--profile` is also supplied, it must name the
finalized build's profile.

Build provenance is opt-in:

```text
canic build <app> <role> --provenance <path>
```

The provenance file is an `EvidenceEnvelopeV1` containing stable
`canic.build_provenance.v1` payload. It records source, Cargo, package
metadata, build-profile, artifact hash, and optional artifact-transform
evidence after a successful build. Each transform record identifies the role,
transform, tool, reported tool version, and outcome. An applied transform
requires a non-empty tool version; unavailable or unrequested transforms cannot
claim one. Missing or inconsistent transform evidence makes the payload
invalid. It does not change deployment truth, install state, controllers,
topology, or artifact registry state.

Saved build provenance can be supplied back to passive evidence envelopes:

```text
canic app adoption report <app> --profile <profile> --evidence-envelope --build-provenance <path>
canic deploy check <fleet> --evidence-envelope --build-provenance <path>
```

Those commands only fingerprint the saved provenance envelope as input
evidence. They do not re-run builds or treat provenance as deployment truth.
See [Build Provenance CI Policy](build-provenance-ci-policy.md) for recommended
CI checks over dirty source state, Cargo lock drift, package metadata identity,
and artifact hashes.

## Canister Artifacts

- Direct Cargo canister builds emit raw wasm under
  `target/wasm32-unknown-unknown/<profile>/canister_<name>.wasm`.
- Canic release artifacts are gzip-compressed `.wasm.gz` files owned by the
  Canic build scripts.
- ICP CLI-visible canister artifacts live under
  `.icp/<artifact-environment>/canisters/<role>/`.

## Candid Extraction

`canic::finish!()` emits the `ic_cdk::export_candid!()` pointer only when the
canister build script selects the local build network. Host builds use that
pointer to extract Candid from the exact selected-profile Wasm used for local
development artifacts.

For `ICP_ENVIRONMENT=local`, Canic:

- builds each configured workspace once in the selected profile;
- extracts App-role Candid from that exact selected Wasm;
- writes `.icp/local/canisters/<role>/<role>.did`;
- embeds public `candid:service` metadata into the local Wasm artifact for
  local `icp canister metadata` inspection.

Fleet Coordinator and Wasm Store are deliberate exceptions to debug
extraction during ordinary builds. Their published source crates own checked-in
canonical DIDs, which local infrastructure builds copy and embed directly:

- `crates/canic-fleet-coordinator/fleet_coordinator.did`
- `crates/canic-wasm-store/wasm_store.did`

After an intentional infrastructure endpoint change, the low-level artifact
builder's `--refresh-canonical-did` option extracts the already-built selected
local Wasm and rewrites that role's canonical contract without a second debug
build. A generated downstream wrapper with no canonical source package also
extracts Candid, but never treats its generated sidecar as source truth.

For `ICP_ENVIRONMENT=ic`, Canic intentionally skips Candid extraction, removes
the generated `.did` sidecar path, and does not embed `candid:service`
metadata. This keeps production Wasm artifacts from carrying local interface
metadata bloat. Running `candid-extractor` directly against an `ic` release
Wasm is not the supported path; use the selected-profile local extraction path
instead.

`ic-wasm` remains optional. When it is present, Canic records its version and
whether shrink or local Candid metadata embedding was applied. When it is
absent, the build remains valid and records `tool_unavailable`. Production
Candid metadata embedding records `not_requested`. A present tool that cannot
report a version or complete the requested transform fails the build.

## Audit Usage

Recurring audits should use this vocabulary instead of restating tool-specific
paths inline. When an audit needs a concrete local artifact-environment path,
use `local` unless the report preamble names another environment.

Ignored `.icp/local/**` Candid sidecars are local build artifacts, not release
evidence. Before release closeout, either regenerate them from the current
checked-in exports or exclude them explicitly and rely on tracked sidecars,
protocol-surface tests, and release-set builds.
