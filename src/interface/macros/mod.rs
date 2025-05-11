pub mod init;
pub mod root;

#[macro_export]
macro_rules! endpoints {
    // root
    ("root") => {
        actor::endpoints_init_root!();
        actor::endpoints_root!();
        actor::endpoints_shared!();
    };

    // system
    ("user") => {
        actor::endpoints_init!(CanisterType::User);
        actor::endpoints_shared!();
    };

    // game
    ("config") => {
        actor::endpoints_init!(CanisterType::Config);
        actor::endpoints_shared!();
    };
    ("game") => {
        actor::endpoints_init!(CanisterType::Game);
        actor::endpoints_shared!();
    };
    ("world") => {
        actor::endpoints_init!(CanisterType::World);
        actor::endpoints_shared!();
    };
    ("world_builder") => {
        actor::endpoints_init!(CanisterType::WorldBuilder);
        actor::endpoints_shared!();
    };

    // test
    ("test") => {
        actor::endpoints_init!(CanisterType::Test);
        actor::endpoints_shared!();
    };

    () => {
        compile_error!(concat!("Unknown canister type: ", $other));
    };
}
