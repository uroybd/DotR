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
        let variables = crate::utils::get_table_from_value(table.get("variables"), "variables")?;
        let dependencies = get_vec_string_from_value(table.get("dependencies"))?;
        let prompts = get_string_hashmap_from_value(table.get("prompts"))?;
        let prompt_backend =
            crate::utils::get_opt_string_from_value(table.get("prompt_backend"), "prompt_backend")?
                .as_deref()
                .map(PromptBackendType::parse)
                .transpose()?;
        let bitwarden_note =
            crate::utils::get_opt_string_from_value(table.get("bitwarden_note"), "bitwarden_note")?;
        let platform = crate::utils::get_opt_string_from_value(table.get("platform"), "platform")?;
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
