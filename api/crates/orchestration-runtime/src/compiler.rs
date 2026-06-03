use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{anyhow, bail, Context, Result};
use serde_json::Value;

use crate::answer_presentation::validate_answer_presentation;
use crate::compiled_plan::{
    CodeExecutorCapability, CodeIsolationProfile, CompileIssue, CompileIssueCode, CompiledBinding,
    CompiledCodeDependency, CompiledCodeRuntime, CompiledEdge, CompiledLlmRouteTarget,
    CompiledLlmRouting, CompiledLlmRuntime, CompiledNode, CompiledOutput, CompiledPlan,
    CompiledPluginRuntime, LlmRoutingMode,
};
use crate::output_schema::{history_messages_schema, output_schema_is_llm_context_messages};
use crate::payload_builder::PublicOutputContract;

mod code_runtime_config;
mod selector_paths;

use code_runtime_config::{
    code_import_aliases, compile_code_isolation_profile, trimmed_config_string,
    validate_code_imports,
};
use selector_paths::extract_selector_paths;

const NODE_CONTRIBUTION_SCHEMA_VERSION: &str = "1flowbase.node-contribution/v2";
const FLOW_SCHEMA_VERSION: &str = "1flowbase.flow/v2";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FlowCompileContext {
    pub provider_families: BTreeMap<String, FlowCompileProviderFamily>,
    pub provider_instances: BTreeMap<String, FlowCompileProviderInstance>,
    pub node_contributions: BTreeMap<String, FlowCompileNodeContribution>,
    pub js_dependencies: BTreeMap<String, FlowCompileJsDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowCompileJsDependency {
    pub alias: String,
    pub target: String,
    pub artifact_path: String,
    pub artifact_hash: String,
    pub integrity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowCompileProviderFamily {
    pub provider_code: String,
    pub protocol: String,
    pub is_ready: bool,
    pub available_models: BTreeSet<String>,
    pub allow_custom_models: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowCompileProviderInstance {
    pub provider_instance_id: String,
    pub provider_code: String,
    pub protocol: String,
    pub is_ready: bool,
    pub is_runnable: bool,
    pub included_in_main: bool,
    pub available_models: BTreeSet<String>,
    pub allow_custom_models: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowCompileNodeContribution {
    pub installation_id: uuid::Uuid,
    pub plugin_unique_identifier: String,
    pub package_id: String,
    pub plugin_id: String,
    pub plugin_version: String,
    pub contribution_code: String,
    pub node_shell: String,
    pub schema_version: String,
    pub contribution_checksum: String,
    pub compiled_contribution_hash: String,
    pub output_schema_snapshot: Vec<CompiledOutput>,
    pub side_effect_policy: String,
    pub dependency_status: String,
}

type NodeTopologyBuild = (
    BTreeMap<String, CompiledNode>,
    Vec<CompiledEdge>,
    Vec<String>,
    Vec<CompileIssue>,
);

pub struct FlowCompiler;

impl FlowCompiler {
    pub fn compile(
        flow_id: uuid::Uuid,
        draft_id: &str,
        document: &Value,
        context: &FlowCompileContext,
    ) -> Result<CompiledPlan> {
        let schema_version = document
            .get("schemaVersion")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("schemaVersion missing"))?
            .to_string();
        if schema_version != FLOW_SCHEMA_VERSION {
            bail!("unsupported flow schemaVersion: {schema_version}");
        }
        let (nodes, edges, topological_order, mut compile_issues) =
            build_nodes_and_topology(document, context)?;

        let mut plan = CompiledPlan {
            flow_id,
            source_draft_id: draft_id.to_string(),
            schema_version,
            topological_order,
            edges,
            nodes,
            compile_issues: Vec::new(),
        };
        compile_issues.extend(validate_answer_presentation(&plan));
        plan.compile_issues = compile_issues;

        Ok(plan)
    }
}

fn build_nodes_and_topology(
    document: &Value,
    context: &FlowCompileContext,
) -> Result<NodeTopologyBuild> {
    let node_values = document
        .get("graph")
        .and_then(|graph| graph.get("nodes"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("graph.nodes missing"))?;

    let edge_values = document
        .get("graph")
        .and_then(|graph| graph.get("edges"))
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("graph.edges missing"))?;

    let mut nodes = BTreeMap::new();
    let mut node_order = Vec::with_capacity(node_values.len());
    let mut compile_issues = Vec::new();

    for node in node_values {
        let compiled = compile_node(node, context, &mut compile_issues)?;

        if nodes.contains_key(&compiled.node_id) {
            bail!("duplicate node id: {}", compiled.node_id);
        }

        node_order.push(compiled.node_id.clone());
        nodes.insert(compiled.node_id.clone(), compiled);
    }

    let mut adjacency = BTreeMap::<String, Vec<String>>::new();
    let mut indegree = BTreeMap::<String, usize>::new();
    let mut compiled_edges = Vec::with_capacity(edge_values.len());

    for node_id in &node_order {
        adjacency.insert(node_id.clone(), Vec::new());
        indegree.insert(node_id.clone(), 0);
    }

    for edge in edge_values {
        let edge_id = edge
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("unknown-edge");
        let source = edge
            .get("source")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("edge {edge_id} missing source"))?;
        let target = edge
            .get("target")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("edge {edge_id} missing target"))?;

        if !nodes.contains_key(source) {
            bail!("edge {edge_id} references unknown source node: {source}");
        }

        if !nodes.contains_key(target) {
            bail!("edge {edge_id} references unknown target node: {target}");
        }

        let source_handle = edge_handle(edge, "sourceHandle", "source_handle")?;
        let target_handle = edge_handle(edge, "targetHandle", "target_handle")?;
        validate_edge_source_handle(
            edge_id,
            nodes
                .get(source)
                .expect("validated source node must exist before edge compilation"),
            source_handle.as_deref(),
        )?;
        compiled_edges.push(CompiledEdge {
            edge_id: edge_id.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            source_handle,
            target_handle,
        });

        let dependency_node_ids = &mut nodes
            .get_mut(target)
            .expect("validated target node must exist")
            .dependency_node_ids;
        if !dependency_node_ids.iter().any(|node_id| node_id == source) {
            dependency_node_ids.push(source.to_string());
        }

        let downstream_node_ids = &mut nodes
            .get_mut(source)
            .expect("validated source node must exist")
            .downstream_node_ids;
        if !downstream_node_ids.iter().any(|node_id| node_id == target) {
            downstream_node_ids.push(target.to_string());
        }

        adjacency
            .get_mut(source)
            .expect("validated source adjacency must exist")
            .push(target.to_string());
        *indegree
            .get_mut(target)
            .expect("validated target indegree must exist") += 1;
    }

    for node in nodes.values_mut() {
        node.dependency_node_ids
            .sort_by_key(|node_id| node_order_index(&node_order, node_id));
        node.downstream_node_ids
            .sort_by_key(|node_id| node_order_index(&node_order, node_id));
    }

    let mut queue = VecDeque::new();

    for node_id in &node_order {
        if indegree.get(node_id).copied().unwrap_or_default() == 0 {
            queue.push_back(node_id.clone());
        }
    }

    let mut topological_order = Vec::with_capacity(node_order.len());

    while let Some(node_id) = queue.pop_front() {
        topological_order.push(node_id.clone());

        if let Some(neighbors) = adjacency.get(&node_id) {
            for neighbor in neighbors {
                let remaining = indegree
                    .get_mut(neighbor)
                    .expect("neighbor indegree must exist after validation");
                *remaining -= 1;

                if *remaining == 0 {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }

    if topological_order.len() != node_order.len() {
        let visited = topological_order.iter().cloned().collect::<BTreeSet<_>>();
        let cycle_nodes = node_order
            .into_iter()
            .filter(|node_id| !visited.contains(node_id))
            .collect::<Vec<_>>();
        bail!(
            "graph contains a cycle involving nodes: {}",
            cycle_nodes.join(", ")
        );
    }

    compile_issues.extend(validate_llm_context_policies(&nodes));

    Ok((nodes, compiled_edges, topological_order, compile_issues))
}

fn edge_handle(edge: &Value, camel_key: &str, snake_key: &str) -> Result<Option<String>> {
    let value = edge.get(camel_key).or_else(|| edge.get(snake_key));

    match value {
        Some(Value::String(handle)) if !handle.trim().is_empty() => Ok(Some(handle.to_string())),
        Some(Value::String(_)) | Some(Value::Null) | None => Ok(None),
        Some(_) => bail!("edge handle {camel_key} must be a string or null"),
    }
}

fn validate_edge_source_handle(
    edge_id: &str,
    source_node: &CompiledNode,
    source_handle: Option<&str>,
) -> Result<()> {
    if source_node.node_type != "if_else" {
        return Ok(());
    }

    let Some(source_handle) = source_handle else {
        bail!(
            "edge {edge_id} from if_else node {} missing sourceHandle",
            source_node.node_id
        );
    };
    let source_handles = if_else_branch_source_handles(source_node)?;

    if !source_handles.contains(source_handle) {
        bail!(
            "edge {edge_id} references unknown if_else sourceHandle {source_handle} for node {}",
            source_node.node_id
        );
    }

    Ok(())
}

fn if_else_branch_source_handles(node: &CompiledNode) -> Result<BTreeSet<String>> {
    let binding = node
        .bindings
        .get("branches")
        .ok_or_else(|| anyhow!("if_else node {} missing branches binding", node.node_id))?;
    if binding.kind != "if_else_branches" {
        bail!(
            "if_else node {} branches binding must be if_else_branches",
            node.node_id
        );
    }

    let branches = binding
        .raw_value
        .get("branches")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("if_else node {} branches must be an array", node.node_id))?;
    let mut source_handles = BTreeSet::new();
    let mut else_branch_count = 0;
    for branch in branches {
        if branch.get("kind").and_then(Value::as_str) == Some("else") {
            else_branch_count += 1;
        }

        let source_handle = branch
            .get("sourceHandle")
            .or_else(|| branch.get("source_handle"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|handle| !handle.is_empty())
            .ok_or_else(|| anyhow!("if_else node {} branch missing sourceHandle", node.node_id))?;
        if !source_handles.insert(source_handle.to_string()) {
            bail!(
                "if_else node {} duplicate branch sourceHandle {source_handle}",
                node.node_id
            );
        }
    }

    if else_branch_count == 0 {
        bail!("if_else node {} must include an else branch", node.node_id);
    }

    if else_branch_count > 1 {
        bail!(
            "if_else node {} must include only one else branch",
            node.node_id
        );
    }

    Ok(source_handles)
}

fn validate_llm_context_policies(nodes: &BTreeMap<String, CompiledNode>) -> Vec<CompileIssue> {
    nodes
        .values()
        .filter(|node| node.node_type == "llm")
        .filter_map(|node| {
            let selector = context_policy_selector(node)?;
            let Some(output) = output_for_selector(nodes, &selector) else {
                return Some(CompileIssue {
                    node_id: node.node_id.clone(),
                    code: CompileIssueCode::InvalidLlmContextSelector,
                    message: format!(
                        "node {} context_selector references unavailable output {}",
                        node.node_id,
                        selector.join(".")
                    ),
                });
            };

            (!output_schema_is_llm_context_messages(&output)).then(|| CompileIssue {
                node_id: node.node_id.clone(),
                code: CompileIssueCode::IncompatibleLlmContextSchema,
                message: format!(
                    "node {} context_selector {} must reference an output with an LLM history-compatible jsonSchema",
                    node.node_id,
                    selector.join(".")
                ),
            })
        })
        .collect()
}

fn context_policy_selector(node: &CompiledNode) -> Option<Vec<String>> {
    node.config
        .get("context_policy")
        .and_then(|policy| policy.get("context_selector"))
        .and_then(Value::as_array)
        .map(|selector| {
            selector
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .filter(|selector| selector.len() >= 2)
}

fn output_for_selector(
    nodes: &BTreeMap<String, CompiledNode>,
    selector: &[String],
) -> Option<CompiledOutput> {
    let node_id = selector.first()?;
    let selector_tail = selector.get(1..)?;
    let output_key = selector_tail.first()?;
    let node = nodes.get(node_id)?;

    if node.node_type == "start" && output_key == "history" {
        return Some(CompiledOutput {
            key: "history".to_string(),
            title: "userinput.history".to_string(),
            value_type: "array".to_string(),
            selector: Vec::new(),
            json_schema: Some(history_messages_schema()),
        });
    }

    node.outputs
        .iter()
        .find(|output| {
            output.selector == selector_tail
                || (selector_tail.len() == 1 && output.key == *output_key)
        })
        .cloned()
}

fn compile_node(
    node: &Value,
    context: &FlowCompileContext,
    compile_issues: &mut Vec<CompileIssue>,
) -> Result<CompiledNode> {
    let node_id = required_string(node, "id")?.to_string();
    let node_type = required_string(node, "type")?.to_string();
    let alias = required_string(node, "alias")?.to_string();
    let container_id = optional_string(node, "containerId")?.map(str::to_string);
    let config = node
        .get("config")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    let raw_bindings = node
        .get("bindings")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("node {node_id} missing bindings"))?;
    let active_bindings = active_binding_values(&node_type, raw_bindings);
    let bindings = compile_bindings(&active_bindings)
        .with_context(|| format!("failed to compile bindings for node {node_id}"))?;
    let mut outputs = compile_outputs(
        node.get("outputs")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("node {node_id} missing outputs"))?,
    )
    .with_context(|| format!("failed to compile outputs for node {node_id}"))?;
    if node_type == "code" {
        for output in &mut outputs {
            if output.selector.len() == 1 && output.selector[0] == output.key {
                output.selector = vec!["result".to_string(), output.key.clone()];
            }
        }
    }
    if node_type == "start" && !outputs.is_empty() {
        bail!("start node {node_id} outputs must be empty");
    }
    let llm_runtime = (node_type == "llm")
        .then(|| compile_llm_runtime(&node_id, &config, context, compile_issues))
        .flatten();
    let plugin_runtime = (node_type == "plugin_node")
        .then(|| compile_plugin_runtime(&node_id, node, &outputs, context, compile_issues))
        .flatten();
    let code_runtime = (node_type == "code").then(|| {
        validate_code_imports(&node_id, &config, context, compile_issues);
        compile_code_runtime(&node_id, &config, context, compile_issues)
    });

    Ok(CompiledNode {
        node_id,
        node_type,
        alias,
        container_id,
        dependency_node_ids: Vec::new(),
        downstream_node_ids: Vec::new(),
        bindings,
        outputs,
        config,
        plugin_runtime,
        llm_runtime,
        code_runtime,
    })
}

fn compile_code_runtime(
    node_id: &str,
    config: &Value,
    context: &FlowCompileContext,
    compile_issues: &mut Vec<CompileIssue>,
) -> CompiledCodeRuntime {
    let language = trimmed_config_string(config, "language").unwrap_or("javascript");
    let source = trimmed_config_string(config, "source").map(str::to_string);
    let source_ref = trimmed_config_string(config, "source_ref")
        .or_else(|| trimmed_config_string(config, "sourceRef"))
        .map(str::to_string);
    let entrypoint = trimmed_config_string(config, "entrypoint").unwrap_or("main");
    let imports = code_import_aliases(config);
    let dependencies = imports
        .iter()
        .filter_map(|alias| {
            let key = js_dependency_lookup_key("backend_code", alias);
            context
                .js_dependencies
                .get(&key)
                .map(|dependency| CompiledCodeDependency {
                    alias: dependency.alias.clone(),
                    target: dependency.target.clone(),
                    artifact_path: dependency.artifact_path.clone(),
                    artifact_hash: dependency.artifact_hash.clone(),
                    integrity: dependency.integrity.clone(),
                })
        })
        .collect();

    CompiledCodeRuntime {
        language: language.to_string(),
        source,
        source_ref,
        entrypoint: entrypoint.to_string(),
        imports,
        dependencies,
        isolation_profile: compile_code_isolation_profile(node_id, config, compile_issues),
    }
}

fn compile_llm_runtime(
    node_id: &str,
    config: &Value,
    context: &FlowCompileContext,
    compile_issues: &mut Vec<CompileIssue>,
) -> Option<CompiledLlmRuntime> {
    let provider_config = config.get("model_provider");
    if provider_config
        .and_then(|value| value.get("routing_mode"))
        .and_then(Value::as_str)
        .is_some_and(|value| value == "failover_queue")
    {
        return compile_failover_queue_runtime(
            node_id,
            provider_config,
            config,
            context,
            compile_issues,
        );
    }

    let provider_code = provider_config
        .and_then(|value| value.get("provider_code"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let model = provider_config
        .and_then(|value| value.get("model_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let Some(provider_code) = provider_code else {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::MissingProviderInstance,
            message: format!("node {node_id} is missing config.model_provider.provider_code"),
        });
        if model.is_none() {
            compile_issues.push(CompileIssue {
                node_id: node_id.to_string(),
                code: CompileIssueCode::MissingModel,
                message: format!("node {node_id} is missing config.model_provider.model_id"),
            });
        }
        return None;
    };

    let Some(model) = model else {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::MissingModel,
            message: format!("node {node_id} is missing config.model_provider.model_id"),
        });
        return None;
    };

    let provider_instance = resolve_fixed_model_provider_instance(
        node_id,
        &provider_code,
        &model,
        context,
        compile_issues,
    )?;

    let context_policy = compile_llm_context_policy(config);

    Some(CompiledLlmRuntime {
        provider_instance_id: provider_instance.provider_instance_id.clone(),
        provider_code: provider_instance.provider_code.clone(),
        protocol: provider_instance.protocol.clone(),
        model: model.clone(),
        routing: Some(fixed_model_routing(
            provider_instance,
            &model,
            context_policy,
        )),
    })
}

fn compile_llm_context_policy(config: &Value) -> Value {
    config
        .get("context_policy")
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| serde_json::json!({ "integration_context": "enabled" }))
}

fn resolve_fixed_model_provider_instance<'a>(
    node_id: &str,
    provider_code: &str,
    model: &str,
    context: &'a FlowCompileContext,
    compile_issues: &mut Vec<CompileIssue>,
) -> Option<&'a FlowCompileProviderInstance> {
    if !context.provider_families.contains_key(provider_code) {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::ProviderInstanceNotFound,
            message: format!("provider {provider_code} was not found"),
        });
        return None;
    }

    let candidates = context
        .provider_instances
        .values()
        .filter(|instance| instance.provider_code == provider_code && instance.included_in_main)
        .collect::<Vec<_>>();

    if candidates.is_empty() {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::ProviderInstanceNotFound,
            message: format!("provider {provider_code} has no included runtime instance"),
        });
        return None;
    }

    let model_candidates = candidates
        .into_iter()
        .filter(|instance| provider_instance_supports_model(instance, model))
        .collect::<Vec<_>>();

    if model_candidates.is_empty() {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::ModelNotAvailable,
            message: format!("model {model} is not available for provider {provider_code}"),
        });
        return None;
    }

    let runnable_candidates = model_candidates
        .iter()
        .copied()
        .filter(|instance| instance.is_ready && instance.is_runnable)
        .collect::<Vec<_>>();

    if runnable_candidates.len() > 1 {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::ProviderInstanceNotFound,
            message: format!(
                "provider {provider_code} model {model} is ambiguous across multiple runtime instances"
            ),
        });
        return None;
    }

    if let Some(provider_instance) = runnable_candidates.first() {
        return Some(*provider_instance);
    }

    compile_issues.push(CompileIssue {
        node_id: node_id.to_string(),
        code: CompileIssueCode::ProviderInstanceNotReady,
        message: format!("provider {provider_code} has no runnable instance for model {model}"),
    });
    None
}

fn provider_instance_supports_model(instance: &FlowCompileProviderInstance, model: &str) -> bool {
    instance.allow_custom_models
        || instance.available_models.is_empty()
        || instance.available_models.contains(model)
}

fn fixed_model_routing(
    provider_instance: &FlowCompileProviderInstance,
    model: &str,
    context_policy: Value,
) -> CompiledLlmRouting {
    CompiledLlmRouting {
        routing_mode: LlmRoutingMode::FixedModel,
        fixed_model_target: Some(serde_json::json!({
            "provider_instance_id": provider_instance.provider_instance_id.clone(),
            "provider_code": provider_instance.provider_code.clone(),
            "protocol": provider_instance.protocol.clone(),
            "upstream_model_id": model,
        })),
        queue_template_id: None,
        queue_snapshot_id: None,
        queue_targets: Vec::new(),
        context_policy,
        stream_policy: serde_json::json!({}),
    }
}

fn compile_failover_queue_runtime(
    node_id: &str,
    provider_config: Option<&Value>,
    config: &Value,
    context: &FlowCompileContext,
    compile_issues: &mut Vec<CompileIssue>,
) -> Option<CompiledLlmRuntime> {
    let queue_template_id = provider_config
        .and_then(|value| value.get("queue_template_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let queue_snapshot_id = provider_config
        .and_then(|value| value.get("queue_snapshot_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let raw_targets = provider_config
        .and_then(|value| value.get("queue_targets"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if queue_template_id.is_none() {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::MissingProviderInstance,
            message: format!("node {node_id} is missing config.model_provider.queue_template_id"),
        });
    }
    let mut targets = Vec::new();
    for (index, target) in raw_targets.iter().enumerate() {
        let Some(compiled_target) =
            compile_failover_queue_target(node_id, index, target, context, compile_issues)
        else {
            continue;
        };
        targets.push(compiled_target);
    }
    let first_target = targets.first().cloned().unwrap_or(CompiledLlmRouteTarget {
        provider_instance_id: String::new(),
        provider_code: String::new(),
        protocol: String::new(),
        upstream_model_id: String::new(),
    });

    Some(CompiledLlmRuntime {
        provider_instance_id: first_target.provider_instance_id.clone(),
        provider_code: first_target.provider_code.clone(),
        protocol: first_target.protocol.clone(),
        model: first_target.upstream_model_id.clone(),
        routing: Some(CompiledLlmRouting {
            routing_mode: LlmRoutingMode::FailoverQueue,
            fixed_model_target: None,
            queue_template_id,
            queue_snapshot_id,
            queue_targets: targets,
            context_policy: compile_llm_context_policy(config),
            stream_policy: serde_json::json!({}),
        }),
    })
}

fn compile_failover_queue_target(
    node_id: &str,
    index: usize,
    target: &Value,
    context: &FlowCompileContext,
    compile_issues: &mut Vec<CompileIssue>,
) -> Option<CompiledLlmRouteTarget> {
    let provider_instance_id = target
        .get("provider_instance_id")
        .or_else(|| target.get("source_instance_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let upstream_model_id = target
        .get("upstream_model_id")
        .or_else(|| target.get("model_id"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let Some(provider_instance_id) = provider_instance_id else {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::MissingProviderInstance,
            message: format!(
                "node {node_id} failover target {index} is missing provider_instance_id"
            ),
        });
        return None;
    };
    let Some(upstream_model_id) = upstream_model_id else {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::MissingModel,
            message: format!("node {node_id} failover target {index} is missing upstream_model_id"),
        });
        return None;
    };
    let Some(provider_instance) = context.provider_instances.get(&provider_instance_id) else {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::ProviderInstanceNotFound,
            message: format!(
                "failover target source_instance_id {provider_instance_id} was not found"
            ),
        });
        return None;
    };

    if !provider_instance.is_ready
        || !provider_instance.is_runnable
        || !provider_instance.included_in_main
    {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::ProviderInstanceNotReady,
            message: format!(
                "failover target source_instance_id {provider_instance_id} is not runnable"
            ),
        });
    }
    if !provider_instance.allow_custom_models
        && !provider_instance.available_models.is_empty()
        && !provider_instance
            .available_models
            .contains(&upstream_model_id)
    {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::ModelNotAvailable,
            message: format!(
                "model {upstream_model_id} is not available for failover target source_instance_id {provider_instance_id}"
            ),
        });
    }

    Some(CompiledLlmRouteTarget {
        provider_instance_id: provider_instance.provider_instance_id.clone(),
        provider_code: target
            .get("provider_code")
            .and_then(Value::as_str)
            .unwrap_or(&provider_instance.provider_code)
            .to_string(),
        protocol: target
            .get("protocol")
            .and_then(Value::as_str)
            .unwrap_or(&provider_instance.protocol)
            .to_string(),
        upstream_model_id,
    })
}

fn compile_plugin_runtime(
    node_id: &str,
    node: &Value,
    compiled_outputs: &[CompiledOutput],
    context: &FlowCompileContext,
    compile_issues: &mut Vec<CompileIssue>,
) -> Option<CompiledPluginRuntime> {
    let schema_version = required_plugin_string(
        node_id,
        node,
        "schema_version",
        CompileIssueCode::MissingSchemaVersion,
        compile_issues,
    )?;
    if schema_version != NODE_CONTRIBUTION_SCHEMA_VERSION {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::UnsupportedPluginContributionSchemaVersion,
            message: format!(
                "node {node_id} uses unsupported plugin contribution schema_version {schema_version}"
            ),
        });
        return None;
    }

    let plugin_unique_identifier = required_plugin_string(
        node_id,
        node,
        "plugin_unique_identifier",
        CompileIssueCode::MissingPluginUniqueIdentifier,
        compile_issues,
    )?;
    let package_id = required_plugin_string(
        node_id,
        node,
        "package_id",
        CompileIssueCode::MissingPackageId,
        compile_issues,
    )?;
    let plugin_id = required_plugin_string(
        node_id,
        node,
        "plugin_id",
        CompileIssueCode::MissingPluginId,
        compile_issues,
    )?;
    let plugin_version = required_plugin_string(
        node_id,
        node,
        "plugin_version",
        CompileIssueCode::MissingPluginVersion,
        compile_issues,
    )?;
    let contribution_code = required_plugin_string(
        node_id,
        node,
        "contribution_code",
        CompileIssueCode::MissingContributionCode,
        compile_issues,
    )?;
    let node_shell = required_plugin_string(
        node_id,
        node,
        "node_shell",
        CompileIssueCode::MissingNodeShell,
        compile_issues,
    )?;
    let contribution_checksum = required_plugin_string(
        node_id,
        node,
        "contribution_checksum",
        CompileIssueCode::MissingContributionChecksum,
        compile_issues,
    )?;
    let compiled_contribution_hash = required_plugin_string(
        node_id,
        node,
        "compiled_contribution_hash",
        CompileIssueCode::MissingCompiledContributionHash,
        compile_issues,
    )?;
    let output_schema_snapshot = compile_output_schema_snapshot(node_id, node, compile_issues)?;

    let lookup_key = node_contribution_lookup_key(
        &plugin_id,
        &plugin_version,
        &contribution_code,
        &node_shell,
        &schema_version,
    );
    let Some(contribution) = context.node_contributions.get(&lookup_key) else {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::MissingPluginContribution,
            message: format!(
                "node {node_id} missing workspace contribution for {plugin_id}:{plugin_version}:{contribution_code}"
            ),
        });
        return None;
    };

    if contribution.plugin_unique_identifier != plugin_unique_identifier
        || contribution.package_id != package_id
    {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::MissingPluginContribution,
            message: format!("node {node_id} contribution identity no longer matches registry"),
        });
    }

    if contribution.dependency_status != "ready" {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::PluginContributionDependencyNotReady,
            message: format!(
                "node {node_id} contribution {contribution_code} is not ready: {}",
                contribution.dependency_status
            ),
        });
    }

    if contribution.contribution_checksum != contribution_checksum
        || contribution.compiled_contribution_hash != compiled_contribution_hash
    {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::PluginContributionChecksumMismatch,
            message: format!(
                "node {node_id} contribution checksum changed for {contribution_code}"
            ),
        });
    }

    if contribution.output_schema_snapshot != output_schema_snapshot
        || compiled_outputs != output_schema_snapshot
    {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::PluginContributionOutputSchemaMismatch,
            message: format!(
                "node {node_id} output schema snapshot changed for {contribution_code}"
            ),
        });
    }

    Some(CompiledPluginRuntime {
        installation_id: contribution.installation_id,
        plugin_unique_identifier: contribution.plugin_unique_identifier.clone(),
        package_id: contribution.package_id.clone(),
        plugin_id: contribution.plugin_id.clone(),
        plugin_version: contribution.plugin_version.clone(),
        contribution_code: contribution.contribution_code.clone(),
        node_shell: contribution.node_shell.clone(),
        schema_version: contribution.schema_version.clone(),
        contribution_checksum: contribution.contribution_checksum.clone(),
        compiled_contribution_hash: contribution.compiled_contribution_hash.clone(),
        output_schema_snapshot: contribution.output_schema_snapshot.clone(),
        side_effect_policy: contribution.side_effect_policy.clone(),
    })
}

fn compile_output_schema_snapshot(
    node_id: &str,
    node: &Value,
    compile_issues: &mut Vec<CompileIssue>,
) -> Option<Vec<CompiledOutput>> {
    let Some(outputs) = node
        .get("output_schema_snapshot")
        .and_then(|snapshot| snapshot.get("outputs"))
        .and_then(Value::as_array)
    else {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code: CompileIssueCode::MissingOutputSchemaSnapshot,
            message: format!("node {node_id} missing output_schema_snapshot.outputs"),
        });
        return None;
    };

    match compile_outputs(outputs) {
        Ok(outputs) => Some(outputs),
        Err(error) => {
            compile_issues.push(CompileIssue {
                node_id: node_id.to_string(),
                code: CompileIssueCode::PluginContributionOutputSchemaMismatch,
                message: format!("node {node_id} has invalid output_schema_snapshot: {error}"),
            });
            None
        }
    }
}

fn required_plugin_string(
    node_id: &str,
    node: &Value,
    field: &str,
    code: CompileIssueCode,
    compile_issues: &mut Vec<CompileIssue>,
) -> Option<String> {
    let value = node
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    if value.is_none() {
        compile_issues.push(CompileIssue {
            node_id: node_id.to_string(),
            code,
            message: format!("node {node_id} missing {field}"),
        });
    }

    value
}

fn node_contribution_lookup_key(
    plugin_id: &str,
    plugin_version: &str,
    contribution_code: &str,
    node_shell: &str,
    schema_version: &str,
) -> String {
    format!("{plugin_id}::{plugin_version}::{contribution_code}::{node_shell}::{schema_version}")
}

pub fn js_dependency_lookup_key(target: &str, alias: &str) -> String {
    format!("{target}::{alias}")
}

fn compile_bindings(
    binding_values: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, CompiledBinding>> {
    let mut bindings = BTreeMap::new();

    for (binding_key, binding_value) in binding_values {
        let kind = required_string(binding_value, "kind")
            .with_context(|| format!("binding {binding_key} missing kind"))?;
        let raw_value = binding_value.get("value").cloned().unwrap_or(Value::Null);
        let selector_paths = extract_selector_paths(kind, &raw_value)
            .with_context(|| format!("binding {binding_key} has invalid selector payload"))?;

        bindings.insert(
            binding_key.clone(),
            CompiledBinding {
                kind: kind.to_string(),
                raw_value,
                selector_paths,
            },
        );
    }

    Ok(bindings)
}

fn active_binding_values(
    node_type: &str,
    binding_values: &serde_json::Map<String, Value>,
) -> BTreeMap<String, Value> {
    if node_type == "llm" {
        return binding_values
            .iter()
            .filter(|(key, _)| key.as_str() == "prompt_messages")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
    }

    let Some(active_keys) = active_data_model_binding_keys(node_type) else {
        return binding_values
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
    };

    binding_values
        .iter()
        .filter(|(key, _)| active_keys.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn active_data_model_binding_keys(node_type: &str) -> Option<&'static [&'static str]> {
    let action = match node_type {
        "data_model_list" => "list",
        "data_model_get" => "get",
        "data_model_create" => "create",
        "data_model_update" => "update",
        "data_model_delete" => "delete",
        _ => return None,
    };

    Some(match action {
        "get" => &["record_id"],
        "create" => &["payload"],
        "update" => &["record_id", "payload"],
        "delete" => &["record_id"],
        _ => &["query"],
    })
}

fn compile_outputs(output_values: &[Value]) -> Result<Vec<CompiledOutput>> {
    let outputs: Vec<CompiledOutput> = output_values
        .iter()
        .map(|output| {
            let key = required_string(output, "key")?.to_string();
            Ok(CompiledOutput {
                selector: read_output_selector(output).unwrap_or_else(|| vec![key.clone()]),
                key,
                title: required_string(output, "title")?.to_string(),
                value_type: required_string(output, "valueType")?.to_string(),
                json_schema: output
                    .get("jsonSchema")
                    .filter(|value| value.is_object())
                    .cloned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    PublicOutputContract::from_compiled_outputs(&outputs)?;

    Ok(outputs)
}

fn read_output_selector(output: &Value) -> Option<Vec<String>> {
    let selector = output.get("selector")?.as_array()?;
    let segments = selector
        .iter()
        .filter_map(|segment| segment.as_str().map(str::to_string))
        .collect::<Vec<_>>();

    if segments.is_empty() {
        None
    } else {
        Some(segments)
    }
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("{key} missing"))
}

fn optional_string<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>> {
    match value.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::String(text)) => Ok(Some(text.as_str())),
        Some(_) => bail!("{key} must be a string or null"),
    }
}

fn node_order_index(node_order: &[String], node_id: &str) -> usize {
    node_order
        .iter()
        .position(|candidate| candidate == node_id)
        .unwrap_or(usize::MAX)
}
