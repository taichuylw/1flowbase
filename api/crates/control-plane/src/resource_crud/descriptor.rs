use serde_json::Value;

use super::{
    batch::{filter_by_tk_values, ResourceBatchSelection},
    filter::filter_resource_records,
};
use crate::errors::ControlPlaneError;

pub trait ResourceFilterTarget {
    fn field_value(&self, field: &str) -> Option<String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceCrudDescriptor {
    pub resource_code: &'static str,
    pub primary_key: &'static str,
}

impl ResourceCrudDescriptor {
    pub const fn new(resource_code: &'static str, primary_key: &'static str) -> Self {
        Self {
            resource_code,
            primary_key,
        }
    }

    pub fn filter_records<T: ResourceFilterTarget>(
        &self,
        records: Vec<T>,
        filter: Option<&Value>,
    ) -> Result<Vec<T>, ControlPlaneError> {
        filter_resource_records(records, filter)
    }

    pub fn select_batch_ids<T, Id, ParseId, GetId>(
        &self,
        records: Vec<T>,
        selection: ResourceBatchSelection,
        parse_id: ParseId,
        get_id: GetId,
    ) -> Result<Vec<Id>, ControlPlaneError>
    where
        T: ResourceFilterTarget,
        ParseId: Fn(String) -> Result<Id, ControlPlaneError>,
        GetId: Fn(&T) -> Id,
    {
        if let Some(filter_by_tk) = selection.filter_by_tk {
            return filter_by_tk_values(filter_by_tk)?
                .into_iter()
                .map(parse_id)
                .collect();
        }

        let Some(filter) = selection.filter else {
            return Err(ControlPlaneError::InvalidInput("filterByTk"));
        };
        if filter.as_object().map_or(true, |object| object.is_empty()) {
            return Err(ControlPlaneError::InvalidInput("filter"));
        }

        let filtered_records = self.filter_records(records, Some(&filter))?;
        Ok(filtered_records.iter().map(get_id).collect())
    }
}
