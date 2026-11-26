use std::{fs, path::PathBuf, process::Command};

use super::{Finding, Source, scan};

fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dotr-scan-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn git(dir: &PathBuf, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed");
}

fn secrets() -> Vec<(String, String)> {
    vec![
        ("TOKEN".into(), "tok-123456".into()),
        ("PASS".into(), "hunter22".into()),
    ]
}

fn found(path: &str, line: Option<usize>, name: &str) -> Finding {
    Finding {
        path: path.into(),
        line,
        name: name.into(),
    }
}

#[test]
fn finds_every_secret_with_its_line_outside_git() {
    let dir = temp_dir();
    fs::write(dir.join("clean.conf"), "a = {{ TOKEN }}\n").unwrap();
    fs::write(
        dir.join("leaky.conf"),
        "a = 1\ntoken = tok-123456\npass = hunter22\n",
    )
    .unwrap();
    fs::write(dir.join("blob.bin"), b"\x00\xfftok-123456").unwrap();
    fs::write(dir.join(".uservariables.toml"), "TOKEN = \"tok-123456\"\n").unwrap();
    fs::create_dir_all(dir.join("deployed")).unwrap();
    fs::write(dir.join("deployed/copy"), "tok-123456").unwrap();

    let (mut findings, files) = scan(&dir, Source::Repository, &secrets()).unwrap();
    findings.sort_by(|a, b| (&a.path, &a.name).cmp(&(&b.path, &b.name)));

    assert_eq!(
        files, 3,
        ".uservariables.toml and deployed are not looked in"
    );
    assert_eq!(
        findings,
        vec![
            found("blob.bin", None, "TOKEN"),
            found("leaky.conf", Some(3), "PASS"),
            found("leaky.conf", Some(2), "TOKEN"),
        ]
    );
    assert_eq!(
        findings[1].describe(),
        "leaky.conf:3: holds the value of the secret `PASS`"
    );
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn in_git_looks_at_what_it_would_commit() {
    let dir = temp_dir();
    git(&dir, &["init", "-q"]);
    fs::write(dir.join(".gitignore"), "ignored.conf\n").unwrap();
    fs::write(dir.join("ignored.conf"), "tok-123456").unwrap();
    fs::write(dir.join("untracked.conf"), "tok-123456").unwrap();

    let (findings, _) = scan(&dir, Source::Repository, &secrets()).unwrap();

    assert_eq!(findings, vec![found("untracked.conf", Some(1), "TOKEN")]);
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn staged_looks_at_the_index_not_the_working_tree() {
    let dir = temp_dir();
    git(&dir, &["init", "-q"]);
    fs::write(dir.join("a.conf"), "token = tok-123456\n").unwrap();
    fs::write(dir.join("b.conf"), "pass = hunter22\n").unwrap();
    git(&dir, &["add", "a.conf"]);
    // Fixed in the working tree but not restaged: the commit would still hold the value.
    fs::write(dir.join("a.conf"), "token = {{ TOKEN }}\n").unwrap();

    let (findings, files) = scan(&dir, Source::Staged, &secrets()).unwrap();

    assert_eq!(files, 1, "b.conf is not staged");
    assert_eq!(findings, vec![found("a.conf", Some(1), "TOKEN")]);

    git(&dir, &["add", "a.conf"]);
    let (findings, _) = scan(&dir, Source::Staged, &secrets()).unwrap();
    assert!(findings.is_empty(), "restaged without the value");
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn staged_outside_git_is_an_error() {
    let dir = temp_dir();
    assert!(scan(&dir, Source::Staged, &secrets()).is_err());
    fs::remove_dir_all(&dir).ok();
}

#[test]
fn staged_works_from_a_folder_inside_a_bigger_repository() {
    let root = temp_dir();
    git(&root, &["init", "-q"]);
    let dir = root.join("dotfiles");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("a.conf"), "x\ntoken = tok-123456\n").unwrap();
    fs::write(root.join("outside.conf"), "tok-123456").unwrap();
    git(&root, &["add", "."]);

    let (findings, files) = scan(&dir, Source::Staged, &secrets()).unwrap();

    assert_eq!(files, 1, "only what is under the dotfiles folder");
    assert_eq!(findings, vec![found("a.conf", Some(2), "TOKEN")]);
    fs::remove_dir_all(&root).ok();
}

#[test]
fn staged_handles_names_with_spaces_and_non_ascii() {
    let dir = temp_dir();
    git(&dir, &["init", "-q"]);
    fs::write(dir.join("my file ü.conf"), "token = tok-123456\n").unwrap();
    fs::write(dir.join("gone.conf"), "x\n").unwrap();
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-q", "-m", "c"]);
    // Staged for deletion: nothing of it goes into the commit, so there is nothing to look in.
    git(&dir, &["rm", "-q", "gone.conf"]);
    fs::write(dir.join("my file ü.conf"), "token = tok-123456\nmore\n").unwrap();
    git(&dir, &["add", "my file ü.conf"]);

    let (findings, files) = scan(&dir, Source::Staged, &secrets()).unwrap();

    assert_eq!(files, 1);
    assert_eq!(findings, vec![found("my file ü.conf", Some(1), "TOKEN")]);
    fs::remove_dir_all(&dir).ok();
}
