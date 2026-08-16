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
total from the exact dated global observation band, and states both that burn
capability is absent and that ordinary query execution still consumes cycles.
The footer's `Canic` label links to the authoritative repository.

The public HTTP surface is intentionally anonymous. Its HTML, status JSON and
CSV contain no `Saltz`, `neon` or publisher-name token, and the source-article
link is absent. The long German document title is the only textual clue to the
artistic source. Internal package and design names retain their truthful
implementation identity.

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

The repository also contains a separate `cycle_burn_probe` qualification
canister. It is not the Burner Component and cannot schedule or replay a
waveform. Its controller-only `burn_once` update has no amount argument: the
Wasm hard-binds exactly one `4 Tcycle` pulse, retains at least `1 Tcycle` and
succeeds only once per install. The controller-only `burn_status` query makes
the immutable envelope and committed receipt recoverable when an update
response is uncertain. Deploy it locally with explicit synthetic funding:

```sh
icp identity new cycle-burn-local
icp deploy cycle_burn_probe -e cycle-burn-local --cycles 6t \
  --identity cycle-burn-local
icp canister call cycle_burn_probe burn_once '()' \
  -e cycle-burn-local \
  --identity cycle-burn-local \
  --candid canisters/test/cycle_burn_probe/cycle_burn_probe.did
```

Keep the CLI default identity separate and pass `--identity cycle-burn-local`
explicitly. On a fresh deployment the creating identity becomes the probe
controller. Existing deployments require a deliberate controller transfer
before use.

The returned receipt proves the local burn primitive and accounting path. It
does not populate or reproduce the public ICP Dashboard, whose cycle-burn-rate
series observes mainnet Subnets only.

The separately authorized B0b mainnet calibration is now complete. One exact
`4 Tcycle` pulse was clearly visible in the owning Subnet's public series, but
the attributable peak was only approximately `0.883 Bcycles/second` and
remained spread across the observed tail. A naive independent 100-second
bucket would have been `40 Bcycles/second`. The API `step` parameter therefore
does not provide the independent approximately 100-second drawing buckets the
current 860-point plan assumed. The probe has no timer or second-burn path; no
additional effect is authorized. See the
[bounded calibration report](../../docs/audits/working/saltz-b0b-calibration/mainnet-calibration.md).

The preview remains excluded from every checked-in IC-mainnet environment. The
calibration probe remains excluded from the broad `ic` environment and is
admitted only through `cycle-burn-calibration-ic`. That dedicated environment
does not itself authorize an effect: the exact identity, mint, creation,
Subnet, burn and retained-balance envelope must be recorded before each use.
