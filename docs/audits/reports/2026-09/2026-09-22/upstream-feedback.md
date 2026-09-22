# Upstream feedback qualification — 2026-09-22

The accepted CANIC-166/172/181 Canic implementation batch is complete locally in
open .36, alongside the earlier metrics work. Targeted regressions, real-Wasm
payload cases and scoped warning-denied lint pass. Packages remain .35. This is
not publication, downstream adoption or proof of live staging convergence.
[Structured evidence](upstream-feedback.json) binds changed source and retained
logs. Toko was read-only throughout this batch.

## CANIC-166: completed source review

The terminal inspector projects supported completed payment/phase evidence from
immutable source documents. It does not construct a current executable journal.
Absent funding observations contribute zero additional execution allowance;
present observations must validate. Unknown journal fields, incomplete work,
malformed/null observations and inconsistent identities reject review. Strict
current journal decoding remains unchanged.

Readiness reports the required terminal review, and explicit reinstall review
reaches the existing terminal preparation owner when the journal cannot decode.
The existing inventory, conservation, archive and interruption-safe handoff
checks still apply. Native tests cover source drift, denial of extra allowance,
recovery at each archive/handoff boundary and fresh inventory requirements.

The read-only opt-in test inspects Toko's exact retained staging source: 49 effects
and two successor phases. The JSON retains all three source document hashes;
the log includes phase hashes and operation identity. Source bytes remain
unchanged. No live inventory, retirement apply, journal rewrite or staging retry
was performed. Adoption followed by the separate reviewed live flow remains
necessary; this is not proof that Toko's deployment has converged.

## CANIC-172: useful funding facts before a build

`fleet readiness --desired ...` reuses the existing configuration startup owner
and observes selected Root native balances with exact Principal/controller
bindings. It reports configured/startup floors, observed shortfalls, unavailable
facts, desired/configuration hashes and the observation window. Configuration or
retained identity drift during collection rejects the snapshot.

`--quote-conversion` uses existing quote/arithmetic owners for the caller's
operator-Ledger estimate. Mint amount, ICP transfer fee, estimated cycle deposit
fee and total debit remain separate. No payment is authorized. Total execution
reserve, current pool/grant usage, actual plan debit, ICP balance and fresh plan
admission remain explicit unknowns. A known native shortfall or unfunded startup
role blocks the early check; unknown balances are not replaced with zero.

The exact Toko configuration loads without artifacts or network access. It derives
100,000,000,000,001 cycles of startup floor and 2,000,000,000,000 cycles of allowance
per execution step. These are the current selected configuration's partial facts,
not a reproduction of the historical 392T whole startup quotation. Artifact-bound
continuation/execution reserve is deliberately unknown here. The inspection does
not claim any current native balance or affordability. Native regressions cover
shortfalls, missing balances, authority conflicts, source drift, conversion fees,
unknown amounts and future-dated rates. Live downstream early-CI integration and
already-issued-operation recovery acceptance remain separate follow-up.

## CANIC-181: compiled payload contract

The declaration pass adds typed version-1 metadata to a dedicated Candid comment
field using the runtime default and registered macro limits. Existing sidecar
hashes bind it. The endpoint tooling exposes default/override ingress ceilings,
raw pre-decode guards and variant-dependent methods; missing metadata is unknown.
Malformed or duplicate metadata rejects inspection. Candid wire types and runtime
enforcement are unchanged; declaration bytes and their hashes change.

The targeted PocketIC probe compares extracted compiled declarations with actual
exact-boundary acceptance and first-byte-over rejection for default, explicit,
renamed and bare-CDK updates. It also retains the inter-canister raw-adapter case.
All three cases pass under the governed fast Wasm profile and PocketIC 16.0.0.
Native metadata/parser checks and CLI endpoint/help checks pass. Downstream
contract adoption still requires a rebuilt release artifact.

## Validation and remaining scope

Retained logs list each selected command's actual results. Scoped all-target,
all-feature Clippy covers canic-core, canic, canic-host, canic-cli,
canic-testing-internal, canic-tests, payload_limit_probe and runtime_probe.
Targeted macro and ordinary build-configuration tests pass. Formatting,
whitespace and edited documentation links pass. No broad workspace test gate ran.

The [earlier metrics measurements](../2026-09-21/history-locality.md) remain their
matched artifact checkpoint. Those implementation/probe sources are unchanged;
this batch adds a test-only host dependency and declaration metadata, not a new
sampler measurement. A concurrent maintainer dependency refresh selected IcyDB
0.261.3 and updated transitive packages during the first Wasm run. The source-drift
guard correctly stopped that run; its log is retained separately. The update was
preserved, and focused native behavior, lint and PocketIC were rerun against the
settled lockfile. Earlier per-command logs retain their original dependency
checkpoint. This report does not qualify the IcyDB lifecycle composition itself.
Real Toko savings remain unmeasured until adoption/rebuild.
The complete accepted .36 batch and both changelog drafts are ready for the
maintainer's release flow. No Git mutation, version bump, release or deployment
ran. B1 and human minor-closeout acceptance remain independent.
