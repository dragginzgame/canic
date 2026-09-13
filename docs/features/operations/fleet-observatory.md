# Fleet observatory

The host-owned observatory collects one terminal Fleet's retained identity and
independent live role observations. `canic-host::observatory` supplies passive
views, a data-only profile, escaped HTML and a framework-neutral HTTP adapter.
The downstream application owns serving, collection scheduling and its own
application sections. Canonical canisters do not contain a renderer or a new
polling service. These are current schema-version-1 contracts, subject to the
pre-1.0 reinstall-only hard cut.

```sh
canic --environment local observatory snapshot demo
canic --environment local observatory snapshot demo --public --profile observatory.json --out snapshot.json
canic --environment local observatory snapshot demo --public --profile observatory.json --html --out report.html
```

A minimal profile is:

```json
{"schema_version":1,"title":"My Fleet","role_labels":{"root":"Fleet Subnet Root"}}
```

Profiles contain labels only. They cannot select endpoints, insert executable
markup, supply Principals or expand authority. Unknown fields reject. Labels
are bounded to 128 bytes, and a profile has at most 128 role labels. A CLI profile
file is limited to 64 KiB. Output files use exclusive creation and private file
permissions; an existing file is not overwritten.

## Evidence and partial results

The private JSON snapshot includes exact selected Fleet/network/release/module
bindings and placement from the validated terminal Ensure inventory. These are
retained observations, not a new live management-module attestation. Discovery
checks the plan digest, selected network and initial Registry, and rechecks the
same authority after collection. Concurrent changes reject the result. A missing
or nonterminal authority produces an unavailable authority field and no remote
discovery; it does not produce an empty healthy Fleet.

Local journal progress remains separately visible during interrupted operations:
completion class, applied and pending effect counts, funding-review requirement
and stalled-observation count. Reading this evidence does not authorize a retry
or imply that an issued effect failed. The current journal is not a global
inventory of every autonomous runtime operation.

Each live field records its observation time, source and either its value or a
closed failure class. The collector never retries, substitutes another role,
infers readiness from a balance, or erases successful observations because an
independent role failed. Selectors are role-specific:

| Role | Public observation | Protected observation |
| --- | --- | --- |
| Coordinator | Exact `Overview` profile and bootstrap readiness | Funding policy, native balance and current Root funding-request count |
| Root | Exact `Overview` profile and bootstrap readiness | Funding; bounded pool inventory header totals |
| Wasm Store | Exact `Overview` profile and bootstrap readiness | Existing catalog `Storage` selector |
| Component | Exact `Overview` profile and bootstrap readiness | No assumed generic funding/estate/Store selector |

Every request requires the retained protocol binding and exact local Candid
sidecar. Overview role and profile must match that binding. The Store keeps its
existing Root/retained installation-controller caller rule. The host adds no
controller bypass. Missing bindings, denial, malformed replies, timeouts and
unsupported selectors remain explicit unavailable values. Funding balances are
exact native cycle integers encoded as decimal strings; they are not Ledger
balances or conservation receipts.

## Store inventory

The existing protected Store status now includes approved catalog-entry count,
expected and retained template-chunk counts, and separate fixture-source,
expected-fixture-chunk and retained-fixture-chunk counts. Counts derive from
current metadata and chunk keys; collecting them does not load chunk payloads.
They supplement occupied/maximum/remaining bytes, retained template/release
counts and GC state.

A retained chunk count is not a content-integrity proof. During partial upload
or GC, expected and retained counts can differ, including source metadata being
removed before its chunks. Occupied Store bytes are the owned storage-accounting
charge, not allocated stable-memory pages. Historical metrics remain event
counters and are not used to infer this inventory.

## Public publication and freshness

`ops::presentation::public_view` removes exact Fleet identities, Principals,
parents, subnets, release/module hashes, admission counts, local journal and
funding. The public contract retains role labels, reporting/bootstrap facts and
aggregate Store inventory. Free-form runtime errors and subprocess diagnostics
are excluded. The data-only profile cannot reintroduce private fields.

`ops::presentation::http_response` serves `/` as escaped HTML and
`/snapshot.json` as JSON from the same curated view. Other paths return 404.
Responses specify `no-store` and a restrictive content-security policy. The
caller supplies the current clock time at each request. Expired observations
and timestamps in the future become `unavailable/stale`; a cached successful
reply is never rendered as currently ready. Static reports are point-in-time
artifacts: they do not acquire live freshness merely by being hosted.

Downstream code should retain private snapshots only in its trusted collector,
pass them through this public projection, and serve the returned public bytes.
Application data needs its own explicit application-owned view.

## Host budgets

| Option | Default | Supported range |
| --- | --- | --- |
| `--maximum-canisters` | 128 | 1–4096 retained instances |
| `--maximum-response-bytes` | 1 MiB | 1 KiB–16 MiB per stdout/stderr stream |
| `--maximum-report-bytes` | 1 MiB | 1 KiB–16 MiB serialized output |
| `--query-timeout-secs` | 5 | 1–60 seconds |
| `--maximum-collection-secs` | 60 | 1–3600 seconds of remote collection |
| `--freshness-secs` | 30 | 1–3600 seconds |

These are selectable host resource envelopes, not application row limits or
runtime capacity promises. Oversized selection rejects instead of silently
truncating. The runner captures bounded streams before JSON/Candid decoding,
kills and reaps its exact timed-out child, and excludes diagnostics from views.
The collection deadline includes the bounded ICP version check and limits each
remaining query. Local authority-file reads are outside the remote-query time
budget. Each role uses at most three queries; unsupported selectors issue none.
Candid decoding also has finite work/skipping quotas. Report serialization uses a finite writer.

## Qualification

The focused Fleet/SDK/observatory case passes in 116.10 seconds (132-second
runner). The private snapshot covers five retained instances with eight queries
and 7,670 bytes. Three real ICP collections—private, public and a deliberately
broken Store binding—take 1.59 seconds. Exact Coordinator/Root funding and Store
inventory respond; public reports exclude every retained Principal; a broken
Store binding leaves the App observable. The initial fixture failures corrected
a Store-only feature gate and restored the App sidecar after a separate
intentional-corruption test. No caller or binding check was weakened.

The synthetic Store uses the same Fast profile and optimizer in both samples:

| Artifact measure | Before OP3 | With OP3 | Increment |
| --- | ---: | ---: | ---: |
| Raw bytes | 3,295,707 | 3,298,921 | 3,214 (0.098%) |
| Code-section bytes | 3,014,786 | 3,017,338 | 2,552 (0.085%) |
| Defined functions | 13,773 | 13,789 | 16 |

Before SHA-256:
`2c70452f52c47a4ae52dabe3451335d19e7642e9702f407895fd061daa7a7afa`.
After SHA-256:
`88b97d4e3ca7a122e3a79054f1fd7eb0602ab33b04edb9819b5adc62242c3b25`.
These are synthetic Store artifacts, not an exact Toko workload or a mainnet
query-instruction/RSS measurement. The renderer adds no canister code. The Store status extension also enlarges
the Root's Store-client decoding: its code section changes from 8,952,039 to
8,956,756 bytes, an increase of 4,717 bytes (0.053%). Coordinator code remains
4,916,487 bytes and App code remains 2,987,420 bytes. Those additional code
measurements come from the same before/after build logs; the named binary
hash comparison above covers the Store only.

The live run preserves all 1,680 recorded source/dependency inputs at base
commit `8907442e028ebe44f47090783c39f64bcf940b04`, with the dirty open .16 work
and package versions still .15. Its final host-only deadline recheck and
exhausted-budget regression are qualified separately by native tests. Store
retirement/upload-count tests, CLI help, scoped Clippy, layering, documentation
and package checks are recorded by the
[batch tracker](../../design/0.110-fleet-runtime-contraction/status.md).
The exact opt-in case is
`pic::fleet_registry::baseline::tests::frontend_handoff_public_cli_and_sdk_preserve_admission_and_local_trust`;
its independent SDK consumer prerequisites are in the
[frontend guide](frontend-handoff.md).

This snapshot selects retained terminal inventory. It does not automatically
crawl later autonomous descendants or post-convergence Component operations;
those need a refreshed exact inventory/binding source. A valid partial report
is not a healthy-Fleet verdict: consumers must inspect each observation's state.
No immutable package publication, live Toko adoption, browser identity-provider
ceremony or external static-asset deployment is claimed.
