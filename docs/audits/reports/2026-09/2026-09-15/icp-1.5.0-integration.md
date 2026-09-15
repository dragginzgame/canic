# ICP CLI 1.5.0: Canic integration and improvement audit

## Verdict

Canic already pins and runs ICP CLI 1.5.0. The highest-value work is correcting
build-environment propagation and configuration inspection, then qualifying the
current tool contract. This review found a concrete environment mismatch in the
ICP-backed artifact helper. New frontend and observation capabilities are useful
follow-ups; adopting every upstream feature would add competing deployment
authority without solving Canic's existing recovery requirements.

This is a completed integration assessment with code traces and disposable CLI
probes, not complete runtime qualification, a 0.110 closeout, or implementation
approval. No product source, dependencies, installed tools, identities, network
state, package versions, or changelogs were changed. Existing dirty work was
preserved. Recommendations below are proposed work, not accepted release batches.

## Scope, method and identities

Reviewer: Codex, one reviewer. Scope: all 1.5.0 release changes and relevant
1.3/1.4 capabilities carried by the installed version, mapped to Canic's build,
configuration, transport, identity, funding, observation, frontend, backup and
tool-installation owners. Exclusions: sibling repositories, live spending,
deployment, production credentials, full Rust/PocketIC suites, whole-workspace
security certification and measured production performance.

Method: scoped application of the manual ownership/parsing review in
[CANIC-DUPLICATION-001/v1](../../../recurring/system/dry-consolidation.md), with
the feature checklist below and an isolated upstream CLI boundary probe. This
does not claim the method's repository-wide inventories or numeric risk score;
those are outside this integration question. The formal method run is partial.
The high-priority finding remains open for maintainer and independent review;
no second-review acceptance or single-review waiver is claimed.

```text
source_commit_full: a875c6498721bd89ca98549e38389b8280910d68
source_tree_hash: 1b3b78e9004d58f00f973d01de9d2dc2dcf882f1
product_tree_hash: not_applicable (dirty working-tree review, not committed product baseline)
clean_worktree: false
cargo_lock_hash: d0a6a2b74a18345a3c0d2265e22bb6e718b6e96137d4035e734fdc7dfe72b945
rust_toolchain: 1.98.1 (repository declaration; no Rust compilation)
target_triple: x86_64-unknown-linux-gnu (installed native ICP)
feature_set: not_applicable (no Cargo targets executed)
audit_method_id: CANIC-DUPLICATION-001 (scoped application)
audit_method_version: 1
audit_method_fingerprint: c4b2b2828f551a5419de394d442ecb04932900d7b15665177a3c8529ee340262
audit_script_hashes: artifacts/icp-1.5.0/sources.sha256
external_tool_versions: ICP CLI 1.5.0; Bash; ShellCheck
fixture_or_seed: artifacts/icp-1.5.0/probe.sh
environment_class: disposable offline filesystem; no replica or credentials
started_at: 2026-09-15T06:57:15Z (recorded source checkpoint; manual review began earlier)
completed_at: 2026-09-15T06:59:44Z (execution evidence)
run_result: partial
result_validity: valid
closeout_verdict: fail (scoped environment correctness; not a minor closeout)
compared_baseline: not_applicable (first assessment; no performance comparison)
upstream_commit: 763a55fa12c2615d4bf5060c616d9f2b6090c3af
native_icp_sha256: ce8b4b89a5f06589bdeff578359f2da9581195484811495ae8c95c0ccd57bec9
```

The Git tree above identifies HEAD, not the uncommitted product. The
[source manifest](artifacts/icp-1.5.0/sources.sha256) identifies the decisive
working-tree files and probe. All four repository archive pins match the
[official 1.5.0 checksums](https://github.com/dfinity/icp-cli/releases/download/v1.5.0/sha256.sum).
This comparison verifies the pins; it does not independently reconstruct the
installed binary from an archive. Both the PATH npm launcher and the native
Cargo-bin executable report 1.5.0. Canic's existing pinned-tool check passes.

## Feature coverage checklist

| Upstream surface | Canic disposition |
| --- | --- |
| 1.5 environment-selected builds | Immediate correctness fix and useful selective-build qualification; findings F1/F2 |
| 1.5 sync interface 0.2.0: named files/fields, metadata, mappings, URLs, additional call targets | Optional frontend consumer; O3 |
| 1.5 recipe plus explicit sync | Optional post-upload verification; not a pre-upload capacity gate; O3 |
| 1.5 dependency-local environment membership | No dependencies in root ICP manifest; reject or deliberately support this in config inspection; F2 |
| 1.5 environment-pruned bundles | Disposable selection proof passes; optional artifact distribution only; O4 |
| 1.4 build-script environment variable | F1: authoritative variable is currently ignored by the artifact helper |
| 1.4 status/snapshot visibility | Useful least-privilege observation; distinct from control; O2/O5 |
| 1.4 status formatting and viewer-edit rules | Current status adapter uses JSON; preserve exact controller checks; qualify JSON shapes in F3 |
| 1.4 replica minimum | Root managed launcher pin already meets the documented threshold; custom configs need diagnosis in F3 |
| 1.4 logs JSON/NDJSON correction | No dedicated maintained host log adapter found in reviewed surfaces; optional O6 |
| 1.4 offline signing | Experimental; cannot replace current management routing or journal recovery; O7 |
| 1.4 upgrade arguments | Not applicable to reinstall-only cross-release policy |
| 1.4 creation-settings fix | Benefit of updated executable; no reason to replace Canic's exact Ledger creation contract |
| 1.4 plugin-runtime fixes | Already inherited with the installed tool; no current Canic sync plugin to rewrite |
| 1.4 completions | Optional ICP setup convenience; not a replacement for Canic's own command surface |
| 1.3 file-backed runtime environment values | Optional frontend inputs; not a replacement for Canic's typed initialization and topology |
| 1.3 Docker diagnostics | Existing replica adapter retains stderr; improved external diagnostics are already available |

Upstream feature authority: [1.5 release](https://github.com/dfinity/icp-cli/releases/tag/v1.5.0),
[1.4 release](https://github.com/dfinity/icp-cli/releases/tag/v1.4.0), and
[1.3 release](https://github.com/dfinity/icp-cli/releases/tag/v1.3.0).
The detailed conclusions below additionally use the tagged implementation and
local probes, rather than assuming every available feature is adopted.

## Confirmed findings and evidence gaps

Shared finding metadata: baseline is the commit plus source manifest above;
first observed 2026-09-15; source audit is this scoped manual review; fixes and
validation commits are absent; waivers are absent. F1/F2 are open, F3 is an open
qualification recommendation. Severity expresses consequence, not a claim that
an affected artifact has been deployed.

### F1 — High: ICP-selected environment does not reach Canic's artifact context

- Finding ID: `CANIC-110-HOST-BUILD-001`; class: `product_defect`;
  severity: P1; confidence: confirmed.
- Owner/intended owner: `canic-host` build context; affected surface: script
  builds in root `icp.yaml`, including `proof` and explicit `ic` selection.
- Current location:
  [build_artifact.rs](../../../../../crates/canic-host/examples/build_artifact.rs),
  environment initialization in `main`.

ICP's [script adapter](https://github.com/dfinity/icp-cli/blob/v1.5.0/crates/icp/src/canister/build/script.rs)
sets `ICP_CLI_ENVIRONMENT` to the selected environment. `ICP_ENVIRONMENT` is the
inherited CLI default, which an explicit `-e` overrides. Canic's helper reads
only `ICP_ENVIRONMENT` and defaults it to `local`.

The probe observed these exact script inputs:

| Invocation | ICP_CLI_ENVIRONMENT | ICP_ENVIRONMENT | Current helper would select |
| --- | --- | --- | --- |
| `icp build -e proof`, default unset | `proof` | unset | `local` |
| Same command with inherited default `local` | `proof` | `local` | `local` |

In the repository, `proof` maps to `ic`. The helper's incorrect selection flows
through `resolve_icp_build_network_from_root`, then
[WorkspaceBuildContext::apply_to_command](../../../../../crates/canic-host/src/canister_build/context.rs),
which deliberately sets the child's `ICP_ENVIRONMENT` to the resolved network
class. [BuildNetworkInfra](../../../../../crates/canic-core/src/infra/ic/build_network/mod.rs)
bakes that value into Wasm. Thus the mismatch affects runtime network predicates,
not just the displayed environment label. No affected production Wasm or live
deployment was executed during this audit.

**Recommendation:** make the ICP-invoked helper consume the authoritative
`ICP_CLI_ENVIRONMENT`. Keep explicit standalone-helper selection and Canic's
normalized compile-time network class separate and named. Do not globally rename
`ICP_ENVIRONMENT`: its compile-time use is intentional. Define and test precedence
when explicit input and inherited defaults coexist; reject ambiguous context.

**Acceptance:** exact `proof -> ic`, custom local environment, unset/default,
conflicting inherited default and unknown-environment tests; the actual helper's
context and child-command environment must be observed before expensive builds.
Then qualify one generated artifact's network identity and target-specific Clippy.
The current probe confirms ICP's side, while the Canic consequence is a code trace.
Deferral here is solely because the requested deliverable is an audit.

### F2 — Medium: line-based config inspection does not validate effective membership

- Finding ID: `CANIC-110-HOST-CONFIG-001`; class: `product_defect`;
  severity: P2; confidence: confirmed by code trace, no compiled Canic repro.
- Owner/intended owner: `canic-host::icp_config`; affected surfaces: App check,
  replica readiness, status/medic config checks and build-network resolution.
- Current location:
  [icp_config/mod.rs](../../../../../crates/canic-host/src/icp_config/mod.rs),
  `inspect_canic_icp_yaml_from_root`, `named_item_block`,
  `local_network_block`, `resolve_icp_build_network_from_root`.

Inspection checks names at the top level, not the selected environment's actual
canister set. All expected names can be present while an environment selects an
empty or incomplete set. The parser also assumes exact indentation and field
order (`- name:` first); some valid YAML spellings are not recognized. `local`
and `ic` return hard-coded network classes before reading explicit environment
overrides. These limitations predate 1.5; its environment filtering makes the
membership gap more consequential.

**Recommendation:** one typed host projection should own supported ICP config
semantics, explicitly validate environment membership, and honor or explicitly
reject named-environment overrides. Use a YAML parser for local syntax, with a
declared supported subset. Detect unsupported recipes/dependencies instead of
reporting them as missing unrelated local entries. If full upstream resolution
is required, assess `icp project show` as a separate, bounded adapter: it resolves
recipes and may involve external inputs, and its output is YAML, not a stable
Canic DTO. Do not run arbitrary resolution while claiming a purely local check.

Full experimental dependency support is new scope. ICP's
[dependency contract](https://github.com/dfinity/icp-cli/blob/v1.5.0/docs/concepts/project-dependencies.md)
assigns membership to each declaring manifest; the root cannot exclude a
dependency's canister by editing only its own list. Canic also deliberately
passes `--project-root-override`, which disables upstream ancestor discovery.
Retain that isolation until explicit workspace-root semantics are designed.

**Acceptance:** actual membership, empty selection, duplicate names, missing
roles, valid formatting variants, explicit network overrides, unsupported
features and root isolation. A full App build must still include its required
infrastructure and role closure. The root ICP manifest currently has no
dependencies or recipes; no deployed dependency regression is claimed.

### F3 — Medium: qualify the adopted contract, not just the executable version

- Finding ID: `CANIC-110-HOST-ICP-001`; class: `evidence_gap`;
  severity: P2; confidence: high; owner: host ICP adapter plus CI tool contract.
- Current locations:
  [version.rs](../../../../../crates/canic-host/src/icp/version.rs),
  [model.rs](../../../../../crates/canic-host/src/icp/model.rs),
  [balance.rs](../../../../../crates/canic-host/src/icp/balance.rs),
  [tool pins](../../../../../tool-versions.env).

The maintainer pin is 1.5.0, but advertised support remains `>=1.2.0, <2.0.0`.
This alone is not a defect. It becomes an unsupported promise if Canic starts
depending on newer build or visibility behavior without revising its floor.
Some balance fixture names still identify 1.3; that is not proof their shape is
wrong and is not a reason to merely rename fixtures.

**Recommendation:** when adopting 1.5 behavior, raise the supported minimum to
1.5.0 in the owner, help/docs and relevant tests, without compatibility lanes.
Capture real current status, public-only status, balances, response bytes,
snapshots and rejected-call contracts using disposable PocketIC. Validate
meaningful fields and typed outcomes, not release-note prose.

The repository managed launcher is already pinned to
`v15.0.0-2026-08-13-03-55`, meeting the upstream status-decoding threshold.
PocketIC tests separately pin 16.0.0. The
[alignment guard](../../../../../scripts/ci/check-pocketic-version-alignment.sh)
checks the latter against Cargo; it does not qualify arbitrary user-managed
launcher pins. Add an early actionable diagnosis for incompatible custom local
networks when those configurations are supported. Do not equate the CLI version,
network launcher version and PocketIC server version.

## Ranked opportunities

### O1 — Selective ICP builds: immediate benefit after F1/F2

The repository's `demo` and `proof` selections contain four of its nine ICP
canisters. The probe proves excluded builds are not executed, unknown and
explicitly excluded selections reject before script execution, and an empty
environment performs no build. This reduces unnecessary ICP script invocations;
it is not a measured 56% build-time saving.

[canic build](../../../../../crates/canic-cli/src/build.rs) intentionally builds
an App's deployable roles plus infrastructure directly through Canic's builder.
Do not shrink that complete release set to the four ICP entries. Document the
two scopes, compare selected artifact sets, and measure cold/warm timing with
identical profiles before claiming end-to-end acceleration. Existing sealed
release reuse remains more relevant to repeated `canic build` work.

### O2 — Least-privilege observation and richer status: useful, moderate scope

ICP supports status viewers who need not control a canister. This can enable a
dedicated monitoring identity and frontend capacity checks without adding that
identity as controller. The current
[status projection](../../../../../crates/canic-host/src/icp/model.rs) drops
visibility settings and query statistics; it could expose those as typed facts.
The [1.5 status serializer](https://github.com/dfinity/icp-cli/blob/v1.5.0/crates/icp-cli/src/commands/canister/status.rs)
uses tagged visibility objects, so parsing must not reuse the YAML setting shape.

Start with reporting actual visibility and distinguishing full status from the
public state-tree projection. Then consider explicitly reviewed allowed viewers.
Preserve exact controller checks for every effect; readable status is never
control authority. Existing Fleet code already rejects incomplete public status
and compares controllers. The
[Observatory](../../../../../crates/canic-host/src/observatory/ops/transport/mod.rs)
uses role-specific queries for funding and estate information; management status
does not replace those facts. It is also an update call, so broader visibility
does not make polling free. Query statistics are observational, not debit receipts.

Acceptance: controller, allowed viewer, denied viewer and revoked-viewer cases;
full versus public projection; no ability for a viewer to mutate; bounded polling
and response sizes; privacy-preserving default. Relevant authority is the
[visibility reference](https://github.com/dfinity/icp-cli/blob/v1.4.0/docs/reference/canister-settings.md).

### O3 — Frontend sync plugins: best new integration feature, optional

Keep [Canic's terminal-review frontend handoff](../../../../../crates/canic-host/src/frontend/workflow/mod.rs)
as authority. An ICP sync plugin can consume its generated files and upload or
verify frontend assets within the upstream declared-file/canister sandbox.
Use version 0.2.0 of the upstream interface, exact artifact hashes and narrowly
declared call targets. That upstream version is not a new Canic protocol generation.

The [plugin contract](https://github.com/dfinity/icp-cli/blob/v1.5.0/docs/concepts/sync-plugins.md)
provides useful mappings and metadata, but mappings are informational and do not
grant call permission. Fleet-created descendants need not exist in ICP's own
ID store; compare against the reviewed Canic handoff instead of assuming the
tables are interchangeable. The sandbox cannot run Canic subprocesses or publish
local files, so the host must prepare the handoff first.

Recipe-appended sync runs after recipe sync. Therefore Canic's
[asset-capacity check](../../../../../crates/canic-host/src/frontend/ops/capacity/mod.rs)
must run before upload, not as an appended post-upload step. Plugin scope could
begin with post-upload verification. Validate changed handoff hashes, undeclared
targets, wrong network/Principal, partial upload and same-operation retry. Plugin
success must not stand in for Fleet readiness or Canic's conservation journal.

### O4 — ICP bundles: useful distribution experiment, lower priority

The disposable probe also packages only the selected canister and confirms the
rewritten manifest omits the excluded one. `project bundle` exists but is hidden
from ordinary project help; it is experimental. It produces an ICP deployment
archive, not Canic's browser handoff bundle or sealed release authority.

Use only as an optional transport container around separately verified artifacts.
Keep exact Canic hashes, network class, role closure and release identity.
Upstream may prune controller references and embeds file-backed environment
values in the manifest; review actual output before distribution. Script sync
steps cannot be bundled. Those constraints make "add a shell sync hook, then
bundle it" a poor combined design. No new bundle publication lane is needed yet.
See the tagged [bundle command](https://github.com/dfinity/icp-cli/blob/v1.5.0/crates/icp-cli/src/commands/project/bundle.rs).

### O5 — Separate snapshot readers: possible later, not a completed backup feature

Snapshot visibility permits a dedicated reader to list/download snapshots while
controllers retain create/restore/delete authority. Reading exposes full state;
it should not inherit a public monitoring policy. Canic has snapshot transport
primitives, but its
[CLI backup preflight](../../../../../crates/canic-cli/src/backup/create/executor/mod.rs)
explicitly reports unavailable Coordinator-backed Component Registry preflight.
The new visibility setting does not implement that missing authority chain.

Complete that owner before advertising end-to-end least-privilege backup. Prove
controller-created snapshot identity, reader download, revocation, lost download
response and same-release restore. Cross-release restoration remains outside the
pre-1.0 reinstall contract. This is a distinct optional batch.

### O6 — Process overhead and structured logs: useful independent improvements

[run.rs](../../../../../crates/canic-host/src/icp/run.rs) checks the executable
version before ordinary commands, adding a subprocess for each wrapper call.
The PATH executable is an npm launcher, while CI already selects the native
binary. The Observatory already bounds and caches a compatibility check inside
its observation context. Consider the same operation-scoped pattern for other
hot paths, with executable identity fixed and checked again for each new operation.
Do not globally cache status, controllers, balances or signer authority.

Direct-local management status also constructs an agent and exports/verifies the
selected identity. Profile that separately before introducing session reuse.
Measure process counts and cold/warm duration first; there is no measured saving
in this audit. This opportunity is adjacent to 1.5, not caused by its release.

If adding a log command, use explicit JSON and incremental NDJSON for follow,
bounded output and cancellation. No maintained dedicated log adapter was found
in the reviewed host/CLI surfaces, so this is new product scope rather than an
upgrade fix. Existing replica stderr propagation already benefits from the
upstream Docker diagnostics fixes.

### O7 — Offline signing: defer as a replacement for management transport

The tagged [call implementation](https://github.com/dfinity/icp-cli/blob/v1.5.0/crates/icp-cli/src/commands/canister/call.rs)
explicitly records that generic signed management calls still use the management
Principal as their destination. Canic's
[management adapter](../../../../../crates/canic-host/src/icp/management.rs)
supplies the effective target canister ID. Removing it would lose a required
routing correction. Dedicated `icp canister status` is a different upstream path.

Offline signing is a possible future key-separation feature for supported calls,
not a transparent transport swap. A signed message has a bounded submission
window; retrying the same envelope is different from signing another. Canic must
bind the exact envelope, destination/root key, operation and original intent,
then reconcile expiry and lost responses. It does not replace durable ledger
deduplication or later same-operation recovery. Do not add this to the current
funding fix simply to avoid PEM export.

## Existing recovery work that 1.5 does not solve

The [mint command](https://github.com/dfinity/icp-cli/blob/v1.5.0/crates/icp-cli/src/commands/cycles/mint.rs)
returns only `deposited` and `new_balance`. The
[mint operation](https://github.com/dfinity/icp-cli/blob/v1.5.0/crates/icp/src/operations/token/mint.rs)
uses no fixed transfer timestamp, retains the transfer block only internally,
and omits transaction identities from its result. Restarting the whole command
is not a safe operation-bound retry after a lost reply.

This is a duplicate of the accepted
[CANIC-172 mint evidence gap](../2026-09-14/canic-172-native-funding.md#operator-mint-evidence-boundary),
not a new finding or a reason to rebaseline balances. Preserve the existing owner:
intent before transfer, stable transfer identity, ICP/CMC/Cycles Ledger receipts,
bounded fees, notification reconciliation and an exact credit to the original
operation's conservation equation. An upstream structured receipt/resume feature
would help, but Canic must meet its contract without assuming it exists. No
upstream issue or message was sent.

Also retain: explicit `--wasm` and hash verification in Fleet install, exact
controller/subnet authority, reinstall-only behavior and bounded effect journals.
ICP's shared per-canister artifact cache must not replace Canic's sealed
release artifacts. See the
[upstream artifact/environment boundary](https://github.com/dfinity/icp-cli/blob/v1.5.0/docs/reference/environment-variables.md).

## Proposed delivery order

| Order | Bounded outcome and owner | Evidence and propagation | Disposition |
| --- | --- | --- | --- |
| 1 | Correct ICP build context and supported config projection; host + CLI | F1/F2 positive, rejection and conflicting-input cases; exact child build class; help/docs; targeted host/CLI/example checks | Recommend first; not implemented |
| 2 | Qualify adopted 1.5 contract; host + CI | F3 current JSON and replica compatibility; selected/full App artifact distinction; native tool checks; update minimum only with adopted behavior | Combine with order 1 where practical |
| Existing | Complete accepted startup/recovery batch; host + control plane | Existing E163/withdrawal/mint/reset/conservation/replay criteria in current handoff | Remains accepted and incomplete; not replaced by this audit |
| 3 | Typed visibility reporting and least-privilege observation; host | Allowed/denied/revoked viewer proof, exact control separation, bounded polling, current projections/docs | Optional proposal |
| 4 | Frontend handoff consumer; host/frontend + optional ICP plugin | Pre-upload capacity, exact handoff and target bindings, partial-result/retry evidence, recipe/bundle constraints | Optional proposal |
| Later | Bundle distribution, snapshot readers, signing, logs | Each needs its own bounded outcome and acceptance; no automatic minor transition | Defer pending concrete demand |

Process-overhead measurement can be a small investigation alongside an accepted
host performance batch. No patch number is allocated per row. Meaningful future
implementation updates the existing open changelog; an audit alone does not
claim newly shipped behavior.

## Verification and limitations

Executed [probe](artifacts/icp-1.5.0/probe.sh), with retained
[results](artifacts/icp-1.5.0/probe.tsv): selected environment propagation,
conflicting inherited default, empty selection, excluded-canister rejection,
unknown-environment rejection and selected-only bundle. The fixture writes an
empty Wasm module; it executes no Rust compiler, canister, install or ledger call.
It isolates ICP global data and temporary outputs and disables telemetry. Its
temporary state is removed on exit. Bash syntax and scoped ShellCheck pass.

Source digests were rechecked after the probes. No product source changed during
the audit. No full suite or live network qualification was run. Multi-project
dependency behavior, real recipe/plugin execution, visibility permissions,
snapshot roles and signed-message recovery were reviewed in source, not exercised
against a replica. No cold/warm build or polling benchmark is claimed.

The integration findings require fixes and focused evidence; the broader
0.110.17 batch also retains its existing recovery work. It is not push-ready or
ready to publish on the strength of this audit, and no minor closeout is implied.
