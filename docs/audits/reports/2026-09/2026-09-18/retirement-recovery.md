# CANIC-166/172 retained retirement accounting

The .25 correction restores admission of Toko's retained operation without
rewriting its accounting vocabulary, plan digest, journal or source evidence.
This supersedes the earlier .25 status that reported no new confirmed blocker.
It does not establish live staging recovery or authorize a release transition.

## Boundary and correction

The embedded completed retirement was decoded as `ActualCycleConservation`,
whose .24 net balance fields differ from the recorded execution/settlement
fields. The active plan otherwise satisfies the current executable schema.
Its one original withdrawal Intent therefore became inaccessible before the
existing funding review or receipt-bound mint selector could inspect it.

`FleetRetirementConservationRecord` separates recorded retirement evidence
from live accounting. Its frozen execution observation preserves every required
field, original canonical field order and decimal encoding. The net balance
observation preserves the current encoding. Both reject missing, mixed and
unrecognized fields; neither substitutes one observation's semantics for the
other. Newly captured retirements record the current net balance report.

There is no general predecessor plan/action/journal decoder, field default,
digest rewrite or inferred payment receipt. Fresh live conservation remains
strict. Existing preparation apply reobserves and verifies the source estate;
the embedded historical numbers never grant new funding or reset authority.

An explicit reinstall against an intact in-progress operation now reports
`RetainedOperationRecoveryRequired` with the original operation and digest.
The supported sequence is ordinary retained funding review, receipt-bound
conversion when required, original-operation recovery and convergence/replay,
then a separately reviewed selected-release reinstall. A pending withdrawal
cannot be discarded merely because a newer desired release is available.

## Exact local evidence

Base: `933a35b66403f6a94f99803252cc52c0aec32958` (`v0.110.24`) with the open .25
worktree. Packages remain .24. Qualification used Rust 1.98.1, PocketIC 16.0.0,
native ICP CLI 1.5.0 and Cargo lock SHA-256
`015c893491c83cb44107f40c6596cb26afd9a3c0460000b40391317ac178ab11`.
A concurrent dependency lock update was preserved, not reverted.

The read-only source probe verified Toko's exact retained plan:

- Operation: `a964ba7cf39d10f3245bdeea9d6ead55933c4eecec97e91c8e974cddc16bf38c`.
- Original plan digest: `7621bd97a8b89b5f3a6655a16d68ba3867500e443f39d5be1a3da60d51c2c08c`.
- Plan file SHA-256: `abcba44e0a3cfea928bca47fd964baadf6615008480721b5cc5af2a206610714`.
- Journal file SHA-256: `b2dffc964f6e06530be41095f5bf0c6849a6f39308a21aa49b92dcf4a73d3ecd`.

The probe called `read_plan`, recomputed `expected_plan_sha256` and matched the
retained journal identities. It copied plan/journal/state into invocation-owned
scratch before calling `retained_in_progress_plan` and the mint selectors, so
their operation lock could not write into Toko. Journal integrity passed,
fresh mint quotation remained unavailable for the active operation, and no
conversion was already retained. All three copied documents remained identical
to their source bytes. The source plan/journal hashes above remained unchanged.
The temporary probe source was removed; its log is
`/tmp/canic-retained-plan-probe.log`. No network observation or remote effect
was part of this probe.

## Focused qualification

- 348 Fleet host native tests pass, three unrelated opt-in checks are
  ignored and governed PocketIC cases are excluded. Coverage includes exact
  historical/current serialization, malformed and mixed evidence, tamper
  rejection by the original digest, readiness without evidence mutation,
  explicit reinstall rejection while pending, original withdrawal recovery,
  conservation and immediate effect-free replay.
  Log: `/tmp/canic-retirement-native.log`.
- The existing exact production-Ledger PocketIC case now contains embedded
  recorded retirement evidence. It passes authenticated mint credit, the
  original native withdrawal, an actual lost successful reply, the Ledger's
  duplicate receipt, terminal accounting and effect-free replay. The source
  plan and original balances/payment identity remain retained. This fixture
  uses one minimal canister, not the complete managed Toko estate.
  Log: `/tmp/canic-retirement-pocketic.log`.
- Warning-denied all-target/all-feature Clippy passes for `canic-host` and
  `canic-cli`. Import-only cleanup followed the runtime tests and is covered
  by this final check. Log: `/tmp/canic-retirement-clippy.log`.
- Changed-file formatting and whitespace checks pass. No broad release gate,
  package bump, commit, push or staging operation was started by this work.

The first native attempt exposed a missing desired record in the readiness
test fixture; that fixture was corrected before the passing run. An editor
workspace Clippy process temporarily contended with the PocketIC runner's
build directory. Neither elapsed build time nor the disposable case's runtime
is a deployment-performance claim.

## Remaining live qualification

The Canic admission defect and recovery diagnostic are fixed locally, and the
selected .25 fixes/changelog are ready for the maintainer-selected release flow.
Publication and adoption precede Toko's live recovery. The exact signer/network,
original request's Ledger retry/receipt status, source seals, full managed
estate, retained artifacts and cycle bounds must still admit that operation.
After its successful completion and replay, Toko must separately review and
apply its qualified 0.3.2 target. This proof does not grant permission to replace
an expired or unresolved withdrawal, reuse a changed timestamp, rewrite source
evidence or bypass a failed live authority check.
