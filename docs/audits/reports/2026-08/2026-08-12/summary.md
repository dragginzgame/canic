# Audit Summary — 2026-08-12

## Wasm Footprint

[CANIC-WASM-001/v3](wasm-footprint-v3.md) establishes the first current-scope
baseline at immutable release `v0.101.53`. The versioned method admits all six
configured application Components plus Fleet Subnet Root, Fleet Coordinator and
Wasm Store. Historical v2 evidence remains valid superseded history and is not
numerically comparable with v3.

The fresh release/debug run passed with risk `5/10`. Release Wasm ranges from
2,597,251 bytes for Wasm Store to 7,539,746 bytes for Fleet Subnet Root. The six
Components have a narrow `1.0444` release-size spread. The root is `2.4012`
times the largest Component, and its largest retained structural item accounts
for 69.1944% of release Wasm. This is size-pressure evidence, not a product
correctness finding.

The run used a clean detached linked worktree at
`23c0328f78b215580d734ef01b52b35fa3e38ade`, isolated local/offline build state,
the repository-pinned `ic-wasm 0.11.0` and no replica, credentials or external
mutation.
