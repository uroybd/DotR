use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::{Table, Value};

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

    pub fn from_table(name: &str, table: &Table) -> Result<Self, anyhow::Error> {
        let mut variables = Table::new();
        if let Some(vars) = table.get("variables") {
            variables = vars
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("Profile '{}' variables must be a table", name))?
                .clone();
        }

        let mut dependencies = Vec::new();
        if let Some(deps) = table.get("dependencies") {
            let deps_array = deps.as_array().ok_or_else(|| {
                anyhow::anyhow!("Profile '{}' dependencies must be an array", name)
            })?;
            for dep in deps_array {
                let dep_str = dep.as_str().ok_or_else(|| {
                    anyhow::anyhow!("Profile '{}' dependency must be a string", name)
                })?;
                dependencies.push(dep_str.to_string());
            }
        }

        let mut prompts = HashMap::new();
        if let Some(prompts_block) = table.get("prompts") {
            let prompts_table = prompts_block
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("Profile '{}' prompts must be a table", name))?;
            for (key, value) in prompts_table {
                let prompt_str = value.as_str().ok_or_else(|| {
                    anyhow::anyhow!("Profile '{}' prompt message must be a string", name)
                })?;
                prompts.insert(key.clone(), prompt_str.to_string());
            }
        }

        Ok(Self {
            name: name.to_string(),
            variables,
            dependencies,
            prompts,
        })
    }

    pub fn to_table(&self) -> Table {
        let mut table = Table::new();
        table.insert(
            "variables".to_string(),
            Value::Table(self.variables.clone()),
        );
        let deps: Vec<Value> = self
            .dependencies
            .iter()
            .map(|d| Value::String(d.clone()))
            .collect();
        table.insert("dependencies".to_string(), Value::Array(deps));

        if !self.prompts.is_empty() {
            let mut prompts_table = Table::new();
            for (key, value) in &self.prompts {
                prompts_table.insert(key.clone(), Value::String(value.clone()));
            }
            table.insert("prompts".to_string(), Value::Table(prompts_table));
        }

        table
    }
}
