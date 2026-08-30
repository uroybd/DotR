#[cfg(test)]
mod from_table_tests {
    use crate::profile::Profile;
    use toml::Table;

    #[test]
    fn new_has_empty_actions() {
        let profile = Profile::new("work");

        assert!(profile.pre_actions.is_empty());
        assert!(profile.post_actions.is_empty());
    }

    #[test]
    fn parses_pre_and_post_actions() {
        let toml_str = r#"
pre_actions = ["mkdir -p ~/.config", "echo ready"]
post_actions = ["reload"]
"#;
        let table: Table = toml_str.parse().unwrap();

        let profile = Profile::from_table("work", &table).unwrap();

        assert_eq!(
            profile.pre_actions,
            vec!["mkdir -p ~/.config", "echo ready"]
        );
        assert_eq!(profile.post_actions, vec!["reload"]);
    }

    #[test]
    fn defaults_to_empty_actions_when_missing() {
        let profile = Profile::from_table("work", &Table::new()).unwrap();

        assert!(profile.pre_actions.is_empty());
        assert!(profile.post_actions.is_empty());
    }

    #[test]
    fn errors_when_pre_actions_is_not_an_array() {
        let toml_str = r#"pre_actions = "echo ready""#;
        let table: Table = toml_str.parse().unwrap();

        assert!(Profile::from_table("work", &table).is_err());
    }

    #[test]
    fn errors_when_a_post_action_is_not_a_string() {
        let toml_str = r#"post_actions = [true]"#;
        let table: Table = toml_str.parse().unwrap();

        assert!(Profile::from_table("work", &table).is_err());
    }
}

#[cfg(test)]
mod get_context_variables_tests {
    use crate::config::Config;
    use crate::context::Context;
    use crate::profile::Profile;
    use std::env;
    use toml::Table;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = env::temp_dir().join(format!("dotr_test_profile_ctx_vars_{}", name));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn var(value: &str) -> toml::Value {
        toml::Value::String(value.to_string())
    }

    #[test]
    fn profile_variable_overrides_platform_and_config() {
        let dir = temp_dir("profile-wins");

        let mut config = Config::new();
        config
            .variables
            .insert("PCV_LAYER".to_string(), var("config"));
        let (mut ctx, _) = Context::new(&dir, &config, &None, false).unwrap();

        let mut platform_variables = Table::new();
        platform_variables.insert("PCV_LAYER".to_string(), var("platform"));
        let mut variables = Table::new();
        variables.insert("PCV_LAYER".to_string(), var("profile"));
        ctx.set_profile(Profile {
            name: "work".to_string(),
            platform: Some("macos".to_string()),
            platform_variables,
            variables,
            ..Default::default()
        });

        let merged = ctx.profile.get_context_variables(&ctx);
        assert_eq!(merged.get("PCV_LAYER"), Some(&var("profile")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn platform_variable_wins_over_config_when_profile_is_silent() {
        let dir = temp_dir("platform-over-config");

        let mut config = Config::new();
        config
            .variables
            .insert("PCV_ONLY".to_string(), var("config"));
        let (mut ctx, _) = Context::new(&dir, &config, &None, false).unwrap();

        let mut platform_variables = Table::new();
        platform_variables.insert("PCV_ONLY".to_string(), var("platform"));
        ctx.set_profile(Profile {
            name: "work".to_string(),
            platform: Some("macos".to_string()),
            platform_variables,
            ..Default::default()
        });

        let merged = ctx.profile.get_context_variables(&ctx);
        assert_eq!(merged.get("PCV_ONLY"), Some(&var("platform")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn user_variable_overrides_profile() {
        let dir = temp_dir("user-wins");
        std::fs::write(dir.join(".uservariables.toml"), r#"PCV_LAYER = "user""#).unwrap();

        let config = Config::new();
        let (mut ctx, _) = Context::new(&dir, &config, &None, false).unwrap();

        let mut variables = Table::new();
        variables.insert("PCV_LAYER".to_string(), var("profile"));
        ctx.set_profile(Profile {
            name: "work".to_string(),
            variables,
            ..Default::default()
        });

        let merged = ctx.profile.get_context_variables(&ctx);
        assert_eq!(merged.get("PCV_LAYER"), Some(&var("user")));

        std::fs::remove_dir_all(&dir).ok();
    }
}
