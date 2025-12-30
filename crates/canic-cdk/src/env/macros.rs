//! Internal macro to expose static `Principal` handles for known canisters.
//! Data lives in `.inc.rs` files and is shared with build.rs via include!().

macro_rules! static_canisters {
    ($($name:ident = $id:expr;)+) => {
        $(
            pub static $name: ::std::sync::LazyLock<::candid::Principal> =
                ::std::sync::LazyLock::new(|| {
                    ::candid::Principal::from_text($id)
                        .expect("principal literal validated by build.rs")
                });
        )+
    }
}

macro_rules! sns_table {
    (
        $(
            $name:ident {
                root: $root:expr,
                governance: $gov:expr,
                index: $idx:expr,
                ledger: $led:expr $(,)?
            }
        ),+ $(,)?
    ) => {
        ///
        /// SnsType
        ///
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub enum SnsType {
            $($name,)+
        }

        /// Alias to enable access like `SNS::OpenChat.ledger()`.
        pub type SNS = SnsType;

        static SNS_CANISTERS: OnceLock<HashMap<SnsType, SnsCanisters>> = OnceLock::new();

        fn init_sns_canisters() -> HashMap<SnsType, SnsCanisters> {
            let mut map = HashMap::new();
            $(
                map.insert(
                    SnsType::$name,
                    SnsCanisters {
                        root: parse_principal(SnsType::$name, "root", $root),
                        governance: parse_principal(SnsType::$name, "governance", $gov),
                        index: parse_principal(SnsType::$name, "index", $idx),
                        ledger: parse_principal(SnsType::$name, "ledger", $led),
                    },
                );
            )+
            map
        }

        impl SnsType {
            fn canisters(self) -> &'static SnsCanisters {
                SNS_CANISTERS
                    .get_or_init(init_sns_canisters)
                    .get(&self)
                    .expect("SNS canister table missing entry")
            }

            #[must_use]
            pub fn principal(self, role: SnsRole) -> Principal {
                let set = self.canisters();
                match role {
                    SnsRole::Root => set.root,
                    SnsRole::Governance => set.governance,
                    SnsRole::Index => set.index,
                    SnsRole::Ledger => set.ledger,
                }
            }

            #[must_use]
            pub fn root(self) -> Principal {
                self.canisters().root
            }

            #[must_use]
            pub fn governance(self) -> Principal {
                self.canisters().governance
            }

            #[must_use]
            pub fn index(self) -> Principal {
                self.canisters().index
            }

            #[must_use]
            pub fn ledger(self) -> Principal {
                self.canisters().ledger
            }
        }
    };
}
