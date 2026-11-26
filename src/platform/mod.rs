use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::utils::{get_vec_string_from_value, is_empty_table};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Platform {
    #[serde(skip)]
    pub name: String,
    #[serde(skip_serializing_if = "is_empty_table")]
    pub variables: Table,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pre_actions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub post_actions: Vec<String>,
}

impl Platform {
    pub fn new(name: &str) -> Self {
        Platform {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn from_table(name: &str, table: &Table) -> anyhow::Result<Self> {
        let variables = crate::utils::get_table_from_value(table.get("variables"), "variables")?;
        let pre_actions = get_vec_string_from_value(table.get("pre_actions"))?;
        let post_actions = get_vec_string_from_value(table.get("post_actions"))?;
        Ok(Self {
            name: name.to_string(),
            variables,
            pre_actions,
            post_actions,
        })
    }
}

pub fn get_platforms_from_table(
    value: Option<&toml::Value>,
) -> anyhow::Result<HashMap<String, Platform>> {
    let mut platforms: HashMap<String, Platform> = HashMap::new();
    for (key, val) in &crate::utils::get_table_from_value(value, "platforms")? {
        let val_table = val
            .as_table()
            .ok_or_else(|| anyhow::anyhow!("Platform '{key}' must be a table"))?;
        let platform = Platform::from_table(key, val_table)?;
        platforms.insert(platform.name.clone(), platform);
    }
    Ok(platforms)
}
