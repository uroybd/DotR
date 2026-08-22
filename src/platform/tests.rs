#[cfg(test)]
mod platform_tests {
    use crate::platform::Platform;
    use toml::Table;

    #[test]
    fn new_sets_name_and_empty_variables() {
        let platform = Platform::new("macos");

        assert_eq!(platform.name, "macos");
        assert!(platform.variables.is_empty());
    }

    #[test]
    fn from_table_parses_variables() {
        let mut table = Table::new();
        let mut variables = Table::new();
        variables.insert("EDITOR".to_string(), toml::Value::String("vim".to_string()));
        table.insert("variables".to_string(), toml::Value::Table(variables));

        let platform = Platform::from_table("macos", &table).unwrap();

        assert_eq!(platform.name, "macos");
        assert_eq!(
            platform.variables.get("EDITOR"),
            Some(&toml::Value::String("vim".to_string()))
        );
    }

    #[test]
    fn from_table_defaults_to_empty_variables_when_missing() {
        let table = Table::new();

        let platform = Platform::from_table("linux", &table).unwrap();

        assert_eq!(platform.name, "linux");
        assert!(platform.variables.is_empty());
    }

    #[test]
    fn from_table_errors_when_variables_is_not_a_table() {
        let mut table = Table::new();
        table.insert(
            "variables".to_string(),
            toml::Value::String("not-a-table".to_string()),
        );

        let result = Platform::from_table("macos", &table);

        assert!(result.is_err());
    }

    #[test]
    fn get_platforms_from_table_returns_empty_map_when_absent() {
        let platforms = crate::platform::get_platforms_from_table(None).unwrap();

        assert!(platforms.is_empty());
    }

    #[test]
    fn get_platforms_from_table_parses_multiple_platforms() {
        let toml_str = r#"
[macos]
variables = { EDITOR = "vim" }

[linux]
variables = { EDITOR = "nano" }
"#;
        let table: Table = toml_str.parse().unwrap();
        let value = toml::Value::Table(table);

        let platforms = crate::platform::get_platforms_from_table(Some(&value)).unwrap();

        assert_eq!(platforms.len(), 2);
        assert_eq!(
            platforms.get("macos").unwrap().variables.get("EDITOR"),
            Some(&toml::Value::String("vim".to_string()))
        );
        assert_eq!(
            platforms.get("linux").unwrap().variables.get("EDITOR"),
            Some(&toml::Value::String("nano".to_string()))
        );
    }

    #[test]
    fn get_platforms_from_table_errors_when_entry_is_not_a_table() {
        let toml_str = r#"macos = "oops""#;
        let table: Table = toml_str.parse().unwrap();
        let value = toml::Value::Table(table);

        let result = crate::platform::get_platforms_from_table(Some(&value));

        assert!(result.is_err());
    }
}
