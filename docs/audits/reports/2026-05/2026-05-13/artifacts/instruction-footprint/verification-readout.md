| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed and wrote the report plus normalized artifacts. |
| `fresh root harness profile per scenario` | PASS | Each scenario used a fresh smallest-profile root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Runtime, PageRequest { limit=512, offset=0 })` | PASS | Update scenarios were sampled before/after through persisted perf rows, and query scenarios used local-only `QueryPerfSample` probe endpoints because query-side perf rows are not committed; normalized rows saved under `/home/adam/projects/canic/docs/audits/reports/2026-05/2026-05-13/artifacts/instruction-footprint/perf-rows.json`. |
| `repo checkpoint scan` | PASS | Found 56 checkpoint call sites. |
| `checkpoint delta capture` | PASS | 4 non-zero checkpoint delta rows were captured under `/home/adam/projects/canic/docs/audits/reports/2026-05/2026-05-13/artifacts/instruction-footprint/checkpoint-deltas.json`. |
| `query perf visibility` | PASS | All sampled query scenarios returned `QueryPerfSample` local instruction counters through the local-only probe endpoints, which avoids relying on non-persisted query-side perf state. |
| `baseline comparison` | BLOCKED | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |
