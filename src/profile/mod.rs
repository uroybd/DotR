use serde::{Deserialize, Serialize};
use toml::{Table, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub variables: Table,
    pub dependencies: Vec<String>,
}

impl Profile {
    pub fn new(name: &str) -> Self {
        Profile {
            name: name.to_string(),
            variables: Table::new(),
            dependencies: Vec::new(),
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

        Ok(Self {
            name: name.to_string(),
            variables,
            dependencies,
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
        table
    }
}
