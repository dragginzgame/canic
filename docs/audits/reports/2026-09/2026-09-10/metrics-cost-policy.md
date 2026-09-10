# Sampling cost reference policy — 2026-09-10

The maintainer directed that an arbitrary instruction number must not become a
future release blocker. Canic now reports the historical 20M reference as
advisory; it does not fail solely for exceeding it. Runtime sampling, instruction
measurement, collection bounds, retention and rejection/recovery are unchanged.

## Provenance and decision

Commit `3ca0a18bfbe3c6c6065274d40febda0b604b0612` introduced the 20M test
assertion in 0.110.11 on 2026-09-08. The recorded large-source sample cost was
4,647,992 instructions. History retained the same reference in 0.110.12; Toko's
real-producer qualification adopted it in commit `9f2a9eb48`. No inspected
source, design or receipt derives exactly 20M from an operating-cost or latency
requirement. The approximately 4.3-fold original headroom was an observation,
not a documented formula for choosing the threshold.

The current test prints first, repeat and scheduled callback instruction costs,
the historical comparison point and whether it was exceeded. It requires
positive Wasm measurements. It preserves the relative source-growth test:
4,096 recorded source checkpoints versus 256 must stay within twice the first
sample's cost. The sixteenfold source increase retains the same bounded prefix;
the twofold tolerance detects source-proportional work while allowing sample
variation. This is a fixture-specific regression, not a universal complexity or
cost proof. Structural tests remain the authority for exact collection and
retention limits.

The [maintained policy](../../../../features/runtime/public-observability.md#sampling-cost-qualification)
requires a workload, measured baseline, justified margin and explicit consequence
for an absolute regression threshold. An application operating budget also needs
cadence, deployment scale and actual latency or accounted cycle-cost requirements.
Absolute cost remains review evidence until such a budget is established.

Historical audit receipts keep their original measurements and original
pass/fail verdicts. The index optimization remains useful measured work; this
policy does not relabel those historical tests or imply that crossing 20M is an
IC runtime failure. There is no larger replacement limit or configurable bypass.

## Downstream boundary

Toko's current source still contains its own absolute assertion. The
[prepared test-only patch](toko-metrics-cost-reference.patch) replaces that
assertion with a positive-measurement check and advisory output. Its
[base and proposed hashes](toko-metrics-cost-reference.json) bind the reviewed
change. It remains unapplied reference material. The maintainer explicitly
restricted this task to Canic on 2026-09-10; downstream source adoption and
qualification are outside the active task.

## Explain handoff

The requested `explain()` matches ICYDB-021 in Toko's IcyDB ledger. IcyDB's
`docs/design/0.257-typed-query-explain/0.257-design.md` owns the accepted SQL-free,
fallible typed terminal over its existing planner and structured diagnostics.
Its current status leaves shared preparation accounting and bounded diagnostic
projection/rendering ahead of the terminal. The facade
`crates/icydb/src/db/query/typed.rs` has no terminal yet. The sibling has substantial
concurrent dirty preparation work; this inspection did not modify it.

The maintainer subsequently clarified that all work must stay in Canic. The
IcyDB terminal is outside this task; the inspection above was read-only and no
IcyDB implementation was started. No Canic planner, SQL-string adapter or
execution-profile feature was introduced.

## Targeted validation

- Governed `make test-pocketic-case CASE=timer_authority`: eight passed,
  19.51 seconds; complete targeted runner 29 seconds. First sample: 15,497,186
  instructions; repeat: 15,547,635; scheduled maximum: 16,586,795.
- Warning-denied `cargo clippy --locked -p canic-tests --test timer_authority`:
  passed.
- Targeted Rust formatting, diff whitespace, edited document links, current
  document semantics and the 0.110.14 release-notes preflight: passed.
- An initial attempt to select the integration test by its function name was
  refused by the runner before execution. The registered integration target
  above is the supported selection and passed.

[Structured results and retained log hashes](metrics-cost-policy.json) preserve
the exact checks. The Canic policy correction and open .14 changelog are ready
for release review. The downstream patch remains unapplied and IcyDB
`explain()` is outside the Canic-only task. No broad gate, version bump, Git
publication or deployment ran.
