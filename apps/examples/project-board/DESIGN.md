# Project board

**Planned.** Make cascading discovery and subtree operations visible. Give projects A and B distinct project canisters. Put two board canisters under A and one under B. Boards own their cards and retain their identity across upgrades. Only the configured parent may provision a board. Root executes approved management effects, and the fleet's Wasm Store holds immutable board releases.

1. Create the projects and boards with v1. Add a recognizable card to each. In the fleet view, expand A to see its two boards and show external controllers separately.
2. Publish board v2 once. Upgrade one A board and verify its ID and cards. Create another board using the still-pinned v1 release and inspect its installed version.
3. Select project A and inspect the proposed subtree upgrade before approving it. Upgrade the eligible boards to v2. Verify the B board still has its original ID, cards and v1 code.
4. Call the board provisioning endpoint directly from an unrelated identity. It must refuse the caller even if the caller knows the project's name.

The operator should be able to tell which boards will change before approval and which actually changed afterward. Logical parentage alone grants no IC controller authority.
