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

const DOTR_PROFILE_KEY: &str = "DOTR_PROFILE";

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
    ) -> anyhow::Result<Table> {
        self.get_prompted_variables_with_io(
            conf,
            packages,
            &mut std::io::stdin().lock(),
            &mut std::io::stdout(),
        )
    }

    // `variables` here is `self.variables` - it already has
    // DOTR_BITWARDEN_NOTE fully resolved and folded in (see
    // `Context::new`), so there's no need to re-resolve it here.
    fn get_backend(
        profile: &Profile,
        conf: &Config,
        variables: &Table,
    ) -> Box<dyn prompt_store::PromptStoreBackend> {
        let backend_type = profile
            .prompt_backend
            .or(conf.prompt_backend)
            .unwrap_or_default();
        match backend_type {
            prompt_store::PromptBackendType::File => Box::new(prompt_store::FileStore::new()),
            prompt_store::PromptBackendType::Keychain => {
                Box::new(prompt_store::KeychainStore::new())
            }
            prompt_store::PromptBackendType::Bitwarden => {
                let note = variables
                    .get(BITWARDEN_NOTE_OVERRIDE_KEY)
                    .and_then(|v| v.as_str())
                    .unwrap_or(prompt_store::DEFAULT_BITWARDEN_NOTE)
                    .to_string();
                Box::new(prompt_store::BitwardenStore::new(note))
            }
        }
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

    fn get_prompted_variables_with_io<R: io::BufRead, W: io::Write>(
        &mut self,
        conf: &Config,
        packages: &Option<Vec<String>>,
        reader: &mut R,
        writer: &mut W,
    ) -> Result<Table, anyhow::Error> {
        // config -> profile -> package, later wins.
        let prompts = Self::get_prompts(&self.profile, conf, packages);
        let missing_keys: Vec<String> = prompts
            .keys()
            .filter(|key| !self.user_variables.contains_key(*key))
            .cloned()
            .collect();

        let mut store = self.user_variables.clone();
        if missing_keys.is_empty() {
            return Ok(store);
        }

        // Only reach for the configured backend once we know something is
        // actually missing from what `Context::new` already loaded from
        // .uservariables.toml - so e.g. `dotr packages list` never touches
        // Keychain or triggers a Bitwarden unlock just because a Context
        // was constructed.
        let mut backend = Self::get_backend(&self.profile, conf, &self.variables);
        let existing = backend.get(&self.working_dir, &missing_keys)?;
        let mut dirty = false;

        for key in &missing_keys {
            if let Some(value) = existing.get(key) {
                store.insert(key.clone(), value.clone());
                continue;
            }
            let message = &prompts[key];
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
            backend.save(&self.working_dir, &store)?;
        }
        self.user_variables = store.clone();
        Ok(store)
    }

    pub fn save_to_uservariables(
        &mut self,
        key: &str,
        val: toml::Value,
    ) -> Result<(), anyhow::Error> {
        let mut on_disk = Self::parse_uservariables(&self.working_dir)?;
        on_disk.insert(key.to_string(), val.clone());
        let toml_string = toml::to_string(&on_disk)?;
        let path = self.working_dir.join(".uservariables.toml");
        fs::write(&path, toml_string)?;
        self.user_variables.insert(key.to_string(), val);
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
        // .uservariables.toml is always plaintext on disk, regardless of
        // the configured backend - reading it is always cheap and never
        // touches Keychain or triggers a Bitwarden unlock, unlike actually
        // constructing those backends (which stays lazy, deferred to
        // `get_prompted_variables_with_io`). So the whole file becomes
        // `user_variables` unconditionally here: users can define any
        // variable in it directly (not just answers to declared prompts),
        // same as before - it's only *prompted* variables that route to
        // whichever backend is configured instead of landing here when
        // that backend isn't `file`. DOTR_PROFILE / DOTR_BITWARDEN_NOTE
        // bootstrap overrides are also read from here, since profile (and
        // therefore backend) selection needs them before it knows which
        // backend to ask.
        let user_variables = Self::parse_uservariables(working_dir)?;
        // DOTR_PROFILE: the file wins over the environment if both are set,
        // for *resolving* which profile is active below - `variables` gets
        // overwritten right after with whichever profile actually ends up
        // active (which also correctly reflects an explicit `--profile`,
        // overriding both env and file). That's what makes it always
        // available in templates, even when nothing overrides it anywhere
        // and `default` is used implicitly - it wouldn't otherwise exist.
        if let Some(value) = user_variables.get(DOTR_PROFILE_KEY) {
            variables.insert(DOTR_PROFILE_KEY.to_string(), value.clone());
        }
        if let Some(platform_vars) = conf.platforms.get(&conf.platform) {
            variables.extend(platform_vars.variables.clone());
        }
        let (profile, created) = Self::get_profile_from_config(
            conf,
            profile_name,
            create_profile_if_missing,
            &variables,
        )?;
        variables.insert(
            DOTR_PROFILE_KEY.to_string(),
            toml::Value::String(profile.name.clone()),
        );
        // DOTR_BITWARDEN_NOTE: resolved fully now, through the same chain
        // `get_backend` used to run lazily - and always stored, even when
        // Bitwarden isn't the active backend at all, for the same
        // always-available-in-templates reason as DOTR_PROFILE above.
        let bitwarden_note = resolve_bitwarden_note(
            env::var(BITWARDEN_NOTE_OVERRIDE_KEY).ok(),
            &user_variables,
            profile.bitwarden_note.as_deref(),
            conf.bitwarden_note.as_deref(),
        );
        variables.insert(
            BITWARDEN_NOTE_OVERRIDE_KEY.to_string(),
            toml::Value::String(bitwarden_note),
        );
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
                    .get(DOTR_PROFILE_KEY)
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
