# Example fleets

The fleet designs below each have a distinct hands-on exercise.

| Fleet design | Status | What to try |
| --- | --- | --- |
| [Counter](counter/DESIGN.md) | Runnable, persistent upgrades | Increment a value, upgrade the executable and read the retained value. |
| [Metrics](metrics/README.md) | Runnable, IC Metrics | Inspect completed updates and handler instruction counts through the controlling Root. |
| [Notebook](notebook/DESIGN.md) | Runnable, local snapshot recovery | Restore “draft” and verify edits stay fenced until the installer reopens them. |
| [Asset library](asset-library/DESIGN.md) | Planned | Release one user’s Blob reference while another keeps the same content. |
| [User directory](user-directory/DESIGN.md) | Runnable, revocation walkthrough planned | Register two accounts on different shards and retry the first signup. |
| [Project board](project-board/DESIGN.md) | Planned | Upgrade one project subtree and inspect the untouched sibling. |
| [Miner pool](miner-pool/DESIGN.md) | Planned, simulator only | Raise and lower a production goal and watch miners start, drain and return. |

Only directories with a `canic.toml` appear in the CLI. From the repository
root, install the CLI and list the examples:

```sh
make canic
cd apps/examples
canic app list
```

Enter an example directory to select it automatically. `canic app build` builds
local artifacts without contacting a fleet or selecting a signing identity.
Run `make qualification-setup` from the repository root once before the
installed local exercises.

## Shared demo commands

Extend the shared interactive prompt used by Counter and Metrics to every
example fleet as its walkthrough is added or updated. Use `metrics` for the
selected application canister’s endpoint measurements, `metrics timers` or
`metrics blob` for another source, and `logs [before]` for
Root’s controller-only operational log.

When adding an optional Canic endpoint, implement its demo command once in the
shared prompt. Offer it wherever the built canisters export the required
methods and the command codec is available.
