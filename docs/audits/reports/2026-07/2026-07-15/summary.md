# Audit Summary - 2026-07-15

## Scope

Post-freeze corrections and immutable-baseline reruns of
`CANIC-INSTRUCTION-001` and `CANIC-WASM-001`, followed by evidence-only
completion of `TRACE-AUTH-001` and `TRACE-CONTROL-001`, against published
`v0.92.0` commit `91736337fc1cfeb891f17d7d62affb5e671348e2`.

## Result

[Instruction footprint v2](instruction-footprint-v2.md) is a valid first v2
baseline with run result `partial` and risk 6/10. The method now:

- builds every fixture through the authoritative root harness;
- executes a fixed roster of 11 update scenarios and one install checkpoint
  group in fresh PocketIC topologies;
- binds persisted perf rows to the exact
  `[perf, endpoint, update, endpoint_name]` label shape;
- recognizes literal and namespaced single-line `perf!` checkpoints;
- uses a root-independent composite fingerprint and selects only an immediate
  predecessor with the same method ID, version, and fingerprint; and
- retains repository-relative evidence paths and verified SHA-256 hashes.

All 12 required rows executed. The run retained 21 non-zero checkpoint deltas
and found 57 static checkpoint sites. The zero exclusive total for
`scale_hub:create_worker:empty-pool` is a measured call with its work retained
under nested scaling checkpoints, not a missing sample.

The result is partial because root proof provisioning and issuer
delegated-token prepare/verification have no internal checkpoints. This fixes
P1 audit-method finding `CANIC-092-AUDIT-015` and records P2 evidence gap
`CANIC-092-PERF-001`.

[Wasm footprint v2](wasm-footprint-v2.md) is a valid first v2 baseline with
run result `pass` and risk 4/10. V2 removes the unsupported direct-Cargo and
inferred pre-shrink artifact flow. All six roles build fresh release and debug
artifacts through the authoritative host builder in a clean linked worktree;
builder gzip round trips, `ic-wasm`, `twiggy`, tracked-source immutability, and
all retained hashes pass. Leaf release sizes have a 1.0526x spread, while the
root bundle is 1.6227x the largest leaf. This fixes P1 audit-method finding
`CANIC-092-AUDIT-016` without creating a product finding.

[Mandatory trace evidence completion](0.92-mandatory-trace-evidence-completion.md)
adds two exact PocketIC cases without changing production implementation. The
auth case proves pre-session generated-endpoint rejection, successful session
bootstrap, guarded-call parity, idempotent replay, conflicting-wallet
rejection, and unchanged authority after rejection. The control case commits
an exact release to a target Wasm store before the root mirror, upgrades root,
and proves deterministic convergence to the same single owner without another
store allocation.

`TRACE-AUTH-001` and `TRACE-CONTROL-001` move from `partial` to `fail` because
their evidence gaps are fixed while their already indexed product defects
remain. The mandatory aggregate is now complete: six pass, four fail, zero
partial, and zero blocked.

[Phase D finding review](0.92-phase-d-finding-review.md) reviews all 23
unresolved findings. Nineteen map to ten bounded implementation candidates,
three P2 measurement/dependency watchpoints are deferred, and the dedicated
secret-scanner P1 has a proposed limitation record that is not yet an accepted
waiver. D1 was the first candidate because the frozen baseline found durable
publication without its declared reserve/quota permit and with flattened typed
rejection causes.

[D1 publication safety](0.92-d1-publication-safety.md) implements that first
bounded product slice. Admin, bootstrap, and post-upgrade reconciliation
publication now reserve distinct workflow-owned quota/cycle permits before any
store or management effect. Publication effect adapters require the
unforgeable permit, while binding and store-GC commands remain outside the
behavior change. Typed conflict, capacity, missing-release, hash, state,
missing-store, and transport causes now project through distinct existing
public error codes.

Focused PocketIC execution admits ten same-window admin publications and
rejects the eleventh with `ResourceExhausted` before fleet mutation. Exact
conflict and capacity cases reject with distinct codes and unchanged fleet
state, direct store authorization remains root-only, and a target-committed
release still converges after root upgrade. `CANIC-092-COST-001` and
`CANIC-092-ERROR-002` are fixed in the candidate; exact fix and validation
commit identity remains pending the maintainer-owned commit.

## Live Ledger

- Retained methods attempted: 22 of 22.
- Valid active results: 22.
- Invalid active results: 0; v1 failures remain preserved as invalid history.
- Mandatory traces: frozen Phase C aggregate `fail` (6 pass, 4 fail, 0 partial,
  0 blocked); D1 moves the current rerun state to 7 pass and 3 fail without
  rewriting the baseline.
- Unresolved findings: 21 (7 P1, 13 P2, one P3).
- Product fixes: D1 implemented and focused validation passes; immutable commit
  and product-tree identities remain pending.

## Validation

- `cargo test --offline --locked -p canic-tests --test instruction_audit`:
  37 passed, report generator intentionally ignored.
- Live ignored generator through `scripts/ci/instruction-audit-report.sh`: one
  passed, 37 filtered out; 12 PocketIC scenarios completed.
- Audit evidence-manifest SHA-256 verification: pass for the primary report
  and all seven compact artifacts.
- `scripts/ci/check-audit-method-catalog.sh`: pass after fingerprint and
  compatible-predecessor guard refresh.
- Retained evidence root/private-path scan: pass.
- `CANIC-WASM-001/v2`: 12 authoritative role/profile builds completed in a
  clean disposable linked worktree; all gzip, `ic-wasm`, `twiggy`, source
  mutation, identity, and compact evidence-hash checks passed.
- Wasm evidence-manifest SHA-256 verification: pass for the primary report and
  all ten compact artifacts.
- Exact delegated-session PocketIC evidence: one passed, 23 filtered out.
- Exact interrupted-publication recovery PocketIC evidence: one passed,
  8 filtered out.
- Targeted host/test-canister formatting and warning-as-error Clippy: pass.
- D1 `canic-control-plane` publication tests: 18 passed; replay/capability
  policy tests: 30 passed; core cost-boundary tests: 4 passed.
- D1 targeted `canic-core` and all-feature/all-target `canic-control-plane`
  warning-as-error Clippy: pass.
- D1 PocketIC publication filters: quota/no-mutation 1 passed; exact conflict 1
  passed; fixed-target conflict/capacity 2 passed; interrupted recovery 1
  passed; root/non-root store authorization 1 passed.
- Layering v2 rerun: expected valid failure with the same 25 ops-to-policy
  files and no new D1 path.

## Next

Record the immutable D1 fix/validation commit and canonical product-tree hash
after the maintainer commit. D2 typed auth/provisioning causes is the next
ordered candidate; D2 through D10 remain bounded rather than blanket
authorization.
