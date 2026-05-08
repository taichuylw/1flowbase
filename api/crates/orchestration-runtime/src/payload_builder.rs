use std::collections::BTreeSet;

use anyhow::{anyhow, Result};
use plugin_framework::provider_contract::ProviderStreamEvent;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::compiled_plan::CompiledOutput;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RawNodeExecutionResult {
    #[serde(default)]
    pub executor_output: Map<String, Value>,
    #[serde(default)]
    pub metrics_facts: Map<String, Value>,
    #[serde(default)]
    pub error_facts: Map<String, Value>,
    #[serde(default)]
    pub debug_facts: Map<String, Value>,
    #[serde(default)]
    pub provider_events: Vec<ProviderStreamEvent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuiltNodePayloads {
    pub output_payload: Value,
    pub metrics_payload: Value,
    pub error_payload: Value,
    pub debug_payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicOutputContract {
    output_keys: BTreeSet<String>,
    allows_structured_expansion: bool,
}

impl PublicOutputContract {
    pub fn from_compiled_outputs(outputs: &[CompiledOutput]) -> Result<Self> {
        for output in outputs {
            if reserved_payload_bucket(&output.key).is_some() {
                return Err(anyhow!(
                    "reserved public output key `{}` cannot be declared by this node output contract",
                    output.key
                ));
            }
        }

        Ok(Self {
            output_keys: outputs.iter().map(|output| output.key.clone()).collect(),
            allows_structured_expansion: false,
        })
    }

    pub fn with_structured_expansion(mut self, allows_structured_expansion: bool) -> Self {
        self.allows_structured_expansion = allows_structured_expansion;
        self
    }

    pub fn allows_structured_expansion(&self) -> bool {
        self.allows_structured_expansion
    }

    pub fn contains_output_key(&self, key: &str) -> bool {
        self.output_keys.contains(key)
    }

    pub fn build_node_payloads(&self, raw: RawNodeExecutionResult) -> Result<BuiltNodePayloads> {
        let mut output_payload = Map::new();
        let mut metrics_payload = Map::new();
        let mut error_payload = Map::new();
        let mut debug_payload = Map::new();

        for (key, value) in raw.executor_output {
            match classify_executor_output_key(&key, self) {
                PayloadBucket::Output => insert_unique(&mut output_payload, key, value, "output")?,
                PayloadBucket::Metrics => {
                    insert_unique(&mut metrics_payload, key, value, "metrics")?
                }
                PayloadBucket::Error => insert_unique(&mut error_payload, key, value, "error")?,
                PayloadBucket::Debug => insert_unique(&mut debug_payload, key, value, "debug")?,
                PayloadBucket::Unknown => {
                    return Err(anyhow!(
                        "unknown public output key `{key}` is not declared by this node output contract"
                    ));
                }
            }
        }

        merge_non_public_facts(&mut metrics_payload, raw.metrics_facts, "metrics")?;
        merge_non_public_facts(&mut error_payload, raw.error_facts, "error")?;
        merge_non_public_facts(&mut debug_payload, raw.debug_facts, "debug")?;

        if !raw.provider_events.is_empty() {
            insert_unique(
                &mut debug_payload,
                "provider_events".to_string(),
                serde_json::to_value(raw.provider_events)?,
                "debug",
            )?;
        }

        reject_public_bucket_overlap("metrics", &output_payload, &metrics_payload)?;
        reject_public_bucket_overlap("error", &output_payload, &error_payload)?;
        reject_public_bucket_overlap("debug", &output_payload, &debug_payload)?;

        Ok(BuiltNodePayloads {
            output_payload: Value::Object(output_payload),
            metrics_payload: Value::Object(metrics_payload),
            error_payload: Value::Object(error_payload),
            debug_payload: Value::Object(debug_payload),
        })
    }
}

pub fn is_reserved_payload_key(key: &str) -> bool {
    reserved_payload_bucket(key).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PayloadBucket {
    Output,
    Metrics,
    Error,
    Debug,
    Unknown,
}

fn classify_executor_output_key(key: &str, contract: &PublicOutputContract) -> PayloadBucket {
    if let Some(bucket) = reserved_payload_bucket(key) {
        return bucket;
    }

    if contract.contains_output_key(key) || contract.allows_structured_expansion() {
        return PayloadBucket::Output;
    }

    PayloadBucket::Unknown
}

fn reserved_payload_bucket(key: &str) -> Option<PayloadBucket> {
    if key.starts_with("__") {
        return Some(PayloadBucket::Debug);
    }

    match key {
        "usage"
        | "route"
        | "attempts"
        | "finish_reason"
        | "provider_instance_id"
        | "provider_code"
        | "protocol"
        | "model"
        | "event_count"
        | "queue_snapshot_id" => Some(PayloadBucket::Metrics),
        "error" => Some(PayloadBucket::Error),
        "metadata"
        | "debug"
        | "provider_metadata"
        | "tool_calls"
        | "mcp_calls"
        | "provider_events"
        | "raw_response_ref"
        | "raw_response_refs"
        | "raw_ref"
        | "raw_refs"
        | "context_projection_ref"
        | "context_projection_refs"
        | "attempt_ref"
        | "attempt_refs" => Some(PayloadBucket::Debug),
        _ => None,
    }
}

fn merge_non_public_facts(
    target: &mut Map<String, Value>,
    facts: Map<String, Value>,
    bucket_name: &'static str,
) -> Result<()> {
    for (key, value) in facts {
        insert_unique(target, key, value, bucket_name)?;
    }
    Ok(())
}

fn insert_unique(
    target: &mut Map<String, Value>,
    key: String,
    value: Value,
    bucket_name: &'static str,
) -> Result<()> {
    if target.contains_key(&key) {
        return Err(anyhow!(
            "duplicate key `{key}` in {bucket_name} payload bucket"
        ));
    }
    target.insert(key, value);
    Ok(())
}

fn reject_public_bucket_overlap(
    non_public_name: &'static str,
    output_payload: &Map<String, Value>,
    non_public_payload: &Map<String, Value>,
) -> Result<()> {
    for key in output_payload.keys() {
        if non_public_payload.contains_key(key) {
            return Err(anyhow!(
                "public output key `{key}` also appears in {non_public_name} payload bucket"
            ));
        }
    }
    Ok(())
}
