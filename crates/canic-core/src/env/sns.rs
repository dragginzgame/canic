//! Preconfigured SNS deployments and helpers for looking up their canisters.

use candid::Principal;
use std::{collections::HashMap, sync::OnceLock};

// -----------------------------------------------------------------------------
// Static storage (std only)
// -----------------------------------------------------------------------------

static SNS_CANISTERS: OnceLock<HashMap<SnsType, SnsCanisters>> = OnceLock::new();

///
/// SnsCanisters
///

#[derive(Clone, Debug)]
pub struct SnsCanisters {
    pub root: Principal,
    pub governance: Principal,
    pub index: Principal,
    pub ledger: Principal,
}

///
/// SnsRole
///

#[derive(Clone, Copy, Debug)]
pub enum SnsRole {
    Root,
    Governance,
    Index,
    Ledger,
}

///
/// SnsType
///

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[remain::sorted]
pub enum SnsType {
    Alice,
    Catalyze,
    DecideAi,
    Dragginz,
    GoldDao,
    Kinic,
    KongSwap,
    Mimic,
    Motoko,
    Neutrinite,
    Nuance,
    OpenChat,
    PokedBots,
    Sneed,
    Swampies,
    TacoDao,
    Tendies,
    Trax,
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
}

// -----------------------------------------------------------------------------
// Initialization
// -----------------------------------------------------------------------------

macro_rules! define_sns_table {
    (
        $map:ident,
        $(
            $name:ident {
                root: $root:expr,
                governance: $gov:expr,
                index: $idx:expr,
                ledger: $led:expr $(,)?
            }
        ),+ $(,)?
    ) => {
        $(
            $map.insert(
                SnsType::$name,
                SnsCanisters {
                    root: parse!($name, "root", $root),
                    governance: parse!($name, "governance", $gov),
                    index: parse!($name, "index", $idx),
                    ledger: parse!($name, "ledger", $led),
                },
            );
        )+
    };
}

#[allow(clippy::too_many_lines)]
fn init_sns_canisters() -> HashMap<SnsType, SnsCanisters> {
    let mut map = HashMap::new();

    macro_rules! parse {
        ($sns:ident, $role:expr, $text:expr) => {
            Principal::from_text($text).unwrap_or_else(|_| {
                panic!(
                    "Invalid SNS {} {} principal: {}",
                    stringify!($sns),
                    $role,
                    $text
                )
            })
        };
    }

    define_sns_table! {
        map,

        Alice {
            root:       "oh4fn-kyaaa-aaaaq-aaega-cai",
            governance: "oa5dz-haaaa-aaaaq-aaegq-cai",
            index:      "mtcaz-pyaaa-aaaaq-aaeia-cai",
            ledger:     "oj6if-riaaa-aaaaq-aaeha-cai",
        },

        Catalyze {
            root:       "uly3p-iqaaa-aaaaq-aabma-cai",
            governance: "umz53-fiaaa-aaaaq-aabmq-cai",
            index:      "ux4b6-7qaaa-aaaaq-aaboa-cai",
            ledger:     "uf2wh-taaaa-aaaaq-aabna-cai",
        },

        DecideAi {
            root:       "x4kx5-ziaaa-aaaaq-aabeq-cai",
            governance: "xvj4b-paaaa-aaaaq-aabfa-cai",
            index:      "xaonm-oiaaa-aaaaq-aabgq-cai",
            ledger:     "xsi2v-cyaaa-aaaaq-aabfq-cai",
        },

        Dragginz {
            root:       "zxeu2-7aaaa-aaaaq-aaafa-cai",
            governance: "zqfso-syaaa-aaaaq-aaafq-cai",
            index:      "zlaol-iaaaa-aaaaq-aaaha-cai",
            ledger:     "zfcdd-tqaaa-aaaaq-aaaga-cai",
        },

        GoldDao {
            root:       "tw2vt-hqaaa-aaaaq-aab6a-cai",
            governance: "tr3th-kiaaa-aaaaq-aab6q-cai",
            index:      "efv5g-kqaaa-aaaaq-aacaa-cai",
            ledger:     "tyyy3-4aaaa-aaaaq-aab7a-cai",
        },

        Kinic {
            root:       "7jkta-eyaaa-aaaaq-aaarq-cai",
            governance: "74ncn-fqaaa-aaaaq-aaasa-cai",
            index:      "7vojr-tyaaa-aaaaq-aaatq-cai",
            ledger:     "73mez-iiaaa-aaaaq-aaasq-cai",
        },

        KongSwap {
            root:       "ormnc-tiaaa-aaaaq-aadyq-cai",
            governance: "oypg6-faaaa-aaaaq-aadza-cai",
            index:      "onixt-eiaaa-aaaaq-aad2q-cai",
            ledger:     "o7oak-iyaaa-aaaaq-aadzq-cai",
        },

        Mimic {
            root:       "4m6il-zqaaa-aaaaq-aaa2a-cai",
            governance: "4l7o7-uiaaa-aaaaq-aaa2q-cai",
            index:      "ks7eq-3yaaa-aaaaq-aaddq-cai",
            ledger:     "4c4fd-caaaa-aaaaq-aaa3a-cai",
        },

        Motoko {
            root:       "ko36b-myaaa-aaaaq-aadbq-cai",
            governance: "k34pm-nqaaa-aaaaq-aadca-cai",
            index:      "5ithz-aqaaa-aaaaq-aaa4a-cai",
            ledger:     "k45jy-aiaaa-aaaaq-aadcq-cai",
        },

        Neutrinite {
            root:       "extk7-gaaaa-aaaaq-aacda-cai",
            governance: "eqsml-lyaaa-aaaaq-aacdq-cai",
            index:      "ft6fn-7aaaa-aaaaq-aacfa-cai",
            ledger:     "f54if-eqaaa-aaaaq-aacea-cai",
        },

        Nuance {
            root:       "rzbmc-yiaaa-aaaaq-aabsq-cai",
            governance: "rqch6-oaaaa-aaaaq-aabta-cai",
            index:      "q5mdq-biaaa-aaaaq-aabuq-cai",
            ledger:     "rxdbk-dyaaa-aaaaq-aabtq-cai",
        },

        OpenChat {
            root:       "3e3x2-xyaaa-aaaaq-aaalq-cai",
            governance: "2jvtu-yqaaa-aaaaq-aaama-cai",
            index:      "2awyi-oyaaa-aaaaq-aaanq-cai",
            ledger:     "2ouva-viaaa-aaaaq-aaamq-cai",
        },

        PokedBots {
            root:       "nb7he-piaaa-aaaaq-aadqq-cai",
            governance: "ni4my-zaaaa-aaaaq-aadra-cai",
            index:      "n535v-yiaaa-aaaaq-aadsq-cai",
            ledger:     "np5km-uyaaa-aaaaq-aadrq-cai",
        },

        Sneed {
            root:       "fp274-iaaaa-aaaaq-aacha-cai",
            governance: "fi3zi-fyaaa-aaaaq-aachq-cai",
            index:      "h3e2i-naaaa-aaaaq-aacja-cai",
            ledger:     "hvgxa-wqaaa-aaaaq-aacia-cai",
        },

        Swampies {
            root:       "l7ra6-uqaaa-aaaaq-aadea-cai",
            governance: "lyqgk-ziaaa-aaaaq-aadeq-cai",
            index:      "ldv2p-dqaaa-aaaaq-aadga-cai",
            ledger:     "lrtnw-paaaa-aaaaq-aadfa-cai",
        },

        TacoDao {
            root:       "lacdn-3iaaa-aaaaq-aae3a-cai",
            governance: "lhdfz-wqaaa-aaaaq-aae3q-cai",
            index:      "kepm7-ciaaa-aaaaq-aae5a-cai",
            ledger:     "kknbx-zyaaa-aaaaq-aae4a-cai",
        },

        Tendies {
            root:       "kwj3g-oyaaa-aaaaq-aae6a-cai",
            governance: "kri5s-daaaa-aaaaq-aae6q-cai",
            index:      "bb4ce-dyaaa-aaaaq-aafaa-cai",
            ledger:     "kylwo-viaaa-aaaaq-aae7a-cai",
        },

        Trax {
            root:       "ecu3s-hiaaa-aaaaq-aacaq-cai",
            governance: "elxqo-raaaa-aaaaq-aacba-cai",
            index:      "e6qbd-qiaaa-aaaaq-aaccq-cai",
            ledger:     "emww2-4yaaa-aaaaq-aacbq-cai",
        },
    }

    map
}

///
/// TESTS
///

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sns_table_initializes_and_validates() {
        // This will panic if ANY principal literal is invalid
        let _ = SNS_CANISTERS.get_or_init(init_sns_canisters);
    }
}
