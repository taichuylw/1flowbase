use serde_json::Value;

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

pub fn parse_resource_filter(filter: Option<&str>) -> Result<Option<Value>, ControlPlaneError> {
    let Some(filter) = filter.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };

    serde_json::from_str(filter)
        .map(Some)
        .map_err(|_| ControlPlaneError::InvalidInput("filter"))
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

pub fn matches_resource_filter<T: ResourceFilterTarget>(
    target: &T,
    filter: &Value,
) -> Result<bool, ControlPlaneError> {
    let Some(object) = filter.as_object() else {
        return Err(ControlPlaneError::InvalidInput("filter"));
    };

    for (key, value) in object {
        match key.as_str() {
            "$and" => {
                let Some(items) = value.as_array() else {
                    return Err(ControlPlaneError::InvalidInput("filter"));
                };
                let mut matched = true;
                for item in items {
                    matched &= matches_resource_filter(target, item)?;
                }
                if !matched {
                    return Ok(false);
                }
            }
            "$or" => {
                let Some(items) = value.as_array() else {
                    return Err(ControlPlaneError::InvalidInput("filter"));
                };
                let mut matched = false;
                for item in items {
                    matched |= matches_resource_filter(target, item)?;
                }
                if !matched {
                    return Ok(false);
                }
            }
            field => {
                if !matches_field_filter(target, field, value)? {
                    return Ok(false);
                }
            }
        }
    }

    Ok(true)
}

pub fn filter_resource_records<T: ResourceFilterTarget>(
    records: Vec<T>,
    filter: Option<&Value>,
) -> Result<Vec<T>, ControlPlaneError> {
    let Some(filter) = filter else {
        return Ok(records);
    };

    let mut filtered_records = Vec::with_capacity(records.len());
    for record in records {
        if matches_resource_filter(&record, filter)? {
            filtered_records.push(record);
        }
    }
    Ok(filtered_records)
}

fn matches_field_filter<T: ResourceFilterTarget>(
    target: &T,
    field: &str,
    expected: &Value,
) -> Result<bool, ControlPlaneError> {
    let (field, dotted_operator) = field
        .rsplit_once('.')
        .filter(|(_, operator)| operator.starts_with('$'))
        .map_or((field, None), |(field, operator)| (field, Some(operator)));

    if let Some(operator) = dotted_operator {
        return matches_field_operator(target, field, operator, expected);
    }

    if let Some(operators) = expected.as_object() {
        if operators.keys().any(|key| key.starts_with('$')) {
            for (operator, value) in operators {
                if !matches_field_operator(target, field, operator, value)? {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
    }

    matches_field_operator(target, field, "$eq", expected)
}

fn matches_field_operator<T: ResourceFilterTarget>(
    target: &T,
    field: &str,
    operator: &str,
    expected: &Value,
) -> Result<bool, ControlPlaneError> {
    let actual = target.field_value(field);
    let expected_text = value_to_filter_text(expected);

    let matched = match operator {
        "$eq" => actual.as_deref() == Some(expected_text.as_str()),
        "$ne" => actual.as_deref() != Some(expected_text.as_str()),
        "$includes" => actual
            .as_deref()
            .is_some_and(|value| value.to_lowercase().contains(&expected_text.to_lowercase())),
        "$notIncludes" => actual.as_deref().map_or(true, |value| {
            !value.to_lowercase().contains(&expected_text.to_lowercase())
        }),
        "$in" => {
            let Some(values) = expected.as_array() else {
                return Err(ControlPlaneError::InvalidInput("filter"));
            };
            values.iter().any(|value| {
                let value = value_to_filter_text(value);
                actual.as_deref() == Some(value.as_str())
            })
        }
        _ => return Err(ControlPlaneError::InvalidInput("filter")),
    };

    Ok(matched)
}

fn value_to_filter_text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| value.to_string())
}

impl ResourceFilterTarget for domain::ModelDefinitionRecord {
    fn field_value(&self, field: &str) -> Option<String> {
        match field {
            "id" => Some(self.id.to_string()),
            "scope_kind" => Some(self.scope_kind.as_str().to_string()),
            "scope_id" => Some(self.scope_id.to_string()),
            "code" => Some(self.code.clone()),
            "title" => Some(self.title.clone()),
            "status" => Some(self.status.as_str().to_string()),
            "api_exposure_status" => Some(self.api_exposure_status.as_str().to_string()),
            "availability_status" => Some(self.availability_status.as_str().to_string()),
            "data_source_instance_id" => self.data_source_instance_id.map(|id| id.to_string()),
            "source_kind" => Some(self.source_kind.as_str().to_string()),
            "external_resource_key" => self.external_resource_key.clone(),
            "external_table_id" => self.external_table_id.clone(),
            "physical_table_name" => Some(self.physical_table_name.clone()),
            _ => None,
        }
    }
}
