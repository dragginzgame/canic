# Audit Report Archive

This directory contains append-only primary audit reports plus the supporting
evidence still needed for findings or comparisons. Start with a month summary
when one exists, then use a day summary before opening individual reports or
generated artifacts.

## Archive Index

Coverage below describes the checked-in tree as of 2026-07-14. A missing
summary is a legacy indexing gap, not evidence that the reports are invalid or
that a retrospective conclusion should be synthesized.

| Month | Run days | Day summaries | Month summary | Archive status |
| --- | ---: | ---: | --- | --- |
| [2026-02](2026-02/) | 1 | 1/1 | [summary](2026-02/summary.md) | Indexed |
| [2026-03](2026-03/) | 9 | 8/9 | [summary](2026-03/summary.md) | Partial legacy coverage |
| [2026-04](2026-04/) | 7 | 0/7 | Not present | Unindexed legacy runs |
| [2026-05](2026-05/) | 16 | 11/16 | [summary](2026-05/summary.md) | Partial legacy coverage |
| [2026-06](2026-06/) | 10 | 6/10 | [summary](2026-06/summary.md) | Partial legacy coverage |
| [2026-07](2026-07/) | 4 | 2/4 | [summary](2026-07/summary.md) | Partial current coverage |

The month summaries are retained historical documents and may describe the
tree as it existed when they were last updated. This index reports structural
coverage only; it does not rewrite their findings or status.

## Legacy Artifact Note

The March-July archive originally contained repeated instruction-footprint and
Wasm measurement outputs in several encodings.
Numbered Markdown reports remain valid same-day reruns under the historical
baseline policy. On 2026-07-14, raw per-canister Wasm tool output and redundant
instruction endpoint/checkpoint text exports were pruned after confirming that
their summarized evidence remained in the primary reports and compact baseline
artifacts.

Do not copy these layouts into new runs. New artifacts must be bounded,
necessary, stored under the owning day, and named by the primary report as
required by [AUDIT-HOWTO.md](../AUDIT-HOWTO.md).
