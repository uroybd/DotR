use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::utils::is_empty_table;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Platform {
    #[serde(skip)]
    pub name: String,
    #[serde(skip_serializing_if = "is_empty_table")]
    pub variables: Table,
}

impl Platform {
    pub fn new(name: &str) -> Self {
        Platform {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn from_table(name: &str, table: &Table) -> anyhow::Result<Self> {
        let variables = match table.get("variables") {
            Some(var_block) => var_block
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("The 'variables' field must be a table"))?
                .clone(),
            None => Table::new(),
        };
        Ok(Self {
            name: name.to_string(),
            variables,
        })
    }
}

pub fn get_platforms_from_table(
    value: Option<&toml::Value>,
) -> anyhow::Result<HashMap<String, Platform>> {
    let mut platforms: HashMap<String, Platform> = HashMap::new();
    if let Some(table) = value.and_then(|v| v.as_table()) {
        for (key, val) in table.iter() {
            let val_table = val
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("Platform '{}' must be a table", key))?;
            let platform = Platform::from_table(key, val_table)?;
            platforms.insert(platform.name.clone(), platform);
        }
    }
    Ok(platforms)
}
