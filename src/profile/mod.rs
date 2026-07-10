use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Table;

use crate::prompt_store::PromptBackend;
use crate::utils::{get_string_hashmap_from_value, get_vec_string_from_value, is_empty_table};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Profile {
    #[serde(skip)]
    pub name: String,
    #[serde(skip_serializing_if = "is_empty_table")]
    pub variables: Table,
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub prompts: HashMap<String, String>,
    /// Default backend for this profile's prompts. Overrides
    /// `Config.prompt_backend` when this profile is active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_backend: Option<PromptBackend>,
    /// Overrides `Config.bitwarden_note` when this profile is active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitwarden_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
}

impl Profile {
    pub fn new(name: &str) -> Self {
        Profile {
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
        let dependencies = get_vec_string_from_value(table.get("dependencies"))?;
        let prompts = get_string_hashmap_from_value(table.get("prompts"))?;
        let prompt_backend = table
            .get("prompt_backend")
            .and_then(|v| v.as_str())
            .map(PromptBackend::parse)
            .transpose()?;
        let bitwarden_note = table
            .get("bitwarden_note")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let platform = table
            .get("platform")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok(Self {
            name: name.to_string(),
            variables,
            dependencies,
            prompts,
            prompt_backend,
            bitwarden_note,
            platform,
        })
    }
}
