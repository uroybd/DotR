#[cfg(test)]
mod prompt_backend_type_tests {
    use crate::prompt_store::PromptBackendType;

    #[test]
    fn test_parse_round_trips_with_as_str() {
        for backend in [
            PromptBackendType::File,
            PromptBackendType::Keychain,
            PromptBackendType::Bitwarden,
        ] {
            assert_eq!(
                PromptBackendType::parse(backend.as_str()).unwrap(),
                backend
            );
        }
    }

    #[test]
    fn test_parse_rejects_unknown_backend() {
        let err = PromptBackendType::parse("not-a-real-backend").unwrap_err();
        assert!(err.to_string().contains("not-a-real-backend"));
    }

    #[test]
    fn test_default_is_file() {
        assert_eq!(PromptBackendType::default(), PromptBackendType::File);
    }

    #[test]
    fn test_serializes_lowercase() {
        // TOML documents must be tables at the root, so serialize each as
        // a HashMap value (a real `[prompts]`-shaped table), not standalone.
        for (backend, expected) in [
            (PromptBackendType::File, "file"),
            (PromptBackendType::Keychain, "keychain"),
            (PromptBackendType::Bitwarden, "bitwarden"),
        ] {
            let mut wrapped = std::collections::HashMap::new();
            wrapped.insert("backend", backend);
            let toml_string = toml::to_string(&wrapped).unwrap();
            assert_eq!(toml_string.trim(), format!("backend = \"{expected}\""));
        }
    }
}

#[cfg(test)]
mod file_store_tests {
    use std::{env, fs};

    use toml::Table;

    use crate::prompt_store::{FileStore, PromptStoreBackend};

    fn temp_dir() -> std::path::PathBuf {
        let dir = env::temp_dir().join(format!("dotr_prompt_store_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_get_returns_empty_table_when_file_missing() {
        let dir = temp_dir();
        let mut store = FileStore::new();
        let records = store.get(&dir, &["MISSING".to_string()]).unwrap();
        assert!(records.is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_get_ignores_the_keys_filter() {
        // Unlike Keychain/Bitwarden, the file already holds everything, so
        // `get` returns the full file regardless of what's asked for.
        let dir = temp_dir();
        fs::write(dir.join(".uservariables.toml"), r#"EMAIL = "a@b.com""#).unwrap();
        let mut store = FileStore::new();

        let records = store.get(&dir, &["SOMETHING_ELSE".to_string()]).unwrap();

        assert_eq!(
            records.get("EMAIL"),
            Some(&toml::Value::String("a@b.com".to_string()))
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_save_persists_to_uservariables_toml() {
        let dir = temp_dir();
        let mut store = FileStore::new();
        let mut records = Table::new();
        records.insert(
            "TOKEN".to_string(),
            toml::Value::String("secret-value".to_string()),
        );

        store.save(&dir, &records).unwrap();

        let content = fs::read_to_string(dir.join(".uservariables.toml")).unwrap();
        assert!(content.contains("TOKEN = \"secret-value\""));
        assert_eq!(
            store.get(&dir, &["TOKEN".to_string()]).unwrap(),
            records
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_save_overwrites_with_exactly_the_given_records() {
        // `save` is handed the caller's already-merged table (see
        // `Context::get_prompted_variables_with_io`), so it writes that
        // table verbatim rather than merging against whatever the file
        // already had.
        let dir = temp_dir();
        let mut store = FileStore::new();
        let mut first = Table::new();
        first.insert("A".to_string(), toml::Value::String("1".to_string()));
        store.save(&dir, &first).unwrap();

        let mut second = Table::new();
        second.insert("B".to_string(), toml::Value::String("2".to_string()));
        store.save(&dir, &second).unwrap();

        let on_disk = store.get(&dir, &[]).unwrap();
        assert_eq!(on_disk.get("A"), None);
        assert_eq!(
            on_disk.get("B"),
            Some(&toml::Value::String("2".to_string()))
        );
        fs::remove_dir_all(&dir).ok();
    }
}
