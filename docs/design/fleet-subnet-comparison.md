# Explicit subnet comparison for a configured fleet

`canic fleet compare-subnets LEFT RIGHT` compares two operator-selected mainnet
subnets using IC Query's existing distribution, metrics and overlap report.
The configured fleet supplies application context and network admission only;
neither subnet is inferred from bindings or claimed to belong to the fleet.

From an existing app directory after `make canic`:

```sh
canic fleet compare-subnets \
  tdb26-jop6k-aogll-7ltgs-eruif-6kk7m-qpktf-gdiqx-mxtrf-vb5e6-eqe \
  brlsh-zidhj-3yy3e-6vqbz-7xnih-xeq2l-as5oc-g32c4-i5pdn-2wwof-oae \
  --app starter --fleet mainnet
```

Add `--json` for the complete Query evidence under `observation`, alongside app
and fleet labels. Use `--config PATH` for an explicit existing catalog. The example
subnets demonstrate syntax, not deployment recommendations. Local fleets,
malformed principals and equal selections fail before network IO. Missing or
ambiguous configuration fails through the existing config selector. No journal,
signer, state-directory override, automatic candidate selection, watch or deployment
operation is involved. Store placement checks remain independent.

Query owns the existing four component reads, their bounds and failures: left
provider/location distribution, left metrics, right distribution, right metrics.
At most 160 asynchronous seconds and 774 logical calls, plus bounded synchronous
work; no additional reads or retries. Canic retains the original report, including
independent versions, source times, coverage and certification claims. A failed
component never erases a successful one. Complete collection exits 0; partial
results, including all-failed reports, remain on stdout with exit 1. Invalid ID
values or network admission exit 1; missing/extra CLI arguments exit 2.

Text uses bordered side-by-side raw resource/configuration and coverage tables,
plus observed shared provider/data-center counts with unknown coverage. Zero
observed overlap does not establish independent failure domains. Missing metrics
stay unavailable, not zero. Values establish neither allocatable headroom,
application suitability, running software nor merge feasibility. JSON preserves
raw evidence and Query's overlap identities; no snapshot or recommendation is added.

The registry package `ic-query-topology` 0.110.6 supplies the schema 1 report.
The Host adapter owns network/config admission; CLI owns parsing and presentation.
The Query comparison contract and its legacy provenance review remain the source
of acquisition policy; Canic does not duplicate the predecessor's catalog or caches.

## Local qualification — 2026-11-15

Two Host fixtures passed for pre-IO network/identity admission, duplicate refusal,
argument order and all-failed evidence. Four CLI fixtures passed for option scope,
no journal access, zero versus unavailable metrics, raw JSON/terminal escaping,
and overlap with unknown coverage and independent Registry versions. Production
Host/CLI Clippy and warning-denied Host Rustdoc passed. These checks qualify this
adapter, not the concurrent Store-maintenance changes.

The installed JSON command compared the example NNS and `brlsh` subnets through
`https://ic0.app`, starting at Unix second 1794752570. It exited 0 after 17.34
seconds with complete collection, matching requested Registry versions, seven
observed shared providers and five shared data centers. The temporary configured
fleet's journal path remained absent. This is one mainnet smoke observation,
not capacity, uptime, independence or hosted macOS qualification.
