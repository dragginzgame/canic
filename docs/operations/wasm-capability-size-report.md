# Wasm Capability Size Report

`scripts/ci/wasm-capability-size-report.sh` produces a machine-readable,
disjoint shallow-byte view of one symbol-preserving Wasm artifact. It separates
authentication and admission, metrics, child provisioning, remaining Canic
runtime, application and upstream code, unattributed stripped code, and Wasm
structure/ABI bytes.

Use a diagnostic artifact that retains function names:

```text
scripts/ci/wasm-capability-size-report.sh \
  --wasm path/to/project_instance.wasm \
  --output path/to/project_instance.size.json \
  --role project_instance \
  --build-profile debug \
  --build-network ic \
  --producer-identity v0.109.24-or-exact-commit \
  --capabilities Runtime,ChildProvisioning \
  --metrics-tiers Core,Runtime,Security \
  --endpoint-exports 273
```

The output schema is `canic.wasm_capability_size.v1`. Each category records
exact shallow-byte totals, item counts, and its 20 largest items. The report
also records the artifact digest and size, immutable producer identity, build
profile and network, Twiggy version, role capabilities, metrics tiers,
endpoint-export count, classification revision, symbol coverage, and explicit
interpretation limits.

This is an attribution aid, not a deployment gate or proof of causal marginal
cost. Rust generic instantiations and dependency code are deliberately grouped
with application/upstream code unless a symbol has an unambiguous Canic owner.
A stripped `code[N]` entry remains `unattributed_code`; the tool never guesses.
Compare reports only when their build profile and toolchain are independently
known to match and the recorded role metadata and classification revision are
equal.

This focused report does not replace or change the retained
`CANIC-WASM-001/v4` recurring audit method. A future audit-method revision may
adopt it after its own method-change gate.
