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
            assert_eq!(PromptBackendType::parse(backend.as_str()).unwrap(), backend);
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
        assert_eq!(store.get(&dir, &["TOKEN".to_string()]).unwrap(), records);
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

/// `BitwardenStore` against a stand-in `bw`: a shell script that logs each call and says the
/// vault is unlocked only when `BW_SESSION` is `valid`, as the real one does for a live session.
#[cfg(all(test, unix))]
mod bitwarden_store_tests {
    use std::{
        env, fs,
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
    };

    use toml::Table;

    use crate::prompt_store::{BitwardenStore, PromptStoreBackend};

    const FAKE_BW: &str = r#"#!/bin/sh
dir="$(dirname "$0")"
echo "$1" >> "$dir/calls"
case "$1" in
  status)
    if [ "$BW_SESSION" = valid ]; then echo '{"status":"unlocked"}'; else echo '{"status":"locked"}'; fi ;;
  sync|lock) ;;
  get)
    if [ -f "$dir/item.json" ]; then cat "$dir/item.json"; else echo "Not found." >&2; exit 1; fi ;;
  create) echo '{"id":"new-id","notes":""}' ;;
  encode) cat ;;
  edit) printf '%s' "$4" > "$dir/edited"; echo '{}' ;;
  unlock) exit 1 ;;
esac
"#;

    fn fake_bw() -> PathBuf {
        let dir = env::temp_dir().join(format!("dotr_fake_bw_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let program = dir.join("bw");
        fs::write(&program, FAKE_BW).unwrap();
        fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
        program
    }

    fn calls(program: &Path) -> Vec<String> {
        let log = program.with_file_name("calls");
        fs::read_to_string(log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// Runs `body` with `BW_SESSION` set to `session` (or unset), putting it back after.
    fn with_session<T>(session: Option<&str>, body: impl FnOnce() -> T) -> T {
        let before = env::var("BW_SESSION").ok();
        // SAFETY: the suite runs single-threaded (`just test` and CI pass `--test-threads=1`),
        // so no other thread reads the environment while it changes.
        unsafe {
            match session {
                Some(value) => env::set_var("BW_SESSION", value),
                None => env::remove_var("BW_SESSION"),
            }
        }
        let result = body();
        // SAFETY: as above; this puts the environment back.
        unsafe {
            match before {
                Some(value) => env::set_var("BW_SESSION", value),
                None => env::remove_var("BW_SESSION"),
            }
        }
        result
    }

    fn keys(names: &[&str]) -> Vec<String> {
        names.iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn test_an_exported_session_is_used_without_unlocking() {
        let program = fake_bw();
        fs::write(
            program.with_file_name("item.json"),
            r#"{"id":"note-id","notes":"TOKEN = \"from-the-vault\""}"#,
        )
        .unwrap();

        let records = with_session(Some("valid"), || {
            let mut store = BitwardenStore::with_program("dotr-secrets".into(), program.clone());
            store.get(Path::new("."), &keys(&["TOKEN"])).unwrap()
        });

        assert_eq!(
            records.get("TOKEN").and_then(|v| v.as_str()),
            Some("from-the-vault")
        );
        let calls = calls(&program);
        assert!(!calls.contains(&"unlock".to_string()), "{calls:?}");
        assert!(calls.contains(&"sync".to_string()), "{calls:?}");
        // The session was the user's, so it is left open.
        assert!(!calls.contains(&"lock".to_string()), "{calls:?}");
        fs::remove_dir_all(program.parent().unwrap()).ok();
    }

    #[test]
    fn test_a_missing_note_is_created_with_an_exported_session_without_unlocking() {
        let program = fake_bw();

        let records = with_session(Some("valid"), || {
            let mut store = BitwardenStore::with_program("dotr-secrets".into(), program.clone());
            store.get(Path::new("."), &keys(&["TOKEN"])).unwrap()
        });

        assert!(records.is_empty());
        let calls = calls(&program);
        assert!(calls.contains(&"create".to_string()), "{calls:?}");
        assert!(!calls.contains(&"unlock".to_string()), "{calls:?}");
        fs::remove_dir_all(program.parent().unwrap()).ok();
    }

    #[test]
    fn test_saving_with_an_exported_session_edits_the_note_without_unlocking() {
        let program = fake_bw();
        fs::write(
            program.with_file_name("item.json"),
            r#"{"id":"note-id","notes":""}"#,
        )
        .unwrap();
        let mut records = Table::new();
        records.insert("TOKEN".into(), toml::Value::String("new-value".into()));

        with_session(Some("valid"), || {
            let mut store = BitwardenStore::with_program("dotr-secrets".into(), program.clone());
            store.save(Path::new("."), &records).unwrap();
        });

        let edited = fs::read_to_string(program.with_file_name("edited")).unwrap();
        assert!(edited.contains(r#"TOKEN = \"new-value\""#), "{edited}");
        assert!(!calls(&program).contains(&"unlock".to_string()));
        fs::remove_dir_all(program.parent().unwrap()).ok();
    }

    #[test]
    fn test_without_a_live_session_it_unlocks() {
        let program = fake_bw();

        for session in [None, Some("expired")] {
            fs::remove_file(program.with_file_name("calls")).ok();
            let err = with_session(session, || {
                let mut store =
                    BitwardenStore::with_program("dotr-secrets".into(), program.clone());
                store.get(Path::new("."), &keys(&["TOKEN"])).unwrap_err()
            });

            assert!(
                err.to_string().contains("Bitwarden authentication failed"),
                "{err}"
            );
            let calls = calls(&program);
            assert!(calls.contains(&"unlock".to_string()), "{calls:?}");
            assert!(!calls.contains(&"get".to_string()), "{calls:?}");
        }
        fs::remove_dir_all(program.parent().unwrap()).ok();
    }
}
