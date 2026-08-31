# Canic 0.102 Reason Ledger

Date: 2026-08-16

## Purpose

`crates/canic-host/diagnostics/reasons.toml` is the source for Canic-owned
diagnostic numbers and host prose. It generates only:

- registered runtime constants; and
- the host reason catalogue.

The ledger and catalogue are host/repository assets and must not enter release
Wasm. 0.102 does not generate a language-neutral JSON or Markdown registry.

## Row Shape

Each row contains:

| Field | Meaning |
| --- | --- |
| `code` | Nonzero `u16` identity |
| `name` | Symbolic host/runtime name |
| `origin` | Stable semantic owner |
| `summary` | Concise host-owned explanation |
| `guidance` | Optional advice safe for every occurrence |
| `retired` | Whether a released reason no longer has an active producer |

Producer sites, retry policy, handling, exposure and audit-frontier data are
not ledger fields.

## Release Rule

Unreleased rows and numbers may be changed or removed freely. Once a reason has
appeared in a released Canic version, `code + name` is its immutable semantic
identity. Summary and guidance may change, origin may change after review and
`retired` may move only from `false` to `true`.

When a released reason is removed, its minimal row remains with
`retired = true`. It has no active runtime constant. The unreleased 991-row
candidate therefore gets replaced directly and creates no retirement history.

## Focused Validation

Targeted checks prove:

1. codes are unique and nonzero; numbers encode no semantics and no ranges are
   reserved;
2. comparison with the latest released ledger rejects a changed `code + name`
   identity or reuse of either member for another cause;
3. generated runtime constants and host catalogue entries match active rows;
4. retired released rows have no active producer constant; and
5. the ledger, host catalogue and temporary B1 evidence are absent from
   representative release Wasm.

Rust privacy is the primary construction guard for
`RegisteredDiagnosticCode` and `Error`. Additional tests cover only invariants
the type system cannot express.
