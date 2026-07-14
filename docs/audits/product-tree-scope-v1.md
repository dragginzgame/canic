# Product Tree Scope Version 1

The 0.92 product tree is the sorted Git mode/blob/path identity of every
tracked path at a named commit except the audit-method and current line
coordination paths listed below.

Excluded audit-system inputs:

- `docs/audits/`;
- the active 0.92 design/tracker directory;
- `docs/status/current.md` and `CHANGELOG.md`;
- the product-tree hash, audit catalog, layering, instruction, and Wasm helper
  scripts; and
- the instruction audit harness and support files.

Everything else is product tree, including manifests, lockfile, source,
tests, canisters, fleets, build/release/operator scripts, active operator
documentation, and CI workflows. Therefore audit-method preparation that
changes release validation, operator contracts, or CI behavior changes the
product tree and must be explicitly reviewed before the product baseline is
established.

Canonical command:

```text
bash scripts/ci/audit-product-tree-hash.sh <full-commit>
```

The helper accepts committed snapshots only. It intentionally does not hash a
dirty working tree as though it were an immutable product baseline.
