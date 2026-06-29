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
        get_vec_string_from_value, is_empty_table, normalize_home_path, resolve_path,
    },
};

#[cfg(test)]
mod tests;

static TEMPLATE_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(\{\{[-]?|[-]?\}\}|\{[%][-]?|[-]?%\}|\{[#][-]?|[-]?#\})")
        .expect("Failed to compile template regex")
});

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_true(b: &bool) -> bool {
    *b
}

fn default_clean() -> bool {
    true
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Package {
    #[serde(skip)]
    pub name: String,
    pub src: String,
    pub dest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<String>>,
    #[serde(skip_serializing_if = "is_empty_table")]
    pub variables: Table,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pre_actions: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub post_actions: Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub targets: HashMap<String, String>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub skip: bool,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub prompts: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignore: Vec<String>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub symlink: bool,
    #[serde(skip_serializing_if = "is_true", default = "default_clean")]
    pub clean: bool,
}

impl Default for Package {
    fn default() -> Self {
        Self::new("", "", "")
    }
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
            clean: true,
        }
    }

    pub fn from_path(args: &ImportArgs, cwd: &Path) -> Result<Self, anyhow::Error> {
        let resolved_path = resolve_path(&args.path, cwd)?;
        if !resolved_path.exists() {
            anyhow::bail!("Path '{}' does not exist", resolved_path.display());
        }
        let (package_name, pkg_ns) = get_pkg_name_and_rel_path(args, cwd)?;
        let src_path_str = format!("dotfiles/{}", pkg_ns);

        let mut dest_path_str = resolved_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path: contains non-UTF-8 characters"))?
            .to_string();
        dest_path_str = normalize_home_path(&dest_path_str);

        if args.path.ends_with('/') && !dest_path_str.ends_with('/') && resolved_path.is_dir() {
            dest_path_str.push('/');
        }

        let mut pkg = Self::new(&package_name, &src_path_str, &dest_path_str);
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
        let pre_actions = get_vec_string_from_value(pkg_val.get("pre_actions"))?;
        let post_actions = get_vec_string_from_value(pkg_val.get("post_actions"))?;
        let targets = get_string_hashmap_from_value(pkg_val.get("targets"))?;
        let src = get_string_from_value(pkg_val.get("src"), "src")?;
        let dest = get_string_from_value(pkg_val.get("dest"), "dest")?;
        let skip = pkg_val
            .get("skip")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let prompts = get_string_hashmap_from_value(pkg_val.get("prompts"))?;
        let ignore = get_vec_string_from_value(pkg_val.get("ignore"))?;
        let symlink = pkg_val
            .get("symlink")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let clean = pkg_val
            .get("clean")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

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
            clean,
        })
    }

    pub fn execute_action(
        &self,
        action: &str,
        variables: &Table,
        working_dir: &Path,
        dry_run: bool,
    ) -> anyhow::Result<()> {
        let compiled_action = compile_string(action, variables)?;
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        if dry_run {
            cprintln(
                &format!("(Dry Run) Would execute action: {}", compiled_action),
                &LogLevel::Info,
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
            cprintln(&msg, &LogLevel::Error);
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
                &LogLevel::Warning,
            );
            return Ok(BackupDeployResult::Skipped);
        }
        let copy_from = self.resolve_dest(ctx)?;
        let copy_to = ctx.working_dir.join(self.src.clone());
        if copy_from.is_dir() {
            let mut all_copied_paths = vec![];
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
                    }
                    all_copied_paths.push(dest_path);
                }
            }
            if self.should_clean(args.clean) {
                clean(&copy_to, &all_copied_paths, &self.ignore, args.dry_run)?;
            }
        } else if !args.dry_run {
            std::fs::copy(&copy_from, &copy_to)?;
        }
        Ok(BackupDeployResult::Success)
    }

    pub fn should_clean(&self, arg_val: Option<bool>) -> bool {
        match arg_val {
            Some(v) => v,
            None => self.clean,
        }
    }

    pub fn resolve_dest(&self, ctx: &Context) -> anyhow::Result<PathBuf> {
        let mut dest = match self.targets.get(ctx.profile.name.as_str()) {
            Some(d) => d.clone(),
            None => self.dest.clone(),
        };
        dest = compile_string(&dest, &self.get_context_variables(ctx))?;
        resolve_path(&dest, &ctx.working_dir)
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
                        &LogLevel::Info,
                    );
                } else {
                    cprintln(
                        &format!(
                            "Changes in {} -> {}:",
                            src.file_name().unwrap_or_default().to_string_lossy(),
                            dest.display()
                        ),
                        &LogLevel::Info,
                    );
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
        let src = resolve_path(&self.src, &ctx.working_dir)?;
        let dest = self.resolve_dest(ctx)?;
        if src.is_dir() {
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
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                if backup && dest.exists() && !self.symlink {
                    let backup_path = create_backup_path(dest);
                    std::fs::copy(dest, &backup_path)?;
                }
                std::fs::write(dest, compiled_content)?;
            }
        } else {
            // Binary file: copy as-is
            if !dry_run {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
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
        let copy_from = resolve_path(&self.src, &ctx.working_dir)?;
        let copy_to = if self.symlink {
            self.resolve_deployment_path(ctx)?
        } else {
            self.resolve_dest(ctx)?
        };
        let mut pre_action_executed = false;
        let mut result = BackupDeployResult::Skipped;
        if copy_from.is_dir() {
            let mut all_deployed_paths = vec![];
            for entry in walkdir::WalkDir::new(&copy_from) {
                let entry = entry?;
                let relative_path = entry.path().strip_prefix(&copy_from)?;
                if self.should_ignore(relative_path) {
                    continue;
                }
                let dest_path = copy_to.join(relative_path);
                if !pre_action_executed {
                    self.execute_pre_actions(ctx, args.dry_run)?;
                    pre_action_executed = true;
                }
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
            if self.should_clean(args.clean) {
                clean(&copy_to, &all_deployed_paths, &self.ignore, args.dry_run)?;
            }
        } else {
            self.execute_pre_actions(ctx, args.dry_run)?;
            pre_action_executed = true;
            if self.symlink
                && !args.dry_run
                && let Some(parent) = copy_to.parent()
            {
                std::fs::create_dir_all(parent)?;
            }
            let dep_result = self.deploy_file(&copy_from, &copy_to, ctx, true, args.dry_run)?;
            if let BackupDeployResult::Success = dep_result {
                result = BackupDeployResult::Success;
                println!("=> Deployed {}", copy_to.display());
            }
        }
        if self.symlink {
            let symlink_to = self.resolve_dest(ctx)?;
            if !args.dry_run {
                if symlink_to.exists() {
                    if symlink_to.is_symlink() {
                        std::fs::remove_file(&symlink_to)?;
                    } else if symlink_to.is_dir() {
                        if symlink_to.read_dir()?.next().is_none() {
                            std::fs::remove_dir(&symlink_to)?;
                        } else {
                            std::fs::remove_dir_all(&symlink_to)?;
                        }
                    } else {
                        std::fs::remove_file(&symlink_to)?;
                    }
                }
                if let Some(parent) = symlink_to.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let symlink_str = symlink_to
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Symlink path contains non-UTF-8 characters"))?
                    .trim_end_matches('/');
                std::os::unix::fs::symlink(&copy_to, symlink_str)?;
            }
        }
        cprintln(
            &format!("Package '{}' deployed", self.name),
            &LogLevel::Info,
        );
        if pre_action_executed {
            self.execute_post_actions(ctx, args.dry_run)?;
        }
        Ok(result)
    }

    pub fn package_is_templated(&self, cwd: &Path) -> bool {
        let src_path = cwd.join(&self.src);
        if !src_path.exists() {
            return false;
        }
        if src_path.is_dir() {
            for entry in walkdir::WalkDir::new(&src_path).into_iter().flatten() {
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
    let path = resolve_path(&args.path, cwd)?;
    let prefix = if path.is_dir() { "d_" } else { "f_" };
    let extension = path.extension().and_then(|ext| ext.to_str());

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

    let sanitized_name = name.replace(['-', '.'], "_");
    let package_name = format!("{}{}", prefix, sanitized_name);

    let pkg_ns = if let Some(ext) = extension {
        let base_name = name.trim_end_matches(&format!(".{}", ext));
        let sanitized_base = base_name.replace(['-', '.'], "_");
        format!("{}{}.{}", prefix, sanitized_base, ext)
    } else {
        package_name.clone()
    };

    Ok((package_name, pkg_ns))
}

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
    if !operation_path.exists() {
        return Ok(());
    }

    let mut dirs = vec![];
    for entry in walkdir::WalkDir::new(operation_path) {
        let entry = entry?;
        let path = entry.path().to_path_buf();
        if path == *operation_path {
            continue;
        }
        let rel_path = path.strip_prefix(operation_path)?;
        if should_ignore(ignore, rel_path) {
            continue;
        }
        if !keep.contains(&path) {
            if path.is_file() {
                if path.extension() == Some(OsStr::new(BACKUP_EXT)) {
                    continue;
                }
                if dry_run {
                    cprintln(
                        &format!("(Dry Run) Would remove file: {}", path.display()),
                        &LogLevel::Info,
                    );
                } else {
                    std::fs::remove_file(&path)?;
                }
            } else if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        if dry_run {
            cprintln(
                &format!("(Dry Run) Would remove directory: {}", dir.display()),
                &LogLevel::Info,
            );
            continue;
        }
        if dir.read_dir()?.next().is_none() {
            std::fs::remove_dir(&dir)?;
        } else {
            std::fs::remove_dir_all(&dir)?;
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
