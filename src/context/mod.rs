use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
};

use serde::Serialize;
use toml::Table;

use crate::{
    config::Config,
    profile::Profile,
    utils::{LogLevel, cprintln},
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Serialize)]
pub struct Context {
    pub working_dir: PathBuf,
    variables: Table,
    user_variables: Table,
    pub profile: Profile,
    pub symlink: bool,
}

impl Context {
    pub fn get_variable(&self, key: &str) -> Option<&toml::Value> {
        self.variables.get(key)
    }

    pub fn get_user_variable(&self, key: &str) -> Option<&toml::Value> {
        self.user_variables.get(key)
    }

    pub fn get_profile_variable(&self, key: &str) -> Option<&toml::Value> {
        self.profile.variables.get(key)
    }

    pub fn get_context_variable(&self, key: &str) -> Option<&toml::Value> {
        self.get_user_variable(key).or_else(|| {
            self.get_profile_variable(key)
                .or_else(|| self.get_variable(key))
        })
    }

    pub fn set_profile(&mut self, profile: Profile) {
        self.profile = profile;
    }

    pub fn get_prompted_variables(
        &mut self,
        conf: &Config,
        packages: &Option<Vec<String>>,
    ) -> anyhow::Result<()> {
        self.get_prompted_variables_with_io(
            conf,
            packages,
            &mut std::io::stdin().lock(),
            &mut std::io::stdout(),
        )?;
        Ok(())
    }

    pub(crate) fn get_prompted_variables_with_io<R: io::BufRead, W: io::Write>(
        &mut self,
        conf: &Config,
        packages: &Option<Vec<String>>,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<Table, anyhow::Error> {
        use crate::prompt_store::{
            BitwardenStore, DEFAULT_BITWARDEN_NOTE, FileStore, KeychainStore, PromptBackend,
            PromptStore,
        };

        // config -> profile -> package, later wins.
        let mut prompts = conf.prompts.clone();
        for (key, prompt) in self.profile.prompts.iter() {
            prompts.insert(key.clone(), prompt.clone());
        }
        if let Ok(filtered_packages) = conf.filter_packages(self, packages, false) {
            for (_, package) in filtered_packages.iter() {
                for (key, prompt) in package.prompts.iter() {
                    prompts.insert(key.clone(), prompt.clone());
                }
            }
        }

        // Built once, reused per key — file_store especially, since each
        // `set` rewrites the whole file from its own in-memory cache.
        let file_store = FileStore::new(self.working_dir.clone(), self.user_variables.clone());
        let keychain_store = KeychainStore::new(&self.working_dir);
        let bitwarden_store = BitwardenStore::new(
            self.profile
                .bitwarden_note
                .clone()
                .or_else(|| conf.bitwarden_note.clone())
                .unwrap_or_else(|| DEFAULT_BITWARDEN_NOTE.to_string()),
        );

        // Every resolved value, any backend, for templating this run. Only
        // file_store's own table (below) ever reaches .uservariables.toml.
        let mut resolved = self.user_variables.clone();

        // The active profile's default backend wins; else the repo's;
        // else `File`. This is a policy choice, not a per-prompt one.
        let backend = self
            .profile
            .prompt_backend
            .or(conf.prompt_backend)
            .unwrap_or_default();
        let store: &dyn PromptStore = match backend {
            PromptBackend::File => &file_store,
            PromptBackend::Keychain => &keychain_store,
            PromptBackend::Bitwarden => &bitwarden_store,
        };

        for (key, message) in prompts.iter() {
            if resolved.contains_key(key) {
                continue;
            }

            let existing = match store.get(key) {
                Ok(existing) => existing,
                Err(e) => {
                    cprintln(
                        &format!(
                            "Error reading prompted variable '{}' from the {} backend: {}",
                            key,
                            backend.as_str(),
                            e
                        ),
                        &LogLevel::Warning,
                    );
                    continue;
                }
            };

            let value = match existing {
                Some(value) => value,
                None => match get_prompted_variables(message, &mut *reader, &mut *writer) {
                    Ok(input) => {
                        // File keeps its historical untrimmed value for
                        // backward compatibility; other backends trim,
                        // since a stray newline would break a real secret.
                        let value = if backend == PromptBackend::File {
                            input
                        } else {
                            input.trim().to_string()
                        };
                        if let Err(e) = store.set(key, &value) {
                            cprintln(
                                &format!(
                                    "Error saving prompted variable '{}' to the {} backend: {}",
                                    key,
                                    backend.as_str(),
                                    e
                                ),
                                &LogLevel::Warning,
                            );
                            continue;
                        }
                        value
                    }
                    Err(e) => {
                        cprintln(
                            &format!("Error getting prompted variable '{}': {}", key, e),
                            &LogLevel::Warning,
                        );
                        continue;
                    }
                },
            };
            resolved.insert(key.clone(), toml::Value::String(value));
        }

        self.user_variables = file_store.into_table();
        Ok(resolved)
    }

    pub fn save_to_uservariables(
        &mut self,
        key: &str,
        val: toml::Value,
    ) -> Result<(), anyhow::Error> {
        let mut user_vars = self.user_variables.clone();
        user_vars.insert(key.to_string(), val);
        let toml_string = toml::to_string(&user_vars)?;
        self.user_variables = user_vars;
        let path = self.working_dir.join(".uservariables.toml");
        fs::write(&path, toml_string)?;
        Ok(())
    }

    pub fn parse_uservariables(cwd: &Path) -> Result<Table, anyhow::Error> {
        let path = cwd.join(".uservariables.toml");
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let table: Table = toml::de::from_str(&content).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse .uservariables.toml at '{}': {}",
                    path.display(),
                    e
                )
            })?;
            Ok(table)
        } else {
            Ok(Table::new())
        }
    }

    pub fn new(
        working_dir: &Path,
        conf: &Config,
        profile_name: &Option<String>,
        create_profile_if_missing: bool,
    ) -> Result<(Self, bool), anyhow::Error> {
        let mut variables = conf.variables.clone();
        for (key, value) in std::env::vars() {
            variables.insert(key, toml::Value::String(value));
        }
        // User variables file must parse correctly if it exists
        let user_variables = Self::parse_uservariables(working_dir)?;
        let mut all_variables = variables.clone();
        all_variables.extend(user_variables.clone());
        let (profile, created) = Self::get_profile_from_config(
            conf,
            profile_name,
            create_profile_if_missing,
            &all_variables,
        )?;
        Ok((
            Self {
                working_dir: working_dir.to_path_buf(),
                variables,
                user_variables,
                profile,
                symlink: conf.symlink,
            },
            created,
        ))
    }

    pub fn get_profile_from_config(
        conf: &Config,
        pname: &Option<String>,
        create_if_missing: bool,
        variables: &Table,
    ) -> anyhow::Result<(Profile, bool)> {
        let profile_name = match pname {
            Some(name) => name.clone(),
            None => {
                if let Ok(env_p_name) = variables
                    .get("DOTR_PROFILE")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("DOTR_PROFILE variable must be a string"))
                {
                    env_p_name.to_string()
                } else {
                    "default".to_string()
                }
            }
        };

        let profile = conf.profiles.get(&profile_name);
        if let Some(prof) = profile {
            return Ok((prof.clone(), false));
        } else if !create_if_missing && profile_name != "default" {
            anyhow::bail!("Profile {} not found", profile_name);
        }
        Ok((Profile::new(&profile_name), true))
    }

    pub fn get_variables(&self) -> &Table {
        &self.variables
    }

    pub fn get_user_variables(&self) -> &Table {
        &self.user_variables
    }

    pub fn get_context_variables(&self) -> Table {
        let mut context_vars = self.variables.clone();
        context_vars.extend(self.profile.variables.clone());
        context_vars.extend(self.user_variables.clone());
        context_vars
    }

    pub fn extend_variables(&mut self, new_vars: Table) {
        self.variables.extend(new_vars);
    }

    pub fn print_variables(&self) {
        let variables = &self.get_context_variables();
        println!("User Variables:");
        if variables.is_empty() {
            println!("  (none)");
        } else {
            for (key, value) in variables.iter() {
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

fn get_prompted_variables<R: io::BufRead, W: io::Write>(
    prompt: &str,
    mut reader: R,
    mut writer: W,
) -> anyhow::Result<String> {
    // Prompt the user for input
    writer.write_all(format!("{}\n>>> ", prompt).as_bytes())?;
    writer.flush()?;
    let mut input = String::new();
    reader.read_line(&mut input)?;
    Ok(input)
}
