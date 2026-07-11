use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::utils::{LogLevel, cprintln};

pub const DEFAULT_BITWARDEN_NOTE: &str = "dotr-secrets";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptBackendType {
    #[default]
    File,
    Keychain,
    Bitwarden,
}

impl PromptBackendType {
    pub fn as_str(&self) -> &'static str {
        match self {
            PromptBackendType::File => "file",
            PromptBackendType::Keychain => "keychain",
            PromptBackendType::Bitwarden => "bitwarden",
        }
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "file" => Ok(PromptBackendType::File),
            "keychain" => Ok(PromptBackendType::Keychain),
            "bitwarden" => Ok(PromptBackendType::Bitwarden),
            other => {
                anyhow::bail!("unknown backend '{other}' (expected file, keychain, or bitwarden)")
            }
        }
    }
}

pub trait PromptStoreBackend {
    fn get_session(&self) -> Option<Table>;
    fn set_session(&mut self, session: Option<Table>);
    fn get(&mut self, cwd: &Path, keys: &[String]) -> anyhow::Result<Table>;
    fn save(&mut self, cwd: &Path, records: &Table) -> anyhow::Result<()>;
}

pub struct FileStore {}

impl Default for FileStore {
    fn default() -> Self {
        Self::new()
    }
}

impl FileStore {
    pub fn new() -> Self {
        FileStore {}
    }

    fn get_file_path(&self, cwd: &Path) -> PathBuf {
        cwd.join(".uservariables.toml")
    }
}

impl PromptStoreBackend for FileStore {
    // Stateless between calls - nothing to carry across instances.
    fn get_session(&self) -> Option<Table> {
        None
    }

    fn set_session(&mut self, _session: Option<Table>) {}

    // Unlike Keychain/Bitwarden, the file already holds everything - no
    // need to scope the read to just the currently-declared prompt keys.
    fn get(&mut self, cwd: &Path, _keys: &[String]) -> anyhow::Result<Table> {
        let path = self.get_file_path(cwd);
        if !path.exists() {
            return Ok(Table::new());
        }
        let content = fs::read_to_string(&path)?;
        let records: Table = toml::from_str(&content)?;
        Ok(records)
    }

    fn save(&mut self, cwd: &Path, records: &Table) -> anyhow::Result<()> {
        let path = self.get_file_path(cwd);
        let content_string = toml::to_string(records)?;
        fs::write(path, content_string)?;
        Ok(())
    }
}

pub struct KeychainStore {}

impl Default for KeychainStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeychainStore {
    pub fn new() -> Self {
        KeychainStore {}
    }

    // No real "spec" for entry naming here — `DOTR`/`DOTR_` prefixes plus
    // the repo path just keep entries identifiable and scoped per-repo.
    fn entry(&self, cwd: &Path, key: &str) -> anyhow::Result<keyring::Entry> {
        let service = format!("DOTR:{}", cwd.display());
        keyring::Entry::new(&service, &format!("DOTR_{key}"))
            .map_err(|e| anyhow::anyhow!("Failed to access the OS keychain for '{key}': {e}"))
    }
}

impl PromptStoreBackend for KeychainStore {
    // The OS keychain needs no session of its own - every call is a fresh,
    // already-authenticated lookup.
    fn get_session(&self) -> Option<Table> {
        None
    }

    fn set_session(&mut self, _session: Option<Table>) {}

    fn get(&mut self, cwd: &Path, keys: &[String]) -> anyhow::Result<Table> {
        let mut records = Table::new();
        for key in keys {
            match self.entry(cwd, key)?.get_password() {
                Ok(value) => {
                    records.insert(key.clone(), toml::Value::String(value));
                }
                Err(keyring::Error::NoEntry) => {}
                Err(e) => {
                    anyhow::bail!("Failed to read '{key}' from the OS keychain: {e}");
                }
            }
        }
        Ok(records)
    }

    fn save(&mut self, cwd: &Path, records: &Table) -> anyhow::Result<()> {
        for (key, value) in records.iter() {
            if let Some(v) = value.as_str() {
                self.entry(cwd, key)?.set_password(v).map_err(|e| {
                    anyhow::anyhow!("Failed to save '{key}' to the OS keychain: {e}")
                })?;
            }
        }
        Ok(())
    }
}

// Bitwarden backend — one secure note stores every bitwarden-backed
// variable for the repo, as a TOML blob in the note's "Notes" field.

struct BitwardenState {
    id: String,
    envelope: serde_json::Value,
    /// Same `key = "value"` TOML format as `.uservariables.toml`, so a
    /// note's content is copy-pasteable to/from the file backend.
    values: Table,
    /// True only if *this* instance called `bw unlock` itself while
    /// loading this state (vs. having a session handed to it via
    /// `set_session`). Only the instance that actually unlocked the vault
    /// should lock it back up on drop - otherwise every short-lived
    /// `BitwardenStore` built with an inflated session would relock the
    /// vault out from under the next one. Lives here rather than on
    /// `BitwardenStore` directly since it's only ever known once we've
    /// actually done the load that state represents.
    unlocked_by_us: bool,
}

pub struct BitwardenStore {
    note: String,
    state: Option<BitwardenState>,
    session: Option<String>,
}

impl BitwardenStore {
    pub fn new(note: String) -> Self {
        Self {
            note,
            state: None,
            session: None,
        }
    }

    fn auth_failed_hint(&self, detail: &str) -> anyhow::Error {
        anyhow::anyhow!(
            "Bitwarden authentication failed: {}\nMake sure `bw login`/`bw unlock` can \
complete in this terminal.",
            detail
        )
    }

    /// Creates the note fresh as an empty Secure Note, matching the exact
    /// item shape `bw create item` expects (verified against
    /// bitwarden/clients' CipherExport/SecureNoteExport templates).
    fn create_note(&self) -> anyhow::Result<std::process::Output> {
        cprintln(
            &format!("Bitwarden note '{}' not found — creating it...", self.note),
            &LogLevel::Info,
        );
        let template = serde_json::json!({
            "organizationId": null,
            "folderId": null,
            "type": 2,
            "name": self.note,
            "notes": "",
            "favorite": false,
            "fields": [],
            "reprompt": 0,
            "secureNote": { "type": 0 },
            "collectionIds": []
        });
        let encoded = run_bw_encode(&serde_json::to_string(&template)?)?;
        let output = self.bw(&["create", "item", &encoded])?;
        if !output.status.success() || output.stdout.is_empty() {
            anyhow::bail!(
                "Failed to create Bitwarden secure note '{}': {}",
                self.note,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(output)
    }

    /// Runs a `bw` subcommand non-interactively, with `--session` appended
    /// if we already have one cached from `authenticate_interactively`.
    fn bw(&self, args: &[&str]) -> anyhow::Result<std::process::Output> {
        let mut cmd = Command::new("bw");
        cmd.args(args);
        if let Some(session) = self.session.as_ref() {
            cmd.args(["--session", session]);
        }
        cmd.stdin(Stdio::null())
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run `bw {}`: {e}", args.join(" ")))
    }

    /// Pulls the latest vault data from the server before we read/write,
    /// so a note edited elsewhere (another device, the web vault) isn't
    /// missed by a stale local cache. Best-effort and silent: called
    /// speculatively before we necessarily know we're authenticated, so a
    /// failure here isn't itself noteworthy - a real problem still
    /// surfaces properly from the get/set call that follows it.
    fn sync(&self) {
        let _ = self.bw(&["sync"]);
    }

    /// Drives `bw login`/`bw unlock` interactively and caches the
    /// resulting session for subsequent `bw` calls.
    fn authenticate_interactively(&mut self) -> anyhow::Result<()> {
        let status_output = Command::new("bw")
            .arg("status")
            .stdin(Stdio::null())
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run `bw status`: {e}"))?;
        let status: serde_json::Value =
            serde_json::from_slice(&status_output.stdout).unwrap_or(serde_json::Value::Null);

        if status.get("status").and_then(|s| s.as_str()) == Some("unauthenticated") {
            cprintln(
                "Not logged in to Bitwarden — running `bw login`...",
                &LogLevel::Info,
            );
            let login_ok = Command::new("bw")
                .arg("login")
                .stdin(Stdio::inherit())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status()
                .map_err(|e| anyhow::anyhow!("Failed to run `bw login`: {e}"))?
                .success();
            if !login_ok {
                anyhow::bail!("`bw login` did not complete successfully");
            }
        }

        cprintln("Unlocking your Bitwarden vault...", &LogLevel::Info);
        // Prompt text goes to stderr (inherited, so it's visible); the raw
        // session key on success is the only thing on stdout (captured).
        let unlock_output = Command::new("bw")
            .args(["unlock", "--raw"])
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run `bw unlock`: {e}"))?;
        if !unlock_output.status.success() || unlock_output.stdout.is_empty() {
            anyhow::bail!("`bw unlock` did not complete successfully");
        }
        self.session = Some(
            String::from_utf8_lossy(&unlock_output.stdout)
                .trim()
                .to_string(),
        );
        // Fresh session — pull the latest vault data before reading/writing,
        // so a note edited elsewhere isn't missed by a stale local cache.
        self.sync();
        Ok(())
    }

    fn ensure_loaded(&mut self) -> anyhow::Result<()> {
        if self.state.is_some() {
            return Ok(());
        }

        // Covers the "already had a valid session" case (e.g. an exported
        // BW_SESSION) - if we end up authenticating below instead, that
        // path syncs again right after, once we know it's worth reporting
        // a failure.
        self.sync();

        let mut output = self.bw(&["get", "item", &self.note])?;
        let mut unlocked_by_us = false;
        if !output.status.success() || output.stdout.is_empty() {
            self.authenticate_interactively()
                .map_err(|e| self.auth_failed_hint(&e.to_string()))?;
            unlocked_by_us = true;
            output = self.bw(&["get", "item", &self.note])?;
        }
        if !output.status.success() || output.stdout.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Only auto-create on a genuine "doesn't exist yet" - anything
            // else (ambiguous name match, permissions, ...) must surface as
            // an error instead, or a lookup failure for any other reason
            // would silently spawn a brand-new empty note, shadowing
            // whatever real note/values you already had.
            if stderr.trim() == "Not found." {
                output = self.create_note()?;
            } else {
                anyhow::bail!(
                    "`bw get item {}` failed: {}\nThis isn't a \"note doesn't exist yet\" \
case, so dotr won't create a new note automatically - fix the underlying issue (e.g. \
if multiple items are named '{}', rename or delete the extras so the name is unique) \
and try again.",
                    self.note,
                    stderr.trim(),
                    self.note
                );
            }
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let envelope: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse `bw get item {}` output as JSON: {e}",
                self.note
            )
        })?;
        let id = envelope
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Bitwarden item '{}' has no id", self.note))?
            .to_string();
        let notes = envelope.get("notes").and_then(|v| v.as_str()).unwrap_or("");
        let values: Table = if notes.trim().is_empty() {
            Table::new()
        } else {
            toml::from_str(notes).map_err(|e| {
                anyhow::anyhow!(
                    "Bitwarden note '{}' doesn't contain valid `key = \"value\"` TOML content \
(the same format as .uservariables.toml): {e}",
                    self.note
                )
            })?
        };

        self.state = Some(BitwardenState {
            id,
            envelope,
            values,
            unlocked_by_us,
        });
        Ok(())
    }
}

impl PromptStoreBackend for BitwardenStore {
    // Backends are reconstructed per call (see `Context::get_backend`), so
    // this is how a `bw unlock` session survives from one call to the
    // next within the same run - the caller round-trips this table through
    // `set_session` on the next `BitwardenStore` it builds, instead of
    // unlocking (and, on drop, re-locking) the vault every single call.
    fn get_session(&self) -> Option<Table> {
        self.session.as_ref().map(|session| {
            let mut table = Table::new();
            table.insert("session".to_string(), toml::Value::String(session.clone()));
            table
        })
    }

    fn set_session(&mut self, session: Option<Table>) {
        self.session = session.and_then(|table| {
            table
                .get("session")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });
    }

    fn get(&mut self, _cwd: &Path, keys: &[String]) -> anyhow::Result<Table> {
        self.ensure_loaded()?;
        let values = &self.state.as_ref().expect("just ensured loaded").values;
        let mut records = Table::new();
        for key in keys {
            if let Some(v) = values.get(key) {
                records.insert(key.clone(), v.clone());
            }
        }
        Ok(records)
    }

    fn save(&mut self, _cwd: &Path, records: &Table) -> anyhow::Result<()> {
        self.ensure_loaded()?;

        let (id, encoded_envelope) = {
            let state = self.state.as_mut().expect("just ensured loaded");
            for (key, value) in records.iter() {
                state.values.insert(key.clone(), value.clone());
            }
            let notes_toml = toml::to_string(&state.values)?;
            state.envelope["notes"] = serde_json::Value::String(notes_toml);
            (state.id.clone(), serde_json::to_string(&state.envelope)?)
        };

        let encoded = run_bw_encode(&encoded_envelope)?;

        let mut edit_output = self.bw(&["edit", "item", &id, &encoded])?;
        if !edit_output.status.success() {
            self.authenticate_interactively()?;
            if let Some(state) = self.state.as_mut() {
                state.unlocked_by_us = true;
            }
            edit_output = self.bw(&["edit", "item", &id, &encoded])?;
            if !edit_output.status.success() {
                anyhow::bail!(
                    "`bw edit item` failed: {}",
                    String::from_utf8_lossy(&edit_output.stderr).trim()
                );
            }
        }
        Ok(())
    }
}

impl Drop for BitwardenStore {
    fn drop(&mut self) {
        // Only lock back up if we're the ones who unlocked it — a
        // pre-existing BW_SESSION from the user's own shell, or a session
        // inflated from a prior `BitwardenStore` via `set_session`, is
        // left alone.
        let unlocked_by_us = self.state.as_ref().is_some_and(|s| s.unlocked_by_us);
        if unlocked_by_us {
            cprintln("Locking your Bitwarden vault...", &LogLevel::Info);
            match self.bw(&["lock"]) {
                Ok(output) if !output.status.success() => cprintln(
                    &format!(
                        "Failed to re-lock the Bitwarden vault: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    ),
                    &LogLevel::Warning,
                ),
                Err(e) => cprintln(
                    &format!("Failed to re-lock the Bitwarden vault: {e}"),
                    &LogLevel::Warning,
                ),
                _ => {}
            }
        }
    }
}

fn run_bw_encode(json: &str) -> anyhow::Result<String> {
    let mut child = Command::new("bw")
        .arg("encode")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to run `bw encode`: {e}"))?;
    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(json.as_bytes())?;
    let output = child
        .wait_with_output()
        .map_err(|e| anyhow::anyhow!("Failed to run `bw encode`: {e}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "`bw encode` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
