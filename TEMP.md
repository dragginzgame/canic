The important implication from your inventory is that the canisters fall into three very different buckets:

Unique infrastructure: root and wasm_store. These are genuinely distinct and should stay distinct. root orchestrates the topology, while wasm_store is the dedicated chunked publication/install support canister.
Thin demo roles: app, minimal, scale, shard, and to a lesser extent user_hub, shard_hub, scale_hub. The README and topology config show that most of the differences here are policy/topology differences in canic.toml, not deeply unique code paths.
Coverage-only canisters: test and the six internal test stubs. These should not influence the public install story at all. The sample-canister docs already frame them as test/demo/example artifacts, not part of the consumer contract.

So the design I would recommend now is this:

Single design from this inventory

Keep only three reference archetypes as first-class demo canisters:

root
wasm_store
a small set of generic role shells:
singleton_shell
replica_shell
hub_shell

Then express the current demo topology mostly through canic.toml and role/pool configuration, not through a large set of almost-empty canister crates.

That fits the topology config already in place, where the behavioral distinctions among app, minimal, scale, shard, user_hub, scale_hub, and shard_hub are primarily encoded in role kind, pool membership, and policy tables.

What I would merge

I would merge these first:

app + minimal into one baseline shell. Their current workspace role is already “placeholder / minimal runtime baseline,” and both are built through the same reference workflow.
scale + shard into one generic worker shell, with behavior determined by config and parent hub policy rather than a separate crate. The current dfx.json shows them as structurally identical custom canisters with the same build path shape.
user_hub + shard_hub into one generic sharding hub shell with configurable pool name and semantics. canic.toml already distinguishes those mostly through pool configuration: user_shards vs shards.

I would not merge scale_hub into the sharding hub archetype yet, because the topology config shows it exercises scaling pools rather than sharding pools, so it is a distinct policy path even if the wrapper shape is similarly thin.

What I would not merge

Do not merge:

root
wasm_store
test

root and wasm_store are the two canisters whose existence expresses real architectural boundaries, not just demo topology. test is separate because it is a coverage harness, and collapsing it into a generic shell will blur test intent with product intent.

Why this helps install

Right now the repo README tells users to cargo add canic, add it in both dependencies and build-dependencies, and then use canic::build! / canic::build_root! with their own config file. That is already the correct public contract.

What still muddies the story is that the same repo also exposes a large reference dfx.json with custom build commands for all demo canisters. When the reference topology has eleven demo canisters plus test-only canisters, it is too easy for downstream users to infer that this is “the Canic way” rather than “the repo demo way.”

Reducing the reference topology to a few archetypes makes it much easier to say:

public lane: use canic, your own dfx.json, your own shells
reference lane: use the repo’s demo topology if you want a worked example

That is the install simplification win.

Why this helps wasm size

Not because deleting demo crates directly shrinks minimal. You already measured that the current floor is deeper than the wasm_store crate boundary. But it still helps in two indirect ways:

First, it stops you from preserving template/runtime/config coupling just because many demo crates happen to touch it. Fewer archetypes make it easier to prove which shared surfaces are truly required by canic-core.

Second, it lets you isolate wasm_store as a reference-only infrastructure canister instead of a topology-wide assumed participant. The reference docs and dfx.json still present wasm_store as part of the demo graph and shared builder flow. Trimming the graph around it makes that separation much clearer.

The concrete next move

I would implement the next slice like this:

Keep root and wasm_store as-is conceptually.

Replace these crates:

app
minimal
scale
shard
user_hub
shard_hub

with 3 reusable demo crates:

demo_singleton
demo_worker
demo_shard_hub
Keep scale_hub separate for now.
Move test and the internal test canisters under an explicitly test-only workspace grouping and out of any public-facing docs/install path.
Rewrite the crates/canisters/README.md and root README so they say plainly that these are reference/demo canisters, not the consumer install surface. That distinction is already partially documented; it just needs to become the dominant framing.
Bottom line

From the canister inventory, the best architectural move is:

preserve root and wasm_store as the unique infrastructure pair, but collapse the rest of the demo topology into a handful of generic reference shells whose behavior is driven by canic.toml, not by nearly-identical canister crates.

That is the cleanest path that improves all three goals at once:

cleaner wasm_store separation
easier install story
less structural pressure keeping demo/reference assumptions alive in the shared codebase

If you want, I can turn that into a concrete rename/delete/merge plan with the exact crates to remove first and the migration order that minimizes breakage.