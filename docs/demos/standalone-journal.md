# Standalone deployment journals

Use this guide for the explicit `--no-config` workflow and its original journal. Run commands from the directory that owns that journal and preserve any original `--state-dir` override. For a named app and fleet, follow the [managed Root walkthrough](root.md#deploy-the-generated-application).

## Create a standalone plan

From the Canic checkout, build the CLI and template, then plan with the operator's public identity. Prepare the default signer with `make install-ic-tools`.

```sh
make canic
canic artifact build --package ex-template --json > target/hello-build.json
canic fleet plan --no-config --json \
  --wasm "$(jq -r .wasm target/hello-build.json)" \
  --network ic \
  --identity canic-mainnet \
  --cycles 1000000000000 > target/hello-plan.json
cat target/hello-plan.json
```

Review the controller, network, artifact and budget. The example selects **1T
cycles from the identity's default cycles-ledger account**, including the ledger fee; the provider's creation fee comes out of the attachment. It
does not convert ICP. The current ledger fee is checked before initial dispatch;
this experimental path assumes that fee remains unchanged during submission.
Plans expire after one hour. Applying the exact digest is the spending approval:

```sh
canic fleet apply --no-config --json "$(jq -r .operation target/hello-plan.json)" \
  --identity canic-mainnet > target/hello-receipt.json
canic fleet status --no-config "$(jq -r .operation target/hello-plan.json)"
```

Enter the identity password in ICP CLI's terminal prompt when requested. Signing
progress goes to stderr; JSON stdout stays machine-readable. The deployment
receipt verifies the certified module hash and sole controller. The greeting
is explicitly an **uncertified query**. `status` reports retained local evidence.

The status summary shows the app/fleet/slot (when named), original network
binding, planned artifact and controller, creation budget, and new-effect
deadline. Its effect table distinguishes an unknown outcome from a retained
reply or rejection. A retained reply alone does not prove success; a receipt
records historical deployment evidence without checking current canister health.
Expiry never proves that an uncertain attempt failed. Add `--json` to retain
the complete structured output for scripts.

Preserve `.canic/`: it contains approved artifacts, signed requests and receipts,
with owner-only file permissions. It is ignored by Git and must survive process
restarts. Protect it as authority-bearing state. Rerun **the same `apply` command**
to resume. Within the original signed submission window, Canic can resend only
the exact retained request. An expired attempt without a retained result stays
blocked; do not delete its directory or create another plan to guess recovery.
The CLI does not currently renew read-state authority after that window.

## Upgrade the starter

From the Canic checkout, edit the greeting returned by
`apps/template/src/lib.rs`. Build the edited application and use its
new module hash to plan an upgrade of the same canister.

```sh
canic artifact build --package ex-template --json > target/hello-build-v2.json
canic fleet plan --no-config --json \
  --wasm "$(jq -r .wasm target/hello-build-v2.json)" \
  --network ic \
  --identity canic-mainnet \
  --target "$(jq -r .canister target/hello-receipt.json)" \
  --previous-module "$(jq -r .module_hash target/hello-receipt.json)" \
  > target/hello-upgrade-plan.json
cat target/hello-upgrade-plan.json
canic fleet apply --no-config --json "$(jq -r .operation target/hello-upgrade-plan.json)" \
  --identity canic-mainnet > target/hello-upgrade-receipt.json
```

An upgrade requires the exact prior module hash and sole controller before its
first submission. It creates no canister and debits no cycles-ledger funds;
execution is charged to the existing canister. Coordinate operations through one
retained state directory. Cross-host fencing and concurrent external changes to
the same canister are not qualified by this first path.

Local plans use `--network local --url http://127.0.0.1:PORT/ --root-key HEX
--root-subnet SUBNET --routing-canister PRINCIPAL`, with the selected replica's
DER trust root, independently obtained root-subnet identity and an existing local
canister for subnet routing. The test harness supplies these;
local creation uses the replica's provisional API, whereas mainnet uses the
[cycles ledger's creation API](https://github.com/dfinity/cycles-ledger/blob/main/README.md).
The local rehearsal does not prove the live ledger/CMC integration.

Historical local approvals without `root_subnet` remain readable with their
original digests, but cannot make network requests through the new transport.
Do not edit those approvals to add trust or recreate uncertain effects. New
local fleets must supply all four inputs. Mainnet's trust inputs are compiled in.
