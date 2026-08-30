use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use toml::Table;

use crate::context::Context;
use crate::prompt_store::PromptBackendType;
use crate::utils::{get_string_hashmap_from_value, get_vec_string_from_value, is_empty_table};

#[cfg(test)]
mod tests;

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
    pub prompt_backend: Option<PromptBackendType>,
    /// Overrides `Config.bitwarden_note` when this profile is active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitwarden_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<String>,
    #[serde(skip)]
    pub platform_variables: Table,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pre_actions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub post_actions: Vec<String>,
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
            .map(PromptBackendType::parse)
            .transpose()?;
        let bitwarden_note = table
            .get("bitwarden_note")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let platform = table
            .get("platform")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let pre_actions = get_vec_string_from_value(table.get("pre_actions"))?;
        let post_actions = get_vec_string_from_value(table.get("post_actions"))?;
        Ok(Self {
            name: name.to_string(),
            variables,
            dependencies,
            prompts,
            prompt_backend,
            bitwarden_note,
            platform,
            platform_variables: Table::new(),
            pre_actions,
            post_actions,
        })
    }

    pub fn get_context_variables(&self, ctx: &Context) -> Table {
        let mut vars = ctx.get_variables().clone();
        vars.extend(ctx.profile.platform_variables.clone());
        vars.extend(self.variables.clone());
        vars.extend(ctx.get_user_variables().clone());
        vars
    }
}
