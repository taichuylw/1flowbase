use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::node_error_policy::{node_uses_error_branch, ERROR_BRANCH_SOURCE_HANDLE};

use super::node_compilation::compile_node;
use super::*;

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

pub(super) fn build_nodes_and_topology(
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

    let if_else_source_handles = collect_if_else_branch_source_handles(&nodes)?;

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
            &if_else_source_handles,
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
    if_else_source_handles: &BTreeMap<String, BTreeSet<String>>,
) -> Result<()> {
    if source_handle == Some(ERROR_BRANCH_SOURCE_HANDLE) {
        if node_uses_error_branch(source_node) {
            return Ok(());
        }

        bail!(
            "edge {edge_id} uses error sourceHandle on node {} without error_branch policy",
            source_node.node_id
        );
    }

    if source_node.node_type != "if_else" {
        return Ok(());
    }

    let Some(source_handle) = source_handle else {
        bail!(
            "edge {edge_id} from if_else node {} missing sourceHandle",
            source_node.node_id
        );
    };
    let source_handles = if_else_source_handles
        .get(&source_node.node_id)
        .ok_or_else(|| {
            anyhow!(
                "if_else node {} missing branch contract",
                source_node.node_id
            )
        })?;

    if !source_handles.contains(source_handle) {
        bail!(
            "edge {edge_id} references unknown if_else sourceHandle {source_handle} for node {}",
            source_node.node_id
        );
    }

    Ok(())
}

fn collect_if_else_branch_source_handles(
    nodes: &BTreeMap<String, CompiledNode>,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let mut source_handles = BTreeMap::new();

    for node in nodes.values().filter(|node| node.node_type == "if_else") {
        source_handles.insert(node.node_id.clone(), if_else_branch_source_handles(node)?);
    }

    Ok(source_handles)
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
        let source_handle = branch
            .get("sourceHandle")
            .or_else(|| branch.get("source_handle"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|handle| !handle.is_empty())
            .ok_or_else(|| anyhow!("if_else node {} branch missing sourceHandle", node.node_id))?;
        if branch.get("kind").and_then(Value::as_str) == Some("else") {
            else_branch_count += 1;
        } else if !condition_group_has_complete_rules(
            branch.get("condition").unwrap_or(&Value::Null),
        ) {
            bail!(
                "if_else node {} branch {source_handle} must include a complete condition",
                node.node_id
            );
        }

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

fn condition_group_has_complete_rules(group: &Value) -> bool {
    let Some(conditions) = group.get("conditions").and_then(Value::as_array) else {
        return false;
    };

    !conditions.is_empty()
        && conditions
            .iter()
            .all(condition_expression_has_required_input)
}

fn condition_expression_has_required_input(condition: &Value) -> bool {
    if condition
        .get("conditions")
        .and_then(Value::as_array)
        .is_some()
    {
        return condition_group_has_complete_rules(condition);
    }

    condition_rule_has_required_input(condition)
}

fn condition_rule_has_required_input(rule: &Value) -> bool {
    if !selector_has_required_input(rule.get("left").unwrap_or(&Value::Null)) {
        return false;
    }

    if matches!(
        rule.get("comparator").and_then(Value::as_str),
        Some("exists" | "empty")
    ) {
        return true;
    }

    let Some(right) = rule.get("right") else {
        return false;
    };

    right.get("kind").and_then(Value::as_str) != Some("selector")
        || selector_has_required_input(right.get("selector").unwrap_or(&Value::Null))
}

fn selector_has_required_input(selector: &Value) -> bool {
    selector.as_array().is_some_and(|segments| {
        segments.len() >= 2
            && segments.iter().all(|segment| {
                segment
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty())
            })
    })
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

fn node_order_index(node_order: &[String], node_id: &str) -> usize {
    node_order
        .iter()
        .position(|candidate| candidate == node_id)
        .unwrap_or(usize::MAX)
}
