| Command | Status | Notes |
| --- | --- | --- |
| `cargo test -p canic-tests --test instruction_audit generate_instruction_footprint_report -- --ignored --nocapture` | PASS | PocketIC runner completed and wrote the report plus normalized artifacts. |
| `setup_root() per scenario` | PASS | Each scenario used a fresh root bootstrap instead of sharing one cumulative perf table. |
| `canic_metrics(MetricsKind::Perf, PageRequest { limit=512, offset=0 })` | PASS | Perf rows were sampled before and after each scenario; normalized rows saved under `/home/adam/projects/canic/docs/audits/reports/2026-04/2026-04-03/artifacts/instruction-footprint/perf-rows.json`. |
| `repo checkpoint scan` | PASS | Found 52 checkpoint call sites. |
| `query perf visibility` | PARTIAL | 6 successful query scenarios left no persisted `MetricsKind::Perf` delta; they are treated as method-limited rather than zero-cost. |
| `baseline comparison` | BLOCKED | First run of day for `instruction-footprint`; baseline deltas are `N/A`. |
