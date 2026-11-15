# Configured fleet placement diagnostics

`canic fleet placements` observes the explicitly configured identities of one
selected mainnet fleet. It does not inspect journals, discover retained creation
outputs, load a signer or authorize deployment. Application roles and parent slots
remain declarations, separate from routing and controller authority.

From a directory containing the existing app configuration, after `make canic`:

```sh
canic fleet placements --app starter --fleet mainnet
canic fleet placements --app starter --fleet mainnet --json
```

Use `--config PATH` for an existing catalog elsewhere. App/fleet selection uses
Canic's existing ambiguity checks. Local networks and journal-only operation are
unsupported. Empty bindings remain visible as unbound slots; a selected mainnet
fleet with no bindings emits its declarations without a network call and exits 1.

Host validates every bound ID, duplicate identities and the 32-ID limit before
calling Query. One collection reads sequentially within a shared 60-second budget,
preserving per-read evidence, failures, cancellation and unstarted rows. Canic
retains the complete Query report and maps canonical slot-name order to collection
indices; it does not infer relationships from common subnet IDs. Exit 0 means all
configured selections were read, not that every application slot is bound. Partial
collection exits 1 while preserving stdout. Text uses Host's bordered renderer;
JSON retains original values. Syntax errors exit 2.

Text summarizes successful/configured reads, unresolved bindings, unbound slots
and distinct subnets among successful reads. Unknown bindings do not contribute
to the subnet count; these summaries add no queries and leave JSON unchanged.

`ic-topology` 0.106.1 is selected from immutable Git revision
`0a3eca919f05d777798406087049cc51ab7ae2a8` of `dragginzgame/ic-query`; packages are
not published to crates.io. Query owns lookup policy and bounded protocol reads.
Canic owns config selection, application labels and output. Store's existing
operation-bound placement checks and `fleet show`/watch budgets are unchanged.

## Local qualification — 2026-11-15

Three Host fixtures cover pre-IO refusal, all-unbound selection and partial
source evidence. Three CLI fixtures cover option scope, terminal escaping/JSON,
and an all-unbound config without journal writes. Production Host/CLI Clippy
and Host Rustdoc passed. The installed all-unbound command exited 1; a two-ID
mainnet collection plus an unbound slot exited 0 and retained all three slots.
The observed queries completed at Unix second 1794750189, with verified query
signatures, noncertified results and no common Registry version. No journal was
created. This smoke check is not an uptime, ownership or native-host matrix claim.
