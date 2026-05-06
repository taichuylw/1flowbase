use anyhow::{anyhow, Result};

pub(super) fn next_node_index(
    compiled_plan: &orchestration_runtime::compiled_plan::CompiledPlan,
    node_id: &str,
) -> Result<usize> {
    let index = compiled_plan
        .topological_order
        .iter()
        .position(|value| value == node_id)
        .ok_or_else(|| anyhow!("compiled node missing from topological order: {node_id}"))?;

    Ok(index + 1)
}

pub(super) fn first_output_key(
    node: &orchestration_runtime::compiled_plan::CompiledNode,
) -> String {
    node.outputs
        .first()
        .map(|output| output.key.clone())
        .unwrap_or_else(|| "output".to_string())
}
