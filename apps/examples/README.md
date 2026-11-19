# Example fleets

| Example | Declared canister tree | What to try locally |
| --- | --- | --- |
| [`ex-greeting`](greeting/) | Greeting | Call a stateless canister, then upgrade it to change the reply. |
| [`ex-counter`](counter/) | Root with Store and counter children | Increment a stored value and read it after an upgrade. |
| [`ex-backup-counter`](backup-counter/) | Root with a counter child | Restore a checkpoint, then accept and reopen the counter through Root. |
| [`ex-user-directory`](user-directory/) | User hub with pooled user shards, plus Root and Store | Register two accounts, sign in to their shards and observe refusal when the pool fills. |

Each directory contains one app definition in `canic.toml`. From the repository
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
