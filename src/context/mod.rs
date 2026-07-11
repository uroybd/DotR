use std::{
    collections::HashMap,
    env, fs,
    io::{self},
    path::{Path, PathBuf},
};

use serde::Serialize;
use toml::Table;

use crate::{
    config::Config,
    profile::Profile,
    prompt_store,
    utils::{LogLevel, cprintln},
};

#[cfg(test)]
mod tests;

/// Machine-local override for the Bitwarden note name, checked as an env
/// var first, then as a key in `.uservariables.toml` - same name either
/// way, so it's one thing to remember.
const BITWARDEN_NOTE_OVERRIDE_KEY: &str = "DOTR_BITWARDEN_NOTE";

#[derive(Debug, Clone, Serialize)]
pub struct Context {
    pub working_dir: PathBuf,
    variables: Table,
    user_variables: Table,
    pub profile: Profile,
    pub symlink: bool,
    /// A backend's `get_session()` snapshot, round-tripped through
    /// `get_backend` so a `bw unlock` session survives across the
    /// per-call backend reconstructions in this `Context`'s lifetime.
    /// Skipped from `Serialize` since it can carry a live credential.
    #[serde(skip)]
    session: Option<Table>,
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

    fn get_backend(
        profile: &Profile,
        conf: &Config,
        user_variables: &Table,
        session: Option<&Table>,
    ) -> Box<dyn prompt_store::PromptStoreBackend> {
        let backend_type = profile
            .prompt_backend
            .or(conf.prompt_backend)
            .unwrap_or_default();
        let mut backend: Box<dyn prompt_store::PromptStoreBackend> = match backend_type {
            prompt_store::PromptBackendType::File => Box::new(prompt_store::FileStore::new()),
            prompt_store::PromptBackendType::Keychain => {
                Box::new(prompt_store::KeychainStore::new())
            }
            prompt_store::PromptBackendType::Bitwarden => {
                let note = resolve_bitwarden_note(
                    env::var(BITWARDEN_NOTE_OVERRIDE_KEY).ok(),
                    user_variables,
                    profile.bitwarden_note.as_deref(),
                    conf.bitwarden_note.as_deref(),
                );
                Box::new(prompt_store::BitwardenStore::new(note))
            }
        };
        backend.set_session(session.cloned());
        backend
    }

    pub fn get_prompts(
        profile: &Profile,
        conf: &Config,
        packages: &Option<Vec<String>>,
    ) -> HashMap<String, String> {
        // config -> profile -> package, later wins.
        let mut prompts = conf.prompts.clone();
        for (key, prompt) in profile.prompts.iter() {
            prompts.insert(key.clone(), prompt.clone());
        }
        if let Ok(filtered_packages) = conf.filter_packages(profile, packages, false) {
            for package in filtered_packages.values() {
                for (key, prompt) in package.prompts.iter() {
                    prompts.insert(key.clone(), prompt.clone());
                }
            }
        }
        prompts
    }

    pub(crate) fn get_prompted_variables_with_io<R: io::BufRead, W: io::Write>(
        &mut self,
        conf: &Config,
        packages: &Option<Vec<String>>,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<Table, anyhow::Error> {
        // config -> profile -> package, later wins.
        let prompts = Self::get_prompts(&self.profile, conf, packages);
        let missing = prompts
            .iter()
            .filter(|(key, _)| !self.user_variables.contains_key(*key));
        let mut store = self.user_variables.clone();
        let mut dirty = false;

        for (key, message) in missing {
            match get_prompted_variables(message, &mut *reader, &mut *writer) {
                Ok(input) => {
                    store.insert(key.clone(), toml::Value::String(input));
                    dirty = true;
                }
                Err(e) => {
                    cprintln(
                        &format!("Error getting prompted variable '{}': {}", key, e),
                        &LogLevel::Warning,
                    );
                }
            }
        }

        if dirty {
            let mut backend = Self::get_backend(
                &self.profile,
                conf,
                &self.user_variables,
                self.session.as_ref(),
            );
            backend.save(&self.working_dir, &store)?;
            self.session = backend.get_session();
            self.user_variables = store.clone();
        }
        Ok(store)
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
        packages: &Option<Vec<String>>,
        create_profile_if_missing: bool,
    ) -> Result<(Self, bool), anyhow::Error> {
        let mut variables = conf.variables.clone();
        for (key, value) in std::env::vars() {
            variables.insert(key, toml::Value::String(value));
        }
        // .uservariables.toml is always plaintext on disk regardless of the
        // configured backend, so DOTR_PROFILE / DOTR_BITWARDEN_NOTE
        // overrides live here specifically - profile (and therefore
        // backend) selection needs to read them before it knows which
        // backend to ask.
        let raw_user_variables = Self::parse_uservariables(working_dir)?;
        let mut all_variables = variables.clone();
        all_variables.extend(raw_user_variables.clone());
        let (profile, created) = Self::get_profile_from_config(
            conf,
            profile_name,
            create_profile_if_missing,
            &all_variables,
        )?;
        let prompt_keys = Self::get_prompts(&profile, conf, packages)
            .keys()
            .cloned()
            .collect::<Vec<String>>();
        let mut backend = Self::get_backend(&profile, conf, &raw_user_variables, None);
        let user_variables = backend.get(working_dir, &prompt_keys)?;
        let session = backend.get_session();
        Ok((
            Self {
                working_dir: working_dir.to_path_buf(),
                variables,
                user_variables,
                profile,
                symlink: conf.symlink,
                session,
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

/// env override -> .uservariables.toml override -> profile -> config ->
/// built-in default. Pure and env-free by design (the caller passes the
/// already-read env var in), so the precedence chain is directly testable
/// without mutating process env.
fn resolve_bitwarden_note(
    env_override: Option<String>,
    user_variables: &Table,
    profile_note: Option<&str>,
    config_note: Option<&str>,
) -> String {
    use crate::prompt_store::DEFAULT_BITWARDEN_NOTE;

    env_override
        .or_else(|| {
            user_variables
                .get(BITWARDEN_NOTE_OVERRIDE_KEY)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| profile_note.map(|s| s.to_string()))
        .or_else(|| config_note.map(|s| s.to_string()))
        .unwrap_or_else(|| DEFAULT_BITWARDEN_NOTE.to_string())
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
