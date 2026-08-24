# Mainnet Playground

`playground` is the reusable App for exercising Canic against the real IC with
small, deliberately silly workloads. It is not one App or Fleet per experiment.
One top-level `playground_hub` Component can own multiple named scaling and
sharding pools containing different Canister roles.

The first workload is the `wenzelrolls` scaling pool, capped at five
`wenzelroll` Canisters on the same physical Subnet. The pool starts empty; each
authenticated hub command moves the configured five-worker target one Canister
closer to full. Every Wenzelroll implements the minimum IC HTTP gateway
interface, serves the embedded `wenzelroll/assets/wenzelroll.png`, and starts
the official YouTube player behind the page. Later experiments can add new
roles and pools alongside it.

One deployed hub can operate every pool already declared in its compiled App
topology. Adding a brand-new role or pool changes that topology and therefore
uses Canic's current pre-1.0 reinstall-only release boundary; it is not a
dynamic mutation of an existing Fleet. Creating, removing, recycling and
recreating instances of an already declared role all remain same-release
lifecycle operations within the Fleet.

The five slots are Component descendants. The `playground_hub` is the
separately counted Component root; Canic's Coordinator, Fleet Subnet Root,
sibling Wasm Store and prepaid reuse inventory are infrastructure outside that
descendant count.

The first iteration intentionally serves uncertified, benign demo content.
Site URLs therefore use `https://<canister-id>.raw.icp.net/`. HTTP response
certification is a later feature step, before any page carries trusted data or
security-sensitive actions.

## Install shape

The reusable App topology is in `canic.toml`. Concrete IC placement and
funding are separate in `deployments/demos/playground-ic.toml`.
The protected Fleet input declares the generation-one Principal set under
`[admission]`. Keep that set explicit for the target Fleet; App source
configuration no longer owns user-ingress membership.

```bash
canic install playground playground-001 \
  --environment playground-ic \
  --fleet-input deployments/demos/playground-ic.toml \
  --profile release
```

Fleet installation activates an empty root-local Component inventory. After
that succeeds, create and activate the admitted `playground` Component through
the controller-owned top-level Component lifecycle. The installed hub exposes:

- `wenzelroll_can_create`: public query reporting whether another Wenzelroll
  is allowed.
- `wenzelroll_create`: whitelisted update creating exactly one Wenzelroll.
- `wenzelrolls`: public query returning every created Wenzelroll principal,
  creation time, and raw HTTP URL.

The sixth creation is rejected by the topology's five-worker cap.

The current operator CLI does not yet wrap the top-level Component lifecycle.
Until it does, the controller submits the root's typed `canic_command`
provision/removal variants with one fixed nonzero operation ID and observes
that exact operation through `canic_status`.

Browsers may block audible autoplay on a newly visited origin. Each child
attempts full-volume playback immediately and withholds the image until audio
is confirmed. If autoplay is denied, the child presents a mandatory start
button; its click starts playback and only then reveals the image. A controlled
demo browser can grant autoplay permission to the child origins for a fully
automatic entrance.

## Physical Canister reuse contract

This demo treats reuse as lifecycle behavior, not source-code copying:

1. Record the hub and up to five Wenzelroll principals.
2. Remove the complete `playground` Component through Canic's managed,
   post-order Component-removal lifecycle.
3. Verify that the removed Component Canisters appear as reset physical assets
   in the root's prepaid pool instead of being deleted. Recycled assets remain
   managed even when the inventory is above its proactive five-ready refill
   ceiling.
4. Recreate an admitted Component in the same Fleet and verify that creation
   claims suitable ready pool assets before paying for new Canisters.
5. For a different Fleet, keep its root on this same physical Subnet, drain
   the old root, hand each empty asset to the new root, and import it there
   through `RootCommand::ImportPoolCanister` on `canic_command`.

A physical Canister cannot move to another Subnet. Cross-Fleet reuse also does
not retain application state or Wasm: handoff/import deliberately uninstalls
and resets the asset before the new application claims it. An active Fleet is
bound to its App topology, so changing to a different App means creating a new
Fleet rather than relabelling the old one.
