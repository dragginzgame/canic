/// endpoints_init
#[macro_export]
macro_rules! endpoints_init {
    ($canister_type:expr) => {
        // init
        #[::mimic::ic::init]
        fn init(root_id: Option<Principal>, parent_id: Option<Principal>) {
            use ::actor::interface::state::core::canister_state;

            match (root_id, parent_id) => {
                (Some(root_id), Some(parent_id)) => {
                    canister_state::set_root_id(root_id).unwrap();
                    canister_state::set_parent_id(parent_id).unwrap();

                    log!(Log::Info, "init: {}", $canister_type);
                },
                (None, None) => {
                    canister_state::set_root_id(canister_self()).unwrap();

                    log!(Log::Info, "init: {} (root)", $canister_type);
                },
                _ => panic!("invalid root_id/parent_id"),
            }

            // type
            canister_state::set_type($canister_type).unwrap();

            mimic_init();
            _init()
        }

        // init_async
        #[::mimic::ic::update]
        async fn init_async() {
            _init_async().await
        }
    };
}
