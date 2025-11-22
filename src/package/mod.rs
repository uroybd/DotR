use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::{
    context::Context,
    utils::{BACKUP_EXT, normalize_home_path, resolve_path},
};

static TEMPLATE_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(\{\{[-]?|[-]?\}\}|\{[%][-]?|[-]?%\}|\{[#][-]?|[-]?#\})").unwrap()
});

// A package represents a dotfile package with its source, destination, and dependencies.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Package {
    pub name: String,
    pub src: String,
    pub dest: String,
    pub dependencies: Option<Vec<String>>,
    pub variables: Table,
    pub pre_actions: Vec<String>,
    pub post_actions: Vec<String>,
    pub targets: HashMap<String, String>, // The key is profile name, the value is dest to override.
    pub skip: bool,
}

impl Package {
    // Create a new Package from a given path, used to import dotfiles.
    // The path can be absolute or relative to the current working directory.
    // That path must exist and it will be set to the dest field.
    pub fn from_path(path: &str, cwd: &Path) -> Result<Self, anyhow::Error> {
        let resolved_path = resolve_path(path, cwd);
        if !resolved_path.exists() {
            anyhow::bail!("Path '{}' does not exist", resolved_path.display());
        }
        let package_name = get_package_name(path, cwd);
        let dest_path_str = format!("dotfiles/{}", package_name);

        // Normalize the path: if it already starts with ~, keep it; otherwise convert if in home dir
        let path_str = if path.starts_with('~') {
            path.to_string()
        } else {
            let resolved_str = resolved_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid path: contains non-UTF-8 characters"))?;
            normalize_home_path(resolved_str)
        };

        Ok(Self {
            name: package_name.clone(),
            dest: path_str,
            src: dest_path_str.clone(),
            dependencies: None,
            variables: Table::new(),
            pre_actions: Vec::new(),
            post_actions: Vec::new(),
            targets: HashMap::new(),
            skip: false,
        })
    }

    pub fn from_table(pkg_name: &str, pkg_val: &Table) -> Result<Self, anyhow::Error> {
        let dependencies: Option<Vec<String>> = match pkg_val.get("dependencies") {
            Some(deps) => {
                let array = deps
                    .as_array()
                    .ok_or_else(|| anyhow::anyhow!("Dependencies should be an array"))?;
                let d = array
                    .iter()
                    .map(|d| {
                        d.as_str()
                            .ok_or_else(|| anyhow::anyhow!("Dependency must be a string"))
                            .map(|s| s.to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Some(d)
            }
            None => None,
        };

        let mut variables = Table::new();
        if let Some(var_block) = pkg_val.get("variables") {
            variables = var_block
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("The 'variables' field must be a table"))?
                .clone();
        }

        let mut pre_actions = Vec::new();
        if let Some(pre_block) = pkg_val.get("pre_actions") {
            let array = pre_block
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("The 'pre_actions' field must be an array"))?;
            pre_actions = array
                .iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| anyhow::anyhow!("Pre-action must be a string"))
                        .map(|s| s.to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
        }

        let mut post_actions = Vec::new();
        if let Some(post_block) = pkg_val.get("post_actions") {
            let array = post_block
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("The 'post_actions' field must be an array"))?;
            post_actions = array
                .iter()
                .map(|v| {
                    v.as_str()
                        .ok_or_else(|| anyhow::anyhow!("Post-action must be a string"))
                        .map(|s| s.to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
        }

        let mut targets = HashMap::new();
        if let Some(targets_block) = pkg_val.get("targets") {
            let targets_table = targets_block
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("The 'targets' field must be a table"))?;
            for (key, value) in targets_table {
                let dest_str = value
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Target dest must be a string"))?;
                targets.insert(key.clone(), dest_str.to_string());
            }
        }

        let src = pkg_val
            .get("src")
            .ok_or_else(|| anyhow::anyhow!("Package src is required"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Package src must be a string"))?
            .to_string();

        let dest = pkg_val
            .get("dest")
            .ok_or_else(|| anyhow::anyhow!("Package dest is required"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Package dest must be a string"))?
            .to_string();

        let skip = pkg_val
            .get("skip")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Self {
            name: pkg_name.to_string(),
            src,
            dest,
            skip,
            dependencies,
            variables,
            pre_actions,
            post_actions,
            targets,
        })
    }

    pub fn to_table(&self) -> Table {
        let mut pkg_table = Table::new();
        pkg_table.insert("src".to_string(), toml::Value::String(self.src.clone()));
        pkg_table.insert("dest".to_string(), toml::Value::String(self.dest.clone()));
        if let Some(deps) = &self.dependencies {
            let deps_val: Vec<toml::Value> = deps
                .iter()
                .map(|d| toml::Value::String(d.clone()))
                .collect();
            pkg_table.insert("dependencies".to_string(), toml::Value::Array(deps_val));
        }
        if !self.variables.is_empty() {
            pkg_table.insert(
                "variables".to_string(),
                toml::Value::Table(self.variables.clone()),
            );
        }
        if !self.pre_actions.is_empty() {
            let pre_actions_val: Vec<toml::Value> = self
                .pre_actions
                .iter()
                .map(|a| toml::Value::String(a.clone()))
                .collect();
            pkg_table.insert(
                "pre_actions".to_string(),
                toml::Value::Array(pre_actions_val),
            );
        }
        if !self.post_actions.is_empty() {
            let post_actions_val: Vec<toml::Value> = self
                .post_actions
                .iter()
                .map(|a| toml::Value::String(a.clone()))
                .collect();
            pkg_table.insert(
                "post_actions".to_string(),
                toml::Value::Array(post_actions_val),
            );
        }
        if !self.targets.is_empty() {
            let mut targets_table = Table::new();
            for (key, value) in &self.targets {
                targets_table.insert(key.clone(), toml::Value::String(value.clone()));
            }
            pkg_table.insert("targets".to_string(), toml::Value::Table(targets_table));
        }
        if self.skip {
            pkg_table.insert("skip".to_string(), toml::Value::Boolean(true));
        }
        pkg_table
    }

    pub fn execute_action(
        &self,
        action: &str,
        variables: &Table,
        working_dir: &Path,
    ) -> anyhow::Result<()> {
        let compiled_action = compile_string(action, variables)?;
        // Get SHELL environment variable or default to /bin/sh
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let status = std::process::Command::new(shell)
            .arg("-c")
            .arg(compiled_action)
            .current_dir(working_dir)
            .status()?;
        if !status.success() {
            let msg = format!(
                "Action '{}' failed to execute with exit code: {:?}",
                action,
                status.code()
            );
            eprintln!("{}", msg);
            return Err(anyhow::anyhow!(msg));
        }
        Ok(())
    }

    pub fn execute_pre_actions(&self, ctx: &Context) -> anyhow::Result<()> {
        let vars = self.get_context_variables(ctx);
        for action in &self.pre_actions {
            self.execute_action(action, &vars, &ctx.working_dir)?;
        }
        Ok(())
    }

    pub fn execute_post_actions(&self, ctx: &Context) -> anyhow::Result<()> {
        let vars = self.get_context_variables(ctx);
        for action in &self.post_actions {
            self.execute_action(action, &vars, &ctx.working_dir)?;
        }
        Ok(())
    }

    pub fn get_context_variables(&self, ctx: &Context) -> Table {
        let mut vars = ctx.get_variables().clone();
        vars.extend(self.variables.clone());
        if let Some(profile) = &ctx.profile {
            vars.extend(profile.variables.clone());
        }
        vars.extend(ctx.get_user_variables().clone());
        vars
    }

    /// Backup the package by copying files from dest to a backup location, recursively.
    pub fn backup(&self, ctx: &Context) -> anyhow::Result<()> {
        if self.package_is_templated(&ctx.working_dir) {
            println!(
                "[INFO] Skipping backup for templated package '{}'",
                self.name
            );
            return Ok(());
        }
        let copy_from = self.resolve_dest(ctx);
        let copy_to = ctx.working_dir.join(self.src.clone());
        if copy_from.is_dir() {
            // Recursively copy directory contents, avoiding files ending with BACKUP_EXT
            for entry in walkdir::WalkDir::new(&copy_from) {
                let entry = entry?;
                let relative_path = entry.path().strip_prefix(&copy_from)?;
                let dest_path = copy_to.clone().join(relative_path);
                if entry.path().is_dir() {
                    std::fs::create_dir_all(&dest_path)?;
                } else if entry.path().extension() != Some(OsStr::new(BACKUP_EXT)) {
                    if let Some(parent) = dest_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::copy(entry.path(), &dest_path)?;
                }
            }
        } else {
            std::fs::copy(&copy_from, &copy_to)?;
        }
        println!(
            "[INFO] Backed up file '{}' to '{}'",
            copy_from.display(),
            copy_to.display()
        );
        Ok(())
    }

    pub fn resolve_dest(&self, ctx: &Context) -> PathBuf {
        if let Some(profile) = &ctx.profile
            && let Some(target_dest) = self.targets.get(profile.name.as_str())
        {
            return resolve_path(target_dest, &ctx.working_dir);
        }
        resolve_path(&self.dest, &ctx.working_dir)
    }

    pub fn deploy_file(
        &self,
        src: &PathBuf,
        dest: &PathBuf,
        ctx: &Context,
        backup: bool,
    ) -> Result<(), anyhow::Error> {
        if let Ok(src_content) = std::fs::read_to_string(src) {
            let compiled_content = if is_templated_str(&src_content) {
                compile_string(&src_content, &self.get_context_variables(ctx))?
            } else {
                src_content
            };

            let mut should_copy = false;
            if !dest.exists() {
                should_copy = true;
            } else {
                let existing_content = std::fs::read_to_string(dest)?;
                if existing_content != compiled_content {
                    should_copy = true;
                }
            }
            if !should_copy {
                println!(
                    "[INFO] Skipping deployment for '{}' as it is unchanged at '{}'",
                    src.display(),
                    dest.display()
                );
                return Ok(());
            }
            if backup && dest.exists() {
                let backup_path = create_backup_path(dest);
                std::fs::copy(dest, &backup_path)?;
            }
            std::fs::write(dest, compiled_content)?;
        } else {
            // It can be a binary file, copy as-is and return Ok
            if backup && dest.exists() {
                let backup_path = create_backup_path(dest);
                std::fs::copy(dest, &backup_path)?;
            }
            std::fs::copy(src, dest)?;
            return Ok(());
        }
        println!(
            "[INFO] Deployed file '{}' to '{}'",
            src.display(),
            dest.display()
        );
        Ok(())
    }

    /// Deploy the package by copying files from src to dest.
    pub fn deploy(&self, ctx: &Context) -> Result<(), anyhow::Error> {
        self.execute_pre_actions(ctx)?;
        let copy_from = resolve_path(&self.src, &ctx.working_dir);
        let copy_to = self.resolve_dest(ctx);
        if copy_from.is_dir() {
            // Recursively copy directory contents
            for entry in walkdir::WalkDir::new(&copy_from) {
                let entry = entry?;
                let relative_path = entry.path().strip_prefix(&copy_from)?;
                let dest_path = copy_to.join(relative_path);
                if entry.path().is_dir() {
                    std::fs::create_dir_all(&dest_path)?;
                } else {
                    self.deploy_file(&entry.path().to_path_buf(), &dest_path, ctx, true)?;
                }
            }
        } else {
            self.deploy_file(&copy_from, &copy_to, ctx, true)?;
        }

        println!(
            "[INFO] Deployed file '{}' to '{}'",
            copy_from.display(),
            copy_to.display()
        );
        self.execute_post_actions(ctx)?;
        Ok(())
    }

    pub fn is_dir(&self) -> bool {
        self.name.starts_with("d_")
    }

    pub fn package_is_templated(&self, cwd: &Path) -> bool {
        // Check if src exists as a directory or file, if not return true:
        let src_path = cwd.join(&self.src);
        if !src_path.exists() {
            return false;
        }
        // Check for following templating indicators using walkdir (when necessary) and regex:
        // {{ and }} for expressions
        // {% and %} for statements
        // {# and #} for comments
        // {{- and -}} for expressions
        // {%- and -%} for statements
        // {#- and -#} for comments

        if src_path.is_dir() {
            for entry in walkdir::WalkDir::new(&src_path) {
                let entry = entry.expect("Failed to read directory entry");
                if entry.path().is_file() {
                    return is_templated(&entry.path().to_path_buf());
                }
            }
        } else if src_path.is_file() {
            return is_templated(&src_path);
        }
        false
    }
}

/// Get a package name from a given path string.
/// The package name is derived from the last component of the path,
/// with any leading '.' removed, and any trailing version numbers removed.
/// Additionally, any '-' or '.' characters are replaced with '_'.
/// If the path is a directory, it should be prepended with d_
/// Or, if it's a file, with f_
pub fn get_package_name(pathstr: &str, cwd: &Path) -> String {
    let path = resolve_path(pathstr, cwd);
    let last_component = path
        .file_name()
        .expect("Failed to get file name")
        .to_str()
        .unwrap();
    let mut package_name = last_component.trim_start_matches('.').to_string();

    // Remove any trailing version numbers
    if let Some(pos) = package_name.rfind('-') {
        package_name.truncate(pos);
    }
    // replace any remaining '-' with '_', and '.' with '_'
    let prefix = if path.is_dir() { "d_" } else { "f_" };
    package_name = format!("{}{}", prefix, package_name);
    package_name.replace(['-', '.'], "_")
}

/// Create a backup path by appending the backup extension to the original path
fn create_backup_path(path: &Path) -> PathBuf {
    let mut backup_path = path.as_os_str().to_os_string();
    backup_path.push(".");
    backup_path.push(BACKUP_EXT);
    PathBuf::from(backup_path)
}

/// Compile a template file at the given path using Tera templating engine with the provided context. and return the rendered content as a String.
pub fn compile_template(path: &Path, context: &Table) -> anyhow::Result<String> {
    let ctx = tera::Context::from_serialize(context)?;
    let template_content = std::fs::read_to_string(path)?;
    Ok(tera::Tera::one_off(&template_content, &ctx, true)?)
}

pub fn compile_string(template_str: &str, context: &Table) -> anyhow::Result<String> {
    let ctx = tera::Context::from_serialize(context)?;
    Ok(tera::Tera::one_off(template_str, &ctx, true)?)
}

pub fn is_templated(p: &PathBuf) -> bool {
    if !p.exists() {
        return false;
    }
    let content = std::fs::read_to_string(p);
    if let Ok(text) = content {
        is_templated_str(&text)
    } else {
        false
    }
}

pub fn is_templated_str(s: &str) -> bool {
    TEMPLATE_REGEX.is_match(s)
}
