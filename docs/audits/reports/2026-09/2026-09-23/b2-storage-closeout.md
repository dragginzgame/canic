# B2 storage ownership and query qualification

Date: 2026-09-23. B2 complete and push-ready with the explicitly accepted
cold-query tradeoff. This is targeted batch qualification, not a full release gate.

## Root payload ownership

Current bootstrap stages application releases in the host-installed sibling
Wasm Store. `workflow/bootstrap/root_store::bootstrap` resolves the exact
adopted Store, validates its catalog and payloads, then mirrors approved
manifests into Root state. `root_missing_staged_release_roles` requires every
manifest binding to resolve in Root's Store inventory. Root's approved-manifest
mirror is required; Root-local chunk payload storage has no current producer.

The remaining `bootstrap` binding special case incorrectly treated a name as
permission to read Root-local chunk storage. The correction removes that branch
from approved install-source resolution and release copying. Every binding now
resolves through `RootWasmStoreStateOps::wasm_store_pid`, with the existing typed
missing-manifest diagnostic for an unknown binding. The management-chunk fallback
and its orphaned metrics are removed. Store-owned chunk operations/readers are
compiled only for the Store (or native storage tests). Global memory declarations
and maintained Store schemas remain unchanged.

The exact-binding native test checks the recorded Principal and typed rejection
of an unregistered binding. Sixteen focused template workflow tests and strict
Root-only feature Clippy pass. All 92 focused core metrics tests and strict
core/control-plane Clippy across all targets/features pass. The exact governed
Root/Store PocketIC case
`pic::fleet_registry::baseline::tests::prepared_root_initial_shard_bootstrap_reaches_terminal_component_membership`
passes in 415 seconds, covering real provisioning, terminal membership and
interruption recovery. Logs are retained in the evidence bundle.

## Query investigation

The [previous canonical checkpoint](b2-canonical-measurements.md) retains two
reproducible history-subspan regressions. Instrumented canonical functions show
that neither synthetic metric-name construction nor cloning publication
configuration explains the extra cost in those two states. The selected malloc
span is also nearly unchanged there. These diagnostic wrappers change calls and
add scalar Wasm globals; they do not change the update heap's allocations.
They are attribution controls, not qualification artifacts.

Two isolated source experiments were excluded from the product:

- Borrowing the publication set saves about 1,200 instructions per query but
  leaves the approximately 80,000-instruction jumps.
- A borrowed chronological history iterator removes the scratch sample vector
  and saves a few thousand instructions, but also leaves the jumps. The product
  history layout and reader remain unchanged.

A separate wrapper retains the original Candid reply bytes, postpones only the
final reply, and appends a diagnostic counter after complete synchronous query
execution. It includes preflight, authorization, handler and response encoding.
Its counter excludes the diagnostic trailer and final deferred reply; both
conditions use identical instrumentation. This extends the measured boundary,
without replacing or suppressing the earlier failing observations.

The final candidate exactly repeats all measured fields. All twelve complete
sampling exports remain within 1%. Four of twelve complete queries exceed 1%:

| Rows | State | Access | Before | After | Extra instructions |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1 | same-Wasm restore | second | 924,071 | 1,001,492 | 77,421 |
| 211 | install | second | 1,084,015 | 1,161,564 | 77,549 |
| 256 | install | first | 1,079,899 | 1,159,757 | 79,858 |
| 256 | install | second | 1,243,382 | 1,323,390 | 80,008 |

A separate diagnostic writes identical bytes once per 4,096-byte heap region
before entering the measured query. It preserves all update/lifecycle vectors,
history sizes and typed response checks. With that first-touch work paid before
the counter, every query comparison is within 1%; the largest increase is
0.146%. Every cold-minus-warm difference is an exact multiple of 80,000. Each
failing cold case adds exactly one such charge; two other cases shed one.
This identifies first-touch heap charging as the difference, rather than an
80,000-instruction increase in allocation or history-processing work. It does
not identify a particular address or assert that a simulator charging unit is
an operating-system page.

The warm control does not replace cold qualification. On 2026-09-23 the
maintainer explicitly accepted these four measured B2 cold-query regressions as
a documented exception. The design's maximum 1% instruction allowance remains
in force elsewhere. This accepts the retained vectors, not an unrestricted
allowance for future regressions. No new CI gate or absolute instruction cap is
added.
The complete cold vectors, repeat and warm controls are retained under
`.tmp/b2-closeout-20260923/{full-query,warm-query-control}/`; the comparison is
`instruction-comparison.json` in that same run root.

## Generic cohort disposition

Retain the accepted B1 `Page<T>` cohort. B2 does not change its nominal types,
page representation, wire contract or selection mechanism. The canonical leaf
at width one remains a positive Fleet-admission control. The refreshed named
reports cover the changed `AccessExpr` evaluator specializations: selection is
one closed boolean (admission used or unused), independent of the number of
`Page<T>` nominal types. No five-width B2 slope or new generic saving is claimed.
The B1 five-width evidence remains attribution; B5 owns final clean-build and
complete fixture qualification.

## Final canonical artifacts

The [structured evidence](b2-storage-closeout.json) and
[compressed records](b2-storage-closeout-records.tar.gz) retain all fifteen
final artifacts, counters, exact interfaces, generated selections, source/tool
identities, named generic bodies and complete canonical function mappings.
The previous checkpoint remains immutable: this final pair measures the last
Root ownership cut against that checkpoint's `after` artifact. Cumulative B2
values compare against its original `before` artifact, not the B1 ablation.

| Artifact | Final code bytes | Last-cut code delta | Cumulative B2 code delta | Cumulative function delta | Cumulative table-slot delta |
| --- | ---: | ---: | ---: | ---: | ---: |
| app | 2,698,748 | +1 | -25,609 | -51 | -22 |
| test | 2,981,672 | +1 | +3,729 | -15 | -20 |
| root | 7,484,879 | -20,746 | -45,651 | -94 | -48 |
| fleet_coordinator | 3,821,562 | +0 | -27,684 | -53 | -25 |
| wasm_store | 2,630,419 | +1 | -24,775 | -62 | -32 |
| index_hub | 2,807,250 | +1 | -24,905 | -48 | -22 |
| index_child | 2,699,634 | +1 | +3,325 | -16 | -20 |
| user_hub | 2,925,920 | +1 | +2,896 | -15 | -21 |
| user_shard | 3,174,061 | +1 | +3,296 | -22 | -20 |
| scale_hub | 2,787,751 | +1 | -25,965 | -49 | -22 |
| scale_replica | 2,706,964 | +1 | -25,609 | -51 | -22 |
| runtime_probe | 2,504,458 | +4 | -23,036 | -54 | -22 |
| payload_limit_probe | 1,885,560 | +1 | -23,778 | -58 | -22 |
| blob_storage_probe | 2,089,126 | +1 | -23,722 | -63 | -26 |
| leaf_probe | 2,722,689 | +1 | +3,314 | -16 | -20 |

Across eleven separate canonical modules, cumulative B2 code falls by
186,952 bytes, defined functions by 476
and table slots by 274. The four fixtures separately lose
67,222 code bytes, 191 functions and
90 slots. These sums are not one canister's headroom.
All final artifacts preserve exact Candid, exports and role selection; all meet
the frozen absolute code/function reserves and total-module limit. No table grows.
Independent section parsing, the replica-local-function counter and optimizer
metrics agree; gzip round trips and structural validation pass.

The last Root cut removes 20,746 code bytes, 29 functions and two table slots.
All six named chunk-storage bodies disappear; all 24 approved-manifest bodies
remain. Complete canonical function-reference mappings accompany source
reachability review; named absence alone is not an inlining proof. Root ends at
7,484,879 code bytes and 10,083 defined
functions, leaving 3,000,881 code bytes and
39,917 functions below the frozen limits.

The final captured source contains 1,994 files and twelve changes relative to
the previous candidate. All production inputs match the shared checkout.
Two native-only test assertions were corrected after capture to expect the
maintained `Store` metric label; their exact overlay and structural `cfg(test)`
boundaries are retained. The corrected 92-test run and final lint include that
overlay. It has no Wasm production input difference.

The subsequent maintainer gate found direct stable-record access in the new
workflow binding test. Its setup now uses the existing
`RootWasmStoreStateOps::import_test_state` boundary, preserving exact binding
and typed-rejection assertions. The
[native correction](b2-layering-correction/evidence.json) records the final
targeted checks and a third native-only overlay. Every byte before that file's
`cfg(test)` module matches the captured artifact source. The layering guard and
production inputs are unchanged; the retained canonical measurements remain
the qualified artifact snapshot.

The final artifacts use the same Release pipeline, paths, lockfile and tools as
the previous candidate, with dependency reuse inside the isolated target. These
are not independent clean repetitions or build-time/resource measurements.
Raw Wasm, source archives and full linker maps remain under
`.tmp/b2-closeout-20260923/` and `.tmp/b2-canonical-20260923/`.
The retained methods and raw-output hashes bind the complete-query, repeat and
warm-control results. Earlier name/config/malloc controls and excluded reader
experiments are diagnostic only.

## Batch decision

Stop B2 at push readiness. Lazy storage, direct selected lifecycle/recovery,
admission-reader selection, Root payload ownership, adversarial/recovery
qualification and artifact/instruction evidence are complete. The maintainer's
four-case query exception closes the measured tradeoff; the general 1% rule
remains unchanged. Earlier B2 focused lifecycle/admission evidence remains in
the linked checkpoint reports. Scoped formatting, whitespace and current-document
semantics pass; the latter retains only the existing parked-history advisories.

The complete accepted in-repository release batch and both open `.39` changelog
surfaces are ready for the maintainer's release flow. Shared package metadata
remains `.37`; published `.38` reconciliation and the version transaction stay
with that flow. No broad suite, version change, commit, push or deployment ran.
B3 is a separate residual record/codec decision, not a prerequisite for shipping
this completed batch. B4 generated-surface work and B5 clean-build/fixture
qualification remain sequenced work; this is not minor closeout.
