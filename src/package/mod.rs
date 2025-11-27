use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::{
    cli::{DeployArgs, ImportArgs, UpdateArgs},
    context::Context,
    utils::{
        self, BACKUP_EXT, LogLevel, cprintln, get_string_from_value, get_string_hashmap_from_value,
        get_vec_string_from_value, normalize_home_path, resolve_path, string_hashmap_to_toml_table,
        vec_string_to_toml_array,
    },
};

mod tests;

static TEMPLATE_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(\{\{[-]?|[-]?\}\}|\{[%][-]?|[-]?%\}|\{[#][-]?|[-]?#\})")
        .expect("Failed to compile template regex")
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
    #[serde(default)]
    pub prompts: HashMap<String, String>, // Package-level prompts
    #[serde(default)]
    pub ignore: Vec<String>, // Patterns to ignore during deployment
    // Symlinks defaults to false
    pub symlink: bool,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Copy)]
pub enum BackupDeployResult {
    Success,
    Skipped,
    Failed,
}

impl Package {
    pub fn new(name: &str, src: &str, dest: &str) -> Self {
        Self {
            name: name.to_string(),
            src: src.to_string(),
            dest: dest.to_string(),
            dependencies: None,
            variables: Table::new(),
            pre_actions: Vec::new(),
            post_actions: Vec::new(),
            targets: HashMap::new(),
            skip: false,
            prompts: HashMap::new(),
            ignore: Vec::new(),
            symlink: false,
        }
    }

    // Create a new Package from a given path, used to import dotfiles.
    // The path can be absolute or relative to the current working directory.
    // That path must exist and it will be set to the dest field.
    pub fn from_path(args: &ImportArgs, cwd: &Path) -> Result<Self, anyhow::Error> {
        let resolved_path = resolve_path(&args.path, cwd);
        if !resolved_path.exists() {
            anyhow::bail!("Path '{}' does not exist", resolved_path.display());
        }
        let (package_name, pkg_ns) = get_pkg_name_and_rel_path(args, cwd)?;
        let src_path_str = format!("dotfiles/{}", pkg_ns);

        // Normalize the path: if it already starts with ~, keep it; otherwise convert if in home dir
        let path_str = if args.path.starts_with('~') {
            args.path.to_string()
        } else {
            let resolved_str = resolved_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid path: contains non-UTF-8 characters"))?;
            normalize_home_path(resolved_str)
        };
        let mut pkg = Self::new(&package_name, &src_path_str, &path_str);
        pkg.symlink = args.symlink;
        Ok(pkg)
    }

    pub fn from_table(pkg_name: &str, pkg_val: &Table) -> Result<Self, anyhow::Error> {
        let dependencies: Option<Vec<String>> =
            get_vec_string_from_value(pkg_val.get("dependencies")).ok();
        let variables = match pkg_val.get("variables") {
            Some(var_block) => var_block
                .as_table()
                .ok_or_else(|| anyhow::anyhow!("The 'variables' field must be a table"))?
                .clone(),
            None => Table::new(),
        };
        let pre_actions = get_vec_string_from_value(pkg_val.get("pre_actions"))
            .expect("The 'pre_actions' field must be an array of strings");
        let post_actions = get_vec_string_from_value(pkg_val.get("post_actions"))
            .expect("The 'post_actions' field must be an array of strings");
        let targets = get_string_hashmap_from_value(pkg_val.get("targets"))
            .expect("The 'targets' field must be a table of string to string mappings");
        let src = get_string_from_value(pkg_val.get("src")).expect("Package src is required");
        let dest = get_string_from_value(pkg_val.get("dest")).expect("Package dest is required");
        let skip = pkg_val
            .get("skip")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let prompts = get_string_hashmap_from_value(pkg_val.get("prompts"))
            .expect("The 'prompts' field must be a table of string to string mappings");
        let ignore = get_vec_string_from_value(pkg_val.get("ignore"))
            .expect("The 'ignore' field must be an array of strings");
        let symlink = pkg_val
            .get("symlink")
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
            prompts,
            ignore,
            symlink,
        })
    }

    pub fn to_table(&self) -> Table {
        let mut pkg_table = Table::new();
        pkg_table.insert("src".to_string(), toml::Value::String(self.src.clone()));
        pkg_table.insert("dest".to_string(), toml::Value::String(self.dest.clone()));
        if let Some(deps) = &self.dependencies {
            pkg_table.insert("dependencies".to_string(), vec_string_to_toml_array(deps));
        }
        if !self.variables.is_empty() {
            pkg_table.insert(
                "variables".to_string(),
                toml::Value::Table(self.variables.clone()),
            );
        }
        if !self.pre_actions.is_empty() {
            pkg_table.insert(
                "pre_actions".to_string(),
                vec_string_to_toml_array(&self.pre_actions),
            );
        }
        if !self.post_actions.is_empty() {
            pkg_table.insert(
                "post_actions".to_string(),
                vec_string_to_toml_array(&self.post_actions),
            );
        }
        if self.skip {
            pkg_table.insert("skip".to_string(), toml::Value::Boolean(true));
        }
        if !self.targets.is_empty() {
            pkg_table.insert(
                "targets".to_string(),
                string_hashmap_to_toml_table(&self.targets),
            );
        }
        if !self.prompts.is_empty() {
            pkg_table.insert(
                "prompts".to_string(),
                string_hashmap_to_toml_table(&self.prompts),
            );
        }
        if !self.ignore.is_empty() {
            pkg_table.insert("ignore".to_string(), vec_string_to_toml_array(&self.ignore));
        }
        pkg_table.insert("symlink".to_string(), toml::Value::Boolean(self.symlink));
        pkg_table
    }

    pub fn execute_action(
        &self,
        action: &str,
        variables: &Table,
        working_dir: &Path,
        dry_run: bool,
    ) -> anyhow::Result<()> {
        let compiled_action = compile_string(action, variables)?;
        // Get SHELL environment variable or default to /bin/sh
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        if dry_run {
            cprintln(
                &format!("(Dry Run) Would execute action: {}", compiled_action),
                &LogLevel::INFO,
            );
            return Ok(());
        }
        let status = std::process::Command::new(shell)
            .arg("-c")
            .arg(compiled_action)
            .current_dir(working_dir)
            .status()?;
        if !status.success() {
            let msg = format!(
                "Action '{}' failed with exit code: {:?}",
                action,
                status.code()
            );
            cprintln(&msg, &LogLevel::ERROR);
            return Err(anyhow::anyhow!(msg));
        }
        Ok(())
    }

    pub fn execute_pre_actions(&self, ctx: &Context, dry_run: bool) -> anyhow::Result<()> {
        let vars = self.get_context_variables(ctx);
        for action in &self.pre_actions {
            self.execute_action(action, &vars, &ctx.working_dir, dry_run)?;
        }
        Ok(())
    }

    pub fn execute_post_actions(&self, ctx: &Context, dry_run: bool) -> anyhow::Result<()> {
        let vars = self.get_context_variables(ctx);
        for action in &self.post_actions {
            self.execute_action(action, &vars, &ctx.working_dir, dry_run)?;
        }
        Ok(())
    }

    pub fn get_context_variables(&self, ctx: &Context) -> Table {
        let mut vars = ctx.get_variables().clone();
        vars.extend(self.variables.clone());
        vars.extend(ctx.profile.variables.clone());
        vars.extend(ctx.get_user_variables().clone());
        vars
    }

    pub fn should_ignore(&self, rel_path: &Path) -> bool {
        should_ignore(&self.ignore, rel_path)
    }

    pub fn resolve_deployment_path(&self, ctx: &Context) -> anyhow::Result<PathBuf> {
        if !self.symlink {
            anyhow::bail!("Package '{}' is not configured for symlinking", self.name);
        }
        Ok(ctx.working_dir.join(utils::SYMLINK_FOLDER).join(&self.name))
    }

    /// Backup the package by copying files from dest to a backup location, recursively.
    pub fn backup(&self, ctx: &Context, args: &UpdateArgs) -> anyhow::Result<BackupDeployResult> {
        if self.package_is_templated(&ctx.working_dir) {
            cprintln(
                &format!("Skipping backup for templated '{}'", self.name),
                &LogLevel::WARNING,
            );
            return Ok(BackupDeployResult::Skipped);
        }
        let copy_from = self.resolve_dest(ctx);
        let copy_to = ctx.working_dir.join(self.src.clone());
        if copy_from.is_dir() {
            let mut all_copied_paths = vec![];
            // Recursively copy directory contents, avoiding files ending with BACKUP_EXT
            for entry in walkdir::WalkDir::new(&copy_from) {
                let entry = entry?;
                let relative_path = entry.path().strip_prefix(&copy_from)?;
                if self.should_ignore(relative_path) {
                    continue;
                }
                let dest_path = copy_to.clone().join(relative_path);
                if entry.path().is_dir() {
                    if !args.dry_run {
                        std::fs::create_dir_all(&dest_path)?;
                    }
                    all_copied_paths.push(dest_path);
                } else if entry.path().extension() != Some(OsStr::new(BACKUP_EXT)) {
                    if !args.dry_run {
                        if let Some(parent) = dest_path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        std::fs::copy(entry.path(), &dest_path)?;
                        println!("=> Backed up {}", entry.path().display());
                    }
                    all_copied_paths.push(dest_path);
                }
            }
            if args.clean {
                // Remove any files in copy_to that were not copied in this operation
                clean(&copy_to, &all_copied_paths, &self.ignore, args.dry_run)?;
            }
        } else {
            if !args.dry_run {
                std::fs::copy(&copy_from, &copy_to)?;
            }
            println!("=> Backed up {}", copy_from.display(),);
        }
        Ok(BackupDeployResult::Success)
    }

    pub fn resolve_dest(&self, ctx: &Context) -> PathBuf {
        if let Some(target_dest) = self.targets.get(ctx.profile.name.as_str()) {
            return resolve_path(target_dest, &ctx.working_dir);
        }
        resolve_path(&self.dest, &ctx.working_dir)
    }

    pub fn diff_file(
        &self,
        src: &PathBuf,
        dest: &PathBuf,
        ctx: &Context,
    ) -> Result<(), anyhow::Error> {
        if let Ok(src_content) = std::fs::read_to_string(src) {
            let compiled_content = if is_templated_str(&src_content) {
                compile_string(&src_content, &self.get_context_variables(ctx))?
            } else {
                src_content
            };

            let mut should_diff = false;
            if dest.exists() {
                let existing_content = std::fs::read_to_string(dest).unwrap_or_default();
                if existing_content != compiled_content {
                    should_diff = true;
                }
                if !should_diff {
                    cprintln(
                        &format!(
                            "No changes in {}",
                            src.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        &LogLevel::INFO,
                    );
                } else {
                    cprintln(
                        &format!(
                            "Changes in {} -> {}:",
                            src.file_name().unwrap_or_default().to_string_lossy(),
                            dest.display()
                        ),
                        &LogLevel::INFO,
                    );
                    // Print line-by-line diff, with - for removed lines, + for added lines, and space for unchanged lines. Add colors if possible.
                    for diff in diff::lines(&existing_content, &compiled_content) {
                        match diff {
                            diff::Result::Left(l) => {
                                let s = format!("-{}", l);
                                print_with_color(s.as_str(), RED);
                            }
                            diff::Result::Both(l, _) => {
                                println!(" {}", l);
                            }
                            diff::Result::Right(r) => {
                                let s = format!("+{}", r);
                                print_with_color(s.as_str(), GREEN);
                            }
                        };
                    }
                }
            }
        }
        Ok(())
    }

    pub fn diff(&self, ctx: &Context) -> Result<(), anyhow::Error> {
        let src = resolve_path(&self.src, &ctx.working_dir);
        let dest = self.resolve_dest(ctx);
        if src.is_dir() {
            // Recursively diff directory contents
            for entry in walkdir::WalkDir::new(&src) {
                let entry = entry?;
                let relative_path = entry.path().strip_prefix(&src)?;
                if self.should_ignore(relative_path) {
                    continue;
                }
                let dest_path = dest.join(relative_path);
                if entry.path().is_file() {
                    self.diff_file(&entry.path().to_path_buf(), &dest_path, ctx)?;
                }
            }
        } else {
            self.diff_file(&src, &dest, ctx)?;
        }
        Ok(())
    }

    pub fn deploy_file(
        &self,
        src: &PathBuf,
        dest: &PathBuf,
        ctx: &Context,
        backup: bool,
        dry_run: bool,
    ) -> Result<BackupDeployResult, anyhow::Error> {
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
                return Ok(BackupDeployResult::Skipped);
            }
            if !dry_run {
                if backup && dest.exists() && !self.symlink {
                    let backup_path = create_backup_path(dest);
                    std::fs::copy(dest, &backup_path)?;
                }
                std::fs::write(dest, compiled_content)?;
            }
        } else {
            // It can be a binary file, copy as-is and return Ok
            if !dry_run {
                if backup && dest.exists() && !self.symlink {
                    let backup_path = create_backup_path(dest);
                    std::fs::copy(dest, &backup_path)?;
                }
                std::fs::copy(src, dest)?;
            }
        }
        Ok(BackupDeployResult::Success)
    }

    /// Deploy the package by copying files from src to dest.
    pub fn deploy(
        &self,
        ctx: &Context,
        args: &DeployArgs,
    ) -> Result<BackupDeployResult, anyhow::Error> {
        self.execute_pre_actions(ctx, args.dry_run)?;
        let copy_from = resolve_path(&self.src, &ctx.working_dir);
        let copy_to = if self.symlink {
            self.resolve_deployment_path(ctx)?
        } else {
            self.resolve_dest(ctx)
        };
        println!(
            "Deploying package '{}'(symlinked: {}) from '{}' to '{}'",
            self.name,
            self.symlink,
            copy_from.display(),
            copy_to.display()
        );
        let mut result = BackupDeployResult::Skipped;
        if copy_from.is_dir() {
            let mut all_deployed_paths = vec![];
            // Recursively copy directory contents
            for entry in walkdir::WalkDir::new(&copy_from) {
                let entry = entry?;
                let relative_path = entry.path().strip_prefix(&copy_from)?;
                if self.should_ignore(relative_path) {
                    continue;
                }
                let dest_path = copy_to.join(relative_path);
                if entry.path().is_dir() && !args.dry_run {
                    std::fs::create_dir_all(&dest_path)?;
                } else {
                    let dep_result = self.deploy_file(
                        &entry.path().to_path_buf(),
                        &dest_path,
                        ctx,
                        true,
                        args.dry_run,
                    )?;
                    if let BackupDeployResult::Success = dep_result {
                        result = BackupDeployResult::Success;
                        println!("=> Deployed {}", dest_path.display());
                    }
                }
                all_deployed_paths.push(dest_path);
            }
            if args.clean {
                // Remove any files in copy_to that were not deployed in this operation
                clean(&copy_to, &all_deployed_paths, &self.ignore, args.dry_run)?;
            }
        } else {
            let dep_result = self.deploy_file(&copy_from, &copy_to, ctx, true, args.dry_run)?;
            if let BackupDeployResult::Success = dep_result {
                result = BackupDeployResult::Success;
                println!("=> Deployed {}", copy_to.display());
            }
        }
        if self.symlink {
            let mut symlink_to = self.resolve_dest(ctx);
            if !args.dry_run {
                if symlink_to.exists() {
                    if symlink_to.is_symlink() {
                        std::fs::remove_file(&symlink_to)?;
                    } else {
                        if symlink_to.is_dir() {
                            if symlink_to.read_dir()?.next().is_none() {
                                std::fs::remove_dir(&symlink_to)?;
                            } else {
                                std::fs::remove_dir_all(&symlink_to)?;
                            }
                        } else {
                            std::fs::remove_file(&symlink_to)?;
                        }
                    }
                }
                println!(
                    "Symlinking... {} -> {}",
                    copy_to.display(),
                    symlink_to.display(),
                );

                if let Some(parent) = symlink_to.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                // Remove trailing slash from symlink_to:
                let symlink = symlink_to.to_str().unwrap().trim_end_matches('/');
                symlink_to = PathBuf::from(symlink);

                std::os::unix::fs::symlink(&copy_to, &symlink_to)?;
            }
        }
        cprintln(
            &format!("Package '{}' deployed", self.name),
            &LogLevel::INFO,
        );
        self.execute_post_actions(ctx, args.dry_run)?;
        Ok(result)
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
        }
        if src_path.is_file() {
            return is_templated(&src_path);
        }
        false
    }
}

/// Get a package name and relative path from the given import arguments and current working directory.
/// The package name is derived from the last component of the path,
/// with any leading '.' removed
/// Additionally, any '-' or '.' characters are replaced with '_'. But extension is preserved.
/// If the path is a directory, it should be prepended with d_
/// Or, if it's a file, with f_
/// Example:
/// - For a directory path "/home/user/.config/nvim", the package name would be "d_nvim", rel_path "d_nvim"
/// - For a file path "/home/user/.bashrc", the package name would be "f_bashrc", rel_path "f_bashrc"
/// - For a file path "~/.config/starship/init.lua", the package name would be "f_init_lua", rel_path "f_init.lua"
/// - For a file path "~/.config/starship/init.lua" with custom arg.name as "starship", the package name would be "f_starship_lua", rel_path "f_starship.lua"
pub fn get_pkg_name_and_rel_path(
    args: &ImportArgs,
    cwd: &Path,
) -> Result<(String, String), anyhow::Error> {
    let path = resolve_path(&args.path, cwd);
    let prefix = if path.is_dir() { "d_" } else { "f_" };
    let extension = path.extension().and_then(|ext| ext.to_str());

    // Get the name (with extension for files, full name for dirs)
    let name = match args.name.as_ref() {
        Some(n) => n.clone(),
        None => {
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or_else(|| anyhow::anyhow!("Failed to get file name"))?;
            file_name.trim_start_matches('.').to_string()
        }
    };

    // Replace special characters with underscores for package_name
    let sanitized_name = name.replace(['-', '.'], "_");
    let package_name = format!("{}{}", prefix, sanitized_name);

    // For pkg_ns, preserve the dot before extension
    let pkg_ns = if extension.is_some() {
        // The name already includes the extension, so just replace chars except the last dot
        let name_with_prefix = format!("{}{}", prefix, name);
        // Replace all '-' and '.' with '_' except for the extension separator
        if let Some(ext) = extension {
            let base_name = name.trim_end_matches(&format!(".{}", ext));
            let sanitized_base = base_name.replace(['-', '.'], "_");
            format!("{}{}.{}", prefix, sanitized_base, ext)
        } else {
            name_with_prefix.replace(['-', '.'], "_")
        }
    } else {
        package_name.clone()
    };

    Ok((package_name, pkg_ns))
}

/// Create a backup path by appending the backup extension to the original path
fn create_backup_path(path: &Path) -> PathBuf {
    let mut backup_path = path.as_os_str().to_os_string();
    backup_path.push(".");
    backup_path.push(BACKUP_EXT);
    PathBuf::from(backup_path)
}

pub fn compile_string(template_str: &str, context: &Table) -> anyhow::Result<String> {
    let ctx = tera::Context::from_serialize(context)?;
    Ok(tera::Tera::one_off(template_str, &ctx, false)?)
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

const RED: &str = "31";
const GREEN: &str = "32";

pub fn print_with_color(s: &str, color_code: &str) {
    println!("\x1b[{}m{}\x1b[0m", color_code, s);
}

pub fn clean(
    operation_path: &PathBuf,
    keep: &[PathBuf],
    ignore: &[String],
    dry_run: bool,
) -> anyhow::Result<()> {
    let mut dirs = vec![];
    for entry in walkdir::WalkDir::new(operation_path) {
        let entry = entry?;
        let path = entry.path().to_path_buf();
        if path == *operation_path {
            continue; // Skip the root operation path
        }
        let rel_path = path.strip_prefix(operation_path)?;
        if should_ignore(ignore, rel_path) {
            continue; // Skip ignored patterns
        }
        if !keep.contains(&path) {
            if path.is_file() {
                if path.extension() == Some(OsStr::new(BACKUP_EXT)) {
                    continue; // Skip backup files
                }
                if dry_run {
                    cprintln(
                        &format!("(Dry Run) Would remove file: {}", path.display()),
                        &LogLevel::INFO,
                    );
                } else {
                    std::fs::remove_file(&path)?;
                }
            } else if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    // Remove empty directories in reverse order (from deepest to shallowest)
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        if dry_run {
            cprintln(
                &format!("(Dry Run) Would remove directory: {}", dir.display()),
                &LogLevel::INFO,
            );
            continue;
        }
        if dir.read_dir()?.next().is_none() {
            std::fs::remove_dir(&dir)?;
        } else {
            std::fs::remove_dir_all(&dir)?; // Remove non-empty directories
        }
    }
    Ok(())
}

pub fn should_ignore(ignore: &[String], rel_path: &Path) -> bool {
    let rel_path_str = rel_path.to_string_lossy();
    for pattern in ignore {
        if glob_match::glob_match(pattern, &rel_path_str) {
            return true;
        }
    }
    false
}
