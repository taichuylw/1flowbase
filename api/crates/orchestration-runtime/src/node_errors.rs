use serde_json::{json, Value};

pub const NODE_TYPE_NOT_IMPLEMENTED_ERROR_CODE: &str = "node_type_not_implemented";

pub fn build_node_type_not_implemented_error_payload(
    node_type: &str,
    runtime: &str,
) -> Value {
    json!({
        "error_code": NODE_TYPE_NOT_IMPLEMENTED_ERROR_CODE,
        "node_type": node_type,
        "message": format!("{node_type} nodes are not implemented in {runtime} runtime"),
    })
}
