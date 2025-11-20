use std::{fs, path::PathBuf};

use dotr::{
    cli::{DeployArgs, ImportArgs, InitArgs, UpdateArgs, run_cli},
    config::Config,
    package::get_package_name,
};

mod common;

const PLAYGROUND_DIR: &str = "tests/playground";
const BASHRC_PATH: &str = "src/.bashrc";

struct TestFixture {
    cwd: PathBuf,
}

impl TestFixture {
    fn new() -> Self {
        Self {
            cwd: PathBuf::from(PLAYGROUND_DIR),
        }
    }

    fn get_cli(&self, command: Option<dotr::cli::Command>) -> dotr::cli::Cli {
        dotr::cli::Cli {
            command,
            working_dir: Some(PLAYGROUND_DIR.to_string()),
        }
    }

    fn init(&self) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Init(InitArgs {}))));
    }

    fn import(&self, path: &str) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Import(ImportArgs {
            path: path.to_string(),
        }))));
    }

    fn deploy(&self, packages: Option<Vec<String>>) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Deploy(DeployArgs { packages }))));
    }

    fn update(&self, packages: Option<Vec<String>>) {
        run_cli(self.get_cli(Some(dotr::cli::Command::Update(UpdateArgs { packages }))));
    }

    fn get_config(&self) -> Config {
        Config::from_path(&self.cwd)
    }

    fn get_package_name(&self, path: &str) -> String {
        get_package_name(path, &self.cwd)
    }

    fn assert_file_exists(&self, path: &str, message: &str) {
        assert!(self.cwd.join(path).exists(), "{}", message);
    }
}

impl Drop for TestFixture {
    fn drop(&mut self) {
        common::teardown(&self.cwd);
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
        &format!("dotfiles/{}", pkg_name),
        "Templated files should be backed up during import",
    );

    // Verify it still has template markers (not compiled)
    let template_content = fs::read_to_string(fixture.cwd.join(format!("dotfiles/{}", pkg_name)))
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
    let package = dotr::package::Package {
        name: "f_bashrc_template".to_string(),
        src: "dotfiles/f_bashrc_template".to_string(),
        dest: "src/.bashrc_output".to_string(),
        dependencies: None,
    };
    config
        .packages
        .insert("f_bashrc_template".to_string(), package);
    config.save(&fixture.cwd);

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
    config.save(&fixture.cwd);

    // Create a templated file
    fs::create_dir_all(fixture.cwd.join("dotfiles")).expect("Failed to create dotfiles dir");
    fs::write(
        fixture.cwd.join("dotfiles/f_config_template"),
        "# {{ APP_NAME }} v{{ VERSION }}\n# Home: {{ HOME }}\n",
    )
    .expect("Failed to create template");

    // Add package manually
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "f_config_template".to_string(),
        src: "dotfiles/f_config_template".to_string(),
        dest: "src/.myconfig".to_string(),
        dependencies: None,
    };
    config
        .packages
        .insert("f_config_template".to_string(), package);
    config.save(&fixture.cwd);

    // Deploy
    fixture.deploy(Some(vec!["f_config_template".to_string()]));

    // Verify substitution
    let content = fs::read_to_string(fixture.cwd.join("src/.myconfig"))
        .expect("Failed to read deployed file");

    assert!(
        content.contains("# MyApp v1.0.0"),
        "Custom variables should be substituted: {}",
        content
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
    let package = dotr::package::Package {
        name: "f_template_test".to_string(),
        src: "dotfiles/f_template_test".to_string(),
        dest: "src/.template_test".to_string(),
        dependencies: None,
    };
    config
        .packages
        .insert("f_template_test".to_string(), package);
    config.save(&fixture.cwd);

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
    config.save(&fixture.cwd);

    // Add package
    let mut config = fixture.get_config();
    let package = dotr::package::Package {
        name: "d_config_dir".to_string(),
        src: "dotfiles/d_config_dir".to_string(),
        dest: "src/.config_output".to_string(),
        dependencies: None,
    };
    config.packages.insert("d_config_dir".to_string(), package);
    config.save(&fixture.cwd);

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
    let package = dotr::package::Package {
        name: "f_templated".to_string(),
        src: "dotfiles/f_templated".to_string(),
        dest: "src/.templated".to_string(),
        dependencies: None,
    };
    config.packages.insert("f_templated".to_string(), package);
    config.save(&fixture.cwd);

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
    let backed_up_content =
        fs::read_to_string(fixture.cwd.join(format!("dotfiles/{}", bashrc_name)))
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
    let package = dotr::package::Package {
        name: "f_advanced_template".to_string(),
        src: "dotfiles/f_advanced_template".to_string(),
        dest: "src/.advanced".to_string(),
        dependencies: None,
    };
    config
        .packages
        .insert("f_advanced_template".to_string(), package);
    config.save(&fixture.cwd);

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
