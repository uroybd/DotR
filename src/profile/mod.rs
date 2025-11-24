use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::{Table, Value};

use crate::utils::{
    get_string_hashmap_from_value, get_vec_string_from_value, string_hashmap_to_toml_table,
    vec_string_to_toml_array,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub variables: Table,
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub prompts: HashMap<String, String>, // Profile-level prompts
}

impl Profile {
    pub fn new(name: &str) -> Self {
        Profile {
            name: name.to_string(),
            variables: Table::new(),
            dependencies: Vec::new(),
            prompts: HashMap::new(),
        }
    }

    pub fn from_table(name: &str, table: &Table) -> anyhow::Result<Self, anyhow::Error> {
        let variables = match table.get("variables") {
            Some(var_block) => var_block
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("The 'variables' field must be a table"))?
                .clone(),
            None => Table::new(),
        };
        let dependencies = get_vec_string_from_value(table.get("dependencies"))?;
        let prompts = get_string_hashmap_from_value(table.get("prompts"))
            .expect("The 'prompts' field must be a table of string to string mappings");
        Ok(Self {
            name: name.to_string(),
            variables,
            dependencies,
            prompts,
        })
    }

    pub fn to_table(&self) -> Table {
        let mut table = Table::new();
        if !self.variables.is_empty() {
            table.insert(
                "variables".to_string(),
                Value::Table(self.variables.clone()),
            );
        }
        table.insert(
            "dependencies".to_string(),
            vec_string_to_toml_array(&self.dependencies),
        );

        if !self.prompts.is_empty() {
            table.insert(
                "prompts".to_string(),
                string_hashmap_to_toml_table(&self.prompts),
            );
        }

        table
    }
}
