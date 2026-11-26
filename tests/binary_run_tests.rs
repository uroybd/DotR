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

use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command};

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
        self.run_with(args, |_| {}).1
    }

    /// Runs dotr with `env` adjusting its environment, and returns whether it succeeded and what it printed. No
    /// editor is set unless `env` sets one, so a test never opens the user's.
    fn run_with(&self, args: &[&str], env: impl FnOnce(&mut Command)) -> (bool, String) {
        let mut command = Command::new(env!("CARGO_BIN_EXE_dotr"));
        command
            .args(args)
            .current_dir(self.path("repo"))
            .env("HOME", self.path("home"))
            .env_remove("DOTR_EDITOR")
            .env_remove("VISUAL")
            .env_remove("EDITOR");
        env(&mut command);
        let output = command.output().unwrap();
        let printed = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.success(), printed)
    }

    /// Makes `a.conf` the template `a = {{ COLOR }}`, deploys it, and edits the deployed line the template computes:
    /// a conflict for the next update.
    fn conflicted_template(&self) {
        fs::write(
            self.path("repo/dotfiles/d_app/sub/a.conf"),
            "a = {{ COLOR }}\n",
        )
        .unwrap();
        self.run(&["deploy"]);
        fs::write(self.deployed("sub/a.conf"), "a = blue\n").unwrap();
    }

    /// Saves `TOKEN` as an answered prompt: a secret.
    fn token(&self) {
        fs::write(
            self.path("repo/.uservariables.toml"),
            "TOKEN = \"tok-123456\"\n",
        )
        .unwrap();
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

#[test]
fn the_update_summary_counts_a_file_left_out() {
    let repo = Repo::new();
    fs::write(
        repo.path("repo/dotfiles/d_app/sub/a.conf"),
        "a = {{ COLOR }}\n",
    )
    .unwrap();
    repo.run(&["deploy"]);
    // A change to a line the template computes: a conflict, which `--conflict skip` leaves unsettled.
    fs::write(repo.deployed("sub/a.conf"), "a = blue\n").unwrap();

    let update = repo.run(&["update", "--conflict", "skip"]);

    assert!(!update.contains("1 backed up"), "{update}");
    assert!(update.contains("1 left out"), "{update}");
}

#[test]
fn a_single_file_left_out_is_not_counted_as_unchanged() {
    let repo = Repo::new();
    fs::write(repo.path("repo/dotfiles/f_rc"), "rc = {{ COLOR }}\n").unwrap();
    let config = fs::read_to_string(repo.path("repo/config.toml")).unwrap();
    fs::write(
        repo.path("repo/config.toml"),
        config.replace(
            "dependencies = [\"d_app\"]",
            "dependencies = [\"f_rc\"]\n\n[packages.f_rc]\nsrc = \"dotfiles/f_rc\"\ndest = \"~/.rc\"",
        ),
    )
    .unwrap();
    repo.run(&["deploy"]);
    fs::write(repo.path("home/.rc"), "rc = blue\n").unwrap();

    let update = repo.run(&["update", "--conflict", "skip"]);

    assert!(!update.contains("no changes"), "{update}");
    assert!(update.contains("1 left out"), "{update}");
}

#[test]
fn scan_fails_on_a_secret_naming_it_never_showing_it() {
    let repo = Repo::new();
    repo.token();
    fs::write(
        repo.path("repo/dotfiles/d_app/sub/a.conf"),
        "a = 1\ntoken = tok-123456\n",
    )
    .unwrap();

    let (ok, printed) = repo.run_with(&["scan"], |_| {});

    assert!(
        !ok,
        "a finding fails the command, so a pre-commit hook stops: {printed}"
    );
    assert!(
        printed.contains("dotfiles/d_app/sub/a.conf:2: holds the value of the secret `TOKEN`"),
        "{printed}"
    );
    assert!(
        !printed.contains("tok-123456"),
        "never the value: {printed}"
    );

    fs::write(
        repo.path("repo/dotfiles/d_app/sub/a.conf"),
        "token = {{ TOKEN }}\n",
    )
    .unwrap();
    let (ok, printed) = repo.run_with(&["scan"], |_| {});
    assert!(ok, "{printed}");
    assert!(printed.contains("No secrets found"), "{printed}");
}

#[test]
fn scan_with_no_secrets_has_nothing_to_look_for() {
    let repo = Repo::new();

    let (ok, printed) = repo.run_with(&["scan"], |_| {});

    assert!(ok, "{printed}");
    assert!(printed.contains("No secrets to look for"), "{printed}");
}

#[test]
fn scan_staged_needs_git() {
    let repo = Repo::new();
    repo.token();

    let (ok, printed) = repo.run_with(&["scan", "--staged"], |_| {});

    assert!(
        !ok,
        "outside a repository there is no index to look in: {printed}"
    );
}

#[test]
fn dump_user_vars_writes_the_answers_to_stdout_or_a_file() {
    let repo = Repo::new();
    repo.token();

    let printed = repo.run(&["dump-user-vars"]);
    assert!(printed.contains("TOKEN = \"tok-123456\""), "{printed}");

    let out = repo.path("dump.toml");
    let printed = repo.run(&["dump-user-vars", "--output", out.to_str().unwrap()]);
    assert!(printed.contains("Wrote"), "{printed}");
    let dumped: toml::Table = fs::read_to_string(&out).unwrap().parse().unwrap();
    assert_eq!(dumped["TOKEN"].as_str(), Some("tok-123456"));
}

#[test]
fn deploy_keeps_a_binary_executable_and_a_deployed_text_file_private() {
    let repo = Repo::new();
    let logo = repo.path("repo/dotfiles/d_app/logo.bin");
    fs::set_permissions(&logo, fs::Permissions::from_mode(0o755)).unwrap();
    repo.run(&["deploy"]);
    let mode = |p: PathBuf| fs::metadata(p).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        mode(repo.deployed("logo.bin")),
        0o755,
        "a binary is copied, mode and all"
    );

    // A deployed text file the user made private stays private when deploy rewrites it.
    fs::set_permissions(
        repo.deployed("sub/a.conf"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    fs::write(repo.path("repo/dotfiles/d_app/sub/a.conf"), "a = 2\n").unwrap();
    repo.run(&["deploy"]);
    assert_eq!(
        fs::read_to_string(repo.deployed("sub/a.conf")).unwrap(),
        "a = 2\n"
    );
    assert_eq!(mode(repo.deployed("sub/a.conf")), 0o600);
}

#[test]
fn a_dry_run_with_conflict_edit_opens_no_editor() {
    let repo = Repo::new();
    repo.conflicted_template();
    let marker = repo.path("editor-ran");
    let editor = repo.path("editor.sh");
    fs::write(
        &editor,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o755)).unwrap();

    let (ok, printed) = repo.run_with(&["update", "--dry-run", "--conflict", "edit"], |c| {
        c.env("DOTR_EDITOR", &editor);
    });

    assert!(ok, "{printed}");
    assert!(printed.contains("Would open"), "{printed}");
    assert!(!marker.exists(), "a dry run opens no editor");
    assert_eq!(
        fs::read_to_string(repo.path("repo/dotfiles/d_app/sub/a.conf")).unwrap(),
        "a = {{ COLOR }}\n"
    );
}

#[test]
fn conflict_edit_without_an_editor_leaves_the_file_out() {
    let repo = Repo::new();
    repo.conflicted_template();

    let (ok, printed) = repo.run_with(&["update", "--conflict", "edit"], |_| {});

    assert!(ok, "the rest of the update goes on: {printed}");
    assert!(
        printed.contains("set DOTR_EDITOR, VISUAL or EDITOR"),
        "{printed}"
    );
    assert!(printed.contains("1 left out"), "{printed}");
    assert_eq!(
        fs::read_to_string(repo.path("repo/dotfiles/d_app/sub/a.conf")).unwrap(),
        "a = {{ COLOR }}\n"
    );
}
