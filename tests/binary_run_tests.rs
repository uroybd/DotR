#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "test code: a failure is the point"
)]
//! What `dotr` prints, run as the real binary on a repository and a HOME of its own (never the playground): deploy,
//! diff and update must agree on what has changed, and their summaries must say what happened.

use std::{fs, path::PathBuf, process::Command};

struct Repo {
    root: PathBuf,
}

impl Repo {
    /// A repository holding one directory package, `d_app` (a text file in a subfolder and a binary file), deployed
    /// to `~/.app`, with `COLOR = "red"` among its variables.
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("dotr-run-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(root.join("home")).unwrap();
        fs::create_dir_all(root.join("repo/dotfiles/d_app/sub")).unwrap();
        let repo = Self { root };
        repo.run(&["init"]);
        fs::write(
            repo.path("repo/config.toml"),
            r#"banner = false
symlink = false

[variables]
COLOR = "red"

[packages.d_app]
src = "dotfiles/d_app"
dest = "~/.app"

[profiles.default]
dependencies = ["d_app"]
"#,
        )
        .unwrap();
        fs::write(repo.path("repo/dotfiles/d_app/sub/a.conf"), "a = 1\n").unwrap();
        fs::write(
            repo.path("repo/dotfiles/d_app/logo.bin"),
            b"\x89PNG\xff\xfe",
        )
        .unwrap();
        repo
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    fn deployed(&self, rel: &str) -> PathBuf {
        self.root.join("home/.app").join(rel)
    }

    /// Runs dotr in the repository and returns everything it printed (stdout, then stderr).
    fn run(&self, args: &[&str]) -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_dotr"))
            .args(args)
            .current_dir(self.path("repo"))
            .env("HOME", self.path("home"))
            .output()
            .unwrap();
        format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

#[test]
fn an_unchanged_binary_file_is_not_deployed_again() {
    let repo = Repo::new();
    repo.run(&["deploy"]);

    let again = repo.run(&["deploy"]);

    assert!(!again.contains("=> Deployed"), "{again}");
    assert!(again.contains("no changes"), "{again}");
    assert!(
        !repo.deployed("logo.bin.dotrbak").exists(),
        "no backup of an identical file"
    );
}

#[test]
fn a_dry_run_lists_files_not_directories() {
    let repo = Repo::new();

    let dry = repo.run(&["deploy", "--dry-run"]);

    let listed: Vec<&str> = dry
        .lines()
        .filter(|l| l.starts_with("=> Deployed"))
        .collect();
    assert_eq!(listed.len(), 2, "a.conf and logo.bin only: {listed:?}");
    assert!(
        listed
            .iter()
            .all(|l| l.ends_with("a.conf") || l.ends_with("logo.bin"))
    );
}

#[test]
fn diff_reports_what_deploy_would_write() {
    let repo = Repo::new();
    repo.run(&["deploy"]);
    fs::remove_file(repo.deployed("sub/a.conf")).unwrap();
    fs::write(repo.deployed("logo.bin"), b"\x89PNG\x00other").unwrap();

    let diff = repo.run(&["diff"]);

    assert!(diff.contains("Changes in a.conf"), "a deleted file: {diff}");
    assert!(
        diff.contains("Changes in logo.bin"),
        "a changed binary file: {diff}"
    );
}
