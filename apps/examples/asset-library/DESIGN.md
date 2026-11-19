# Asset library

**Planned.** Give Alice and Bob private libraries in one application canister. They each save the same bytes. The application authorizes catalog changes and owns the entries. Embedded Blob owns object identity, distinct references, grants and accounting. Both participants use the application's one Storage bootstrap.

1. Save identical content as Alice and Bob. Verify one underlying object with two independently owned references using authorized test observations.
2. Have Bob try to release Alice’s entry. Refuse the operation and verify both entries still resolve.
3. Have Alice release her own entry. Bob can still read his copy. Replaying Alice’s earlier operation must not resurrect her released reference.
4. Reject a conflicting catalog entry before mutating Blob. Exercise a failure after mutation and verify the catalog and Blob changes roll back together.

Use the [installed asset-catalog witness](../../../docs/demos/blob.md) as the starting point. Keep the exercise local and retain Blob’s finite replay history and declared-byte accounting assumptions.
