# CANIC-159 Accepted-phase failure propagation — 2026-09-10

The current Canic runtime passes the missing Store-outage journey. Coordinator
retains Root's exact originating failure while Root is Accepted, then the same
operation recovers to terminal runtime activation after Store restarts. Normal
Coordinator-dependent publication waits still allow convergence. This is Canic
PocketIC evidence; no downstream pin, sibling source or live Fleet changed.

## Maintained regression

Extend the existing five-component activation journey in
`crates/canic-testing-internal/src/pic/fleet_registry/baseline.rs`. Reuse its
ordinary compiled Root, Coordinator, Store and workload artifacts, controller
checks, protocol actions and terminal replay. No new fixture mode, scheduler,
endpoint or production test hook is introduced.

The journey already stops Store before Root acceptance, then restarts it and
reissues the same provisioning command. The added boundary waits for Root's
Accepted phase and stops that same Store again. Protected Root and Coordinator
queries must observe one matching failure:

- Root phase: Accepted; originating stage: Provisioning.
- Origin target: the exact Root; origin operation: the existing provisioning ID.
- Origin diagnostic: `PLATFORM_UNAVAILABLE` (66); retry category: Backoff,
  with a retained retry deadline.
- Coordinator retry stage: RootProvisioning, bound to that Root and operation.
- Every origin field equals Root's observation, including the original failure
  timestamp. Polling waits for matching observations instead of assuming both
  independently scheduled owners update in one tick.

The observed outer Coordinator code is 140, with structured origin code 66.
The original source timestamp is 1620328636000000146 ns. The distinction between
outer and originating diagnostics remains visible; no error-string comparison
stands in for typed evidence.

Store restarts under its existing controller. The journey then reaches Published,
exercises its existing later StoreCatalog outage/backoff and restoration, and
finishes with all five workloads and Root runtime active. Coordinator's pending
failure and Root's last failure clear. The immediate compiled replay has no
nonterminal actions and therefore issues no updates. Existing protected-access
and permanent-failure fixture ownership remain intact.

## Validation and scope

The exact governed PocketIC case passes: one executed test, 257.30 seconds
including artifact builds; complete runner 275 seconds. This is not a full
Fleet or workspace suite. The runner closes its shared PocketIC resources and
invocation-owned scratch. An initial native compile found a test-only comparison
between `u16` and `DiagnosticCode`; explicit raw-code conversion corrected it
before the successful run. The final import is test-scoped for the ordinary
library build; this annotation does not change the passing runtime journey.

The case is
`pic::fleet_registry::baseline::tests::fresh_five_component_provisioning_reaches_runtime_active_and_publishes_catalog`,
selected through `make test-pocketic-case CASE=...` with locked offline Cargo
and `RUSTC_WRAPPER=`. [Structured evidence](canic159-runtime.json) records source
hashes, commands and retained logs. All-target/all-feature test-package Clippy passes with warnings denied.
Targeted formatting, diff whitespace, edited document links, current-document
semantics and the 0.110.14 release-notes preflight pass. Import cleanup briefly
displaced an existing test-only annotation; restoring the grouped import
resolved that lint failure before the successful final check.

This closes the candidate-runtime evidence gap identified by the earlier
[four-item audit](../2026-09-09/toko-feedback.md#canic-154159-retained-estate-recovery-and-origin).
Publication, downstream adoption and staging recovery remain separate. The
existing open 0.110.14 draft owns this regression; no version bump or Git
publication is included.

The accepted Canic correction/qualification batch and open .14 changelog are
ready for release review. This completes the runtime-proof follow-up before
expanding changed-role artifact reuse; CANIC-141 remains deferred. An unrelated
editor workspace check started during artifact building; it is not counted as
validation here. The shared target was idle before the final lint run.
