# CANIC-150: observed retry deadlines

Date: 2026-09-24. Canic implementation only; Toko Miner remains read-only.

## Feedback and implementation

Toko Miner's September 24 CANIC-150 reassessment adopts the published .39 phase
summary and batch-progress corrections. Its remaining diagnostic request is to
retain a retry deadline only when the originating owner supplies one. The
reported activation wait showed ComponentRuntime/ComponentMembership, code 137,
Backoff, with an unavailable deadline. This correction does not claim that every
such wait has a deadline: a newly attached local failure still carries none.

Root status already exposes its persisted retry schedule. The Coordinator's
conversion into protected failure context dropped that value. The internal
failure view and current `ProvisioningFailureOrigin` now carry `retry_at_ns`.
Root and child-allocation projections copy their recorded deadlines; a fresh
local failure supplies `None`. Coordinator persistence and status retain the
observation without recomputing a delay. The nullable Serde field is required
explicitly, including `null`; canonical Coordinator Candid includes `opt nat64`.
This changes the current pre-1.0 status/storage contract under reinstall-only
release policy, without a compatibility reader or new protocol generation.

Existing host progress and timing receipts preserve the field automatically.
TTY, plain and stale-heartbeat output render known deadlines as UTC timestamps,
labelled observed. The TTY deadline occupies its own line so an owner identifier
cannot push it beyond an ordinary 80-column display. They do not consult the local wall clock, promise completion,
or turn an expired observation into another countdown. Unknown observations
replace earlier known values. Deadline churn stays outside host durable-progress
identity, CLI last-change age and receipt advancement evidence.

No polling, retry scheduling, paid effects, authority checks or workflow decisions
change. The larger internal diagnostic required boxing the nested publication
transport error so its result remains bounded; typed diagnostic mapping is
unchanged. No dependency or package version changes are part of this correction.

## Qualification

All targeted qualification passes. Logs and source/lockfile hashes are retained
under `.tmp/canic-150-deadlines-20260924/`:

- 33 selected native tests across core serialization, control-plane failure
  projection/restoration and publication diagnostics, host progress identity,
  and CLI output/receipts. The final 23 CLI cases also pass in package isolation,
  including actual 80-column painting, unknown/expired deadlines, deadline-only
  changes and interrupted receipt retention.
- Three Candid checks cover reported/unknown deadline round trips, exact
  canonical Coordinator type equality and the retained failure-stage contract.
- Strict all-target/all-feature Clippy explicitly selects `canic-core`,
  `canic-control-plane`, `canic-host`, `canic-cli`, `canic-testing-internal` and
  `canic`. The final rendering revision passes the same selected graph.
- The exact governed PocketIC case
  `pic::fleet_registry::baseline::tests::fresh_five_component_provisioning_reaches_runtime_active_and_publishes_catalog`
  passes in 285.82 seconds; the runner takes 404 seconds including harness
  preparation. Its existing Store outage now verifies the Coordinator carries
  the same deadline as Root status, then recovers the same operation. These are
  local qualification durations, not a performance comparison.
- Scoped formatting, layering, whitespace, current-document semantics and draft
  release-notes preflight pass. The document guard retains two existing layout
  advisories about the parked metrics-history design.

The first simulator attempt could not bind localhost inside the sandbox and ran
no tests. The successful rerun used the same exact case with local simulator
access. No full workspace suite ran. The pre-existing lockfile edit is unchanged.

## Delivery boundary

This completes the bounded retry-deadline diagnostic portion of CANIC-150.
Published CLI adoption and normal/interrupted live
output acceptance remain downstream work. Per-target timing attribution and
controlled application-scale build measurements remain separate follow-ups;
no speed improvement is claimed.

Keep this operator-feedback correction with CANIC-182 in the open .40 batch on
the affected .110 line under the documented corrective-release cadence exception.
No broad suite, release/version transaction, staging, commit, push, deployment or
external repository mutation is part of this work.
