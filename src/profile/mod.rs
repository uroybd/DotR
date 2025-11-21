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

    pub fn from_table(name: &str, table: &Table) -> Self {
        let mut variables = Table::new();
        if let Some(vars) = table.get("variables").and_then(|v| v.as_table()) {
            variables = vars.clone();
        }
        let mut dependencies = Vec::new();
        if let Some(deps) = table.get("dependencies").and_then(|d| d.as_array()) {
            for dep in deps {
                if let Some(dep_str) = dep.as_str() {
                    dependencies.push(dep_str.to_string());
                }
            }
        }
        Self {
            name: name.to_string(),
            variables,
            dependencies,
        }
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
