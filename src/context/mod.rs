use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;
use toml::Table;

use crate::profile::Profile;

#[derive(Debug, Clone, Serialize)]
pub struct Context {
    pub working_dir: PathBuf,
    variables: Table,
    user_variables: Table,
    pub profile: Option<Profile>,
}

impl Context {
    pub fn get_variable(&self, key: &str) -> Option<&toml::Value> {
        self.variables.get(key)
    }

    pub fn get_user_variable(&self, key: &str) -> Option<&toml::Value> {
        self.user_variables.get(key)
    }

    pub fn get_profile_variable(&self, key: &str) -> Option<&toml::Value> {
        if let Some(profile) = &self.profile {
            profile.variables.get(key)
        } else {
            None
        }
    }

    pub fn get_context_variable(&self, key: &str) -> Option<&toml::Value> {
        self.get_user_variable(key).or_else(|| {
            self.get_profile_variable(key)
                .or_else(|| self.get_variable(key))
        })
    }

    pub fn set_profile(&mut self, profile: Option<Profile>) {
        self.profile = profile;
    }

    pub fn parse_uservariables(cwd: &Path) -> Result<Table, anyhow::Error> {
        let path = cwd.join(".uservariables.toml");
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let table: Table = toml::de::from_str(&content).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to parse .uservariables.toml at '{}': {}",
                    path.display(),
                    e
                )
            })?;
            Ok(table)
        } else {
            Ok(Table::new())
        }
    }
    
    pub fn new(working_dir: &Path) -> Result<Self, anyhow::Error> {
        let mut variables = Table::new();
        for (key, value) in std::env::vars() {
            variables.insert(key, toml::Value::String(value));
        }
        let user_variables = Self::parse_uservariables(working_dir)?;

        Ok(Self {
            working_dir: working_dir.to_path_buf(),
            variables,
            user_variables,
            profile: None,
        })
    }

    pub fn get_variables(&self) -> &Table {
        &self.variables
    }

    pub fn get_user_variables(&self) -> &Table {
        &self.user_variables
    }

    pub fn get_context_variables(&self) -> Table {
        let mut context_vars = self.variables.clone();
        if let Some(profile) = &self.profile {
            context_vars.extend(profile.variables.clone());
        }
        context_vars.extend(self.user_variables.clone());
        context_vars
    }

    pub fn extend_variables(&mut self, new_vars: Table) {
        self.variables.extend(new_vars);
    }

    pub fn print_variables(&self) {
        let variables = &self.get_context_variables();
        println!("User Variables:");
        if variables.is_empty() {
            println!("  (none)");
        } else {
            for (key, value) in variables.iter() {
                print_variable(key, value, 1);
            }
        }
    }
}

pub fn print_variable(key: &str, value: &toml::Value, level: usize) {
    let indent = "  ".repeat(level);
    match value {
        toml::Value::String(s) => {
            println!("{}{} = {}", indent, key, s);
        }
        toml::Value::Integer(i) => {
            println!("{}{} = {}", indent, key, i);
        }
        toml::Value::Float(f) => {
            println!("{}{} = {}", indent, key, f);
        }
        toml::Value::Boolean(b) => {
            println!("{}{} = {}", indent, key, b);
        }
        toml::Value::Table(t) => {
            println!("{}{} =", indent, key);
            for (k, v) in t.iter() {
                print_variable(k, v, level + 1);
            }
        }
        toml::Value::Array(arr) => {
            println!("{}{} = [", indent, key);
            for v in arr.iter() {
                let item_indent = "  ".repeat(level + 1);
                match v {
                    toml::Value::String(s) => {
                        println!("{}- {}", item_indent, s);
                    }
                    toml::Value::Integer(i) => {
                        println!("{}- {}", item_indent, i);
                    }
                    toml::Value::Float(f) => {
                        println!("{}- {}", item_indent, f);
                    }
                    toml::Value::Boolean(b) => {
                        println!("{}- {}", item_indent, b);
                    }
                    toml::Value::Table(_) | toml::Value::Array(_) => {
                        println!("{}-", item_indent);
                        print_variable("", v, level + 2);
                    }
                    _ => {
                        println!("{}- {:?}", item_indent, v);
                    }
                }
            }
            println!("{}]", indent);
        }
        _ => {
            println!("{}{} = {:?}", indent, key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn create_temp_dir() -> PathBuf {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir =
            std::env::temp_dir().join(format!("dotr_test_{}_{}", std::process::id(), counter));
        fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
        temp_dir
    }

    #[test]
    fn test_context_new() {
        let temp_dir = create_temp_dir();
        let ctx = Context::new(&temp_dir).expect("Failed to create context");

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
        let ctx = Context::new(&temp_dir).expect("Failed to create context");

        // HOME should always be in environment
        assert!(
            ctx.get_variable("HOME").is_some(),
            "Should have HOME env var"
        );
    }

    #[test]
    fn test_parse_uservariables_no_file() {
        let temp_dir = create_temp_dir();
        let user_vars = Context::parse_uservariables(&temp_dir).expect("Failed to parse uservariables");

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

        let user_vars = Context::parse_uservariables(&temp_dir).expect("Failed to parse uservariables");

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

        let user_vars = Context::parse_uservariables(&temp_dir).expect("Failed to parse uservariables");

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
        assert!(
            result.is_err(),
            "Should return error on invalid TOML"
        );
    }

    #[test]
    fn test_get_variable() {
        let temp_dir = create_temp_dir();
        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");

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

        let ctx = Context::new(&temp_dir).expect("Failed to create context");

        assert_eq!(
            ctx.get_user_variable("USER_VAR"),
            Some(&toml::Value::String("user_value".to_string()))
        );
        assert_eq!(ctx.get_user_variable("NONEXISTENT"), None);
    }

    #[test]
    fn test_get_context_variable_priority() {
        let temp_dir = create_temp_dir();
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(uservars_path, r#"PRIORITY_VAR = "user_value""#)
            .expect("Failed to write .uservariables.toml");

        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");
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
        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");

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
        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");

        ctx.variables
            .insert("TEST".to_string(), toml::Value::String("value".to_string()));

        let vars = ctx.get_variables();
        assert!(vars.contains_key("TEST"));
        assert!(vars.contains_key("HOME")); // Env var
    }

    #[test]
    fn test_get_user_variables() {
        let temp_dir = create_temp_dir();
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(uservars_path, r#"USER_VAR = "value""#)
            .expect("Failed to write .uservariables.toml");

        let ctx = Context::new(&temp_dir).expect("Failed to create context");
        let user_vars = ctx.get_user_variables();

        assert_eq!(user_vars.len(), 1);
        assert!(user_vars.contains_key("USER_VAR"));
    }

    #[test]
    fn test_get_context_variables_merges_correctly() {
        let temp_dir = create_temp_dir();
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(
            uservars_path,
            r#"
USER_VAR = "user_value"
OVERRIDE_VAR = "user_override"
"#,
        )
        .expect("Failed to write .uservariables.toml");

        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");
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
    fn test_extend_variables() {
        let temp_dir = create_temp_dir();
        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");

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
        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");

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

        let ctx = Context::new(&temp_dir).expect("Failed to create context");
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
        let ctx = Context::new(&temp_dir).expect("Failed to create context");

        assert_eq!(ctx.working_dir, temp_dir);
    }

    #[test]
    fn test_context_debug_format() {
        let temp_dir = create_temp_dir();
        let ctx = Context::new(&temp_dir).expect("Failed to create context");

        // Should have Debug implementation
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("Context"));
    }

    #[test]
    fn test_multiple_contexts_independent() {
        let temp_dir1 = create_temp_dir();
        let temp_dir2 = create_temp_dir();

        fs::write(temp_dir1.join(".uservariables.toml"), r#"VAR = "dir1""#)
            .expect("Failed to write");

        fs::write(temp_dir2.join(".uservariables.toml"), r#"VAR = "dir2""#)
            .expect("Failed to write");

        let ctx1 = Context::new(&temp_dir1).expect("Failed to create context");
        let ctx2 = Context::new(&temp_dir2).expect("Failed to create context");

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
        let uservars_path = &temp_dir.join(".uservariables.toml");

        fs::write(
            uservars_path,
            r#"
VAR1 = "user_value1"
VAR2 = "user_value2"
"#,
        )
        .expect("Failed to write .uservariables.toml");

        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");

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

        let user_vars = Context::parse_uservariables(&temp_dir).expect("Failed to parse uservariables");
        assert!(user_vars.is_empty());
    }

    #[test]
    fn test_context_clone() {
        let temp_dir = create_temp_dir();
        let ctx = Context::new(&temp_dir).expect("Failed to create context");
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
        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");
        ctx.variables.clear(); // Clear all variables including env vars
        ctx.print_variables();
        // No assertion - just testing that it doesn't panic
    }

    #[test]
    fn test_print_variables_with_complex_types() {
        // Test print_variables with various types
        let temp_dir = create_temp_dir();
        let mut ctx = Context::new(&temp_dir).expect("Failed to create context");

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
}
