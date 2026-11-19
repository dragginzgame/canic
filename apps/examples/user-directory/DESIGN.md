# User directory

A returning account must reach its original private row even after a lost signup reply. The hub owns the user's ID and route, Root assigns the verified account key to a bounded pool, and each shard accepts provisioning only from its configured parent. Each receiver checks its own Auth session.

The [runnable fleet](README.md) places two accounts on separate shards and refuses a third with `PoolFull`. Each shard admits one key. Root may create at most two shards. Retry the first signup and verify the same user ID and shard. Signing in to the hub alone must leave shard access closed.

Extend the walkthrough with independent revocation. Under one Root, enroll the hub and both shards, then establish sessions on each. Revoke one shard and refresh it through its authorized controller path. Check that both its old session and a new admission fail, while the hub and sibling shard still serve their authorized users. An unrelated caller must be unable to trigger the protected refresh.

Use fixture credentials for this local demonstration. Toko owns its Internet Identity sign-in and account policy.
