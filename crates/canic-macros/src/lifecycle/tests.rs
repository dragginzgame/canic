use super::*;

#[test]
fn lifecycle_argument_policy_accepts_both_hooks_and_rejects_conflicting_shapes() {
    for kind in [quote!(init), quote!(post_upgrade)] {
        assert!(
            expand(
                quote!(#kind, decode = LIMITS),
                quote!(
                    fn hook() {}
                )
            )
            .is_ok()
        );
        assert!(
            expand(
                quote!(#kind),
                quote!(
                    fn hook() {}
                )
            )
            .is_ok()
        );
        assert!(
            expand(
                quote!(#kind, decode = FIRST, decode = SECOND),
                quote!(
                    fn hook() {}
                ),
            )
            .is_err()
        );
        assert!(
            expand(
                quote!(#kind, decode = LIMITS),
                quote!(
                    async fn hook() {}
                )
            )
            .is_err()
        );
        assert!(
            expand(
                quote!(#kind, decode = LIMITS),
                quote!(
                    fn hook() -> u64 {
                        0
                    }
                )
            )
            .is_err()
        );
    }
    assert!(
        expand(
            quote!(update, decode = LIMITS),
            quote!(
                fn hook() {}
            )
        )
        .is_err()
    );
}
