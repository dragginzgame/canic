# MODULAR AUDIT - Module Cleanup Runner

Use this workflow when the user asks to clean up a named Canic module or crate
using Module Surface Hardening rules.

This is the implementation runner for
`docs/audits/modular/module-surface-hardening.md`. The MSH document owns policy,
taxonomy, authority rules, and full audit reporting. This runner owns the short,
repeatable cleanup loop.

## Purpose

Clean a named module to production grade by deleting, narrowing, inlining,
moving, or explicitly retaining code with authority. Do not redesign the module.
Do not perform style-only cleanup unless it is adjacent to a real
removal/narrowing patch.

A module is production-grade when every retained item has an owner and authority
reason, every removable stale surface is gone, every public/exported item is
intentionally public, tests do not force production visibility wider than
needed, and hot or wasm-sensitive cleanup does not change runtime shape without
proof.

## Inputs

Capture before editing:

| Field | Value |
| ---- | ---- |
| target module | path |
| owning crate | crate name |
| audit tier | `Tier 0` / `Tier 1` / `Tier 2` |
| expected hotness | `cold` / `warm` / `hot-runtime` / `encode-decode-hot` / `query-executor-hot` / `wasm-sensitive` / `test-only` |
| patch mode | `implementation-requested` |
| full MSH escalation needed? | yes/no |

## Audit Tier

Use the lowest tier that can honestly classify the module.

| Tier | Use when | Output |
| ---- | ---- | ---- |
| `Tier 0` | Cold/warm internal modules with no facade, generated-boundary, storage, recovery, stable format, query-hot, or wasm-sensitive involvement. | Cleanup runner only. |
| `Tier 1` | Some public/internal exposure risk, test-only production surface, duplicate helpers, or stale compatibility, but no persisted/runtime authority change. | Compact MSH report. |
| `Tier 2` | Facade API, generated-boundary uncertainty, stable-memory/storage records, backup/recovery, lifecycle/install/upgrade, deployment-truth, authority reconciliation, query/encode hot paths, or wasm-sensitive changes with unclear runtime shape. | Full MSH report. |

Escalate to the full MSH report when the module touches facade API,
macro-generated code, `canic::__internal`, `canic::__build`,
`canic_core::__reexports`, storage records, stable-memory formats, exported
snapshots, deployment-truth evidence, authority reconciliation, capability or
replay enforcement, lifecycle hooks, canister install/upgrade, control-plane
state, wasm-store payloads, encode/decode loops, query execution, commit or
recovery behavior, or wasm-sensitive code with unclear runtime shape.

## Phase 1 - Mechanical Inventory

List:

* public, `pub(crate)`, `pub(super)`, and `pub(in ...)` items
* `#[doc(hidden)]` items
* cfg-gated and test-only items
* re-exports, facade exports, and generated-boundary exports
* one-caller helpers
* public helpers consumed only by tests

Search for:

* `allow(dead_code)`, `expect(dead_code)`, and `expect(unused_imports)`
* `legacy`, `compat`, `compatibility`, `fallback`, `shim`, and `deprecated`
* runtime reconstruction from DTOs, exported snapshots, or deployment-truth
  evidence where stable records, config, or observed state should be authority
* duplicate endpoint, lifecycle, deployment, backup, storage, metrics, or
  control-plane entrypoints
* test-only production consumers

Identify direct consumers through compiler output, direct imports, focused
search, and tests. Do not classify from text counts alone.

## Phase 2 - Classify

For each candidate, assign:

| Field | Values |
| ---- | ---- |
| surface class | `live-authority`, `live-generated-boundary`, `live-diagnostics`, `live-test-support`, `stale-compatibility`, `stale-generated-fallback`, `orphaned-helper`, `overexposed-internal`, `duplicate-surface`, `unclear` |
| confidence | `high`, `medium`, `low`, `blocked` |
| disposition | `DELETE NOW`, `NARROW NOW`, `INLINE NOW`, `MOVE OWNER`, `MOVE TO TEST`, `RETAIN WITH OWNER`, `DEFER WITH TRIGGER`, `RETAIN HOT PATH`, `MEASURE FIRST`, `PATCH WITH PROOF`, `REJECT CLEANUP`, `BLOCKED` |

Owner means the smallest Canic boundary that owns the invariant or contract:
module, crate, facade boundary, generated boundary, storage/recovery contract,
deployment-truth workflow, diagnostics/test support, or named maintainer when
an architectural decision is required.

## Phase 3 - Patch Safe Items Only

Allowed by default:

| Class | Default action |
| ---- | ---- |
| `orphaned-helper` with high confidence | delete or inline |
| `overexposed-internal` with medium/high confidence | narrow visibility |
| test-only production helper | move to test support |
| generated-only public surface | move behind `__internal`, `__build`, `__reexports`, or the narrow generated boundary |
| stale compatibility before `1.0.0` | delete when compile/tests prove it |
| one-caller helper with no invariant | inline |

Pre-`1.0.0` hard-cut applies to unsupported internal protocols by default. It
does not by itself authorize deletion of stable-state, backup, exported
snapshot, deployment-truth, or live operator data compatibility without an
owner decision and migration/recovery proof.

Not allowed without proof or owner decision:

| Surface | Default action |
| ---- | ---- |
| public facade removal | `BLOCKED` or `DEFER WITH TRIGGER` |
| generated-boundary removal | `BLOCKED` until macro expansion, generated output, or derive/endpoint tests prove safety |
| persisted format, stable-memory schema, backup, or recovery behavior | full MSH report and owner decision |
| hot-path shape change | `MEASURE FIRST` unless shape is unchanged |
| closure/generic/iterator rewrite in encode/decode, stable-memory, query, or scheduler loops | `MEASURE FIRST` or `RETAIN HOT PATH` |
| allocation, clone, formatting, or dynamic dispatch added to success path | `REJECT CLEANUP` unless proof exists |

## Phase 4 - Validate

Run the smallest meaningful validation:

* `cargo fmt --all` after code edits
* `cargo check -p <owning-crate>` for the owning crate
* focused tests for the module
* clippy for the owning crate when the slice is ready
* dependency cleanup check if `Cargo.toml` changed
* raw wasm byte comparison if runtime canister payload or wasm-sensitive code
  changed
* focused benchmark or instruction proof if hot-path shape changed

For documentation-only edits, use docs-appropriate validation such as
`git diff --check`. Do not start or stop the local ICP/DFX network for this
audit.

Do not repeatedly rerun expensive failing commands. Capture the first failure,
fix the direct cause when it belongs to the slice, and report anything broader.

## Stop Condition

Stop the cleanup slice when:

* all high-confidence safe actions are patched or explicitly rejected
* every remaining item has owner, authority reason, and trigger/proof
* validation has been run or a blocking failure is reported
* no new architectural redesign is required

## Phase 5 - Compact Report

Use this report shape for ordinary module cleanup:

```markdown
# MSH Module Cleanup: <module>

## Verdict
- Risk score:
- Tier:
- Patch mode:
- Cleanup result:

## Evidence Log
| Evidence | Command / Inspection | Result | Artifact |
| ---- | ---- | ---- | ---- |
| public surface inventory |  |  |  |
| stale-signal scan |  |  |  |
| consumer check |  |  |  |
| validation |  |  |  |

## Removed / Narrowed / Inlined / Moved
| Item | Action | Why safe | Validation |
| ---- | ---- | ---- | ---- |

## Retained With Owner
| Item | Owner | Authority reason | Trigger to revisit |
| ---- | ---- | ---- | ---- |

## Blocked / Measure First
| Item | Reason | Required proof |
| ---- | ---- | ---- |

## Verification
- cargo check:
- focused tests:
- clippy:
- wasm/raw-size check, if relevant:
```

Use the full MSH report only for high-risk modules, public/facade surfaces,
generated-boundary involvement, storage/encoding/query hot paths, recovery,
authority reconciliation, deployment truth, backup, control-plane state, or
unclear authority.
