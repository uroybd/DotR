//! `dotr scan`: looking for secret values in what the repository holds, or in what is staged for commit, so a
//! pre-commit hook can stop one before it reaches git history. A secret is what `update` keeps out of the
//! repository (`Context::secrets`); only its name is ever reported, never its value.

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use crate::{context::USERVARIABLES_FILE, package::find_secrets};

#[cfg(test)]
mod tests;

/// Which files to look in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// What is staged for commit: the git index, not the working tree, since that is what the commit will hold.
    Staged,
    /// The files in the repository: in git, every file it tracks or would add (not ignored); elsewhere, every file
    /// under it except `.git`, the `deployed` staging folder and `.uservariables.toml`.
    Repository,
}

/// A secret found in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// The file, relative to the repository.
    pub path: PathBuf,
    /// The 1-based line it is first on, when the file is text.
    pub line: Option<usize>,
    /// The variable whose value it is.
    pub name: String,
}

impl Finding {
    /// `path:line: holds the value of the secret `NAME``, never the value.
    pub fn describe(&self) -> String {
        let place = self.line.map_or_else(
            || self.path.display().to_string(),
            |line| format!("{}:{line}", self.path.display()),
        );
        format!("{place}: holds the value of the secret `{}`", self.name)
    }
}

/// Looks for every secret's value in the files `source` names, under the repository at `working_dir`. Returns the
/// findings and how many files were looked in.
pub fn scan(
    working_dir: &Path,
    source: Source,
    secrets: &[(String, String)],
) -> anyhow::Result<(Vec<Finding>, usize)> {
    let files = match source {
        Source::Staged => staged_files(working_dir)?,
        Source::Repository if is_git_repository(working_dir) => git_files(working_dir)?,
        Source::Repository => walked_files(working_dir)?,
    };
    let mut findings = vec![];
    for path in &files {
        let bytes = match source {
            Source::Staged => git(working_dir, &["show", &format!(":./{}", path.display())])?,
            Source::Repository => std::fs::read(working_dir.join(path))?,
        };
        findings.extend(
            find_secrets(&bytes, secrets)
                .into_iter()
                .map(|(name, line)| Finding {
                    path: path.clone(),
                    line,
                    name: name.to_string(),
                }),
        );
    }
    Ok((findings, files.len()))
}

/// Runs git in `working_dir` and returns what it printed, or its error.
fn git(working_dir: &Path, args: &[&str]) -> anyhow::Result<Vec<u8>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(working_dir)
        .args(args)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to run git: {e}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

fn is_git_repository(working_dir: &Path) -> bool {
    git(working_dir, &["rev-parse", "--is-inside-work-tree"]).is_ok()
}

/// The NUL-separated paths git printed.
fn paths(listing: &[u8]) -> Vec<PathBuf> {
    listing
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| PathBuf::from(String::from_utf8_lossy(p).into_owned()))
        .collect()
}

/// Files staged for commit with content (added, copied, modified or renamed; a deletion holds nothing).
fn staged_files(working_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let listing = git(
        working_dir,
        &[
            "diff",
            "--cached",
            "--name-only",
            "-z",
            "--diff-filter=ACMR",
            "--relative",
        ],
    )?;
    Ok(paths(&listing))
}

/// Files git tracks or would add (untracked and not ignored) that are there to read.
fn git_files(working_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let listing = git(
        working_dir,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    Ok(paths(&listing)
        .into_iter()
        .filter(|p| working_dir.join(p).is_file())
        .collect())
}

/// Every file under `working_dir` but `.git`, `deployed` and `.uservariables.toml`, for a repository not in git.
fn walked_files(working_dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let skip = |p: &Path| {
        p.starts_with(".git") || p.starts_with("deployed") || p == Path::new(USERVARIABLES_FILE)
    };
    let mut files = vec![];
    for entry in walkdir::WalkDir::new(working_dir)
        .into_iter()
        .filter_entry(|e| {
            e.path()
                .strip_prefix(working_dir)
                .map_or(true, |rel| !skip(rel))
        })
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            files.push(entry.path().strip_prefix(working_dir)?.to_path_buf());
        }
    }
    Ok(files)
}
