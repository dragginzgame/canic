# Canic Host tooling changelog

Covers `canic-host` and `canic-cli`. Both continue their existing 0.114.0 package
history in one independent release group; coordinated history remains once in
[shared history](../history.md). Runtime changes belong in the
[framework ledger](../framework/CHANGELOG.md).

## [0.115.0] - 2026-11-04

### ⚠️ Breaking

- Require exactly `build-support` on compiled-config Canic build dependencies.
  Update scaffold and generated Fleet manifests with the same selection;
  runtime-only generated packages retain no build dependency.

- Adopt Host Tools 0.82.0, Build 0.74.2 and Platform 0.22.0. Retain Query's
  separately typed native Platform 0.19 certificate boundary. CLI metric rounding
  is owned locally and preserves wide totals and nearest-half-up formatting;
  remove its arithmetic-only Metrics dependency.

- Adopt Testkit 0.85.1 as the sole PocketIC client owner for local Fleet and Host
  fixtures. Exposed client/error types now use `ic_testkit::pocket_ic`; callers
  must align to Testkit 0.85. Remove Platform's separate PocketIC client feature.

- Adopt Backup/Agent 0.58.0. Callers exchanging the checkpoint adapter's exposed
  Backup model types must use 0.58 identities. Rust 1.99.1 is now the minimum
  supported compiler.
- Adopt Build 0.74.2 and Process 0.74.3 together. Callers exchanging exposed Cargo
  selections, invocations or process execution records must align those package
  identities. Keep the matching published owners without conversion wrappers.

### 🔧 Changed

- Select the retained Auth browser archive in the frontend consumer's npm lock.
  Its signed admission/logout transport acceptance is recorded in the framework
  ledger, including lost accepted-response retries without extending sessions;
  controlled-local issuer HTTP and browser recovery are now qualified.
- Add account-signed issuer HTTP and Rust-generated Wire codecs to the browser
  fixture. Persist exact intent before dispatch and recover after page reopen
  through the upstream admission client without reissuing. Exercise unauthorized
  issuance and lookup with browser, unrelated and anonymous callers; the original
  account still retrieves the exact retained result afterward.
- Add pinned esbuild/Playwright tooling for the explicit Chromium durable-logout
  acceptance case. Use a separately prepared browser and isolated context;
  production key custody and account-login UI remain application work.

- Add `canic auth policy` and bounded `canic auth sessions` inspection through
  the target's current Fleet Root. Verify the selected Root method contract,
  preserve typed receiver refusals and report configured policy separately from
  current enrollment and retained sessions.

- Propagate Testkit 0.85 PocketIC client errors from local Fleet operations,
  preserving accepted-call and uncertain-execution context instead of catching
  simulator panics. Discovery and status now fail on observation errors.
  Update installed fixtures to distinguish provider failures from canister refusals;
  qualify Backup checkpoint replay through its bounded advancement API.

- Reuse the bounded whole-payload digest for Store artifact evidence, template
  manifests, chunk preparations and bootstrap catalogs. Reuse the manifest
  digest across its authority check and upload, retaining per-chunk hashes and
  exact owned-byte custody ([#647](https://github.com/dragginzgame/canic/issues/647)).
- Use canonical level-1 gzip for ordinary Host parsing, representation, recovery
  and generated-manifest test fixtures. Preserve frozen compressed inputs and
  production artifact encoding at level 9
  ([#698](https://github.com/dragginzgame/canic/issues/698)).
- Adopt Host Tools 0.82.0, Host Text 0.74.2 and Query 0.89.0 alongside the
  framework's Platform 0.22/Runtime 0.19 family. Native qualification remains
  coupled to the framework's matching Auth supplier and composed graph.
- Remove the CLI's unused normal and test-support Process declarations. Host
  retains process execution ownership; CLI commands keep using its APIs.
- Release Host and CLI together independently of the framework and adapters in
  the existing workspace. Host-only changes preserve framework package versions;
  Host releases own the installed CLI default and their release receipt.
- Bind runtime admission, generated application dependencies, Fleet records and
  infrastructure protocol checks to the exact framework compiled into the tool.
  Keep tool versions in producer provenance and generator caches. Framework
  contracts remain exact; a Host release does not enable another runtime family.
- Consume the framework's canonical observation DTOs and owned Wire drain
  contracts and Storage directory reports. Reject multiple Storage identities in
  each selected Wasm graph. Their supplying framework release must be available before this
  Host batch is published; implementation details remain in its ledger.

- Adopt Query 0.88 and Host Tools 0.78.1 with Artifacts/FS/Process
  0.74.2 and Host Text 0.74. Preserve catalog assurance, cancellation, bounded
  deadlines, complete cache publication and exact retry on Rust 1.99.1.
- Select the released Build 0.74 metadata editor independently of runtime
  dependencies. Qualify the four release-group transactions, rollback and exact
  candidate provenance with the new editor; retain the complete release gate.

- Stream Fleet plans/journals, network authority and local Fleet records through
  shared durable publication with their encoded byte limits. Stream capacity-import
  journals and surveys, and count completion reservations without materializing
  complete JSON buffers. Preserve typed producer, cleanup and publication-phase
  evidence and original operation custody
  ([#458 — Host convergence and native qualification](https://github.com/dragginzgame/canic/issues/458)).

- Delegate CLI output-parent resolution and JSON file publication to Host FS,
  retaining final-file symlink refusal, exact replacement authority and complete
  previous reports on producer failure. Remove the private tool-install cleanup
  guard in favor of Host FS's invocation-owned directory guard
  ([#458 — Host convergence and native qualification](https://github.com/dragginzgame/canic/issues/458)).

- Remove duplicate CLI calendar conversion and clipping implementations. Use
  Host Text for UTC conversion and escaped terminal-column clipping, retaining
  nanosecond deadline precision, Unicode labels and original JSON report data
  ([#458 — Host convergence and native qualification](https://github.com/dragginzgame/canic/issues/458)).

- Decode raw ICP manifests and CI-authority fixtures through Host Text's strict
  YAML boundary. Remove the duplicate document walk and direct serde_yaml_ng
  dependency, preserving byte limits, complete-document admission and Canic's
  membership/environment validation ([#719 — Use Host Text YAML admission](https://github.com/dragginzgame/canic/issues/719)).

- Generate App scaffolds against Platform 0.20, matching the selected framework
  runtime family. Package versions remain independently owned by their groups.

- Consume Query's grouped subnet classification and
  acquisition evidence. Update the synthetic growth catalog to the grouped
  classification contract while retaining its cache-assurance and routing proof.
  Collection, agreement, cache validation and publication
  remain upstream; Canic retains its acquisition deadline and typed projection.

- Use Platform's external Agent certificate type in certified custody and
  retirement projections. Preserve typed certified absence, incomplete/pruned
  evidence refusal and the existing certificate verification boundary.

### 🩹 Fixed

- Allow library builders to select explicit `ic-wasm` and `wasm-opt` paths through
  the existing toolchain admission. Retain optimizer checksums and macOS library
  checks, profile binding and exact tool paths in artifact/reuse evidence; default
  CLI lookup is unchanged ([#705](https://github.com/dragginzgame/canic/issues/705)).
- Use Host Tools' Platform 0.20 ledger values and Query 0.89's Platform 0.19
  certificate lookup results when verifying mint receipts. Retain the native
  certificate facade explicitly and preserve certificate, chain and
  deposit-binding checks without converting between supplier model types.
- Verify every selected predecessor before resuming package uploads. A missing
  or unavailable Host package refuses before publishing CLI; preserve exact
  version observations and registry failure evidence for every release group.
- Retry failed host-tool archive transfers up to three times, including DNS and
  connection failures, with a bounded retry window and connection timeout.
  HTTPS restrictions, pinned hashes and executable admission still apply before
  publication; exhausted retries preserve the installed tool
  ([#450 — Deliver CI fixture and release-guard repairs](https://github.com/dragginzgame/canic/issues/450)).
