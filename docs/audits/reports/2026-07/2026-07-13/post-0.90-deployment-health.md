# Post-0.90 Deployment Health Audit - 2026-07-13

## Scope

- Snapshot: published `v0.90.2` at `4289dcc7`.
- Reviewed: role admission at configuration and release boundaries, local
  canister artifact paths, release-set construction and publication, build
  provenance, and complete-build ownership.
- Excluded: downstream Toko code, generalized Cargo modeling, broad error-type
  cleanup, compatibility work, full workspace tests, and release versioning.

## Verdict

- Result: **two bounded deployment findings**.
- Risk: **6 / 10** until both boundaries are hard-cut.
- No third finding was added merely to fill a quota. In particular, optional
  `ic-wasm` metadata embedding is an existing documented behavior with focused
  coverage, not evidence for another cleanup slice.
- The findings fit one small deployment-truth line, but implementation should
  begin only after a design fixes the exact owners and deletion list.

## Finding 1 - Role Names Are Not Admitted by One Canonical Rule

**Severity: high.**

Core configuration validation limits canister roles to 40 bytes but does not
reject empty roles, separators, `..`, or other non-identifier characters.
`CanisterRole` intentionally remains a passive identifier wrapper and its
string conversions accept any text. A separate host mutation helper does apply
the narrower ASCII alphanumeric, underscore, or hyphen rule, so CLI-authored
configuration and hand-edited configuration do not share one admission
contract.

The mismatch reaches deployment artifacts. The canister builder joins the
validated configuration role directly into both the artifact directory and
filenames under `.icp/local/canisters`. A role such as `..` or `../name` can
therefore resolve outside its intended per-role directory even though the
configuration passed core validation. Release-set validation checks only that
a role is nonempty and unique before deriving its template identity.

### Required hard cut

- Put one public role-name admission function with the core configuration
  owner: 1 to 40 bytes, an ASCII alphanumeric or `_` first byte, and remaining
  ASCII alphanumeric, `_`, or `-` bytes.
- Reuse it for complete config validation and release-set manifest validation.
- Delete the duplicated host mutation character rule and map the canonical
  typed failure into the existing host error at that boundary.
- Reject invalid input; do not sanitize it, fall back to a basename, or retain
  the permissive path.
- Keep `CanisterRole` as the shared data shape. Requiring every internal
  constructor to return `Result` would broaden the slice without improving the
  actual admission boundaries.

### Focused proof

- A hand-edited config with `..`, a slash, an empty role, or punctuation is
  rejected before artifact path construction.
- Maintained leading-underscore and internal/trailing-hyphen role forms remain
  accepted; leading-hyphen roles are rejected because they are ambiguous at
  positional CLI boundaries.
- Release-set manifest validation rejects the same invalid names through the
  same rule.
- An admitted role contributes one lexical path segment and cannot introduce
  a separator, dot component, or another relative/absolute path component.
- Require the install-root filesystem to distinguish admitted role bytes,
  including ASCII case; other install roots are outside this release
  guarantee.

## Finding 2 - A Single-Role Build Can Publish a Mixed Release Set

**Severity: high.**

After building one ordinary role, the artifact builder asks the release-set
writer to emit a full fleet manifest whenever every configured `.wasm.gz` path
exists. Readiness is therefore based on filesystem existence, not on every
artifact having succeeded in the current complete build.

The artifact tree persists across builds and version changes. On the first
single-role build after a version bump, old artifacts for the remaining roles
can satisfy the existence check. The emitted manifest then takes the current
root package version while hashing a mixture of new and stale role bytes. Its
hashes are self-consistent, so ordinary manifest validation cannot discover
that semantic version mismatch. The optional manifest is also included in
single-role build provenance. During the install-root sequence, each role
build can publish this intermediate manifest before the orchestration owner
performs its explicit final emission; a later build failure may leave the
mixed manifest behind.

### Required hard cut

- Remove release-set emission from the single-role artifact builder.
- Delete the `emit_*_if_ready` functions and the optional manifest field on
  `CanisterArtifactBuildOutput` when the consumer inventory confirms they have
  no remaining owner.
- Remove single-role provenance treatment of a fleet manifest.
- Let only the complete fleet/install orchestration owner publish the manifest.
  It passes the same validated target snapshot and exact role-labelled outputs
  collected from the current invocation to the writer.
- Parameterize every required builder from that snapshot; complete-build
  participants must not independently reload deployment identity, build
  selection, release version, or expected output paths.
- Require exact output coverage and hash only the gzip paths carried by those
  outputs. The writer must not reload configuration or reconstruct completion
  from configured-path existence.
- Complete coverage, exact-path, artifact-read, hash, manifest-validation, and
  serialization work before publication so rejection leaves an existing
  manifest unchanged.
- Preserve the existing manifest schema, artifact hashes, staging behavior,
  and explicit complete-build writer.
- Do not introduce a build epoch, sidecar provenance database, artifact
  transaction framework, or stale-artifact compatibility path. Correct
  ownership removes the ambiguity directly.

### Focused proof

- Building one role never creates, rewrites, truncates, deletes, reports, or
  attests a fleet release-set manifest.
- A complete successful role sequence emits exactly one manifest.
- Missing, duplicate, or unexpected current outputs block publication even
  when stale artifact paths exist.
- Path-mismatched, unreadable, or unhashable current outputs leave an existing
  manifest unchanged.
- A failed partial build leaves a prior manifest byte-for-byte unchanged; if a
  covered artifact changed, normal hash validation rejects that old manifest.
- Successful entry order comes from the validated snapshot rather than output
  collection or map iteration order.
- Existing manifest loading, validation, and staging tests remain unchanged.

## Proposed Next Line

Use a small role-admission and complete-build publication design with exactly
two slices:

1. canonical role-name admission at config and release boundaries; and
2. release-set publication from one validated snapshot and the exact current
   build outputs.

The slices are related by deployment identity but do not need a shared
framework. The design should name exact deleted functions and output fields,
preserve all successful manifest bytes and deployment behavior, state the
single-mutating-command precondition, and remain a pre-1.0 hard cut.

## Evidence and Validation

- Source inspection followed the role from core config validation through
  host mutation, artifact path construction, manifest validation, and staging.
- Source inspection followed release-set publication from single-role builds
  and build provenance through the explicit install-root manifest operation.
- `cargo test -p canic-core receipt_backed_` passed all seven targeted 0.90
  closeout tests; this audit changed no Rust code.
- Full workspace, broad PocketIC, deployment, and release suites were not run
  under the targeted-validation policy.

## Decision

0.90 remains closed at `0.90.2`. These are next-line design inputs, not reasons
for a 0.90 correction or an implicit version bump.
