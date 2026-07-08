#[cfg(test)]
mod prompt_backend_tests {
    use crate::prompt_store::PromptBackend;

    #[test]
    fn test_parse_round_trips_with_as_str() {
        for backend in [
            PromptBackend::File,
            PromptBackend::Keychain,
            PromptBackend::Bitwarden,
        ] {
            assert_eq!(PromptBackend::parse(backend.as_str()).unwrap(), backend);
        }
    }

    #[test]
    fn test_parse_rejects_unknown_backend() {
        let err = PromptBackend::parse("not-a-real-backend").unwrap_err();
        assert!(err.to_string().contains("not-a-real-backend"));
    }

    #[test]
    fn test_default_is_file() {
        assert_eq!(PromptBackend::default(), PromptBackend::File);
    }

    #[test]
    fn test_serializes_lowercase() {
        // TOML documents must be tables at the root, so serialize each as
        // a HashMap value (a real `[prompts]`-shaped table), not standalone.
        for (backend, expected) in [
            (PromptBackend::File, "file"),
            (PromptBackend::Keychain, "keychain"),
            (PromptBackend::Bitwarden, "bitwarden"),
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

    use crate::prompt_store::{FileStore, PromptStore};

    fn temp_dir() -> std::path::PathBuf {
        let dir = env::temp_dir().join(format!("dotr_prompt_store_test_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_get_returns_none_for_missing_key() {
        let dir = temp_dir();
        let store = FileStore::new(dir.clone(), toml::Table::new());
        assert_eq!(store.get("MISSING").unwrap(), None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_get_returns_cached_value_without_touching_disk() {
        let dir = temp_dir();
        let mut cache = toml::Table::new();
        cache.insert(
            "EMAIL".to_string(),
            toml::Value::String("a@b.com".to_string()),
        );
        let store = FileStore::new(dir.clone(), cache);

        assert_eq!(store.get("EMAIL").unwrap(), Some("a@b.com".to_string()));
        assert!(
            !dir.join(".uservariables.toml").exists(),
            "a plain get() must not write .uservariables.toml"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_set_persists_to_uservariables_toml() {
        let dir = temp_dir();
        let store = FileStore::new(dir.clone(), toml::Table::new());
        store.set("TOKEN", "secret-value").unwrap();

        let content = fs::read_to_string(dir.join(".uservariables.toml")).unwrap();
        assert!(content.contains("TOKEN = \"secret-value\""));
        assert_eq!(
            store.get("TOKEN").unwrap(),
            Some("secret-value".to_string())
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_into_table_reflects_new_values() {
        let dir = temp_dir();
        let store = FileStore::new(dir.clone(), toml::Table::new());
        store.set("A", "1").unwrap();
        store.set("B", "2").unwrap();
        let table = store.into_table();
        assert_eq!(table.get("A").unwrap().as_str(), Some("1"));
        assert_eq!(table.get("B").unwrap().as_str(), Some("2"));
        fs::remove_dir_all(&dir).ok();
    }
}
