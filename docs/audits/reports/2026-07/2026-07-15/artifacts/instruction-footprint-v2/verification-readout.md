| Command | Status | Notes |
| --- | --- | --- |
| `cargo test --offline --locked -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed through authoritative root-harness artifacts and wrote the report plus normalized artifacts. |
| `fresh authoritative root harness profile per scenario` | PASS | Each scenario used a fresh topology/capability/scaling/sharding root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Runtime, PageRequest { limit=512, offset=0 })` | PASS | Update scenarios were sampled before/after through persisted perf rows; the install scenario groups retained bootstrap checkpoints. Normalized rows are under `artifacts/instruction-footprint-v2/perf-rows.json`. |
| `repo checkpoint scan` | PASS | Found 57 checkpoint call sites. |
| `checkpoint delta capture` | PASS | 21 non-zero checkpoint delta rows were captured under `artifacts/instruction-footprint-v2/checkpoint-deltas.json`. |
| `fixed v2 update/install scenario roster` | PASS | All twelve required scenarios completed; query instruction totals are outside this method version. |
| `baseline comparison` | PASS | No comparable v2 report exists; this valid run establishes the first v2 baseline and deltas are `N/A`. |
