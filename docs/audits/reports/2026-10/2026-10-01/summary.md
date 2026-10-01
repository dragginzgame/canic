# Audit summary — 2026-10-01

| Report | Result / validity | Outcome and limits |
| --- | --- | --- |
| [Module surface hardening](module-surface-hardening.md) | partial / valid for named samples | Read-only stale-surface and active-document review; no exhaustive reachability or release verdict |
| [Surface cleanup and subsystem usefulness](surface-cleanup-and-subsystem-usefulness.md) | partial / valid for named samples | Earlier finding-backed cuts and retained retirement/fixture/backup owners; different scope from the later audit |
| [Complexity, hard cuts and feature gaps](complexity-hard-cuts-and-feature-gaps.md) | partial / valid for screening and named traces | Frozen `.49`/`.48` source census; five test-residue fixes and one CI detector correction; 102 focused tests and targeted Clippy pass. Three Host shape cuts remain deferred, twelve incomplete/limited outcomes classified, and full semantic reachability/redundancy remains unreviewed. Concurrent FR1 excluded by maintainer instruction |
| [Complexity hard-cut follow-up](complexity-hard-cuts-and-feature-gaps-2.md) | pass / valid for named follow-up | Closes the three deferred Host families after the maintainer stopped concurrent implementation; removes missing-reviewed-input execution fallback, scope assertion and obsolete guide prose. Current contracts remain v1. All 241 focused tests and Host/CLI Clippy pass; 101 net Rust lines removed across both slices. Feature gaps, exhaustive semantic review and FR1 qualification retain their own boundaries |

Each report has a companion JSON manifest. Historical results are retained;
the follow-up closes named findings without rewriting earlier scoped evidence.
No report closes 0.110 or qualifies the concurrent Fleet release batch.
