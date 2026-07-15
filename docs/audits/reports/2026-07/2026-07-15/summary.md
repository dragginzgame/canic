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
`CANIC-092-ERROR-002` are fixed in implementation commit `daa67913...`, with
focused lint validation in `d9dc6304...` and release in `v0.92.1`.

[D2 auth typed-cause preservation](0.92-d2-auth-typed-causes.md) removes
string flattening from proof callbacks and preserves exact issuer-application,
transport, pending, expired, stale, and invalid causes through one existing
public error boundary. All seven auth methods and the current auth trace pass.
A new PocketIC case proves wrong-issuer, expired, and corrupt proof installs
reject with typed codes without replacing the active proof. Candid, token
formats, and stable state are unchanged. D2 was released in `v0.92.2`.

[D3 canonical layer contract](0.92-d3-canonical-layer-contract.md) makes
`AGENTS.md` the sole active architecture authority. Public core docs, module
headers, and hygiene guidance now agree on
`endpoints -> workflow -> policy -> ops -> model`, model-owned state/storage
invariants, and passive storage representations. This is documentation-only;
the immutable baseline's 25 product-code ops-to-policy violations remain
visible for later finding-backed slices. D3 was released with D2 in `v0.92.2`.

[D4 root-issuer admission ownership](0.92-d4-root-issuer-admission-ownership.md)
moves persisted issuer policy/renewal shapes from policy to model and gives
policy/template admission one workflow-to-pure-policy path. Ops now converts
and persists only. Positive and every request-rejection boundary are proved
directly with unchanged state/epoch and skipped timer checks. Public DTO,
Candid, and stable record shapes are unchanged. This fixes
`CANIC-092-TEST-001`, reduces the live layering guard from 25 to 18 violations,
and leaves `CANIC-092-LAYERING-005` open for separately reviewed subsystems.
D4 is released in `v0.92.3`.

[D5 blob-billing workflow ownership](0.92-d5-blob-billing-workflow-ownership.md)
moves Cashier sequencing, reserve/recovery, gateway synchronization, and
readiness observation out of the API into one workflow. Pure policy owns
configuration, funding, and readiness decisions; ops retains boundary
conversion, single calls, guards, and state access. Reserve refusal performs
no partial top-up, transient Cashier failure releases the guard for retry, and
configured/missing-config state survives upgrade in PocketIC. Public DTOs,
Candid, prices/protocol, and stable records are unchanged. This fixes
`CANIC-092-LAYERING-001`; the current blob trace passes. D5 is released in
`v0.92.4`.

[D6 passive RPC DTO ownership](0.92-d6-passive-rpc-dto-ownership.md) removes
capability-family, replay-metadata, canonical-payload, and duplicate-command
behavior from the root RPC DTO. One workflow command now owns family,
descriptor, replay identity, and admitted metadata; ops owns the mechanical
signed-payload projection. Exact identical-replay and cross-family-conflict
PocketIC cases pass. Public protocol, replay behavior, and stable state are
unchanged. This fixes `CANIC-092-LAYERING-002`; the capability trace remains
pass. D6 is released in `v0.92.5`.

[D7 internal surface hard cuts](0.92-d7-internal-surface-hard-cuts.md) delete
the duplicate public proof-install request/outcome and the unnecessary direct
core error root. Scheduled renewal consumes the existing internal ops plan,
the explicit root facade enters the private workflow directly, and the model
owns the four persisted failure classifications. The deliberate control-plane
support bridge remains the sole sibling error surface. Maintained issuer
Candid, stable state, typed causes, and exact stored diagnostic labels are
unchanged. This fixes `CANIC-092-LAYERING-004` and
`CANIC-092-SURFACE-001` without aliases or compatibility paths. D7 is released
in `v0.92.6`.

[D8 reproducible root artifacts](0.92-d8-reproducible-root-artifacts.md)
remove absolute source/asset paths from shipped root runtime records and
lifecycle diagnostics. Two fresh isolated offline lanes produce identical
root/bootstrap raw and gzip artifacts plus identical semantic provenance, and
neither final root contains a lane path. Stable build provenance now requires
the role, transform, optional mode, tool, version, and outcome, fixing
`CANIC-092-BUILD-001` and `CANIC-092-BUILD-002` without a compatibility
decoder or alternate build path.

## Live Ledger

- Retained methods attempted: 22 of 22.
- Valid active results: 22.
- Invalid active results: 0; v1 failures remain preserved as invalid history.
- Mandatory traces: frozen Phase C aggregate `fail` (6 pass, 4 fail, 0 partial,
  0 blocked); D1/D2/D5/D6/D7/D8 leave current reruns at 10 pass and 0 fail
  without rewriting the baseline.
- Unresolved findings: 11 (4 P1, 6 P2, one P3).
- Phase D fixes: D1 released in `v0.92.1`, D2/D3 in `v0.92.2`, D4 in
  `v0.92.3`, D5 in `v0.92.4`, D6 in `v0.92.5`, D7 in `v0.92.6`, and D8
  implemented with focused validation passing.

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
- D2 auth library selection: 263 passed; exact trust, capability, audience, and
  replay method selections pass; targeted warning-as-error Clippy passes.
- D2 PocketIC: role attestation, capability, root facade, session bootstrap,
  renewal, and invalid-proof/no-mutation selections each pass.
- D3 active-contract scans, changed-file Rust formatting, strict `canic-core`
  Clippy, and package verification pass. Layering self-tests pass; the full
  guard retains the same expected 25 product-code violations. Core rustdoc
  still fails only on the separately indexed D10 `InternalError` link.
- D4: 881 all-feature core library tests, four policy/DTO boundary guards, 19
  protocol tests, strict core Clippy, and two PocketIC proof/renewal regressions
  pass. Layering fixtures pass and the live violation set is 18.
- D5: 878 all-feature core library tests, 50 focused blob-storage tests, four
  policy/DTO boundary guards, 19 protocol tests, strict all-feature/all-target
  core Clippy, and four PocketIC billing selections pass. Layering fixtures,
  targeted formatting, and diff hygiene pass; the live violation set remains
  18 with no new upward edge.
- D6: 51 workflow RPC tests, eight RPC ops tests, four replay-manifest tests,
  four policy/DTO guards, 19 protocol tests, strict all-feature/all-target core
  Clippy, and exact PocketIC identical-replay and cross-family-conflict cases
  pass. Layering fixtures and diff hygiene pass; the live violation set remains
  18.
- D7: all-feature/all-target core and control-plane checks, four provisioning
  tests, 20 chain-key batch tests, three DTO/serialization guards, 19 protocol
  tests, all-feature facade check, strict core/control-plane Clippy, offline
  core/facade package verification, the control-plane facade test, and exact
  PocketIC new-issuer provisioning pass. Layering fixtures retain the same 18
  known upward edges. Core rustdoc still fails only on the separately indexed
  D10 broken link.
- D8: 7 artifact-transform tests, 15 build-provenance/policy tests, 12
  release-set manifest tests, targeted checks and strict Clippy, and two fresh
  isolated offline root builds pass. Root/bootstrap hashes and semantic
  provenance match across lanes, root paths are absent, and all four local
  transforms record `ic-wasm 0.9.11`.

## Next

D9 release execution integrity is the next ordered candidate. D9, D10, and the
remaining layering subsystems remain separately bounded.
