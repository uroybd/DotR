use std::{
    collections::HashMap,
    ffi::OsStr,
    io::{self, IsTerminal},
    path::{Path, PathBuf},
    sync::LazyLock,
};

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::{
    cli::{DeployArgs, ImportArgs, UpdateArgs},
    context::Context,
    merge::{self, Change, Outcome, Resolution},
    outln,
    profile::Profile,
    utils::{
        self, BACKUP_EXT, LogLevel, compile_string, cprintln, execute_action,
        get_string_from_value, get_string_hashmap_from_value, get_vec_string_from_value,
        is_empty_table, normalize_home_path, resolve_path,
    },
};

#[cfg(test)]
mod tests;

#[expect(
    clippy::expect_used,
    reason = "a fixed pattern: it compiles, and every test that detects a template runs it"
)]
static TEMPLATE_REGEX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(\{\{[-]?|[-]?\}\}|\{[%][-]?|[-]?%\}|\{[#][-]?|[-]?#\})")
        .expect("Failed to compile template regex")
});

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
fn is_false(b: &bool) -> bool {
    !*b
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
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
        let src_path_str = format!("dotfiles/{pkg_ns}");

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
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);
        let prompts = get_string_hashmap_from_value(pkg_val.get("prompts"))?;
        let ignore = get_vec_string_from_value(pkg_val.get("ignore"))?;
        let symlink = pkg_val.get("symlink").and_then(toml::Value::as_bool);
        let unfold_symlink = pkg_val
            .get("unfold_symlink")
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);
        let clean = pkg_val
            .get("clean")
            .and_then(toml::Value::as_bool)
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

    pub fn execute_pre_actions(&self, ctx: &Context, dry_run: bool) -> anyhow::Result<()> {
        let vars = self.get_context_variables(ctx);
        for action in &self.pre_actions {
            execute_action(action, &vars, &ctx.working_dir, dry_run)?;
        }
        Ok(())
    }

    pub fn execute_post_actions(&self, ctx: &Context, dry_run: bool) -> anyhow::Result<()> {
        let vars = self.get_context_variables(ctx);
        for action in &self.post_actions {
            execute_action(action, &vars, &ctx.working_dir, dry_run)?;
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
        let secrets = ctx.secrets();
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
                        self.backup_templated(&dest_path, entry.path(), ctx, args)?;
                        all_copied_paths.push(dest_path);
                        continue;
                    }
                    // Not a template (binary files included): copied byte for byte, unless it holds a secret.
                    if holds_secret(&dest_path, entry.path(), &secrets)? {
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
        } else if is_templated(&copy_to) {
            return self.backup_templated(&copy_to, &copy_from, ctx, args);
        } else if holds_secret(&copy_to, &copy_from, &secrets)? {
            return Ok(BackupDeployResult::Skipped);
        } else if !args.dry_run {
            std::fs::copy(&copy_from, &copy_to)?;
        }
        Ok(BackupDeployResult::Success)
    }

    /// Backs up one templated file by merging its deployed copy into the template (see [`crate::merge`]), and reports
    /// every change made on the way: nothing reaches the template differently from the deployed file unannounced.
    fn backup_templated(
        &self,
        template_path: &Path,
        deployed_path: &Path,
        ctx: &Context,
        args: &UpdateArgs,
    ) -> anyhow::Result<BackupDeployResult> {
        let template = std::fs::read_to_string(template_path)?;
        let deployed = std::fs::read(deployed_path)?;
        let vars = self.get_context_variables(ctx);
        let secrets = ctx.secret_names();
        let interactive =
            args.conflict.is_none() && io::stdin().is_terminal() && io::stdout().is_terminal();
        let fixed = args.conflict.unwrap_or(Resolution::Skip);
        let mut resolve = |conflict: &merge::Conflict| {
            if interactive {
                merge::ask(
                    template_path,
                    conflict,
                    &mut io::stdin().lock(),
                    &mut io::stdout(),
                )
            } else {
                Ok(fixed)
            }
        };
        let outcome = merge::merge_template(&template, &deployed, &vars, &secrets, &mut resolve)?;
        let (t, d) = (template_path.display(), deployed_path.display());
        let not_backed_up = |why: &str| {
            cprintln(
                &format!("Not backing up templated file '{t}': {why}"),
                &LogLevel::Warning,
            );
            Ok(BackupDeployResult::Skipped)
        };
        match outcome {
            Outcome::Unchanged => Ok(BackupDeployResult::Skipped),
            Outcome::NotText => not_backed_up(&format!("its deployed copy '{d}' is not text")),
            Outcome::RoundTripFailed { error } => not_backed_up(&match error {
                Some(error) => format!(
                    "the merged template does not compile ({error}); the deployed file may hold \
template syntax, such as `{{{{`, as text. Edit the template by hand"
                ),
                None => format!(
                    "the merged template would not render back to '{d}'; edit the template by hand"
                ),
            }),
            Outcome::Conflicts(conflicts) => {
                let how = if interactive {
                    "skipped at the prompt"
                } else {
                    "rerun in a terminal, or with --conflict ours|theirs|both|edit, to settle"
                };
                let result = not_backed_up(&format!(
                    "{} conflict(s) with lines the template computes ({how})",
                    conflicts.len()
                ));
                for conflict in &conflicts {
                    for line in merge::describe(conflict) {
                        outln!("{line}");
                    }
                }
                result
            }
            Outcome::Merged { template, changes } => {
                report_merge(template_path, deployed_path, &changes, args.dry_run);
                if !args.dry_run {
                    std::fs::write(template_path, template)?;
                }
                Ok(BackupDeployResult::Success)
            }
            Outcome::Edit { draft, changes } => {
                if args.dry_run {
                    report_merge(template_path, deployed_path, &changes, true);
                    cprintln(
                        &format!("Would open '{t}' in your editor to settle its conflicts"),
                        &LogLevel::Info,
                    );
                    return Ok(BackupDeployResult::Skipped);
                }
                let Some(editor) = merge::editor_command() else {
                    return not_backed_up(
                        "no editor to settle its conflicts in: set DOTR_EDITOR, VISUAL or EDITOR",
                    );
                };
                let deployed = std::str::from_utf8(&deployed)?;
                let mut draft = draft;
                loop {
                    let saved = merge::edit_in_editor(&editor, &draft, template_path)?;
                    match merge::check_edit(&saved, deployed, &vars, &secrets) {
                        Ok(edited) => {
                            report_merge(template_path, deployed_path, &changes, false);
                            report_edit(template_path, deployed_path, edited);
                            std::fs::write(template_path, saved)?;
                            return Ok(BackupDeployResult::Success);
                        }
                        Err(problem) => {
                            cprintln(
                                &format!("Your edit of '{t}' can't be used: {problem}"),
                                &LogLevel::Warning,
                            );
                            let reopen = (io::stdin().is_terminal() && io::stdout().is_terminal())
                                && merge::ask_reopen(&mut io::stdin().lock(), &mut io::stdout())?;
                            if !reopen {
                                return not_backed_up("the edit was not finished");
                            }
                            draft = saved;
                        }
                    }
                }
            }
        }
    }

    /// Whether a file this package backs up into is a template, so `update` has to render it to merge. A source
    /// not there yet (a package never backed up) has none.
    pub fn has_templated_source(&self, working_dir: &Path) -> anyhow::Result<bool> {
        let src = resolve_path(&self.src, working_dir)?;
        if !src.exists() {
            return Ok(false);
        }
        for entry in walkdir::WalkDir::new(&src) {
            let entry = entry?;
            let relative = entry.path().strip_prefix(&src)?;
            if entry.file_type().is_file()
                && !self.should_ignore(relative)
                && is_templated(entry.path())
            {
                return Ok(true);
            }
        }
        Ok(false)
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
        let dest = compile_string(
            self.raw_dest(&ctx.profile),
            &self.get_context_variables(ctx),
        )?;
        resolve_path(&dest, &ctx.working_dir)
    }

    /// The uncompiled deployment path `resolve_dest` would render for this
    /// profile, chosen by the same precedence.
    pub fn raw_dest(&self, profile: &Profile) -> &str {
        self.targets
            .get(profile.name.as_str())
            .or_else(|| {
                profile
                    .platform
                    .as_ref()
                    .and_then(|platform| self.targets.get(platform.as_str()))
            })
            .unwrap_or(&self.dest)
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

            if dest.exists() {
                let existing = std::fs::read(dest)?;
                let Some(existing_content) = utils::as_text(&existing) else {
                    // A deployed file that is not text is told apart, never diffed line by line.
                    cprintln(
                        &format!(
                            "{} {} -> {}: the deployed file is not text",
                            if existing == compiled_content.as_bytes() {
                                "No changes in"
                            } else {
                                "Changes in"
                            },
                            src.file_name().unwrap_or_default().to_string_lossy(),
                            dest.display()
                        ),
                        &LogLevel::Info,
                    );
                    return Ok(());
                };
                if existing_content == compiled_content {
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
                    for diff in diff::lines(existing_content, &compiled_content) {
                        match diff {
                            diff::Result::Left(l) => {
                                let s = format!("-{l}");
                                print_with_color(s.as_str(), RED);
                            }
                            diff::Result::Both(l, _) => {
                                outln!(" {l}");
                            }
                            diff::Result::Right(r) => {
                                let s = format!("+{r}");
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
                // Bytes, not text: the deployed file may be binary, and is then simply replaced.
                if std::fs::read(dest)? != compiled_content.as_bytes() {
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
                        outln!("=> Deployed {}", dest_path.display());
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
                outln!("=> Deployed {}", copy_to.display());
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
    let package_name = format!("{prefix}{sanitized_name}");

    let pkg_ns = if let Some(ext) = extension {
        let base_name = name.trim_end_matches(&format!(".{ext}"));
        let sanitized_base = base_name.replace(['-', '.'], "_");
        format!("{prefix}{sanitized_base}.{ext}")
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

/// Whether the file at `p` is a template: text (see [`utils::as_text`]) holding template tags.
pub fn is_templated(p: &Path) -> bool {
    std::fs::read(p)
        .ok()
        .as_deref()
        .and_then(utils::as_text)
        .is_some_and(is_templated_str)
}

/// Whether text holds template tags. Text with a NUL byte is binary, however its bytes happen to read.
pub fn is_templated_str(s: &str) -> bool {
    utils::is_text(s) && TEMPLATE_REGEX.is_match(s)
}

const RED: &str = "31";
const GREEN: &str = "32";

/// Where the first secret value at least [`merge::MIN_VALUE_CHARS`] long appears in `bytes`: the variable, and the
/// 1-based line when the bytes are text.
pub fn find_secret<'a>(
    bytes: &[u8],
    secrets: &'a [(String, String)],
) -> Option<(&'a str, Option<usize>)> {
    secrets
        .iter()
        .filter(|(_, value)| value.chars().count() >= merge::MIN_VALUE_CHARS)
        .find_map(|(name, value)| {
            let at = bytes
                .windows(value.len())
                .position(|window| window == value.as_bytes())?;
            let line = utils::as_text(bytes).and_then(|text| {
                let before = text.get(..at)?;
                Some(before.matches('\n').count() + 1)
            });
            Some((name.as_str(), line))
        })
}

/// Whether a plain file's deployed copy holds a secret, and so must not be copied into the repository; says so if it
/// does (the variable's name, never its value).
fn holds_secret(
    repo: &Path,
    deployed: &Path,
    secrets: &[(String, String)],
) -> anyhow::Result<bool> {
    if secrets.is_empty() {
        return Ok(false);
    }
    let Some((name, line)) = find_secret(&std::fs::read(deployed)?, secrets) else {
        return Ok(false);
    };
    let place = line.map_or_else(String::new, |n| format!("line {n} of "));
    cprintln(
        &format!(
            "Not backing up '{}': {place}'{}' holds the value of the secret `{name}`. Put {{{{ {name} }}}} in \
its place in '{}' (making it a template, which `update` then merges), or take the value out of the \
deployed file.",
            repo.display(),
            deployed.display(),
            repo.display()
        ),
        &LogLevel::Warning,
    );
    Ok(true)
}

/// Reports a merge into a template: one line per change it made to the deployed content on the way.
fn report_merge(template: &Path, deployed: &Path, changes: &[Change], dry_run: bool) {
    let (t, d) = (template.display(), deployed.display());
    let verb = if dry_run { "Would merge" } else { "Merged" };
    cprintln(
        &format!("{verb} changes from '{d}' into template '{t}'"),
        &LogLevel::Info,
    );
    for change in changes {
        let (message, level) = match change {
            Change::Substituted { line, name, secret } => (
                format!(
                    "  {t}:{line}: wrote {{{{ {name} }}}} in place of {}",
                    if *secret {
                        "a secret value"
                    } else {
                        "its value"
                    }
                ),
                LogLevel::Info,
            ),
            Change::Ambiguous { line, names } => (
                format!(
                    "  {t}:{line}: left a value as text: it is the value of {} alike",
                    names.join(", ")
                ),
                LogLevel::Warning,
            ),
            Change::Rewritten { lines } => (
                format!(
                    "  {t}: {}: rewritten from the deployed file, its variables put back as tags",
                    merge::span(lines)
                ),
                LogLevel::Info,
            ),
            Change::KeptTemplate {
                lines,
                deployed_lines,
            } => (
                format!(
                    "  {t}: {}: kept the template's; the deployed {} left out (the next deploy puts the \
template's back)",
                    merge::span(lines),
                    merge::span(deployed_lines)
                ),
                LogLevel::Warning,
            ),
            Change::TookDeployed { lines } => (
                format!(
                    "  {t}: {}: replaced by the deployed lines; any template logic there is gone",
                    merge::span(lines)
                ),
                LogLevel::Warning,
            ),
            Change::Union { lines, added } => (
                format!(
                    "  {t}: {}: kept, with the deployed lines added after them as {}: the file now renders \
both; tidy it before the next deploy",
                    merge::span(lines),
                    merge::span(added)
                ),
                LogLevel::Warning,
            ),
        };
        cprintln(&message, &level);
    }
}

/// Reports a template saved from the editor, and how its render compares with the deployed file.
fn report_edit(template: &Path, deployed: &Path, edited: merge::Edited) {
    let (t, d) = (template.display(), deployed.display());
    if edited.renders_deployed() {
        cprintln(
            &format!("Saved '{t}' from your editor; it renders '{d}' exactly"),
            &LogLevel::Info,
        );
    } else {
        cprintln(
            &format!(
                "Saved '{t}' from your editor; its render differs from '{d}' ({} line(s) left out or \
changed, {} added): the next deploy will change the deployed file",
                edited.missing, edited.added
            ),
            &LogLevel::Warning,
        );
    }
}

pub fn print_with_color(s: &str, color_code: &str) {
    outln!("\x1b[{color_code}m{s}\x1b[0m");
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
        && std::fs::read_link(path).is_ok_and(|target| target.starts_with(staged_root))
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
