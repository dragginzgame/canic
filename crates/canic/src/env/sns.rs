use crate::{Error, env::EnvError};
use candid::Principal;
use thiserror::Error as ThisError;

///
/// SnsError
///

#[derive(Debug, ThisError)]
pub enum SnsError {
    #[error("invalid principal: {0} ({0})")]
    InvalidPrincipal(String, String),
}

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

#[derive(Clone, Copy, Debug)]
#[remain::sorted]
pub enum SnsType {
    Alice,
    Catalyze,
    CecilTheLion,
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
    Origyn,
    PokedBots,
    Sneed,
    Swampies,
    TacoDao,
    Trax,
}

// ---- Helpers ----

fn parse_required(name: &str, text: &str) -> Result<Principal, SnsError> {
    Principal::from_text(text)
        .map_err(|_| SnsError::InvalidPrincipal(name.to_string(), text.to_string()))
}

fn bundle(root: &str, gov: &str, idx: &str, led: &str) -> Result<SnsCanisters, SnsError> {
    Ok(SnsCanisters {
        root: parse_required("root", root)?,
        governance: parse_required("governance", gov)?,
        index: parse_required("index", idx)?,
        ledger: parse_required("ledger", led)?,
    })
}

// ---- Table + impl (DRY via macro) ----

macro_rules! define_sns_table {
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
        impl SnsType {
            pub fn principal(self, role: SnsRole) -> Result<Principal, Error> {
                let set = self.principals()?;
                Ok(match role {
                    SnsRole::Root       => set.root,
                    SnsRole::Governance => set.governance,
                    SnsRole::Index      => set.index,
                    SnsRole::Ledger     => set.ledger,
                })
            }

            pub fn principals(self) -> Result<SnsCanisters, Error> {
                match self {
                    $(
                        Self::$name => bundle($root, $gov, $idx, $led),
                    )+
                }
                .map_err(EnvError::from)
                .map_err(Error::from)
            }
        }

        // Optional: test all non-empty entries parse (runs in `cargo test`)
        #[cfg(test)]
        mod __sns_parse_tests {
            use super::*;
            #[test]
            fn all_configured_ids_parse() {
                $(
                    // If any of these are non-empty, principals() must succeed
                    if !($root.is_empty() || $gov.is_empty() || $idx.is_empty() || $led.is_empty()) {
                        let _ = SnsType::$name.principals().expect(concat!("failed for ", stringify!($name)));
                    }
                )+
            }
        }
    }
}

// ---- Fill the table once (short & readable) ----

define_sns_table! {

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

    CecilTheLion {
        root:       "ju4gz-6iaaa-aaaaq-aaeva-cai",
        governance: "jt5an-tqaaa-aaaaq-aaevq-cai",
        index:      "jiy4i-jiaaa-aaaaq-aaexa-cai",
        ledger:     "jg2ra-syaaa-aaaaq-aaewa-cai",
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

    Origyn      {
        root:       "leu43-oiaaa-aaaaq-aadgq-cai",
        governance: "lnxxh-yaaaa-aaaaq-aadha-cai",
        index:      "jqkzp-liaaa-aaaaq-aadiq-cai",
        ledger:     "lkwrt-vyaaa-aaaaq-aadhq-cai",
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

    Trax {
        root:       "ecu3s-hiaaa-aaaaq-aacaq-cai",
        governance: "elxqo-raaaa-aaaaq-aacba-cai",
        index:      "e6qbd-qiaaa-aaaaq-aaccq-cai",
        ledger:     "emww2-4yaaa-aaaaq-aacbq-cai",
    },
}
