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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symlink: Option<bool>,
    #[serde(skip_serializing_if = "is_false", default)]
    pub unfold_symlink: bool,
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
            symlink: None,
            unfold_symlink: false,
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
        // Only force it on explicitly; leaving it unset (rather than `Some(false)`)
        // lets a newly imported package still pick up a global `symlink = true`
        // by default, the same as any other package would.
        if args.symlink {
            pkg.symlink = Some(true);
        }
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
        let symlink = pkg_val.get("symlink").and_then(|v| v.as_bool());
        let unfold_symlink = pkg_val
            .get("unfold_symlink")
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
            unfold_symlink,
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
        vars.extend(ctx.profile.platform_variables.clone());
        vars.extend(self.variables.clone());
        vars.extend(ctx.profile.variables.clone());
        vars.extend(ctx.get_user_variables().clone());
        vars
    }

    pub fn should_ignore(&self, rel_path: &Path) -> bool {
        should_ignore(&self.ignore, rel_path)
    }

    /// Whether a directory package should be deployed as individual per-file
    /// symlinks (leaving the destination a real directory) rather than a
    /// single symlink for the whole directory. Non-empty `ignore` implies
    /// this, since an ignored path has nowhere real to live under a single
    /// whole-directory symlink.
    pub fn should_unfold(&self) -> bool {
        self.unfold_symlink || !self.ignore.is_empty()
    }

    /// Whether this package should be deployed as a symlink. An explicit
    /// per-package `symlink` setting always wins - `Some(true)` forces it on
    /// and `Some(false)` opts out - regardless of the global config flag;
    /// only when the package leaves it unset does the global flag apply.
    pub fn effective_symlink(&self, ctx: &Context) -> bool {
        self.symlink.unwrap_or(ctx.symlink)
    }

    pub fn resolve_deployment_path(&self, ctx: &Context) -> anyhow::Result<PathBuf> {
        if !self.effective_symlink(ctx) {
            anyhow::bail!("Package '{}' is not configured for symlinking", self.name);
        }
        Ok(ctx.working_dir.join(utils::SYMLINK_FOLDER).join(&self.name))
    }

    /// Backup the package by copying files from dest to a backup location, recursively.
    pub fn backup(&self, ctx: &Context, args: &UpdateArgs) -> anyhow::Result<BackupDeployResult> {
        let copy_from = self.resolve_dest(ctx)?;
        let copy_to = ctx.working_dir.join(self.src.clone());
        let unfolded_staging_root = if self.effective_symlink(ctx) && self.should_unfold() {
            Some(self.resolve_deployment_path(ctx)?)
        } else {
            None
        };
        if copy_from.is_dir() {
            let mut all_copied_paths = vec![];
            for entry in walkdir::WalkDir::new(&copy_from) {
                let entry = entry?;
                let relative_path = entry.path().strip_prefix(&copy_from)?;
                let dest_path = copy_to.clone().join(relative_path);
                if self.should_ignore(relative_path) {
                    continue;
                }
                if let Some(staged_root) = &unfolded_staging_root
                    && !entry.path().is_dir()
                    && !is_managed_symlink(entry.path(), staged_root)
                {
                    // Foreign, untracked content living alongside unfolded
                    // symlinks - never pull it into the tracked repo.
                    continue;
                }
                if entry.path().is_dir() {
                    if !args.dry_run {
                        std::fs::create_dir_all(&dest_path)?;
                    }
                    all_copied_paths.push(dest_path);
                } else if entry.path().extension() != Some(OsStr::new(BACKUP_EXT)) {
                    if is_templated(&dest_path) {
                        cprintln(
                            &format!(
                                "Skipping backup for templated file '{}'",
                                dest_path.display()
                            ),
                            &LogLevel::Warning,
                        );
                        all_copied_paths.push(dest_path);
                        continue;
                    }
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
            if is_templated(&copy_to) {
                cprintln(
                    &format!("Skipping backup for templated file '{}'", copy_to.display()),
                    &LogLevel::Warning,
                );
                return Ok(BackupDeployResult::Skipped);
            }
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

    /// Resolves this package's effective deployment path for the active
    /// profile: an entry in `targets` keyed by the profile's own name wins
    /// (most specific), then one keyed by the profile's `platform` (shared
    /// across every profile with that platform), then the package's `dest`.
    pub fn resolve_dest(&self, ctx: &Context) -> anyhow::Result<PathBuf> {
        let dest = self
            .targets
            .get(ctx.profile.name.as_str())
            .or_else(|| {
                ctx.profile
                    .platform
                    .as_ref()
                    .and_then(|platform| self.targets.get(platform.as_str()))
            })
            .unwrap_or(&self.dest);
        let dest = compile_string(dest, &self.get_context_variables(ctx))?;
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
                if backup && dest.exists() && !self.effective_symlink(ctx) {
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
                if backup && dest.exists() && !self.effective_symlink(ctx) {
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
        let copy_to = if self.effective_symlink(ctx) {
            self.resolve_deployment_path(ctx)?
        } else {
            self.resolve_dest(ctx)?
        };
        if !args.skip_pre_actions && !args.skip_actions {
            self.execute_pre_actions(ctx, args.dry_run)?;
        }
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
            if self.effective_symlink(ctx)
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
        if self.effective_symlink(ctx) {
            let symlink_to = self.resolve_dest(ctx)?;
            if copy_from.is_dir() && self.should_unfold() {
                if !args.dry_run {
                    self.deploy_unfolded_symlinks(
                        &copy_to,
                        &symlink_to,
                        self.should_clean(args.clean),
                    )?;
                }
            } else if !args.dry_run {
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
        if !args.skip_post_actions && !args.skip_actions {
            self.execute_post_actions(ctx, args.dry_run)?;
        }
        Ok(result)
    }

    /// Deploy a directory package as individual per-file symlinks rather than
    /// one symlink for the whole directory, leaving `dest_root` a real
    /// directory so untracked, unmanaged content can coexist in it.
    ///
    /// Never overwrites a path that already exists at `dest_root` unless it
    /// is itself a symlink we created (i.e. resolves into `staged_root`) -
    /// anything else is left completely alone.
    fn deploy_unfolded_symlinks(
        &self,
        staged_root: &Path,
        dest_root: &Path,
        should_clean: bool,
    ) -> anyhow::Result<()> {
        if dest_root.is_symlink() || (dest_root.exists() && !dest_root.is_dir()) {
            std::fs::remove_file(dest_root)?;
        }
        std::fs::create_dir_all(dest_root)?;

        let mut linked = vec![];
        for entry in walkdir::WalkDir::new(staged_root) {
            let entry = entry?;
            let path = entry.path();
            if path == staged_root {
                continue;
            }
            let relative_path = path.strip_prefix(staged_root)?;
            let target_path = dest_root.join(relative_path);
            if path.is_dir() {
                std::fs::create_dir_all(&target_path)?;
                continue;
            }
            if target_path.is_symlink() {
                let current_target = std::fs::read_link(&target_path)?;
                if current_target == path {
                    linked.push(target_path);
                    continue;
                }
                std::fs::remove_file(&target_path)?;
            } else if target_path.is_dir() {
                // Pre-existing real content at a path we manage (e.g. the
                // first deploy after enabling symlinking) - replace it, the
                // same way whole-directory symlinking replaces existing
                // content. This never touches paths outside `staged_root`,
                // which is what keeps genuinely foreign files untouched.
                std::fs::remove_dir_all(&target_path)?;
            } else if target_path.exists() {
                std::fs::remove_file(&target_path)?;
            }
            if let Some(parent) = target_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::os::unix::fs::symlink(path, &target_path)?;
            linked.push(target_path);
        }

        if should_clean {
            clean_unfolded(dest_root, staged_root, &linked, &self.ignore)?;
        }
        Ok(())
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

/// Remove stale per-file symlinks left over from a previous unfolded deploy.
///
/// Unlike `clean`, this only ever touches entries at `dest_root` that are
/// themselves symlinks resolving into `staged_root` (i.e. ones this package
/// created). Real files/directories - anyone else's content living
/// alongside the managed symlinks - are always left untouched, and a
/// directory is only removed once it's completely empty (never
/// `remove_dir_all`, since it may still hold foreign content).
pub fn clean_unfolded(
    dest_root: &Path,
    staged_root: &Path,
    keep: &[PathBuf],
    ignore: &[String],
) -> anyhow::Result<()> {
    if !dest_root.exists() {
        return Ok(());
    }

    let mut dirs = vec![];
    for entry in walkdir::WalkDir::new(dest_root) {
        let entry = entry?;
        let path = entry.path().to_path_buf();
        if path == *dest_root {
            continue;
        }
        let rel_path = path.strip_prefix(dest_root)?;
        if should_ignore(ignore, rel_path) {
            continue;
        }
        if path.is_symlink() {
            if keep.contains(&path) {
                continue;
            }
            if is_managed_symlink(&path, staged_root) {
                std::fs::remove_file(&path)?;
            }
        } else if path.is_dir() {
            dirs.push(path);
        }
    }
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        if dir.read_dir()?.next().is_none() {
            std::fs::remove_dir(&dir)?;
        }
    }
    Ok(())
}

/// Whether `path` is a symlink this package created - i.e. one that
/// resolves into `staged_root` - as opposed to unrelated/foreign content.
pub fn is_managed_symlink(path: &Path, staged_root: &Path) -> bool {
    path.is_symlink()
        && std::fs::read_link(path)
            .map(|target| target.starts_with(staged_root))
            .unwrap_or(false)
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
