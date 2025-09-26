#[macro_export]
macro_rules! icu_eager_static {
    // match: vis static NAME: TYPE = INIT;
    ($vis:vis static $name:ident : $ty:ty = $init:expr;) => {
        thread_local! {
            $vis static $name: $ty = $init;
        }

        $crate::eager_init!({
            $crate::eager::ICU_EAGER_TLS.with(|v| {
                v.borrow_mut().push(|| {
                    $name.with(|_| {}); // force one touch
                });
            });
        });
    };
}
