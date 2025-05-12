pub mod endpoints;
pub mod state;

/// icu_start
#[macro_export]
macro_rules! icu_start {
    ($canister_path:expr) => {
        #[::icu::ic::init]
        fn init(root_id: Option<Principal>, parent_id: Option<Principal>) {
            use ::icu::interface::state::core::canister_state;

            canister_state::set_root_id(root_id).unwrap();
            canister_state::set_parent_id(parent_id).unwrap();

            log!(Log::Info, "init: {}", $canister_type);

            // type
            canister_state::set_type($canister_type).unwrap();

            _init();

            icu_endpoints!();
        };
    };
}

/// icu_start_root
#[macro_export]
macro_rules! icu_start_root {
    ($canister_path:expr) => {
        #[::icu::ic::init]
        fn init() {
            use ::icu::interface::state::core::canister_state;

            canister_state::set_root_id(canister_self()).unwrap();

            log!(Log::Info, "init: {} (root)",);

            // type
            canister_state::set_type($canister_type).unwrap();

            _init();

            icu_endpoints_root!();
            icu_endpoints!();
        };
    };
}

// log
#[macro_export]
macro_rules! log {
    // Match when only the format string is provided (no additional args)
    ($level:expr, $fmt:expr) => {{
        // Pass an empty set of arguments to @inner
        log!(@inner $level, $fmt,);
    }};

    // Match when additional arguments are provided
    ($level:expr, $fmt:expr, $($arg:tt)*) => {{
        log!(@inner $level, $fmt, $($arg)*);
    }};

    // Inner macro for actual logging logic to avoid code duplication
    (@inner $level:expr, $fmt:expr, $($arg:tt)*) => {{
        let formatted_message = format!($fmt, $($arg)*);  // Apply formatting with args

        let msg = match $level {
            $crate::Log::Ok => format!("\x1b[32mOK\x1b[0m: {}", formatted_message),
            $crate::Log::Perf => format!("\x1b[35mPERF\x1b[0m: {}", formatted_message),
            $crate::Log::Info => format!("\x1b[34mINFO\x1b[0m: {}", formatted_message),
            $crate::Log::Warn => format!("\x1b[33mWARN\x1b[0m: {}", formatted_message),
            $crate::Log::Error => format!("\x1b[31mERROR\x1b[0m: {}", formatted_message),
        };

        $crate::ic::println!("{}", msg);
    }};
}

// impl_storable_bounded
#[macro_export]
macro_rules! impl_storable_bounded {
    ($ident:ident, $max_size:expr, $is_fixed_size:expr) => {
        impl $crate::ic::structures::storable::Storable for $ident {
            fn to_bytes(&self) -> ::std::borrow::Cow<[u8]> {
                ::std::borrow::Cow::Owned(::icu::serialize::serialize(self).unwrap())
            }

            fn from_bytes(bytes: ::std::borrow::Cow<[u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap()
            }

            const BOUND: $crate::ic::structures::storable::Bound =
                $crate::ic::structures::storable::Bound::Bounded {
                    max_size: $max_size,
                    is_fixed_size: $is_fixed_size,
                };
        }
    };
}

// impl_storable_unbounded
#[macro_export]
macro_rules! impl_storable_unbounded {
    ($ident:ident) => {
        impl $crate::ic::structures::storable::Storable for $ident {
            fn to_bytes(&self) -> ::std::borrow::Cow<[u8]> {
                ::std::borrow::Cow::Owned($crate::serialize::serialize(self).unwrap())
            }

            fn from_bytes(bytes: ::std::borrow::Cow<[u8]>) -> Self {
                $crate::serialize::deserialize(&bytes).unwrap()
            }

            const BOUND: $crate::ic::structures::storable::Bound =
                $crate::ic::structures::storable::Bound::Unbounded;
        }
    };
}

// perf
#[macro_export]
macro_rules! perf {
    () => {
        ::icu::export::defer::defer!($crate::log!(
            ::icu::Log::Perf,
            "api call used {} instructions ({})",
            ::icu::ic::api::performance_counter(1),
            module_path!()
        ));
    };
}
