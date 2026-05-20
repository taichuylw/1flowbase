use serde_json::Value;

use crate::errors::ControlPlaneError;

#[derive(Debug, Clone)]
pub struct ResourceBatchSelection {
    pub filter_by_tk: Option<Value>,
    pub filter: Option<Value>,
}

impl ResourceBatchSelection {
    pub fn new(filter_by_tk: Option<Value>, filter: Option<Value>) -> Self {
        Self {
            filter_by_tk,
            filter,
        }
    }
}

pub fn filter_by_tk_values(filter_by_tk: Value) -> Result<Vec<String>, ControlPlaneError> {
    match filter_by_tk {
        Value::Array(items) => items
            .into_iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .ok_or(ControlPlaneError::InvalidInput("filterByTk"))
            })
            .collect(),
        Value::String(value) => Ok(vec![value]),
        _ => Err(ControlPlaneError::InvalidInput("filterByTk")),
    }
}
