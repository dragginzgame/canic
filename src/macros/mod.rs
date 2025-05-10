pub mod endpoints;

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

// memory_manager
#[macro_export]
macro_rules! memory_manager {
    () => {
        thread_local! {

            ///
            /// Define MEMORY_MANAGER thread-locally for the entire scope
            ///
            pub static MEMORY_MANAGER: ::std::cell::RefCell<
                $crate::ic::structures::memory::MemoryManager<
                    $crate::ic::structures::DefaultMemoryImpl,
                >,
            > = ::std::cell::RefCell::new($crate::ic::structures::memory_manager::MemoryManager::init(
                $crate::ic::structures::DefaultMemoryImpl::default(),
            ));

        }
    };
}

// perf
#[macro_export]
macro_rules! perf {
    () => {
        $crate::export::defer::defer!($crate::log!(
            $crate::Log::Perf,
            "api call used {} instructions ({})",
            $crate::ic::api::performance_counter(1),
            module_path!()
        ));
    };
}

#[macro_export]
macro_rules! icu_endpoints {
    () => {
        // canister_upgrade_children
        // canister_id : None means upgrade all children
        /*
                #[$crate::ic::update(guard = "guard_update")]
                async fn canister_upgrade_children(
                    canister_id: Option<Principal>,
                ) -> Result<(), ActorError> {
                    allow_any(vec![Auth::Controller]).await?;

                    // send a request for each matching canister
                    for (child_id, path) in child_index() {
                        if canister_id.is_none() || canister_id == Some(child_id) {
                            let req = ::actor::interface::request::Request::new_canister_upgrade(
                                child_id,
                                path.clone(),
                            );

                            if let Err(e) = ::actor::interface::request::request_api(req).await {
                                log!(Log::Warn, "{child_id} ({path}): {e}");
                            }
                        }
                    }

                    Ok(())
                }

                // app_state_cascade
                #[$crate::ic::update]
                async fn app_state_cascade(data: ::actor::state::core::AppStateData) -> Result<(), String> {
                    allow_any(vec![Auth::Parent]).await?;

                    // set state and cascade
                    ::actor::interface::state::core::app_state::set_data_api(data)?;
                    ::actor::interface::cascade::app_state_cascade_api().await?;

                    Ok(())
                }

                // subnet_index_cascade
                #[$crate::ic::update]
                async fn subnet_index_cascade(
                    data: ::actor::state::core::SubnetIndexData,
                ) -> Result<(), String> {
                    allow_any(vec![Auth::Parent]).await?;

                    // set index and cascade
                    ::actor::interface::state::core::subnet_index::set_data(data);
                    ::actor::interface::cascade::subnet_index_cascade_api().await?;

                    Ok(())
                }
        */

        //
        // IC API ENDPOINTS
        // these are specific endpoints defined by the IC spec
        //

        // ic_cycles_accept
        //    #[$crate::ic::update]
        //    fn ic_cycles_accept(max_amount: u128) -> u128 {
        //        $crate::ic::api::msg_cycles_accept(max_amount)
        //    }

        //
        // ICU STATE ENDPOINTS
        //

        // app_state
        #[$crate::ic::query]
        fn app_state() -> $crate::state::AppStateData {
            $crate::state::APP_STATE.with_borrow(|this| this.get_data())
        }

        // canister_state
        #[$crate::ic::query]
        fn canister_state() -> $crate::state::CanisterStateData {
            $crate::state::CANISTER_STATE.with_borrow(|this| this.get_data())
        }

        // child_index
        #[$crate::ic::query]
        fn child_index() -> $crate::state::ChildIndexData {
            $crate::state::CHILD_INDEX.with_borrow(|this| this.get_data())
        }

        // subnet_index
        #[$crate::ic::query]
        fn subnet_index() -> $crate::state::SubnetIndexData {
            $crate::state::SUBNET_INDEX.with_borrow(|this| this.get_data())
        }
    };
}
