# canic-installer

Published installer and release-set tooling for downstream Canic workspaces.

This crate owns the thin-root staging path:

- emit `.dfx/<network>/canisters/root/root.release-set.json`
- stage the ordinary release set into `root`
- resume root bootstrap
- drive the local reference-topology install flow against an already running `dfx` replica

Typical installed binaries:

- `canic-emit-root-release-set-manifest`
- `canic-stage-root-release-set`
- `canic-install-reference-topology`
