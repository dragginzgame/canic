# CANIC-172/174 child funding observations

Date: 2026-09-15. Development packages remain 0.110.16 on published base
`a875c6498721bd89ca98549e38389b8280910d68`; delivery extends the open 0.110.17
draft. Qualification uses IcyDB 0.257.13, ic-query 0.43.1, ic-memory 0.13.3,
Rust 1.98.1, ICP CLI 1.5.0 and PocketIC 16. No publication or live spending occurred.

## Maintained behavior

Root and managed-role controller observability now accepts `ChildFunding(child)`.
It projects the existing child ledger, exact child-grant replay receipts and
retained cost reservations. No new stable store, timer, paid effect or grant
admission rule exists. Root permits this read while Prepared. The generic host
observer supports direct and relayed reads and validates the returned parent
and child against the requested participants. Store does not own child grants
and rejects the selector explicitly.

The response contains `parent`, `child`, `observed_at_ns`, `accounted_cycles`,
`last_accounted_at_secs`, `pending_operations` and optional `reserved_cycles`.
Ledger charges precede transfer completion and are restored on rejection, so
accounted cycles can include an unresolved transfer. They are not an unconditional
settled-success total. Reservations are reported separately and must not be
subtracted again from an observed native balance.

Pending replay evidence survives ordinary receipt expiry when external-effect
reconciliation remains unresolved. Reservation amounts require the exact
retained value-transfer intent for the parent and a child deposit effect.
Missing, invalid, expired or overflowing reservation evidence gives `None`,
meaning unknown. Settled reservations contribute zero even when response
recovery remains pending. The query does not manufacture zero from uncertainty
and does not mutate or repair receipts. Querying an unused child key reports no
recorded usage; this does not establish membership or eligibility.

Generation binds selected installed Root code, operator and exact participants,
then resolves each current seeded Workload claim through Root operation status.
The exact operation, Component, Root, canister and committed installation
binding determine the funding parent, role and Component Spec. A pool asset's
physical Root custody does not imply a direct funding relationship. Only
Root-funded Workloads query Root's ledger; nested children retain their actual
parent and explicitly require the existing update relay. Ready and unresolved
assets do not imply zero usage. No relay update is introduced into generation.
Each current Workload adds one allocation query. The CLI prints charged cycles,
pending count and known reservations or `unknown`. Invalid or unavailable
observations remain unavailable. The report remains outside spending authority
and does not alter desired state or reviewed debit bounds.

The live allowance projection retains allocation release-set evidence and Spec
hash, then requires the selected release-build identity and compiled Spec hash.
It interprets the configured role policy using the same pure lifetime-budget,
cooldown and request-clamping functions as runtime admission. The CLI shows the
lifetime limit, remaining allowance after charges, remaining cooldown and
next-request policy cap. Reservations are not subtracted again from charged
usage. Any pending operation leaves the cap unknown, including settled transfer
costs with unresolved response recovery. Invalid no-pending reservation evidence
or future charge timestamps reject. Exhausted/over-limit charges saturate at
zero exactly as runtime policy does. No additional IC calls or native recovery
conditions are introduced.

## Focused evidence

- Two native core tests cover retained pending receipts after replay expiry,
  wrong payer, expired/missing reservation evidence, settlement with unresolved
  response, terminal commit, other-child exclusion, repeat-read stability and
  Candid roundtrip. Log: `/tmp/canic-child-usage-core.log`.
- Ten host startup cases and the CLI rendering case pass, including exact
  participant/timestamp validation and unknown-versus-zero accounting.
  Log: `/tmp/canic-child-usage-host-cli.log`.
- Direct/relayed participant binding, owning-Root routing and generated-estate
  planning/apply/effect-free replay pass.
  Log: `/tmp/canic-child-usage-bindings-generation.log`.
- All-target/all-feature warning-denied Clippy passes for core, facade, host,
  CLI and internal testing. Log: `/tmp/canic-child-usage-final-clippy.log`.
- The existing IC case
  `pic::fleet_registry::baseline::tests::funding_deadline::child_grant_refreshes_root_funding_deadline_without_repeating_credit`
  passes using real sealed canisters. It checks Prepared Root access,
  non-controller rejection, zero initial usage, a 2T grant, unchanged usage on
  exact replay and 3T cumulative usage after the next 1T grant. It retains the
  earlier deadline, automatic allowance and no-spurious-burn assertions.
  Log: `/tmp/canic-child-usage-ic.log`.

## Limits and remaining work

The live allowance follow-up passes sixteen host startup cases, one CLI rendering
case, five authoritative runtime-policy cases and generated-estate planning,
apply and effect-free replay in `/tmp/canic-live-allowance-tests.log`. Tests
cover exact cooldown expiry, request and lifetime clamping, exhaustion, pending
and missing evidence, no double deduction and release/Spec/role mismatch.
The shared runtime arithmetic retains its existing decision semantics. These
are native/host tests, not new IC recovery or remaining-demand qualification.
Final scoped all-target/all-feature core/host/CLI Clippy passes in
`/tmp/canic-live-allowance-final-clippy.log`. Formatting, layering, current
document semantics and whitespace checks pass.

The parent-attribution follow-up passes thirteen host startup tests and CLI
rendering (`/tmp/canic-funding-parent-tests.log`), generated-estate planning,
apply and effect-free replay (`/tmp/canic-funding-parent-generation.log`), and
all-target/all-feature host/CLI Clippy
(`/tmp/canic-funding-parent-final-clippy.log`). New cases cover Root versus
nested funding parents, exact allocation mismatch and incomplete settlement,
query-only transport, fresh failure visibility and skipping Ready assets before
transport discovery. This follow-up changes host projection only. The existing
IC result above covers the unchanged runtime query, not the new host mapping.

Pending and expired reservation observations are native state-machine evidence;
the IC case observes settled transfers and replay, not a paused in-flight call.
The generation preview reads ledgers for seeded Root-funded Workloads and
reports nested funding-parent attribution. It does not yet read each
descendant's parent ledger or map complete live placements to
the configured role scenario. Its initial-demand calculation still assumes
fresh child ledgers and zero burn. These observations are not a live
remaining-demand quote or an activation funding guarantee.

Descendant ledger collection must use the existing budgeted recovery observation
path; generation retains its query-only boundary. These separate observations
are not an atomic estate snapshot or authority to spend. Per-child policy
allowance does not establish parent liquidity, enablement or window admission.
Minimum native Root recovery must remain independent of descendant telemetry
availability: an underfunded Root may need that credit before a relay is affordable.
Exact live placement matching and startup/recovery quote
integration remain open, along with exact operator mint receipts and the
whole-recovery funding preview. No new live Toko recovery or full-suite result
is claimed. The accepted 0.110.17 batch remains not push-ready.
