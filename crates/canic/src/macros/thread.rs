/// Declare a thread-local static and schedule an eager initialization touch.
///
/// Expands to a `thread_local!` block and ensures the TLS slot is accessed
/// during the eager-init phase so subsequent calls observe a fully
/// initialized value. Use this for caches that must exist before canister
/// entry points run.
#[macro_export]
macro_rules! eager_static {
    // match: vis static NAME: TYPE = INIT;
    ($vis:vis static $name:ident : $ty:ty = $init:expr;) => {
        thread_local! {
            $vis static $name: $ty = $init;
        }

        $crate::eager_init!({
            $crate::eager::CANIC_EAGER_TLS.with(|v| {
                v.borrow_mut().push(|| {
                    $name.with(|_| {}); // force one touch
                });
            });
        });
    };
}
