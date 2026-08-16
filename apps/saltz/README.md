# Saltz Waveform Application

Saltz is an experimental Canic application for rendering the selected neon
mountain trace in the public ICP Cycle Burn Rate graph. The checked-in
application is deliberately inert: it cannot burn cycles, arm a run, schedule
a waveform or accept a funding authorization.

This first implementation boundary contains:

- one ordinary Fleet Subnet Root package;
- one inert Burner Component package; and
- one pinned numeric waveform CSV consumed by the standalone preview.

It also contains an independent `saltz_preview` canister. The preview is not a
Fleet role and has no Canic runtime dependency. It exposes one read-only
`http_request` query that serves:

- `/` — the exact 860-point waveform as a code-native T2-style SVG graph;
- `/waveform.csv` — the digest-verified reference CSV; and
- `/api/status.json` — bounded machine-readable runtime, qualification,
  observation and provenance facts.

The preview has no update call, timer, stable state, scheduler, authorization
surface or `cycles_burn` path. The repository and preview contain no source
photograph, raster derivative, graphical overlay or image route. Ordinary
canister execution consumes normal platform cycles; the displayed zero refers
specifically to intentional waveform burn.

The page makes no network-status claim merely because it responded. It labels
itself as an inert served preview, distinguishes the static proposed Dashboard
total from the exact dated global observation band, identifies the restaurant
image as absent, and states both that burn capability is absent and that
ordinary query execution still consumes cycles. The footer's `Canic` label
links to the authoritative repository.

The public HTTP surface is intentionally anonymous. Its HTML, status JSON and
CSV contain no `Saltz`, `neon` or publisher-name token, and the source-article
link is absent. The long German document title is the only textual clue to the
artistic source. Internal package names and offline design evidence retain their
truthful implementation identity.

The checked-in CSV preserves the provisional numeric waveform and exact
rational 24-hour time axis. The removed image-authoring pipeline no longer
exists in the workspace, so the CSV is presentation evidence rather than an
executable `RunPlan` or reproducible source-extraction claim. Dashboard
qualification and any future privately held source evidence remain open.

The current review mapping is a provisional `100..=150 Bcycles/second`
combined visible-rate target. Its `50 Bcycles/second` relief replaces the
former `16.667 Bcycles/second` candidate after dated global IC observations
showed that the smaller mountain could be lost in baseline volatility. This
raises the zero-background 24-hour exposure to approximately
`10_464.206204 Tcycles`; B0 visibility and economic qualification remain
mandatory before any destructive implementation or authorization.

No application runtime code deploys another Canister or performs a destructive
external effect. Burner implementation remains gated by the accepted shared
`ic-timers` consumer hard cut, Dashboard qualification and an explicit
external-effect authorization contract.

Build and test the standalone preview without creating a canister:

```sh
cargo test -p saltz_preview
icp build saltz_preview -e saltz-preview-local
```

For a local browser proof, start the configured local network and deploy only
the preview environment:

```sh
icp network start -e saltz-preview-local -d
icp deploy saltz_preview -e saltz-preview-local
```

The preview intentionally serves an uncertified query response and therefore
uses a raw gateway URL. Locally, open the deployed principal as:

```text
http://<canister-id>.raw.localhost:8002/
```

On IC mainnet the equivalent public form is:

```text
https://<canister-id>.raw.icp0.io/
```

An IC-mainnet deploy is a separate external effect. It requires an
authenticated dedicated identity plus an explicitly approved environment,
Subnet/controller set and cycle ceiling. The prepared environment contains
only this standalone canister:

```text
saltz-preview-ic -> saltz_preview
```
