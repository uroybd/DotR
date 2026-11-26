#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "test code: a failure is the point"
)]

use std::{fs, path::PathBuf};

use dotr_dear::{
    cli::{ImportArgs, UpdateArgs, run_cli},
    merge::Resolution,
    package::get_pkg_name_and_rel_path,
};

mod common;

use common::Playground as TestFixture;

const BASHRC_PATH: &str = "src/.bashrc";

impl TestFixture {
    fn update_with(&self, package: &str, conflict: Resolution, dry_run: bool) {
        run_cli(
            self.get_cli(Some(dotr_dear::cli::Command::Update(UpdateArgs {
                packages: Some(vec![package.to_string()]),
                clean: Some(false),
                dry_run,
                conflict: Some(conflict),
                ..Default::default()
            }))),
        )
        .expect("Update failed");
    }

    /// A single-file package `name` from `dotfiles/{name}` (holding `template`) to `src/.{name}`, with `vars` as
    /// config variables, deployed. Returns the deployed path.
    fn deployed_template(&self, name: &str, template: &str, vars: &[(&str, &str)]) -> PathBuf {
        fs::create_dir_all(self.cwd.join("dotfiles")).unwrap();
        fs::write(self.cwd.join("dotfiles").join(name), template).unwrap();
        let mut config = self.get_config();
        for (key, value) in vars {
            config.variables.insert(
                (*key).to_string(),
                toml::Value::String((*value).to_string()),
            );
        }
        config.packages.insert(
            name.to_string(),
            dotr_dear::package::Package {
                name: name.to_string(),
                src: format!("dotfiles/{name}"),
                dest: format!("src/.{name}"),
                ..Default::default()
            },
        );
        config.save(&self.cwd).unwrap();
        self.deploy(Some(vec![name.to_string()]));
        self.cwd.join(format!("src/.{name}"))
    }

    fn template(&self, name: &str) -> String {
        fs::read_to_string(self.cwd.join("dotfiles").join(name)).unwrap()
    }

    fn get_package_name(&self, path: &str) -> String {
        let args = ImportArgs {
            path: path.to_string(),
            ..Default::default()
        };
        get_pkg_name_and_rel_path(&args, &self.cwd).unwrap().0
    }

    fn get_package_ns(&self, path: &str) -> String {
        let args = ImportArgs {
            path: path.to_string(),
            ..Default::default()
        };
        get_pkg_name_and_rel_path(&args, &self.cwd).unwrap().1
    }

    fn assert_file_exists(&self, path: &str, message: &str) {
        assert!(self.cwd.join(path).exists(), "{}", message);
    }
}

#[test]
fn test_template_detection_simple() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a templated file
    let template_file = fixture.cwd.join("src/.bashrc.template");
    fs::write(
        &template_file,
        "# User: {{ USER }}\nexport PATH=\"{{ HOME }}/bin:$PATH\"\n",
    )
    .expect("Failed to create template file");

    // Import the templated file
    fixture.import("src/.bashrc.template");

    // Verify it was imported
    let config = fixture.get_config();
    let pkg_name = fixture.get_package_name("src/.bashrc.template");
    assert!(config.packages.contains_key(&pkg_name));

    // The file SHOULD exist in dotfiles after import (templates are backed up during import)
    fixture.assert_file_exists(
        &format!(
            "dotfiles/{}",
            fixture.get_package_ns("src/.bashrc.template")
        ),
        "Templated files should be backed up during import",
    );

    // Verify it still has template markers (not compiled)
    let pkg_ns = fixture.get_package_ns("src/.bashrc.template");
    let template_content = fs::read_to_string(fixture.cwd.join(format!("dotfiles/{pkg_ns}")))
        .expect("Failed to read template");
    assert!(
        template_content.contains("{{ USER }}"),
        "Template markers should be preserved during import"
    );
}

#[test]
fn test_template_deployment_with_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a templated bashrc in dotfiles directory
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_bashrc_template"),
        "# Generated config\n# User: {{ USER }}\n# Home: {{ HOME }}\nexport EDITOR=\"vim\"\n",
    )
    .expect("Failed to create template");

    // Manually add package to config
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_bashrc_template".to_string(),
        src: "dotfiles/f_bashrc_template".to_string(),
        dest: "src/.bashrc_output".to_string(),
        ..Default::default()
    };
    config
        .packages
        .insert("f_bashrc_template".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy the package
    fixture.deploy(Some(vec!["f_bashrc_template".to_string()]));

    // Check that the deployed file has variables substituted
    let deployed_content = fs::read_to_string(fixture.cwd.join("src/.bashrc_output"))
        .expect("Failed to read deployed file");

    assert!(
        deployed_content.contains("# Generated config"),
        "Template should be deployed"
    );
    assert!(
        !deployed_content.contains("{{ USER }}"),
        "Variables should be substituted"
    );
    assert!(
        !deployed_content.contains("{{ HOME }}"),
        "Variables should be substituted"
    );
}

#[test]
fn test_template_with_custom_variables() {
    let fixture = TestFixture::new();
    fixture.init();

    // Add custom variables
    let mut config = fixture.get_config();
    config.variables.insert(
        "APP_NAME".to_string(),
        toml::Value::String("MyApp".to_string()),
    );
    config.variables.insert(
        "VERSION".to_string(),
        toml::Value::String("1.0.0".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Create a templated file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_config_template"),
        "# {{ APP_NAME }} v{{ VERSION }}\n# Home: {{ HOME }}\n",
    )
    .expect("Failed to create template");

    // Add package manually
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_config_template".to_string(),
        src: "dotfiles/f_config_template".to_string(),
        dest: "src/.myconfig".to_string(),
        ..Default::default()
    };
    config
        .packages
        .insert("f_config_template".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_config_template".to_string()]));

    // Verify substitution
    let content = fs::read_to_string(fixture.cwd.join("src/.myconfig"))
        .expect("Failed to read deployed file");

    assert!(
        content.contains("# MyApp v1.0.0"),
        "Custom variables should be substituted: {content}"
    );
    assert!(
        !content.contains("{{ APP_NAME }}"),
        "Template markers should be gone"
    );
    assert!(
        !content.contains("{{ VERSION }}"),
        "Template markers should be gone"
    );
}

#[test]
fn test_template_not_backed_up_on_update() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a templated file in dotfiles
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_template_test"),
        "# Template: {{ USER }}\n",
    )
    .expect("Failed to create template");

    // Create the deployed version (modified)
    fs::write(
        fixture.cwd.join("src/.template_test"),
        "# Modified by user\n# This should NOT be backed up\n",
    )
    .expect("Failed to create deployed file");

    // Add package
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_template_test".to_string(),
        src: "dotfiles/f_template_test".to_string(),
        dest: "src/.template_test".to_string(),
        ..Default::default()
    };
    config
        .packages
        .insert("f_template_test".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Try to update - should skip backup
    fixture.update(Some(vec!["f_template_test".to_string()]));

    // The template file should still have the original template markers
    let template_content = fs::read_to_string(fixture.cwd.join("dotfiles/f_template_test"))
        .expect("Failed to read template");

    assert!(
        template_content.contains("{{ USER }}"),
        "Template should not be overwritten by backup"
    );
}

#[test]
fn test_clean_does_not_delete_templated_file() {
    let fixture = TestFixture::new();
    fixture.init();

    // Mirrors the real-world d_nushell package: a plain file plus env.nu,
    // which contains template markers. Regression test for a bug where
    // clean() deleted skipped templated files from the repo because the
    // granular backup skip never recorded them as "kept".
    let pkg_dir = fixture.cwd.join("dotfiles/d_nushell");
    fs::create_dir_all(&pkg_dir).expect("Failed to create package dir");
    fs::write(pkg_dir.join("atuin.nu"), "# original atuin config\n").unwrap();
    fs::write(pkg_dir.join("env.nu"), "$env.HOME = \"{{ HOME }}\"\n").unwrap();

    let deployed_dir = fixture.cwd.join("src/.config/nushell");
    fs::create_dir_all(&deployed_dir).expect("Failed to create deployed dir");
    fs::write(deployed_dir.join("atuin.nu"), "# modified\n").unwrap();
    fs::write(deployed_dir.join("env.nu"), "# compiled, no markers\n").unwrap();

    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "d_nushell".to_string(),
        src: "dotfiles/d_nushell".to_string(),
        dest: "src/.config/nushell".to_string(),
        ..Default::default()
    };
    config.packages.insert("d_nushell".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Call update with clean left at the package/CLI default (true),
    // unlike the TestFixture::update() helper which hardcodes clean=false.
    run_cli(
        fixture.get_cli(Some(dotr_dear::cli::Command::Update(UpdateArgs {
            packages: Some(vec!["d_nushell".to_string()]),
            profile: None,
            ignore_errors: false,
            clean: None,
            dry_run: false,
            conflict: Some(dotr_dear::merge::Resolution::Skip),
            allow_secrets: false,
        }))),
    )
    .expect("Update failed");

    assert!(
        pkg_dir.join("env.nu").exists(),
        "Templated file should not be deleted from src by clean()"
    );
    let env_content = fs::read_to_string(pkg_dir.join("env.nu")).expect("Failed to read env.nu");
    assert!(
        env_content.contains("{{ HOME }}"),
        "Templated file should retain its template markers"
    );

    // Plain file should still be backed up as normal.
    let atuin_content =
        fs::read_to_string(pkg_dir.join("atuin.nu")).expect("Failed to read atuin.nu");
    assert!(
        atuin_content.contains("# modified"),
        "Non-templated file should still be backed up when clean is enabled"
    );
}

#[test]
fn test_directory_backup_skips_only_templated_files() {
    let fixture = TestFixture::new();
    fixture.init();

    // Mirrors the real-world d_nushell package: several plain files plus
    // env.nu, which is the only file containing template markers
    // ({{ HOME }}, {{ JIRA_API_TOKEN }}, etc). Backup should be granular:
    // only env.nu should be skipped, the plain files should still be
    // backed up with their modified (deployed) content.
    let pkg_dir = fixture.cwd.join("dotfiles/d_nushell");
    fs::create_dir_all(&pkg_dir).expect("Failed to create package dir");
    fs::write(pkg_dir.join("atuin.nu"), "# original atuin config\n").unwrap();
    fs::write(
        pkg_dir.join("env.nu"),
        "$env.HOME = \"{{ HOME }}\"\n$env.JIRA_API_TOKEN = \"{{ JIRA_API_TOKEN }}\"\n",
    )
    .unwrap();

    // Deployed (live) versions, both modified by the user since last deploy.
    let deployed_dir = fixture.cwd.join("src/.config/nushell");
    fs::create_dir_all(&deployed_dir).expect("Failed to create deployed dir");
    fs::write(
        deployed_dir.join("atuin.nu"),
        "# Modified by user\n# This SHOULD be backed up\n",
    )
    .unwrap();
    fs::write(
        deployed_dir.join("env.nu"),
        "# Compiled/live content with no markers left\n# This should NOT be backed up\n",
    )
    .unwrap();

    // Add package
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "d_nushell".to_string(),
        src: "dotfiles/d_nushell".to_string(),
        dest: "src/.config/nushell".to_string(),
        ..Default::default()
    };
    config.packages.insert("d_nushell".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    fixture.update(Some(vec!["d_nushell".to_string()]));

    // Plain file should have been backed up with the modified deployed content.
    let atuin_content =
        fs::read_to_string(pkg_dir.join("atuin.nu")).expect("Failed to read backed up atuin.nu");
    assert!(
        atuin_content.contains("Modified by user"),
        "Non-templated file in a mixed directory should still be backed up"
    );

    // Templated file should retain its original template markers, untouched.
    let env_content = fs::read_to_string(pkg_dir.join("env.nu")).expect("Failed to read env.nu");
    assert!(
        env_content.contains("{{ HOME }}") && env_content.contains("{{ JIRA_API_TOKEN }}"),
        "Templated file in a mixed directory should not be overwritten by backup"
    );
}

#[test]
fn test_template_directory_deployment() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create a templated directory structure
    fs::create_dir_all(fixture.cwd.join("dotfiles/d_config_dir/nested"))
        .expect("Failed to create template dir");
    fs::write(
        fixture.cwd.join("dotfiles/d_config_dir/app.conf"),
        "app_name={{ APP_NAME }}\nversion={{ VERSION }}\n",
    )
    .expect("Failed to create template file");
    fs::write(
        fixture
            .cwd
            .join("dotfiles/d_config_dir/nested/settings.conf"),
        "user={{ USER }}\nhome={{ HOME }}\n",
    )
    .expect("Failed to create nested template");

    // Add variables
    let mut config = fixture.get_config();
    config.variables.insert(
        "APP_NAME".to_string(),
        toml::Value::String("TestApp".to_string()),
    );
    config.variables.insert(
        "VERSION".to_string(),
        toml::Value::String("2.0.0".to_string()),
    );
    config.save(&fixture.cwd).expect("Failed to save config");

    // Add package
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "d_config_dir".to_string(),
        src: "dotfiles/d_config_dir".to_string(),
        dest: "src/.config_output".to_string(),
        ..Default::default()
    };
    config.packages.insert("d_config_dir".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["d_config_dir".to_string()]));

    // Verify all files are compiled
    let app_conf = fs::read_to_string(fixture.cwd.join("src/.config_output/app.conf"))
        .expect("Failed to read app.conf");
    assert!(
        app_conf.contains("app_name=TestApp"),
        "Variables should be substituted in app.conf"
    );
    assert!(
        app_conf.contains("version=2.0.0"),
        "Variables should be substituted in app.conf"
    );

    let settings_conf =
        fs::read_to_string(fixture.cwd.join("src/.config_output/nested/settings.conf"))
            .expect("Failed to read settings.conf");
    assert!(
        !settings_conf.contains("{{ USER }}"),
        "Variables should be substituted in nested files"
    );
    assert!(
        !settings_conf.contains("{{ HOME }}"),
        "Variables should be substituted in nested files"
    );
}

#[test]
fn test_mixed_template_and_regular_files() {
    let fixture = TestFixture::new();
    fixture.init();

    // Import a regular file (no template markers)
    fixture.import(BASHRC_PATH);

    // Create a templated file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_templated"),
        "# User: {{ USER }}\n",
    )
    .expect("Failed to create template");

    // Add templated package
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_templated".to_string(),
        src: "dotfiles/f_templated".to_string(),
        dest: "src/.templated".to_string(),
        ..Default::default()
    };
    config.packages.insert("f_templated".to_string(), package);
    config
        .profiles
        .get_mut("default")
        .unwrap()
        .dependencies
        .push("f_templated".to_string());
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy all
    fixture.deploy(None);

    // Regular file should exist
    fixture.assert_file_exists("src/.bashrc", "Regular file should be deployed");

    // Templated file should be compiled
    let templated_content = fs::read_to_string(fixture.cwd.join("src/.templated"))
        .expect("Failed to read templated file");
    assert!(
        !templated_content.contains("{{ USER }}"),
        "Template should be compiled"
    );

    // Try to update - only regular file should be backed up
    fs::write(fixture.cwd.join("src/.bashrc"), "# Modified regular file\n")
        .expect("Failed to modify regular file");

    fs::write(
        fixture.cwd.join("src/.templated"),
        "# Modified templated file\n",
    )
    .expect("Failed to modify templated file");

    fixture.update(None);

    // Regular file backup should reflect changes
    let bashrc_name = fixture.get_package_name(BASHRC_PATH);
    let backed_up_content = fs::read_to_string(fixture.cwd.join(format!("dotfiles/{bashrc_name}")))
        .expect("Failed to read backed up regular file");
    assert!(
        backed_up_content.contains("# Modified regular file"),
        "Regular file should be backed up with modifications"
    );

    // Templated file should still have template markers (not overwritten)
    let template_content = fs::read_to_string(fixture.cwd.join("dotfiles/f_templated"))
        .expect("Failed to read template");
    assert!(
        template_content.contains("{{ USER }}"),
        "Template should not be overwritten by backup"
    );
}

#[test]
fn test_template_with_tera_statements() {
    let fixture = TestFixture::new();
    fixture.init();

    // Create template with Tera control structures
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_advanced_template"),
        "# Config\n{% if USER %}user={{ USER }}{% endif %}\n{# This is a comment #}\n",
    )
    .expect("Failed to create template");

    // Add package
    let mut config = fixture.get_config();
    let package = dotr_dear::package::Package {
        name: "f_advanced_template".to_string(),
        src: "dotfiles/f_advanced_template".to_string(),
        dest: "src/.advanced".to_string(),
        ..Default::default()
    };
    config
        .packages
        .insert("f_advanced_template".to_string(), package);
    config.save(&fixture.cwd).expect("Failed to save config");

    // Deploy
    fixture.deploy(Some(vec!["f_advanced_template".to_string()]));

    // Verify Tera syntax was processed
    let content = fs::read_to_string(fixture.cwd.join("src/.advanced"))
        .expect("Failed to read deployed file");

    assert!(
        !content.contains("{% if"),
        "Tera statements should be processed"
    );
    assert!(!content.contains("{#"), "Tera comments should be removed");
    assert!(
        content.contains("user="),
        "Tera conditionals should be evaluated"
    );
}

#[test]
fn test_update_merges_deployed_edits_into_the_template() {
    let fixture = TestFixture::new();
    fixture.init();
    let deployed = fixture.deployed_template(
        "f_merge_plain",
        "dir = {{ APP_DIR }}\nmode = fast\n",
        &[("APP_DIR", "/opt/app-test")],
    );
    let edited = "dir = /opt/app-test\nmode = slow\ncache = /opt/app-test/cache\n";
    fs::write(&deployed, edited).unwrap();

    fixture.update_with("f_merge_plain", Resolution::Skip, false);

    assert_eq!(
        fixture.template("f_merge_plain"),
        "dir = {{ APP_DIR }}\nmode = slow\ncache = {{ APP_DIR }}/cache\n"
    );
    // The merged template renders the deployed file exactly, so deploying again changes nothing.
    fixture.deploy(Some(vec!["f_merge_plain".to_string()]));
    assert_eq!(fs::read_to_string(&deployed).unwrap(), edited);
}

#[test]
fn test_update_keeps_the_variable_in_an_edited_line() {
    let fixture = TestFixture::new();
    fixture.init();
    let deployed = fixture.deployed_template(
        "f_merge_var_line",
        "data = \"{{ APP_DIR }}/Data\"\n",
        &[("APP_DIR", "/opt/app-test")],
    );
    fs::write(&deployed, "data = \"/opt/app-test/Documents\"\n").unwrap();

    fixture.update_with("f_merge_var_line", Resolution::Skip, false);

    assert_eq!(
        fixture.template("f_merge_var_line"),
        "data = \"{{ APP_DIR }}/Documents\"\n"
    );
}

#[test]
fn test_update_conflicts_follow_the_chosen_resolution() {
    let fixture = TestFixture::new();
    fixture.init();
    let template = "name = {{ NAME | upper }}\nmode = fast\n";
    let deployed = fixture.deployed_template("f_merge_conflict", template, &[("NAME", "dotr")]);
    fs::write(&deployed, "name = Custom\nmode = fast\n").unwrap();

    fixture.update_with("f_merge_conflict", Resolution::Skip, false);
    assert_eq!(
        fixture.template("f_merge_conflict"),
        template,
        "skip leaves it"
    );
    fixture.update_with("f_merge_conflict", Resolution::Ours, false);
    assert_eq!(
        fixture.template("f_merge_conflict"),
        template,
        "ours keeps it"
    );

    fixture.update_with("f_merge_conflict", Resolution::Both, false);
    assert_eq!(
        fixture.template("f_merge_conflict"),
        "name = {{ NAME | upper }}\nname = Custom\nmode = fast\n"
    );

    fs::write(fixture.cwd.join("dotfiles/f_merge_conflict"), template).unwrap();
    fixture.update_with("f_merge_conflict", Resolution::Theirs, false);
    assert_eq!(
        fixture.template("f_merge_conflict"),
        "name = Custom\nmode = fast\n"
    );
}

#[test]
fn test_update_dry_run_leaves_the_template_alone() {
    let fixture = TestFixture::new();
    fixture.init();
    let template = "dir = {{ APP_DIR }}\nmode = fast\n";
    let deployed =
        fixture.deployed_template("f_merge_dry", template, &[("APP_DIR", "/opt/app-test")]);
    fs::write(&deployed, "dir = /opt/app-test\nmode = slow\n").unwrap();

    fixture.update_with("f_merge_dry", Resolution::Theirs, true);

    assert_eq!(fixture.template("f_merge_dry"), template);
}

/// Writes `.uservariables.toml` with `TOKEN`, runs `body`, and removes it again (the playground teardown does not).
fn with_token<T>(fixture: &TestFixture, body: impl FnOnce() -> T) -> T {
    let path = fixture.cwd.join(".uservariables.toml");
    fs::write(&path, "TOKEN = \"tok-123456\"\n").unwrap();
    let result = body();
    fs::remove_file(&path).unwrap();
    result
}

#[test]
fn test_update_does_not_copy_a_plain_file_holding_a_secret() {
    let fixture = TestFixture::new();
    fixture.init();
    let deployed = fixture.deployed_template("f_plain_secret", "plain = yes\n", &[]);
    fs::write(&deployed, "plain = yes\ntoken = tok-123456\n").unwrap();

    with_token(&fixture, || {
        fixture.update_with("f_plain_secret", Resolution::Skip, false);
    });

    assert_eq!(fixture.template("f_plain_secret"), "plain = yes\n");
}

#[test]
fn test_update_allow_secrets_backs_the_file_up_anyway() {
    let fixture = TestFixture::new();
    fixture.init();
    let deployed = fixture.deployed_template("f_plain_secret_allowed", "plain = yes\n", &[]);
    fs::write(&deployed, "plain = yes\ntoken = tok-123456\n").unwrap();

    with_token(&fixture, || {
        run_cli(
            fixture.get_cli(Some(dotr_dear::cli::Command::Update(UpdateArgs {
                packages: Some(vec!["f_plain_secret_allowed".to_string()]),
                clean: Some(false),
                conflict: Some(Resolution::Skip),
                allow_secrets: true,
                ..Default::default()
            }))),
        )
        .expect("Update failed");
    });

    assert_eq!(
        fixture.template("f_plain_secret_allowed"),
        "plain = yes\ntoken = tok-123456\n"
    );
}

#[test]
fn test_update_keeps_other_files_and_the_skipped_one_in_a_directory() {
    let fixture = TestFixture::new();
    fixture.init();
    let pkg_dir = fixture.cwd.join("dotfiles/d_secret_dir");
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(pkg_dir.join("ok.conf"), "ok = 1\n").unwrap();
    fs::write(pkg_dir.join("leaky.conf"), "leaky = 1\n").unwrap();
    fs::write(pkg_dir.join("blob.bin"), b"\x00\xffold").unwrap();
    let mut config = fixture.get_config();
    config.packages.insert(
        "d_secret_dir".into(),
        dotr_dear::package::Package {
            name: "d_secret_dir".into(),
            src: "dotfiles/d_secret_dir".into(),
            dest: "src/.d_secret_dir".into(),
            ..Default::default()
        },
    );
    config.save(&fixture.cwd).unwrap();
    fixture.deploy(Some(vec!["d_secret_dir".to_string()]));
    let deployed = fixture.cwd.join("src/.d_secret_dir");
    fs::write(deployed.join("ok.conf"), "ok = 2\n").unwrap();
    fs::write(deployed.join("leaky.conf"), "leaky = tok-123456\n").unwrap();
    fs::write(deployed.join("blob.bin"), b"\x00\xfftok-123456").unwrap();

    with_token(&fixture, || {
        run_cli(
            fixture.get_cli(Some(dotr_dear::cli::Command::Update(UpdateArgs {
                packages: Some(vec!["d_secret_dir".to_string()]),
                clean: Some(true),
                conflict: Some(Resolution::Skip),
                ..Default::default()
            }))),
        )
        .expect("Update failed");
    });

    assert_eq!(
        fs::read_to_string(pkg_dir.join("ok.conf")).unwrap(),
        "ok = 2\n"
    );
    assert_eq!(
        fs::read_to_string(pkg_dir.join("leaky.conf")).unwrap(),
        "leaky = 1\n",
        "not copied, and not removed by clean either"
    );
    assert_eq!(fs::read(pkg_dir.join("blob.bin")).unwrap(), b"\x00\xffold");
}

#[test]
fn test_update_writes_a_secret_back_as_its_tag() {
    let fixture = TestFixture::new();
    fixture.init();
    let deployed = fixture.deployed_template(
        "f_merge_secret",
        "dir = {{ APP_DIR }}\n",
        &[("APP_DIR", "/opt/app-test")],
    );
    fs::write(&deployed, "dir = /opt/app-test\ntoken = tok-123456\n").unwrap();

    with_token(&fixture, || {
        fixture.update_with("f_merge_secret", Resolution::Skip, false);
    });

    assert_eq!(
        fixture.template("f_merge_secret"),
        "dir = {{ APP_DIR }}\ntoken = {{ TOKEN }}\n"
    );
}

#[test]
fn test_update_copies_binary_files_as_is_next_to_a_template() {
    let fixture = TestFixture::new();
    fixture.init();
    // The playground keeps deployed files between runs; start from none.
    fs::remove_dir_all(fixture.cwd.join("src/.d_merge_binary")).ok();
    let pkg_dir = fixture.cwd.join("dotfiles/d_merge_binary");
    fs::create_dir_all(&pkg_dir).unwrap();
    fs::write(pkg_dir.join("app.conf"), "dir = {{ APP_DIR }}\n").unwrap();
    fs::write(
        pkg_dir.join("icon.png"),
        b"\x89PNG\r\n\x1a\n\x00\xff{{ x }}",
    )
    .unwrap();
    let mut config = fixture.get_config();
    config.variables.insert(
        "APP_DIR".into(),
        toml::Value::String("/opt/app-test".into()),
    );
    config.packages.insert(
        "d_merge_binary".into(),
        dotr_dear::package::Package {
            name: "d_merge_binary".into(),
            src: "dotfiles/d_merge_binary".into(),
            dest: "src/.d_merge_binary".into(),
            ..Default::default()
        },
    );
    config.save(&fixture.cwd).unwrap();
    fixture.deploy(Some(vec!["d_merge_binary".to_string()]));
    let deployed = fixture.cwd.join("src/.d_merge_binary");
    let new_icon: &[u8] = b"\x89PNG\r\n\x1a\n\x00\xfe\xfd changed";
    fs::write(deployed.join("icon.png"), new_icon).unwrap();
    // A templated file whose deployed copy turned binary is left alone, not diffed.
    fs::write(deployed.join("app.conf"), b"\xff\xfe\x00binary").unwrap();

    fixture.update_with("d_merge_binary", Resolution::Theirs, false);

    assert_eq!(fs::read(pkg_dir.join("icon.png")).unwrap(), new_icon);
    assert_eq!(
        fs::read_to_string(pkg_dir.join("app.conf")).unwrap(),
        "dir = {{ APP_DIR }}\n"
    );
}

#[cfg(unix)]
#[test]
fn test_update_edit_uses_what_the_editor_saved() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = TestFixture::new();
    fixture.init();
    let deployed = fixture.deployed_template(
        "f_merge_edit",
        "name = {{ NAME | upper }}\nmode = fast\n",
        &[("NAME", "dotr")],
    );
    fs::write(&deployed, "name = Custom\nmode = fast\n").unwrap();
    let editor = fixture.cwd.join("src/.merge-editor.sh");
    fs::write(
        &editor,
        "#!/bin/sh\ngrep -q '^<<<<<<< template' \"$1\" || exit 1\nprintf 'name = {{ NAME | lower }}\\nmode = fast\\n' > \"$1\"\n",
    )
    .unwrap();
    fs::set_permissions(&editor, fs::Permissions::from_mode(0o755)).unwrap();

    // SAFETY: the integration tests run single-threaded (`just test` and CI pass `--test-threads=1`), so no other
    // thread reads the environment while it changes.
    unsafe { std::env::set_var("DOTR_EDITOR", fs::canonicalize(&editor).unwrap()) };
    fixture.update_with("f_merge_edit", Resolution::Edit, false);
    // SAFETY: as above; this puts the environment back.
    unsafe { std::env::remove_var("DOTR_EDITOR") };
    fs::remove_file(&editor).unwrap();

    assert_eq!(
        fixture.template("f_merge_edit"),
        "name = {{ NAME | lower }}\nmode = fast\n"
    );
}

#[test]
fn test_deploy_and_diff_handle_a_deployed_file_that_is_not_text() {
    let fixture = TestFixture::new();
    fixture.init();
    let deployed = fixture.deployed_template(
        "f_binary_dest",
        "dir = {{ APP_DIR }}\n",
        &[("APP_DIR", "/opt/app-test")],
    );
    fs::write(&deployed, b"\xff\xfe\x00 not text").unwrap();

    run_cli(fixture.get_cli(Some(dotr_dear::cli::Command::Diff(
        dotr_dear::cli::DiffArgs {
            packages: Some(vec!["f_binary_dest".to_string()]),
            ..Default::default()
        },
    ))))
    .expect("diff must not fail on a binary deployed file");
    fixture.deploy(Some(vec!["f_binary_dest".to_string()]));

    assert_eq!(
        fs::read_to_string(&deployed).unwrap(),
        "dir = /opt/app-test\n"
    );
}
