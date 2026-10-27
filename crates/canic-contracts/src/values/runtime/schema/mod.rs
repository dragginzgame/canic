//! Verified native metadata for renamed unit enums; Serde and storage stay unchanged.
use super::*;
use ic_wire::typescript::{GenerationError, SchemaContext, Wire};
use serde_json::{Value, json};

impl Wire for FailureSeverity {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"info","id":0,"value":{"k":"unit"}},
        {"name":"warning","id":1,"value":{"k":"unit"}},
        {"name":"error","id":2,"value":{"k":"unit"}},
        {"name":"critical","id":3,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for RuntimeFieldVisibility {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"public_safe","id":0,"value":{"k":"unit"}},
        {"name":"operator_only","id":1,"value":{"k":"unit"}},
        {"name":"controller_only","id":2,"value":{"k":"unit"}},
        {"name":"feature_gated","id":3,"value":{"k":"unit"}},
        {"name":"disabled","id":4,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for RuntimeCheckStatus {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"pass","id":0,"value":{"k":"unit"}},
        {"name":"warn","id":1,"value":{"k":"unit"}},
        {"name":"fail","id":2,"value":{"k":"unit"}},
        {"name":"not_evaluated","id":3,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for RuntimeDiagnosticSeverity {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"info","id":0,"value":{"k":"unit"}},
        {"name":"warning","id":1,"value":{"k":"unit"}},
        {"name":"blocked","id":2,"value":{"k":"unit"}},
        {"name":"unsupported","id":3,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for RuntimeStateDomainStatus {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"ok","id":0,"value":{"k":"unit"}},
        {"name":"warning","id":1,"value":{"k":"unit"}},
        {"name":"failing","id":2,"value":{"k":"unit"}},
        {"name":"not_evaluated","id":3,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for HealthStatus {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"healthy","id":0,"value":{"k":"unit"}},
        {"name":"degraded","id":1,"value":{"k":"unit"}},
        {"name":"unhealthy","id":2,"value":{"k":"unit"}},
        {"name":"unknown","id":3,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for ReadinessStatus {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"ready","id":0,"value":{"k":"unit"}},
        {"name":"degraded","id":1,"value":{"k":"unit"}},
        {"name":"not_ready","id":2,"value":{"k":"unit"}},
        {"name":"not_evaluated","id":3,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for RuntimeStatus {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"ok","id":0,"value":{"k":"unit"}},
        {"name":"degraded","id":1,"value":{"k":"unit"}},
        {"name":"failing","id":2,"value":{"k":"unit"}},
        {"name":"unknown","id":3,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for TimerRegistrationStatus {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"unregistered","id":0,"value":{"k":"unit"}},
        {"name":"scheduled","id":1,"value":{"k":"unit"}},
        {"name":"running","id":2,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for TimerProcessCondition {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"disabled","id":0,"value":{"k":"unit"}},
        {"name":"idle","id":1,"value":{"k":"unit"}},
        {"name":"active","id":2,"value":{"k":"unit"}},
        {"name":"retrying","id":3,"value":{"k":"unit"}},
        {"name":"failed","id":4,"value":{"k":"unit"}},
        {"name":"missing_registration","id":5,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for TimerSchedulingMode {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"once","id":0,"value":{"k":"unit"}},
        {"name":"after_completion","id":1,"value":{"k":"unit"}},
        {"name":"deadline","id":2,"value":{"k":"unit"}},
        {"name":"retry","id":3,"value":{"k":"unit"}},
        {"name":"continuation","id":4,"value":{"k":"unit"}},
        {"name":"watchdog","id":5,"value":{"k":"unit"}}
        ]}))
    }
}

impl Wire for TimerExecutionOutcome {
    fn native_schema(_: &mut SchemaContext) -> Result<Value, GenerationError> {
        Ok(json!({"k":"enum", "variants":[
        {"name":"success","id":0,"value":{"k":"unit"}},
        {"name":"no_work","id":1,"value":{"k":"unit"}},
        {"name":"retryable_failure","id":2,"value":{"k":"unit"}},
        {"name":"invariant_failure","id":3,"value":{"k":"unit"}},
        {"name":"unacknowledged","id":4,"value":{"k":"unit"}}
        ]}))
    }
}

#[cfg(test)]
mod tests;
