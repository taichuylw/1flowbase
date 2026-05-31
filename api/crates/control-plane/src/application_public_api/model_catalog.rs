use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

const DEFAULT_AGENT_MODEL_ID: &str = "1flowbase";
const DEFAULT_AGENT_CONTEXT_WINDOW: u64 = 257_000;
const DEFAULT_AGENT_MAX_CONTEXT_WINDOW: u64 = 128_000;
const DEFAULT_AGENT_MAX_OUTPUT_TOKENS: u64 = 32_000;
const DEFAULT_AGENT_AUTO_COMPACT_PERCENT: u64 = 85;
const DEFAULT_AGENT_REASONING_EFFORT: &str = "medium";
const DEFAULT_AGENT_REASONING_EFFORTS: [&str; 5] = ["minimal", "low", "medium", "high", "xhigh"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentModelCapabilities {
    pub reasoning: bool,
    pub tool_call: bool,
    pub multimodal: bool,
    pub structured_output: bool,
}

impl AgentModelCapabilities {
    pub fn is_empty(&self) -> bool {
        !self.reasoning && !self.tool_call && !self.multimodal && !self.structured_output
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentModelReasoning {
    pub default_effort: Option<String>,
    pub supported_efforts: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentModelDescriptor {
    pub id: String,
    pub name: Option<String>,
    pub context_window: Option<u64>,
    pub max_context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub auto_compact_token_limit: Option<u64>,
    pub capabilities: AgentModelCapabilities,
    pub reasoning: Option<AgentModelReasoning>,
}

pub fn extract_agent_model_catalog_from_start_node(document: &Value) -> Vec<AgentModelDescriptor> {
    let Some(nodes) = document
        .get("graph")
        .and_then(|graph| graph.get("nodes"))
        .and_then(Value::as_array)
    else {
        return default_model_catalog();
    };
    let Some(start_node) = nodes
        .iter()
        .find(|node| node.get("type").and_then(Value::as_str) == Some("start"))
    else {
        return default_model_catalog();
    };
    let Some(model_list) = start_node
        .get("config")
        .and_then(|config| config.get("model_list"))
        .and_then(Value::as_array)
    else {
        return default_model_catalog();
    };

    let mut models = Vec::new();
    for value in model_list {
        if let Some(model) = normalize_model_descriptor(value) {
            if !models
                .iter()
                .any(|existing: &AgentModelDescriptor| existing.id == model.id)
            {
                models.push(model);
            }
        }
    }
    if models.is_empty() {
        default_model_catalog()
    } else {
        models
    }
}

pub fn find_agent_model<'a>(
    models: &'a [AgentModelDescriptor],
    model_id: &str,
) -> Option<&'a AgentModelDescriptor> {
    models.iter().find(|model| model.id == model_id)
}

fn default_model_catalog() -> Vec<AgentModelDescriptor> {
    vec![AgentModelDescriptor {
        id: DEFAULT_AGENT_MODEL_ID.to_string(),
        name: Some(DEFAULT_AGENT_MODEL_ID.to_string()),
        context_window: Some(DEFAULT_AGENT_CONTEXT_WINDOW),
        max_context_window: Some(DEFAULT_AGENT_MAX_CONTEXT_WINDOW),
        max_output_tokens: Some(DEFAULT_AGENT_MAX_OUTPUT_TOKENS),
        auto_compact_token_limit: Some(
            (DEFAULT_AGENT_CONTEXT_WINDOW * DEFAULT_AGENT_AUTO_COMPACT_PERCENT) / 100,
        ),
        capabilities: AgentModelCapabilities {
            reasoning: true,
            tool_call: true,
            multimodal: true,
            structured_output: true,
        },
        reasoning: Some(AgentModelReasoning {
            default_effort: Some(DEFAULT_AGENT_REASONING_EFFORT.to_string()),
            supported_efforts: DEFAULT_AGENT_REASONING_EFFORTS
                .iter()
                .map(|effort| (*effort).to_string())
                .collect(),
        }),
    }]
}

fn normalize_model_descriptor(value: &Value) -> Option<AgentModelDescriptor> {
    if let Some(id) = value.as_str().map(str::trim).filter(|id| !id.is_empty()) {
        return Some(AgentModelDescriptor {
            id: id.to_string(),
            name: None,
            context_window: None,
            max_context_window: None,
            max_output_tokens: None,
            auto_compact_token_limit: None,
            capabilities: AgentModelCapabilities::default(),
            reasoning: None,
        });
    }

    let object = value.as_object()?;
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())?;
    let name = object
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned);

    Some(AgentModelDescriptor {
        id: id.to_string(),
        name,
        context_window: model_token_u64(object, "context_window"),
        max_context_window: model_token_u64(object, "max_context_window"),
        max_output_tokens: model_token_u64(object, "max_output_tokens"),
        auto_compact_token_limit: model_token_u64(object, "auto_compact_token_limit"),
        capabilities: normalize_capabilities(object),
        reasoning: normalize_reasoning(object.get("reasoning")),
    })
}

fn normalize_capabilities(object: &Map<String, Value>) -> AgentModelCapabilities {
    let capabilities = object.get("capabilities").and_then(Value::as_object);

    AgentModelCapabilities {
        reasoning: capabilities
            .and_then(|value| value.get("reasoning"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        tool_call: capabilities
            .and_then(|value| value.get("tool_call"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        multimodal: capabilities
            .and_then(|value| value.get("multimodal"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        structured_output: capabilities
            .and_then(|value| value.get("structured_output"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn normalize_reasoning(value: Option<&Value>) -> Option<AgentModelReasoning> {
    let object = value.and_then(Value::as_object)?;
    let default_effort = object
        .get("default_effort")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let supported_efforts = object
        .get("supported_efforts")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(AgentModelReasoning {
        default_effort,
        supported_efforts,
    })
}

fn model_token_u64(object: &Map<String, Value>, key: &str) -> Option<u64> {
    object.get(key).and_then(Value::as_u64)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn extracts_start_node_model_catalog_with_capabilities() {
        let document = json!({
            "graph": {
                "nodes": [
                    {
                        "id": "node-start",
                        "type": "start",
                        "config": {
                            "model_list": [
                                {
                                    "id": "qwen3.6-35b-a3b",
                                    "name": "Qwen 3.6 35B",
                                    "context_window": 128000,
                                    "max_output_tokens": 32000,
                                    "auto_compact_token_limit": 110000,
                                    "capabilities": {
                                        "reasoning": true,
                                        "tool_call": true,
                                        "multimodal": false,
                                        "structured_output": true
                                    },
                                    "reasoning": {
                                        "default_effort": "medium",
                                        "supported_efforts": ["low", "medium", "high"]
                                    }
                                },
                                "deepseek-v4-flash",
                                {"id": "deepseek-v4-flash", "name": "Duplicate"}
                            ]
                        }
                    }
                ]
            }
        });

        let models = extract_agent_model_catalog_from_start_node(&document);

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].id, "qwen3.6-35b-a3b");
        assert_eq!(models[0].context_window, Some(128000));
        assert_eq!(models[0].max_output_tokens, Some(32000));
        assert!(models[0].capabilities.reasoning);
        assert_eq!(
            models[0]
                .reasoning
                .as_ref()
                .expect("reasoning should be present")
                .supported_efforts,
            vec!["low", "medium", "high"]
        );
        assert_eq!(models[1].id, "deepseek-v4-flash");
    }
}
