# Backup counter

Root controls one [counter](../counter/). Its build recipe enables Root’s
`backup` feature. This local topology exercises the retained inline creation
path. The managed starter example also includes a versioned Store.

From this directory with the Canic CLI installed:

```sh
canic app canister list
canic app build
canic artifact inspect artifacts/root.wasm
```

For the installed exercise on Linux, run these commands from the repository
root with the simulator prepared:

```sh
make backup-signer-setup
make backup-signer-test
```

The test uses a disposable signer and synthetic cycles. It installs the two
artifacts from this config, captures and restores the counter, and checks that
the operator must separately accept and reopen the restored state through Root.
