use crate::{
    ops::{ic::IcOps, runtime::log::LogOps},
    storage::stable::env::Env,
};
use candid::CandidType;
use serde::{Deserialize, Serialize};
use std::cell::Cell;

///
/// Debug
///

#[derive(
    Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, CandidType, Deserialize, Serialize,
)]
pub enum Level {
    Debug,
    Info,
    Ok,
    Warn,
    Error,
}

impl Level {
    #[must_use]
    pub const fn ansi_label(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "\x1b[34mINFO \x1b[0m",
            Self::Ok => "\x1b[32m OK  \x1b[0m",
            Self::Warn => "\x1b[33mWARN \x1b[0m",
            Self::Error => "\x1b[31mERROR\x1b[0m",
        }
    }
}

///
/// Topic
///

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[remain::sorted]
pub enum Topic {
    App,
    Auth,
    CanisterLifecycle,
    CanisterPool,
    Config,
    Cycles,
    Icrc,
    Init,
    Memory,
    Perf,
    Rpc,
    Sharding,
    Sync,
    Topology,
    Wasm,
}

impl Topic {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::App => "App",
            Self::Auth => "Auth",
            Self::CanisterLifecycle => "CanisterLifecycle",
            Self::CanisterPool => "CanisterPool",
            Self::Config => "Config",
            Self::Cycles => "Cycles",
            Self::Icrc => "Icrc",
            Self::Init => "Init",
            Self::Memory => "Memory",
            Self::Perf => "Perf",
            Self::Rpc => "Rpc",
            Self::Sharding => "Sharding",
            Self::Sync => "Sync",
            Self::Topology => "Topology",
            Self::Wasm => "Wasm",
        }
    }

    #[must_use]
    pub const fn log_label(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Auth => "auth",
            Self::CanisterLifecycle => "canister_lifecycle",
            Self::CanisterPool => "canister_pool",
            Self::Config => "config",
            Self::Cycles => "cycles",
            Self::Icrc => "icrc",
            Self::Init => "init",
            Self::Memory => "memory",
            Self::Perf => "perf",
            Self::Rpc => "rpc",
            Self::Sharding => "sharding",
            Self::Sync => "sync",
            Self::Topology => "topology",
            Self::Wasm => "wasm",
        }
    }
}

thread_local! {
    static LOG_READY: Cell<bool> = const { Cell::new(false) };
}

pub fn set_ready() {
    LOG_READY.with(|ready| ready.set(true));
}

#[must_use]
pub fn is_ready() -> bool {
    LOG_READY.with(Cell::get)
}

#[macro_export]
macro_rules! log {
    ($topic:expr, $level:ident, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        $crate::log!(@inner Some($topic), $crate::log::Level::$level, $fmt $(, $arg)*);
    }};

    ($level:ident, $fmt:expr $(, $arg:expr)* $(,)?) => {{
        $crate::log!(@inner None::<$crate::log::Topic>, $crate::log::Level::$level, $fmt $(, $arg)*);
    }};

    (@inner $topic:expr, $level:expr, $fmt:expr $(, $arg:expr)*) => {{
        if $crate::log::is_ready() {
            let level = $level;
            let topic_opt: Option<$crate::log::Topic> = $topic;
            let message = format!($fmt $(, $arg)*);
            $crate::log::__emit_runtime_log(env!("CARGO_PKG_NAME"), topic_opt, level, &message);
        }
    }};
}

///
/// Helpers
/// (should remain public)
///

pub fn __append_runtime_log(crate_name: &str, topic: Option<Topic>, level: Level, message: &str) {
    let created_at = IcOps::now_secs();

    if let Err(err) = LogOps::append_runtime_log(crate_name, topic, level, message, created_at) {
        #[cfg(debug_assertions)]
        crate::cdk::println!("log append failed: {err}");

        #[cfg(not(debug_assertions))]
        let _ = err;
    }
}

#[doc(hidden)]
pub fn __emit_runtime_log(crate_name: &str, topic: Option<Topic>, level: Level, message: &str) {
    __append_runtime_log(crate_name, topic, level, message);

    let line = __render_runtime_log_line(topic, level, message);
    crate::cdk::println!("{line}");
}

#[doc(hidden)]
#[must_use]
pub fn __render_runtime_log_line(topic: Option<Topic>, level: Level, message: &str) -> String {
    let role = __canister_role_label();
    let topic_prefix = topic.map_or_else(String::new, |topic| format!("[{}] ", topic.as_str()));

    format!(
        "{}|{:^12}| {}{}",
        level.ansi_label(),
        role,
        topic_prefix,
        message
    )
}

#[doc(hidden)]
#[must_use]
pub fn __canister_role_label() -> String {
    Env::get_canister_role().map_or_else(
        || "...".to_string(),
        |role| crate::format::truncate(role.as_str(), 12),
    )
}
