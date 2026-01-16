use crate::{
    cdk::{
        candid::CandidType,
        types::{BoundedString64, Principal},
    },
    dto::error::Error,
    workflow::ic::call::{
        CallBuilder as WorkflowCallBuilder, CallResult as WorkflowCallResult, CallWorkflow,
        IntentSpec as WorkflowIntentSpec,
    },
};
use candid::utils::{ArgumentDecoder, ArgumentEncoder};
use serde::de::DeserializeOwned;

///
/// Call
///
/// Public IC call façade.
///

pub struct Call;

impl Call {
    #[must_use]
    pub fn bounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallWorkflow::bounded_wait(canister_id, method),
        }
    }

    #[must_use]
    pub fn unbounded_wait(canister_id: impl Into<Principal>, method: &str) -> CallBuilder {
        CallBuilder {
            inner: CallWorkflow::unbounded_wait(canister_id, method),
        }
    }
}

///
/// IntentKey
///

pub struct IntentKey(String);

impl IntentKey {
    pub fn try_new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        let bounded = BoundedString64::try_new(value).map_err(Error::invalid)?;

        Ok(Self(bounded.0))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for IntentKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl From<IntentKey> for String {
    fn from(key: IntentKey) -> Self {
        key.0
    }
}

///
/// IntentReservation
///

pub struct IntentReservation {
    key: IntentKey,
    quantity: u64,
    ttl_secs: Option<u64>,
    max_in_flight: Option<u64>,
}

impl IntentReservation {
    #[must_use]
    pub const fn new(key: IntentKey, quantity: u64) -> Self {
        Self {
            key,
            quantity,
            ttl_secs: None,
            max_in_flight: None,
        }
    }

    #[must_use]
    pub const fn with_ttl_secs(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = Some(ttl_secs);
        self
    }

    #[must_use]
    pub const fn with_max_in_flight(mut self, max_in_flight: u64) -> Self {
        self.max_in_flight = Some(max_in_flight);
        self
    }

    pub(crate) fn into_spec(self) -> WorkflowIntentSpec {
        WorkflowIntentSpec::new(
            self.key.into_string(),
            self.quantity,
            self.ttl_secs,
            self.max_in_flight,
        )
    }
}

///
/// CallBuilder (api)
///

pub struct CallBuilder {
    inner: WorkflowCallBuilder,
}

impl CallBuilder {
    // ---------- arguments ----------

    #[must_use]
    pub fn with_arg<A>(self, arg: A) -> Self
    where
        A: CandidType,
    {
        Self {
            inner: self.inner.with_arg(arg),
        }
    }

    #[must_use]
    pub fn with_args<A>(self, args: A) -> Self
    where
        A: ArgumentEncoder,
    {
        Self {
            inner: self.inner.with_args(args),
        }
    }

    pub fn try_with_arg<A>(self, arg: A) -> Result<Self, Error>
    where
        A: CandidType,
    {
        Ok(Self {
            inner: self.inner.try_with_arg(arg).map_err(Error::from)?,
        })
    }

    pub fn try_with_args<A>(self, args: A) -> Result<Self, Error>
    where
        A: ArgumentEncoder,
    {
        Ok(Self {
            inner: self.inner.try_with_args(args).map_err(Error::from)?,
        })
    }

    // ---------- cycles ----------

    #[must_use]
    pub fn with_cycles(self, cycles: u128) -> Self {
        Self {
            inner: self.inner.with_cycles(cycles),
        }
    }

    // ---------- intent ----------

    #[must_use]
    pub fn with_intent(self, intent: IntentReservation) -> Self {
        Self {
            inner: self.inner.with_intent(intent.into_spec()),
        }
    }

    // ---------- execution ----------

    pub async fn execute(self) -> Result<CallResult, Error> {
        Ok(CallResult {
            inner: self.inner.execute().await.map_err(Error::from)?,
        })
    }
}

///
/// CallResult (api)
///
/// Public, stable result wrapper.
///

pub struct CallResult {
    inner: WorkflowCallResult,
}

impl CallResult {
    pub fn candid<R>(&self) -> Result<R, Error>
    where
        R: CandidType + DeserializeOwned,
    {
        self.inner.candid().map_err(Error::from)
    }

    pub fn candid_tuple<R>(&self) -> Result<R, Error>
    where
        R: for<'de> ArgumentDecoder<'de>,
    {
        self.inner.candid_tuple().map_err(Error::from)
    }
}
