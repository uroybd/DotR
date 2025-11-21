use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use toml::Table;

#[derive(Debug, Clone, Serialize)]
pub struct Context {
    pub working_dir: PathBuf,
    pub variables: Table,
}

impl Context {
    pub fn get_variable(&self, key: &str) -> Option<&toml::Value> {
        self.variables.get(key)
    }

    pub fn parse_uservariables(cwd: &Path) -> Table {
        let path = cwd.join(".uservariables.toml");
        if path.exists() {
            let content = fs::read_to_string(&path).expect("Failed to read .uservariables.toml");
            toml::de::from_str(&content).unwrap_or_else(|e| {
                eprintln!(
                    "Failed to parse .uservariables.toml at '{}': {}",
                    path.display(),
                    e
                );
                Table::new()
            })
        } else {
            Table::new()
        }
    }
    pub fn new(working_dir: &Path) -> Self {
        let mut variables = Table::new();
        for (key, value) in std::env::vars() {
            variables.insert(key, toml::Value::String(value));
        }

        Self {
            working_dir: working_dir.to_path_buf(),
            variables,
        }
    }
    pub fn apply_variables_with_user_overrides(&mut self, new_vars: Table) {
        self.variables.extend(new_vars);
        self.variables
            .extend(Self::parse_uservariables(&self.working_dir));
    }

    pub fn print_variables(&self) {
        println!("User Variables:");
        if self.variables.is_empty() {
            println!("  (none)");
        } else {
            for (key, value) in self.variables.iter() {
                print_variable(key, value, 1);
            }
        }
    }
}

pub fn print_variable(key: &str, value: &toml::Value, level: usize) {
    let indent = "  ".repeat(level);
    match value {
        toml::Value::String(s) => {
            println!("{}{} = {}", indent, key, s);
        }
        toml::Value::Integer(i) => {
            println!("{}{} = {}", indent, key, i);
        }
        toml::Value::Float(f) => {
            println!("{}{} = {}", indent, key, f);
        }
        toml::Value::Boolean(b) => {
            println!("{}{} = {}", indent, key, b);
        }
        toml::Value::Table(t) => {
            println!("{}{} =", indent, key);
            for (k, v) in t.iter() {
                print_variable(k, v, level + 1);
            }
        }
        toml::Value::Array(arr) => {
            println!("{}{} = [", indent, key);
            for v in arr.iter() {
                let item_indent = "  ".repeat(level + 1);
                match v {
                    toml::Value::String(s) => {
                        println!("{}- {}", item_indent, s);
                    }
                    toml::Value::Integer(i) => {
                        println!("{}- {}", item_indent, i);
                    }
                    toml::Value::Float(f) => {
                        println!("{}- {}", item_indent, f);
                    }
                    toml::Value::Boolean(b) => {
                        println!("{}- {}", item_indent, b);
                    }
                    toml::Value::Table(_) | toml::Value::Array(_) => {
                        println!("{}-", item_indent);
                        print_variable("", v, level + 2);
                    }
                    _ => {
                        println!("{}- {:?}", item_indent, v);
                    }
                }
            }
            println!("{}]", indent);
        }
        _ => {
            println!("{}{} = {:?}", indent, key, value);
        }
    }
}
