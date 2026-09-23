# Canonical optimized symbol correspondence

This trace names the already-retained canonical cohort bodies. It supplies no
new size delta, determinism, runtime-parity or build-performance claim.

## Inputs and compiler capture

Both diagnostic runs use the original cohort preparation on immutable `.5`,
the original strip setting, and the real pinned host finalizer/tools. The
retained metadata includes both exact compiler wrappers. A wrapper asks the
leaf's existing linker invocation for `--Map` and `--no-demangle`, then copies
its raw Wasm after successful compilation. It changes no optimization, target,
feature, endpoint, type or source input. The first run captures width 1; the
second captures widths 2–5. The second diagnostic keeps the native driver
build target across widths, asking Cargo to check it each time. Its native
executable hashes agree; this is not a retained clean-repetition run.

Every rebuilt diagnostic has exactly the original canonical executable
sections, including type, import, function, table, export, element and code
sections. Data differs only in equal-length random characters of the private
`/tmp/canic-wasm-ablation.XXXXXX/` path. All other initialized bytes and all
addresses match. Payload hashes, lengths, Candid, direct function counts and
captured methods are independently checked. The source and original lock are
clean after each completed run.

## Names and final optimizer

Parse the linker map's exact symbol, body offset and entry size. Wasm LLD's
function offsets omit the code section's header, so account for that parsed
header length; validate each offset and complete size against a real raw code
entry. Do not guess function identities from ordering or demangled text.
Append these linker-derived names as a custom section, then run the same
`ic-wasm shrink`, public Candid metadata insertion and Binaryen 132 `-Oz`
transforms, preserving the name section. The emitted optimizer feature flags
are retained per width. The actual compiler, optimizer and transform inputs
are hash-bound in the verification record.

Names can reorder function/type indices. Parse both optimized modules with
WABT's debug-name-free output. Resolve each type index to its exact parameter
and result signature. Compare every function's signature, locals, control
flow and instructions; only direct function-reference indices and type indices
may be renamed. Constant values, local/global indices, memory operands and
indirect-table slots are not normalized away.

Construct candidates from identical bodies under those reference placeholders,
then remove candidates whose direct-call references cannot correspond. Choose
one complete bijection and verify every call under that exact assignment,
including cyclic calls. Require every import, exported function and active
table binding to retain its exact owner. The retained witness maps all
4,071/4,074/4,077/4,080/4,081 functions and imports. Some identical internal
bodies have interchangeable structural candidates; the complete binding/call
check, not an arbitrary partial match, validates the selected witness.
All other non-custom sections, including initialized data, are exact between
the rebuilt canonical module and this name-carrying transform.

## Canonical report

Demangle the surviving optimizer names with the retained Rust demangler and
apply the verified bijection. The report records fully qualified names,
concrete generic arguments, exact original canonical indices, original
canonical body lengths/hashes and diagnostic counterparts. Since the rebuilt
canonical code section exactly equals the retained measurement's code section,
these are bodies from the canonical final artifact, not substituted diagnostic
bodies. The original Wasm/Candid hashes and full vectors remain authoritative.

The report maps 15/18/21/24/25 selected bodies. At widths 1–4 it includes the
935-byte `Page<GenericCohortNominal1>` type body and 636-byte serializer, plus
maintained `Page<LogEntry>` type, serializer and drop bodies. At width 5 the
two separate nominal Page entries are no longer named; the fifteen nominal
type-identity bodies remain. No missing name alone establishes elimination,
inlining or a recoverable saving. Body sizes are not summed into the
inclusive artifact delta, and no linear per-type cost is inferred.

The earlier strip-none diagnostic remains a counterexample: it changed code
by minus 71 bytes and functions by minus ten. Its whole-module indices and
larger-body guesses are not inputs to this canonical trace.
