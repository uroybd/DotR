#[cfg(test)]
mod context_tests {
    use crate::config::Config;
    use crate::context::{Context, print_variable};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use toml::Table;

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn create_temp_dir() -> PathBuf {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir =
            std::env::temp_dir().join(format!("dotr_test_{}_{}", std::process::id(), counter));
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
        temp_dir
    }

    fn create_test_config(temp_dir: &Path) -> Config {
        let config_path = temp_dir.join("config.toml");
        if !config_path.exists() {
            fs::write(&config_path, "").expect("Failed to create config.toml");
        }
        Config::from_path(temp_dir).expect("Failed to load config")
    }

    #[test]
    fn test_context_new() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        assert_eq!(&ctx.working_dir, &temp_dir);
        assert!(
            !ctx.variables.is_empty(),
            "Should have environment variables"
        );
        assert!(
            ctx.user_variables.is_empty(),
            "Should have no user variables initially"
        );
    }

    #[test]
    fn test_context_contains_env_variables() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        // HOME should always be in environment
        assert!(
            ctx.get_variable("HOME").is_some(),
            "Should have HOME env var"
        );
    }

    #[test]
    fn test_parse_uservariables_no_file() {
        let temp_dir = create_temp_dir();
        let user_vars =
            Context::parse_uservariables(&temp_dir).expect("Failed to parse uservariables");

        assert!(
            user_vars.is_empty(),
            "Should return empty table when no file exists"
        );
    }

    #[test]
    fn test_parse_uservariables_simple() {
        let temp_dir = create_temp_dir();
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(
            uservars_path,
            r#"
TEST_VAR = "test_value"
ANOTHER_VAR = "another_value"
"#,
        )
        .expect("Failed to write .uservariables.toml");

        let user_vars =
            Context::parse_uservariables(&temp_dir).expect("Failed to parse uservariables");

        assert_eq!(user_vars.len(), 2);
        assert_eq!(
            user_vars.get("TEST_VAR"),
            Some(&toml::Value::String("test_value".to_string()))
        );
        assert_eq!(
            user_vars.get("ANOTHER_VAR"),
            Some(&toml::Value::String("another_value".to_string()))
        );
    }

    #[test]
    fn test_parse_uservariables_nested() {
        let temp_dir = create_temp_dir();
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(
            uservars_path,
            r#"
[database]
host = "localhost"
port = 5432

[api]
key = "secret-key"
"#,
        )
        .expect("Failed to write .uservariables.toml");

        let user_vars =
            Context::parse_uservariables(&temp_dir).expect("Failed to parse uservariables");

        assert!(user_vars.contains_key("database"));
        assert!(user_vars.contains_key("api"));

        if let Some(toml::Value::Table(db)) = user_vars.get("database") {
            assert_eq!(
                db.get("host"),
                Some(&toml::Value::String("localhost".to_string()))
            );
            assert_eq!(db.get("port"), Some(&toml::Value::Integer(5432)));
        } else {
            panic!("database should be a table");
        }
    }

    #[test]
    fn test_parse_uservariables_invalid_toml() {
        let temp_dir = create_temp_dir();
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(uservars_path, "invalid toml {{{").expect("Failed to write .uservariables.toml");

        // Should return an error for invalid TOML
        let result = Context::parse_uservariables(&temp_dir);
        assert!(result.is_err(), "Should return error on invalid TOML");
    }

    #[test]
    fn test_get_variable() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.variables.insert(
            "TEST_VAR".to_string(),
            toml::Value::String("test_value".to_string()),
        );

        assert_eq!(
            ctx.get_variable("TEST_VAR"),
            Some(&toml::Value::String("test_value".to_string()))
        );
        assert_eq!(ctx.get_variable("NONEXISTENT"), None);
    }

    #[test]
    fn test_get_user_variable() {
        let temp_dir = create_temp_dir();
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(uservars_path, r#"USER_VAR = "user_value""#)
            .expect("Failed to write .uservariables.toml");

        let mut config = create_test_config(&temp_dir);
        config
            .prompts
            .insert("USER_VAR".to_string(), "Enter USER_VAR".to_string());
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");
        ctx.get_prompted_variables(&config, &None)
            .expect("Failed to resolve prompted variables");

        assert_eq!(
            ctx.get_user_variable("USER_VAR"),
            Some(&toml::Value::String("user_value".to_string()))
        );
        assert_eq!(ctx.get_user_variable("NONEXISTENT"), None);
    }

    #[test]
    fn test_get_context_variable_priority() {
        let temp_dir = create_temp_dir();
        let mut config = create_test_config(&temp_dir);
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(uservars_path, r#"PRIORITY_VAR = "user_value""#)
            .expect("Failed to write .uservariables.toml");
        config
            .prompts
            .insert("PRIORITY_VAR".to_string(), "Enter PRIORITY_VAR".to_string());

        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");
        ctx.get_prompted_variables(&config, &None)
            .expect("Failed to resolve prompted variables");
        ctx.variables.insert(
            "PRIORITY_VAR".to_string(),
            toml::Value::String("config_value".to_string()),
        );

        // User variable should have priority
        assert_eq!(
            ctx.get_context_variable("PRIORITY_VAR"),
            Some(&toml::Value::String("user_value".to_string()))
        );
    }

    #[test]
    fn test_get_context_variable_fallback_to_config() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.variables.insert(
            "CONFIG_ONLY".to_string(),
            toml::Value::String("config_value".to_string()),
        );

        // Should fallback to config variable
        assert_eq!(
            ctx.get_context_variable("CONFIG_ONLY"),
            Some(&toml::Value::String("config_value".to_string()))
        );
    }

    #[test]
    fn test_get_variables() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.variables
            .insert("TEST".to_string(), toml::Value::String("value".to_string()));

        let vars = ctx.get_variables();
        assert!(vars.contains_key("TEST"));
        assert!(vars.contains_key("HOME")); // Env var
    }

    #[test]
    fn test_get_user_variables() {
        let temp_dir = create_temp_dir();
        let mut config = create_test_config(&temp_dir);
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(uservars_path, r#"USER_VAR = "value""#)
            .expect("Failed to write .uservariables.toml");
        config
            .prompts
            .insert("USER_VAR".to_string(), "Enter USER_VAR".to_string());

        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");
        ctx.get_prompted_variables(&config, &None)
            .expect("Failed to resolve prompted variables");
        let user_vars = ctx.get_user_variables();

        assert_eq!(user_vars.len(), 1);
        assert!(user_vars.contains_key("USER_VAR"));
    }

    #[test]
    fn test_get_context_variables_merges_correctly() {
        let temp_dir = create_temp_dir();
        let mut config = create_test_config(&temp_dir);
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(
            uservars_path,
            r#"
USER_VAR = "user_value"
OVERRIDE_VAR = "user_override"
"#,
        )
        .expect("Failed to write .uservariables.toml");
        config
            .prompts
            .insert("USER_VAR".to_string(), "Enter USER_VAR".to_string());
        config
            .prompts
            .insert("OVERRIDE_VAR".to_string(), "Enter OVERRIDE_VAR".to_string());

        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");
        ctx.get_prompted_variables(&config, &None)
            .expect("Failed to resolve prompted variables");
        ctx.variables.insert(
            "CONFIG_VAR".to_string(),
            toml::Value::String("config_value".to_string()),
        );
        ctx.variables.insert(
            "OVERRIDE_VAR".to_string(),
            toml::Value::String("config_value".to_string()),
        );

        let merged = ctx.get_context_variables();

        // Should have both config and user variables
        assert!(merged.contains_key("CONFIG_VAR"));
        assert!(merged.contains_key("USER_VAR"));

        // User variable should override config variable
        assert_eq!(
            merged.get("OVERRIDE_VAR"),
            Some(&toml::Value::String("user_override".to_string()))
        );
    }

    #[test]
    fn test_get_platform_variable() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.profile.platform_variables.insert(
            "PLATFORM_VAR".to_string(),
            toml::Value::String("platform_value".to_string()),
        );

        assert_eq!(
            ctx.get_platform_variable("PLATFORM_VAR"),
            Some(&toml::Value::String("platform_value".to_string()))
        );
        assert_eq!(ctx.get_platform_variable("NONEXISTENT"), None);
    }

    #[test]
    fn test_get_context_variable_falls_back_to_platform_variable() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.profile.platform_variables.insert(
            "PLATFORM_ONLY".to_string(),
            toml::Value::String("platform_value".to_string()),
        );

        assert_eq!(
            ctx.get_context_variable("PLATFORM_ONLY"),
            Some(&toml::Value::String("platform_value".to_string()))
        );
    }

    /// A profile's own `variables` are more specific than the variables it
    /// inherits from its shared `platform`, so they must win when both set
    /// the same key.
    #[test]
    fn test_get_context_variable_profile_variable_overrides_platform_variable() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.profile.platform_variables.insert(
            "SHARED".to_string(),
            toml::Value::String("platform_value".to_string()),
        );
        ctx.profile.variables.insert(
            "SHARED".to_string(),
            toml::Value::String("profile_value".to_string()),
        );

        assert_eq!(
            ctx.get_context_variable("SHARED"),
            Some(&toml::Value::String("profile_value".to_string()))
        );
    }

    /// Platform variables are a stand-in for config-level variables (the
    /// platform-specific subset of them), so a config-level variable of the
    /// same name must not shadow the platform's - the platform is more
    /// specific.
    #[test]
    fn test_get_context_variable_platform_variable_overrides_config_variable() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.variables.insert(
            "SHARED".to_string(),
            toml::Value::String("config_value".to_string()),
        );
        ctx.profile.platform_variables.insert(
            "SHARED".to_string(),
            toml::Value::String("platform_value".to_string()),
        );

        assert_eq!(
            ctx.get_context_variable("SHARED"),
            Some(&toml::Value::String("platform_value".to_string()))
        );
    }

    #[test]
    fn test_get_context_variables_includes_platform_variables() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.profile.platform_variables.insert(
            "PLATFORM_VAR".to_string(),
            toml::Value::String("platform_value".to_string()),
        );

        let merged = ctx.get_context_variables();

        assert_eq!(
            merged.get("PLATFORM_VAR"),
            Some(&toml::Value::String("platform_value".to_string()))
        );
    }

    #[test]
    fn test_extend_variables() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        let mut new_vars = Table::new();
        new_vars.insert(
            "NEW_VAR".to_string(),
            toml::Value::String("new_value".to_string()),
        );

        ctx.extend_variables(new_vars);

        assert_eq!(
            ctx.get_variable("NEW_VAR"),
            Some(&toml::Value::String("new_value".to_string()))
        );
    }

    #[test]
    fn test_extend_variables_overwrites() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.variables.insert(
            "EXISTING".to_string(),
            toml::Value::String("old_value".to_string()),
        );

        let mut new_vars = Table::new();
        new_vars.insert(
            "EXISTING".to_string(),
            toml::Value::String("new_value".to_string()),
        );

        ctx.extend_variables(new_vars);

        assert_eq!(
            ctx.get_variable("EXISTING"),
            Some(&toml::Value::String("new_value".to_string()))
        );
    }

    #[test]
    fn test_context_with_complex_user_variables() {
        let temp_dir = create_temp_dir();
        let mut config = create_test_config(&temp_dir);
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(
            uservars_path,
            r#"
string_var = "string"
int_var = 42
float_var = 9.14
bool_var = true

[nested]
key1 = "value1"
key2 = "value2"

[[array]]
name = "item1"
value = 1

[[array]]
name = "item2"
value = 2
"#,
        )
        .expect("Failed to write .uservariables.toml");
        for key in [
            "string_var",
            "int_var",
            "float_var",
            "bool_var",
            "nested",
            "array",
        ] {
            config
                .prompts
                .insert(key.to_string(), format!("Enter {key}"));
        }

        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");
        ctx.get_prompted_variables(&config, &None)
            .expect("Failed to resolve prompted variables");
        let user_vars = ctx.get_user_variables();

        assert_eq!(
            user_vars.get("string_var"),
            Some(&toml::Value::String("string".to_string()))
        );
        assert_eq!(user_vars.get("int_var"), Some(&toml::Value::Integer(42)));
        assert_eq!(user_vars.get("float_var"), Some(&toml::Value::Float(9.14)));
        assert_eq!(user_vars.get("bool_var"), Some(&toml::Value::Boolean(true)));
        assert!(user_vars.contains_key("nested"));
        assert!(user_vars.contains_key("array"));
    }

    #[test]
    fn test_context_working_dir() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        assert_eq!(ctx.working_dir, temp_dir);
    }

    #[test]
    fn test_context_debug_format() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        // Should have Debug implementation
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("Context"));
    }

    #[test]
    fn test_multiple_contexts_independent() {
        let temp_dir1 = create_temp_dir();
        let temp_dir2 = create_temp_dir();

        let mut config1 = create_test_config(&temp_dir1);
        let mut config2 = create_test_config(&temp_dir2);
        config1
            .prompts
            .insert("VAR".to_string(), "Enter VAR".to_string());
        config2
            .prompts
            .insert("VAR".to_string(), "Enter VAR".to_string());

        fs::write(temp_dir1.join(".uservariables.toml"), r#"VAR = "dir1""#)
            .expect("Failed to write");

        fs::write(temp_dir2.join(".uservariables.toml"), r#"VAR = "dir2""#)
            .expect("Failed to write");

        let (mut ctx1, _) =
            Context::new(&temp_dir1, &config1, &None, false).expect("Failed to create context");
        let (mut ctx2, _) =
            Context::new(&temp_dir2, &config2, &None, false).expect("Failed to create context");
        ctx1.get_prompted_variables(&config1, &None)
            .expect("Failed to resolve prompted variables");
        ctx2.get_prompted_variables(&config2, &None)
            .expect("Failed to resolve prompted variables");

        assert_eq!(
            ctx1.get_user_variable("VAR"),
            Some(&toml::Value::String("dir1".to_string()))
        );
        assert_eq!(
            ctx2.get_user_variable("VAR"),
            Some(&toml::Value::String("dir2".to_string()))
        );
    }

    #[test]
    fn test_user_variables_override_in_merged_context() {
        let temp_dir = create_temp_dir();
        let mut config = create_test_config(&temp_dir);
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(
            uservars_path,
            r#"
VAR1 = "user_value1"
VAR2 = "user_value2"
"#,
        )
        .expect("Failed to write .uservariables.toml");
        config
            .prompts
            .insert("VAR1".to_string(), "Enter VAR1".to_string());
        config
            .prompts
            .insert("VAR2".to_string(), "Enter VAR2".to_string());

        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");
        ctx.get_prompted_variables(&config, &None)
            .expect("Failed to resolve prompted variables");

        // Add some config variables
        ctx.variables.insert(
            "VAR1".to_string(),
            toml::Value::String("config_value1".to_string()),
        );
        ctx.variables.insert(
            "VAR3".to_string(),
            toml::Value::String("config_value3".to_string()),
        );

        let merged = ctx.get_context_variables();

        // VAR1 should be overridden by user variable
        assert_eq!(
            merged.get("VAR1"),
            Some(&toml::Value::String("user_value1".to_string()))
        );
        // VAR2 should come from user variables
        assert_eq!(
            merged.get("VAR2"),
            Some(&toml::Value::String("user_value2".to_string()))
        );
        // VAR3 should come from config variables
        assert_eq!(
            merged.get("VAR3"),
            Some(&toml::Value::String("config_value3".to_string()))
        );
    }

    #[test]
    fn test_empty_user_variables_file() {
        let temp_dir = create_temp_dir();
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(uservars_path, "").expect("Failed to write .uservariables.toml");

        let user_vars =
            Context::parse_uservariables(&temp_dir).expect("Failed to parse uservariables");
        assert!(user_vars.is_empty());
    }

    #[test]
    fn test_context_clone() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");
        let cloned = ctx.clone();

        assert_eq!(ctx.working_dir, cloned.working_dir);
        assert_eq!(ctx.variables.len(), cloned.variables.len());
        assert_eq!(ctx.user_variables.len(), cloned.user_variables.len());
    }

    #[test]
    fn test_print_variable_float() {
        // Test float value printing (covers line 99-100)
        let value = toml::Value::Float(2.5);
        print_variable("float_var", &value, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variable_boolean() {
        // Test boolean value printing (covers line 102-103)
        let value_true = toml::Value::Boolean(true);
        let value_false = toml::Value::Boolean(false);
        print_variable("bool_var_true", &value_true, 1);
        print_variable("bool_var_false", &value_false, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variable_array_with_strings() {
        // Test array with string values (covers line 111-118)
        let arr = vec![
            toml::Value::String("item1".to_string()),
            toml::Value::String("item2".to_string()),
        ];
        let value = toml::Value::Array(arr);
        print_variable("string_array", &value, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variable_array_with_integers() {
        // Test array with integer values (covers line 119-121)
        let arr = vec![
            toml::Value::Integer(1),
            toml::Value::Integer(2),
            toml::Value::Integer(3),
        ];
        let value = toml::Value::Array(arr);
        print_variable("int_array", &value, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variable_array_with_floats() {
        // Test array with float values (covers line 122-124)
        let arr = vec![
            toml::Value::Float(1.1),
            toml::Value::Float(2.2),
            toml::Value::Float(3.3),
        ];
        let value = toml::Value::Array(arr);
        print_variable("float_array", &value, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variable_array_with_booleans() {
        // Test array with boolean values (covers line 125-127)
        let arr = vec![
            toml::Value::Boolean(true),
            toml::Value::Boolean(false),
            toml::Value::Boolean(true),
        ];
        let value = toml::Value::Array(arr);
        print_variable("bool_array", &value, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variable_array_with_nested_table() {
        // Test array with nested table (covers line 128-131)
        let mut table = toml::map::Map::new();
        table.insert("key".to_string(), toml::Value::String("value".to_string()));

        let arr = vec![toml::Value::Table(table)];
        let value = toml::Value::Array(arr);
        print_variable("table_array", &value, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variable_array_with_nested_array() {
        // Test array with nested array (covers line 128-131)
        let inner_arr = vec![
            toml::Value::String("nested1".to_string()),
            toml::Value::String("nested2".to_string()),
        ];
        let arr = vec![toml::Value::Array(inner_arr)];
        let value = toml::Value::Array(arr);
        print_variable("nested_array", &value, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variable_array_with_datetime() {
        // Test array with datetime value (covers line 132-134)
        use toml::value::Datetime;
        let datetime_str = "1979-05-27T07:32:00Z";
        let datetime = datetime_str.parse::<Datetime>().unwrap();
        let arr = vec![toml::Value::Datetime(datetime)];
        let value = toml::Value::Array(arr);
        print_variable("datetime_array", &value, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variable_datetime() {
        // Test datetime value directly (covers line 139-141)
        use toml::value::Datetime;
        let datetime_str = "1979-05-27T07:32:00Z";
        let datetime = datetime_str.parse::<Datetime>().unwrap();
        let value = toml::Value::Datetime(datetime);
        print_variable("datetime_var", &value, 1);
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variables_empty() {
        // Test print_variables with empty variables (covers line 80-81)
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");
        ctx.variables.clear(); // Clear all variables including env vars
        ctx.print_variables();
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variables_with_complex_types() {
        // Test print_variables with various types
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);
        let (mut ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        ctx.variables
            .insert("float_var".to_string(), toml::Value::Float(2.5));
        ctx.variables
            .insert("bool_var".to_string(), toml::Value::Boolean(true));

        let arr = vec![
            toml::Value::Integer(1),
            toml::Value::Float(2.5),
            toml::Value::Boolean(false),
        ];
        ctx.variables
            .insert("mixed_array".to_string(), toml::Value::Array(arr));

        ctx.print_variables();
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_get_prompted_variables_function() {
        // Test the get_prompted_variables helper function with mock input
        let prompt = "Enter your name";
        let input = b"John Doe\n";
        let mut output = Vec::new();

        let result = super::super::get_prompted_variables(prompt, &input[..], &mut output).unwrap();

        assert_eq!(result, "John Doe\n");
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Enter your name"));
        assert!(output_str.contains(">>>"));
    }

    #[test]
    fn test_get_prompted_variables_function_empty_input() {
        let prompt = "Enter value";
        let input = b"\n";
        let mut output = Vec::new();

        let result = super::super::get_prompted_variables(prompt, &input[..], &mut output).unwrap();

        assert_eq!(result, "\n");
    }

    #[test]
    fn test_get_prompted_variables_function_multiline_prompt() {
        let prompt = "Enter your API key\n(Found in settings)";
        let input = b"secret-key-123\n";
        let mut output = Vec::new();

        let result = super::super::get_prompted_variables(prompt, &input[..], &mut output).unwrap();

        assert_eq!(result, "secret-key-123\n");
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Enter your API key"));
        assert!(output_str.contains("(Found in settings)"));
    }

    #[test]
    fn test_get_prompted_variables_function_special_chars() {
        let prompt = "Enter password (min 8 chars)";
        let input = b"P@ssw0rd!\n";
        let mut output = Vec::new();

        let result = super::super::get_prompted_variables(prompt, &input[..], &mut output).unwrap();

        assert_eq!(result, "P@ssw0rd!\n");
    }

    #[test]
    fn test_get_prompted_variables_with_io_basic() {
        use crate::config::Config;
        use std::fs;

        let temp_dir = create_temp_dir();

        // Create config with prompts
        let config_path = temp_dir.join("config.toml");
        fs::write(
            &config_path,
            r#"
[prompts]
USER_NAME = "Enter your name"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        // Mock input
        let input = b"John Doe\n";
        let mut output = Vec::new();

        let result = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .unwrap();

        // Verify prompted variables were captured
        assert_eq!(
            result.get("USER_NAME"),
            Some(&toml::Value::String("John Doe\n".to_string()))
        );

        // Verify .uservariables.toml was created
        let uservars_path = temp_dir.join(".uservariables.toml");
        assert!(uservars_path.exists());

        // Verify content was saved
        let saved_content = fs::read_to_string(&uservars_path).unwrap();
        assert!(saved_content.contains("USER_NAME"));
    }

    #[test]
    fn test_get_prompted_variables_with_io_skips_existing() {
        use crate::config::Config;
        use std::fs;

        let temp_dir = create_temp_dir();

        // Pre-populate user variables
        let uservars_path = temp_dir.join(".uservariables.toml");
        fs::write(&uservars_path, r#"USER_EMAIL = "existing@example.com""#).unwrap();

        // Create config with prompts
        let config_path = temp_dir.join("config.toml");
        fs::write(
            &config_path,
            r#"
[prompts]
USER_EMAIL = "Enter your email"
USER_NAME = "Enter your name"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        // Mock input - only provide input for USER_NAME
        let input = b"John Doe\n";
        let mut output = Vec::new();

        let result = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .unwrap();

        // Verify existing variable was preserved
        assert_eq!(
            result.get("USER_EMAIL"),
            Some(&toml::Value::String("existing@example.com".to_string()))
        );

        // Verify new variable was prompted
        assert_eq!(
            result.get("USER_NAME"),
            Some(&toml::Value::String("John Doe\n".to_string()))
        );
    }

    #[test]
    fn test_get_prompted_variables_with_io_no_prompts() {
        use crate::config::Config;
        use std::fs;

        let temp_dir = create_temp_dir();

        // Create config without prompts
        let config_path = temp_dir.join("config.toml");
        fs::write(&config_path, "").unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let input = b"";
        let mut output = Vec::new();

        let result = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .unwrap();

        // Should return empty user variables
        assert!(result.is_empty() || !result.contains_key("NEW_VAR"));
    }

    #[test]
    fn test_get_prompted_variables_with_io_profile_prompts() {
        use crate::config::Config;
        use crate::profile::Profile;
        use std::collections::HashMap;
        use std::fs;

        let temp_dir = create_temp_dir();

        // Create config
        let config_path = temp_dir.join("config.toml");
        fs::write(&config_path, r#""#).unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        // Set profile with prompts
        let mut profile_prompts = HashMap::new();
        profile_prompts.insert("WORK_EMAIL".to_string(), "Enter work email".to_string());

        let profile = Profile {
            name: "work".to_string(),
            prompts: profile_prompts,
            ..Default::default()
        };

        ctx.set_profile(profile);

        // Mock input
        let input = b"work@example.com\n";
        let mut output = Vec::new();

        let result = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .unwrap();

        // Verify profile prompt was processed
        assert_eq!(
            result.get("WORK_EMAIL"),
            Some(&toml::Value::String("work@example.com\n".to_string()))
        );
    }

    #[test]
    fn test_get_prompted_variables_with_io_saves_to_file() {
        use crate::config::Config;
        use std::fs;

        let temp_dir = create_temp_dir();

        // Create config with prompts
        let config_path = temp_dir.join("config.toml");
        fs::write(
            &config_path,
            r#"
[prompts]
API_KEY = "Enter your API key"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        // Mock input
        let input = b"secret-key-123\n";
        let mut output = Vec::new();

        ctx.get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .unwrap();

        // Verify file was written
        let uservars_path = temp_dir.join(".uservariables.toml");
        assert!(uservars_path.exists());

        // Verify context was updated
        assert_eq!(
            ctx.get_user_variable("API_KEY"),
            Some(&toml::Value::String("secret-key-123\n".to_string()))
        );
    }

    #[test]
    fn test_get_prompted_variables_with_io_error_handling() {
        use crate::config::Config;
        use std::fs;

        let temp_dir = create_temp_dir();

        // Create config with prompts
        let config_path = temp_dir.join("config.toml");
        fs::write(
            &config_path,
            r#"
[prompts]
VAR1 = "Enter value 1"
VAR2 = "Enter value 2"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        // Provide insufficient input (only one value instead of two)
        let input = b"value1\n";
        let mut output = Vec::new();

        // Should not panic, should handle the error gracefully
        let result =
            ctx.get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output);

        // Should succeed even with error (error is printed to stderr)
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_prompted_variables_with_io_no_dirty_no_save() {
        use crate::config::Config;
        use std::fs;

        let temp_dir = create_temp_dir();

        // Pre-populate all variables
        let uservars_path = temp_dir.join(".uservariables.toml");
        fs::write(&uservars_path, r#"USER_EMAIL = "existing@example.com""#).unwrap();

        // Create config with same prompt
        let config_path = temp_dir.join("config.toml");
        fs::write(
            &config_path,
            r#"
[prompts]
USER_EMAIL = "Enter your email"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        // Get last modified time before
        let metadata_before = fs::metadata(&uservars_path).unwrap();
        let modified_before = metadata_before.modified().unwrap();

        // Wait a tiny bit to ensure timestamps would differ
        std::thread::sleep(std::time::Duration::from_millis(10));

        let input = b"";
        let mut output = Vec::new();

        let result = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .unwrap();

        // Verify existing variable was preserved
        assert_eq!(
            result.get("USER_EMAIL"),
            Some(&toml::Value::String("existing@example.com".to_string()))
        );

        // Verify file was NOT modified (no dirty flag)
        let metadata_after = fs::metadata(&uservars_path).unwrap();
        let modified_after = metadata_after.modified().unwrap();

        assert_eq!(
            modified_before, modified_after,
            "File should not be modified when no new prompts are answered"
        );
    }

    #[test]
    fn test_get_prompted_variables_with_io_file_backend_untrimmed() {
        let temp_dir = create_temp_dir();
        fs::write(
            temp_dir.join(".uservariables.toml"),
            r#"CACHED = "cached-value""#,
        )
        .unwrap();
        fs::write(
            temp_dir.join("config.toml"),
            r#"
[prompts]
CACHED = "Enter cached"
NEW = "Enter new"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let input = b"typed-value\n";
        let mut output = Vec::new();
        let resolved = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .unwrap();

        assert_eq!(
            resolved.get("CACHED"),
            Some(&toml::Value::String("cached-value".to_string()))
        );
        assert_eq!(
            resolved.get("NEW"),
            Some(&toml::Value::String("typed-value\n".to_string())),
            "the file backend keeps its historical untrimmed behavior"
        );
    }

    #[test]
    fn test_prompt_backend_config_default_applies_when_unset() {
        let temp_dir = create_temp_dir();
        fs::write(
            temp_dir.join("config.toml"),
            r#"
prompt_backend = "file"

[prompts]
VAR = "Enter a value"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        assert_eq!(
            config.prompt_backend,
            Some(crate::prompt_store::PromptBackendType::File)
        );
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let input = b"a-value\n";
        let mut output = Vec::new();
        let resolved = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .unwrap();

        assert_eq!(
            resolved.get("VAR"),
            Some(&toml::Value::String("a-value\n".to_string()))
        );
        let saved = fs::read_to_string(temp_dir.join(".uservariables.toml")).unwrap();
        assert!(
            saved.contains("VAR") && saved.contains("a-value"),
            "explicit top-level prompt_backend = \"file\" should still save to the file"
        );
    }

    #[test]
    fn test_profile_prompt_backend_overrides_config_default() {
        // Profile-level "file" should win over the repo-wide "keychain"
        // default for prompts in that profile, again without touching a
        // real keychain.
        let temp_dir = create_temp_dir();
        fs::write(
            temp_dir.join("config.toml"),
            r#"
prompt_backend = "keychain"

[prompts]
VAR = "Enter a value"

[profiles.default]
dependencies = []
prompt_backend = "file"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        assert_eq!(
            config.profiles.get("default").unwrap().prompt_backend,
            Some(crate::prompt_store::PromptBackendType::File)
        );
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let input = b"a-value\n";
        let mut output = Vec::new();
        let resolved = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .unwrap();

        assert_eq!(
            resolved.get("VAR"),
            Some(&toml::Value::String("a-value\n".to_string()))
        );
        let saved = fs::read_to_string(temp_dir.join(".uservariables.toml")).unwrap();
        assert!(
            saved.contains("VAR") && saved.contains("a-value"),
            "profile-level prompt_backend = \"file\" must win over the config-level \
\"keychain\" default"
        );
    }

    #[test]
    #[ignore = "touches the real OS keychain; run manually with --ignored"]
    fn test_get_prompted_variables_with_io_keychain_round_trip() {
        let temp_dir = create_temp_dir();
        fs::write(
            temp_dir.join("config.toml"),
            r#"
prompt_backend = "keychain"

[prompts]
KEYCHAIN_SECRET = "Enter a secret"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let input = b"super-secret\n";
        let mut output = Vec::new();
        let resolved = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .expect("a real OS keychain must be available to run this test");
        assert_eq!(
            resolved.get("KEYCHAIN_SECRET"),
            Some(&toml::Value::String("super-secret".to_string()))
        );
        let uservars_path = temp_dir.join(".uservariables.toml");
        assert!(
            !uservars_path.exists()
                || !fs::read_to_string(&uservars_path)
                    .unwrap()
                    .contains("KEYCHAIN_SECRET")
        );

        // Should read the cached entry back, not prompt again.
        let input2 = b"";
        let mut output2 = Vec::new();
        let resolved2 = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input2[..], &mut output2)
            .unwrap();
        assert_eq!(
            resolved2.get("KEYCHAIN_SECRET"),
            Some(&toml::Value::String("super-secret".to_string()))
        );

        let service = format!("DOTR:{}", temp_dir.display());
        let entry = keyring::Entry::new(&service, "DOTR_KEYCHAIN_SECRET").unwrap();
        entry.delete_credential().ok();
    }

    #[test]
    #[ignore = "needs the `bw` CLI installed, unlocked, and a pre-created 'dotr-secrets' \
Secure Note; run manually with --ignored"]
    fn test_get_prompted_variables_with_io_bitwarden_round_trip() {
        let temp_dir = create_temp_dir();
        fs::write(
            temp_dir.join("config.toml"),
            r#"
prompt_backend = "bitwarden"

[prompts]
BW_SECRET = "Enter a secret"
"#,
        )
        .unwrap();

        let config = Config::from_path(&temp_dir).unwrap();
        let (mut ctx, _) = Context::new(&temp_dir, &config, &None, false).unwrap();

        let input = b"bw-secret-value\n";
        let mut output = Vec::new();
        let resolved = ctx
            .get_prompted_variables_with_io(&config, &None, &mut &input[..], &mut output)
            .expect("bw must be installed, unlocked, with a 'dotr-secrets' note pre-created");
        assert_eq!(
            resolved.get("BW_SECRET"),
            Some(&toml::Value::String("bw-secret-value".to_string()))
        );

        let uservars_path = temp_dir.join(".uservariables.toml");
        let contents = if uservars_path.exists() {
            fs::read_to_string(&uservars_path).unwrap()
        } else {
            String::new()
        };
        assert!(!contents.contains("BW_SECRET"));
    }

    #[test]
    fn test_dotr_profile_is_folded_into_variables() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);

        fs::write(
            temp_dir.join(".uservariables.toml"),
            r#"DOTR_PROFILE = "default""#,
        )
        .expect("Failed to write .uservariables.toml");

        let (ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        // The resolved profile is always folded into `variables` as a
        // fallback (see below), independent of whatever's in the file -
        // but since the whole file also loads into `user_variables`
        // unconditionally, an explicit `DOTR_PROFILE` entry shows up
        // there too (it's a real, ordinary key in the file, from
        // `user_variables`'s perspective).
        assert_eq!(
            ctx.get_variable("DOTR_PROFILE"),
            Some(&toml::Value::String("default".to_string()))
        );
        assert_eq!(
            ctx.get_user_variable("DOTR_PROFILE"),
            Some(&toml::Value::String("default".to_string()))
        );
    }

    #[test]
    fn test_dotr_bitwarden_note_is_folded_into_variables() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);

        fs::write(
            temp_dir.join(".uservariables.toml"),
            r#"DOTR_BITWARDEN_NOTE = "custom-note""#,
        )
        .expect("Failed to write .uservariables.toml");

        let (ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        assert_eq!(
            ctx.get_variable("DOTR_BITWARDEN_NOTE"),
            Some(&toml::Value::String("custom-note".to_string()))
        );
        assert_eq!(
            ctx.get_user_variable("DOTR_BITWARDEN_NOTE"),
            Some(&toml::Value::String("custom-note".to_string()))
        );
    }

    #[test]
    fn test_dotr_profile_and_bitwarden_note_resolve_with_no_override_anywhere() {
        // Regression test: neither key is set in .uservariables.toml, the
        // environment, config, or the profile - `default` and the
        // built-in Bitwarden note are used implicitly. Both must still be
        // real values in `variables`, not simply absent, or a template
        // referencing `{{ DOTR_PROFILE }}`/`{{ DOTR_BITWARDEN_NOTE }}`
        // fails to compile even though nothing is actually misconfigured.
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);

        let (ctx, _) =
            Context::new(&temp_dir, &config, &None, false).expect("Failed to create context");

        assert_eq!(
            ctx.get_variable("DOTR_PROFILE"),
            Some(&toml::Value::String("default".to_string()))
        );
        assert_eq!(
            ctx.get_variable("DOTR_BITWARDEN_NOTE"),
            Some(&toml::Value::String("dotr-secrets".to_string()))
        );
    }

    #[test]
    fn test_dotr_profile_reflects_explicit_profile_override_not_stale_file_value() {
        // Regression test: an explicit `--profile` (or equivalently a
        // non-None `profile_name`) must win over a stale DOTR_PROFILE
        // already sitting in .uservariables.toml, since that's the
        // profile actually in effect for this run.
        let temp_dir = create_temp_dir();
        fs::write(
            temp_dir.join("config.toml"),
            r#"
[profiles.work]
dependencies = []
"#,
        )
        .unwrap();
        let config = Config::from_path(&temp_dir).unwrap();

        fs::write(
            temp_dir.join(".uservariables.toml"),
            r#"DOTR_PROFILE = "default""#,
        )
        .expect("Failed to write .uservariables.toml");

        let (ctx, _) = Context::new(&temp_dir, &config, &Some("work".to_string()), false)
            .expect("Failed to create context");

        assert_eq!(
            ctx.get_variable("DOTR_PROFILE"),
            Some(&toml::Value::String("work".to_string()))
        );
    }

    #[test]
    fn test_dotr_profile_from_file_wins_over_env() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);

        fs::write(
            temp_dir.join(".uservariables.toml"),
            r#"DOTR_PROFILE = "default""#,
        )
        .expect("Failed to write .uservariables.toml");

        // SAFETY: single-threaded within this test; restored immediately
        // after constructing the context.
        unsafe {
            std::env::set_var("DOTR_PROFILE", "env-value");
        }
        let result = Context::new(&temp_dir, &config, &None, false);
        unsafe {
            std::env::remove_var("DOTR_PROFILE");
        }
        let (ctx, _) = result.expect("Failed to create context");

        assert_eq!(
            ctx.get_variable("DOTR_PROFILE"),
            Some(&toml::Value::String("default".to_string())),
            "the file should win over the environment for DOTR_PROFILE"
        );
    }

    #[test]
    fn test_dotr_bitwarden_note_from_env_wins_over_file() {
        let temp_dir = create_temp_dir();
        let config = create_test_config(&temp_dir);

        fs::write(
            temp_dir.join(".uservariables.toml"),
            r#"DOTR_BITWARDEN_NOTE = "file-note""#,
        )
        .expect("Failed to write .uservariables.toml");

        // SAFETY: single-threaded within this test; restored immediately
        // after constructing the context.
        unsafe {
            std::env::set_var("DOTR_BITWARDEN_NOTE", "env-note");
        }
        let result = Context::new(&temp_dir, &config, &None, false);
        unsafe {
            std::env::remove_var("DOTR_BITWARDEN_NOTE");
        }
        let (ctx, _) = result.expect("Failed to create context");

        assert_eq!(
            ctx.get_variable("DOTR_BITWARDEN_NOTE"),
            Some(&toml::Value::String("env-note".to_string())),
            "the environment should win over the file for DOTR_BITWARDEN_NOTE"
        );
    }

    mod resolve_bitwarden_note_tests {
        use crate::context::resolve_bitwarden_note;
        use toml::Table;

        #[test]
        fn falls_back_to_default_when_nothing_set() {
            assert_eq!(
                resolve_bitwarden_note(None, &Table::new(), None, None),
                "dotr-secrets"
            );
        }

        #[test]
        fn config_level_used_when_only_config_set() {
            assert_eq!(
                resolve_bitwarden_note(None, &Table::new(), None, Some("config-note")),
                "config-note"
            );
        }

        #[test]
        fn profile_level_overrides_config_level() {
            assert_eq!(
                resolve_bitwarden_note(
                    None,
                    &Table::new(),
                    Some("profile-note"),
                    Some("config-note")
                ),
                "profile-note"
            );
        }

        #[test]
        fn uservariables_override_wins_over_profile_and_config() {
            let mut user_variables = Table::new();
            user_variables.insert(
                "DOTR_BITWARDEN_NOTE".to_string(),
                toml::Value::String("local-note".to_string()),
            );

            assert_eq!(
                resolve_bitwarden_note(
                    None,
                    &user_variables,
                    Some("profile-note"),
                    Some("config-note")
                ),
                "local-note"
            );
        }

        #[test]
        fn env_override_wins_over_uservariables_override() {
            let mut user_variables = Table::new();
            user_variables.insert(
                "DOTR_BITWARDEN_NOTE".to_string(),
                toml::Value::String("local-note".to_string()),
            );

            assert_eq!(
                resolve_bitwarden_note(
                    Some("env-note".to_string()),
                    &user_variables,
                    Some("profile-note"),
                    Some("config-note")
                ),
                "env-note"
            );
        }

        #[test]
        fn uservariables_key_with_non_string_value_is_ignored() {
            let mut user_variables = Table::new();
            user_variables.insert("DOTR_BITWARDEN_NOTE".to_string(), toml::Value::Integer(42));

            assert_eq!(
                resolve_bitwarden_note(None, &user_variables, Some("profile-note"), None),
                "profile-note"
            );
        }
    }
}
