use std::collections::BTreeSet;

use anyhow::Result;
use serde_json::{json, Map, Value};

use crate::{
    compiled_plan::{CompiledNode, CompiledPlan},
    execution_state::NodeExecutionFailure,
    node_error_policy::{
        error_default_output, node_error_policy, NodeErrorPolicy, ERROR_BRANCH_SOURCE_HANDLE,
    },
};

use super::{
    branching::activate_downstream_nodes, can_continue_to_terminal_template_nodes,
    first_output_key, project_node_variable_payload,
};

pub(super) struct NodeErrorPolicyApplication<'a> {
    pub(super) plan: &'a CompiledPlan,
    pub(super) failed_node_index: usize,
    pub(super) active_node_ids: &'a mut BTreeSet<String>,
    pub(super) variable_pool: &'a mut Map<String, Value>,
    pub(super) pending_failure: &'a mut Option<NodeExecutionFailure>,
    pub(super) node: &'a CompiledNode,
    pub(super) output_payload: &'a Value,
    pub(super) error_payload: Value,
    pub(super) allow_terminal_template_fallback: bool,
}

pub(super) fn apply_node_error_policy(
    application: NodeErrorPolicyApplication<'_>,
) -> Result<Option<NodeExecutionFailure>> {
    let NodeErrorPolicyApplication {
        plan,
        failed_node_index,
        active_node_ids,
        variable_pool,
        pending_failure,
        node,
        output_payload,
        error_payload,
        allow_terminal_template_fallback,
    } = application;
    let failure = NodeExecutionFailure {
        node_id: node.node_id.clone(),
        node_alias: node.alias.clone(),
        error_payload: error_payload.clone(),
    };

    match node_error_policy(node) {
        NodeErrorPolicy::DefaultValue => {
            let default_output_payload = configured_default_output_payload(node);
            variable_pool.insert(
                node.node_id.clone(),
                project_node_variable_payload(node, &default_output_payload)?,
            );
            activate_downstream_nodes(plan, active_node_ids, node, None);
            Ok(None)
        }
        NodeErrorPolicy::ErrorBranch => {
            variable_pool.insert(
                node.node_id.clone(),
                project_node_variable_payload(node, output_payload)?,
            );
            if activate_downstream_nodes(
                plan,
                active_node_ids,
                node,
                Some(ERROR_BRANCH_SOURCE_HANDLE),
            ) {
                return Ok(None);
            }

            Ok(Some(failure))
        }
        NodeErrorPolicy::None => {
            if allow_terminal_template_fallback {
                variable_pool.insert(
                    node.node_id.clone(),
                    project_node_variable_payload(node, output_payload)?,
                );
                let mut next_active_node_ids = active_node_ids.clone();
                activate_downstream_nodes(plan, &mut next_active_node_ids, node, None);
                if can_continue_to_terminal_template_nodes(
                    plan,
                    failed_node_index,
                    &next_active_node_ids,
                ) {
                    *active_node_ids = next_active_node_ids;
                    *pending_failure = Some(failure);
                    return Ok(None);
                }
            }

            Ok(Some(failure))
        }
    }
}

fn configured_default_output_payload(node: &CompiledNode) -> Value {
    match error_default_output(node) {
        Some(value @ Value::Object(_)) => value,
        Some(value) => json!({ first_output_key(node): value }),
        None => json!({}),
    }
}
