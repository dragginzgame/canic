# canic

Blank Rust library workspace. Product code lives in `crates/canic/src/lib.rs`.

This is the fresh Canic rewrite. The predecessor is maintained separately as
[canic-legacy](https://github.com/dragginzgame/canic-legacy).

The proposed [Canic system design](docs/design/canic-system-design.md) defines
the restart: product scope, architecture, capability ownership, deferred ideas,
qualification and delivery milestones. It is a design for review; runtime
implementation has not started.

Rust 1.100.0 is pinned in `rust-toolchain.toml`. Install it with Rustup, then run:

```sh
make install-tools
make tools-check
make toko-tooling-check
make check
```

Run `make help` for development commands. `make ci` is the aggregate local check;
the individual targets select narrower checks. `make install-hooks` optionally
enables the shared formatting hook.

`toko-tooling` is pinned as a development dependency in the root Cargo catalog.
Its authenticated source snapshot supplies `TOKO-TOOLING.md`, shared Make
commands and their supporting scripts. Read `AGENTS.md` and `TOKO-TOOLING.md`
before making changes. No application behavior has been added.

To update tooling, deliberately change the root version pin, run
`cargo update -p toko-tooling`, then `make toko-tooling-preview` and
`make toko-tooling-update`. Verify with `make toko-tooling-check`.
