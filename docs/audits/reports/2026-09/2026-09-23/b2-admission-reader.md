# B2 admission reader selection

Date: 2026-09-23. Status: focused implementation and diagnostic evidence;
complete B2 qualification remains open.

## Behavior and ownership

Generated endpoint guards now select the Fleet-admission reader from their
complete declared expression, including nested `all`, `any` and `not`. Endpoints
without that predicate instantiate the evaluator without its storage reader.
Custom predicates retain their own declared implementation dependencies. Manually
constructed expressions still use the maintained full evaluator; this is a
runtime expression API, not a predecessor compatibility lane.

The evaluator remains responsible for short-circuit order, normalized typed
denials and exactly one denial metric. Reaching a predicate absent from the
selected evaluator is terminal: negation and alternative branches cannot turn
that selection error into admission. No dynamic storage callback registry was
introduced. The existing custom-predicate and recursive-future abstractions are
unchanged in purpose.

`canic::access::auth::is_fleet_admitted(caller)` is now synchronous, reflecting its
local stable-state read. Direct users remove `.await`; endpoint predicate syntax
and the observed-transport-caller helper remain unchanged. This is a pre-1.0
hard cut. The async version left a resume-state call in the disabled evaluator,
so a constant branch alone did not eliminate the reader from optimized Wasm.

## Qualification

The focused access tests cover typed denials, exact transport identity, boolean
short-circuit order, custom predicate order, empty expressions, and denial metric
parity between the selected and full evaluators. The macro test parses generated
Rust and checks its actual const selection for ordinary, explicit-admission,
custom and nested declarations. It does not pin rendered explanatory text.

All 43 focused native tests pass (28 access, 13 macro expansion and two endpoint
compilation cases), as does strict core/macros all-target/all-feature Clippy.
The final exact PocketIC admission case passes after the synchronous change:
fenced access, admitted/unlisted callers, target/digest binding and same-release
restoration remain correct. The governed runner completed in 134 seconds.
The [structured evidence](b2-admission-reader.json) binds source and log hashes.

## Paired diagnostic

The local method is retained under `.tmp/b2-admission-reader-20260923/`, with
exact source archives, harness, locked dependency graph, build logs, raw and
optimized artifacts, and section/name records. All dependency package identities
match the workspace lock; only the probe package is added. Before/after use the
same paths, compiler, profile, manifest and flags, with source replacement made
visible to Cargo. Export names and kinds agree for every pair.

The three fixtures request only a parent guard, only an admission guard, or both
through distinct endpoints. They intentionally omit lifecycle setup and are
compile/reachability controls, not representative Canic runtime journeys.
Rust uses Release `z`, LTO and one codegen unit, preserving names for attribution;
Binaryen 132 applies `-Oz` with the raw module's supported feature set. This is
not the product's stripped/shrunk/Candid-annotated canonical pipeline. Named
function absence alone does not prove that inlined code is absent, and these
vectors must not be substituted for B2's per-role artifact qualification.

| Guard control | Code bytes before → after | Function delta | Table-slot delta |
| --- | ---: | ---: | ---: |
| parent | 1,178,604 → 1,155,078 | -23 | +0 |
| admission | 1,178,604 → 1,178,898 | +0 | +0 |
| mixed | 1,180,295 → 1,185,679 | +22 | +2 |

The parent control loses 23,526 code bytes and 23 functions. Its named admission
record-construction, drop and installed-projection-validation functions disappear;
the required data-only allocation declaration remains. The admission-only control
gains 294 code bytes; the mixed control gains 5,384. Keep this tradeoff visible in
the required canonical measurements rather than crediting only the negative delta.

The mixed table grows from 488 to 490 slots. Before selection it contains one
evaluator drop/poll pair; afterward it contains the `false` and `true` pairs.
Every other table-entry name and multiplicity agrees after removing only Binaryen's
numeric name suffixes. Exact slots, function indices and raw names are retained
in the evidence. This explains the diagnostic table growth; it is not a body-level
bijection proof for canonical role artifacts.

An earlier all-features optimizer attempt produced an unsupported import format
for the section reader; it is excluded. An early source replacement preserved
archive timestamps and Cargo reused baseline objects; that comparison is also
excluded. Corrected builds explicitly dirty replaced inputs and retain compiler
logs. The superseded async specialization is retained as a counterexample: the
parent fixture gained 416 code bytes with admission storage still reachable;
the mixed fixture gained 5,721 code bytes and two table slots. None of those
superseded outputs qualifies the final source.

No controlled build-speed, peak-resource, maintained-workload instruction or
canonical production-footprint claim follows from this diagnostic. B2 still
requires the current full role/fixture artifact matrix, instruction comparisons
that include first access, and accepted absolute reserves. No broad validation,
versioning, commit, publication or sibling modification ran.
