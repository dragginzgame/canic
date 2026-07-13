# Codebase Health Audit - 2026-07-13

## Scope

- Snapshot: published `v0.87.1` at `d2f5c859`, plus the documented 0.87
  closeout correction described below.
- Reviewed: runtime and host ownership, artifact persistence, CLI output,
  fleet configuration errors, layering, stale surface signals, dependency
  advisories, duplicate dependency versions, and manifest hygiene.
- Excluded: full workspace tests, PocketIC, deployment, network fetches, broad
  Wasm rebuilds, and historical documentation.

## Executive Summary

- Risk score: **5 / 10**.
- All three intended 0.87 slices shipped in 0.87. The audit found one missed
  install-root ICP wording classifier; the current closeout correction moves
  it into the existing typed adapter rather than carrying it into 0.88.
- Layering guards and dependency-manifest checks pass. Cargo audit reports no
  known vulnerability and the same four upstream unmaintained transitive
  dependencies.
- The strongest next risks are durability claims that are not fully backed by
  filesystem syncs, direct CLI artifact writes that can expose partial files,
  and one fleet-config boundary that erases typed causes across many paths.

## 0.87 Closeout Correction

Install-root recognized three additional ICP CLI forms for a missing canister
ID in a local helper after converting a boxed error to text. This contradicted
0.87 Slice B's rule that the ICP adapter is the only owner of external wording.

The current correction:

- adds those forms to the existing `IcpDiagnostic::CanisterIdMissing`
  classifier;
- keeps `IcpCommandError` typed through root-canister ID resolution;
- deletes the install-root string classifier and its duplicate tests; and
- leaves creation, retry, diagnostics, and exit behavior unchanged.

The post-correction production scan finds no ICP diagnostic wording outside
the host ICP adapter.

## Findings

### High - Backup artifact directories are labelled durable before a durable commit

Evidence:

- Snapshot capture and backup-runner finalization each rename a downloaded
  directory into place and immediately advance its journal entry to
  `ArtifactState::Durable`.
- Neither path syncs downloaded files, nested directories, or the artifact
  parent directory before the journal records `Durable`.
- Resume logic skips durable entries, integrity admission requires the durable
  state, and manifest construction consumes it as finalized evidence.

Impact:

A process or host crash can leave the journal claiming that an artifact is
durable even though the directory entry or downloaded bytes were not forced to
stable storage. Recovery may then skip the artifact and publish a manifest for
state that did not survive the crash.

Recommended hard cut:

1. Give `canic-backup` one private durable artifact-directory commit function.
2. Sync regular files and nested directories before the rename, then sync the
   owning directory after the rename.
3. Use it from both snapshot capture and backup-runner finalization.
4. Advance the journal to `Durable` only after the commit succeeds.

Do not add a cross-crate filesystem framework, journal protocol, alternate
archive format, or recovery daemon.

### Medium - CLI evidence and report files use direct truncating writes

Evidence:

- Thirty production call sites route JSON or text output through the shared
  CLI output helper.
- File output in that helper serializes first but then uses `fs::write`, which
  truncates an existing destination and exposes the write in place.
- Deployment-plan output separately uses direct create-new writes; a crash can
  leave a partial file that blocks retry because the destination now exists.
- The CLI already depends on the host crate, whose bounded durable byte writer
  is used by scaffold and host artifacts.

Impact:

Interrupted output can leave malformed evidence envelopes, restore reports,
metrics/cycles reports, adoption reports, backup reports, or build provenance.
Downstream automation can observe the partial file, and create-new paths can
require manual cleanup before retry.

Recommended hard cut:

1. Serialize complete bytes before any destination mutation.
2. Route replacing output through the existing host durable writer.
3. Add one bounded create-new durable mode for paths whose no-overwrite
   contract must remain intact.
4. Keep stdout behavior and every JSON/text schema unchanged.

Do not add an output transaction service, multi-file commit protocol, or
compatibility writer.

### Medium - Fleet configuration ownership erases typed errors

Evidence:

- The host fleet-config subtree contains 47 boxed dynamic-error signatures.
- It contains 49 string-built validation, parsing, projection, mutation, or
  rollback error sites.
- Repeated `invalid <path>: <error>` mappings flatten the core configuration
  source before it reaches command boundaries.

Impact:

Callers cannot distinguish invalid input, parse failure, mutation conflict,
I/O failure, or rollback failure without inspecting display text. Adding or
changing one config failure can therefore drift across scaffold, build,
release-set, Medic, and fleet command consumers.

Recommended hard cut:

1. Define one focused fleet-config error enum in the existing config owner.
2. Preserve core config and I/O errors as sources.
3. Give input and mutation conflicts bounded typed variants rather than one
   variant for every sentence.
4. Remove boxed/string errors from the config subtree in the same slice.

Do not introduce a global host error framework, error-code schema, or
compatibility conversion layer.

## Lower-Priority Signals Not Selected

- `canic-core` emits 1,628 `unreachable_pub` warnings when that optional lint
  is enabled. Most describe effective crate-private visibility beneath private
  ancestors. A workspace-wide mechanical rewrite would create large churn
  without fixing one authority or runtime problem, so it is not a 0.88 slice.
  One unused RPC adapter re-export remains a high-confidence future narrowing
  candidate.
- Cargo audit reports unmaintained `backoff`, `instant`, `paste`, and
  `serde_cbor`. All are transitive through maintained IC/PocketIC/Candid
  dependencies. Canic has no direct `serde_cbor` use and should not fork or
  replace upstream protocol crates merely to silence the advisory.
- Duplicate `reqwest`, `k256`, and IC transport versions come from separate
  current host, cryptographic, and test dependency lines. No local dependency
  declaration can unify them safely today.
- Host and backup durable writers remain intentionally separate because they
  own different crate boundaries.

## Passing Boundaries

| Boundary | Result |
| --- | --- |
| Layering | PASS - the maintained layering guard reports no violation. |
| Manifest dependencies | PASS - Cargo Machete finds no unused dependency. |
| Security vulnerabilities | PASS - Cargo audit finds no known vulnerability. |
| Role/feature/memory authority | PASS - no second mapping authority found. |
| Production environment mutation | PASS - no global mutation helper returned. |
| External ICP wording | PASS after the current 0.87 closeout correction. |
| Canic-owned CBOR | PASS - no direct `serde_cbor` ownership returned. |

## Recommended 0.88 Scope

1. Make backup artifact-directory finalization genuinely durable.
2. Make CLI file output failure-atomic while preserving replace/create-new
   contracts.
3. Give fleet configuration one typed error boundary.

After those three slices, close 0.88. Do not expand the line into broad
visibility churn, dependency forks, a filesystem framework, or a global error
architecture.

## Evidence Log

- `bash scripts/ci/run-layering-guards.sh`
- `cargo audit --no-fetch`
- `cargo machete`
- `cargo tree -d --workspace --locked`
- inverse dependency trees for the four advisory crates and duplicate
  `reqwest`/`k256` versions
- production scans for direct writes, artifact renames, boxed errors, string
  classifiers, lint suppressions, unsafe code, and stale compatibility terms
- focused inspection of backup artifact finalization, CLI output, durable host
  I/O, install-root ICP handling, and fleet-config ownership

Full workspace, PocketIC, deployment, and broad Wasm suites were not run under
the targeted-validation policy.
