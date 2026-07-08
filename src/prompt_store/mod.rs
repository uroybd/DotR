//! Storage backends for prompted variables: `.uservariables.toml` (the
//! default), the OS keychain, or a shared Bitwarden secure note. All three
//! implement [`PromptStore`], so [`crate::context`] never needs to know
//! which one it's talking to.

use std::{
    cell::RefCell,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use toml::Table;

use crate::utils::{LogLevel, cprintln};

#[cfg(test)]
mod tests;

/// Default name of the Bitwarden secure note used to store every
/// bitwarden-backed prompt in a repository, when `Config.bitwarden_note`
/// isn't set.
pub const DEFAULT_BITWARDEN_NOTE: &str = "dotr-secrets";

/// Where a prompted variable's answer is stored between runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptBackend {
    /// `.uservariables.toml` in the repo root — today's only behavior.
    #[default]
    File,
    /// The OS keychain (macOS Keychain / Windows Credential Manager /
    /// Linux Secret Service), namespaced per-repository.
    Keychain,
    /// A single Bitwarden secure note, shared by every bitwarden-backed
    /// prompt in the repository.
    Bitwarden,
}

impl PromptBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            PromptBackend::File => "file",
            PromptBackend::Keychain => "keychain",
            PromptBackend::Bitwarden => "bitwarden",
        }
    }

    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s {
            "file" => Ok(PromptBackend::File),
            "keychain" => Ok(PromptBackend::Keychain),
            "bitwarden" => Ok(PromptBackend::Bitwarden),
            other => {
                anyhow::bail!("unknown backend '{other}' (expected file, keychain, or bitwarden)")
            }
        }
    }
}

/// The transparent interface every backend implements. `get` returning
/// `Ok(None)` means "no cached answer — go ahead and prompt"; the caller
/// is responsible for calling `set` with whatever the user types.
pub trait PromptStore {
    fn get(&self, key: &str) -> anyhow::Result<Option<String>>;
    fn set(&self, key: &str, value: &str) -> anyhow::Result<()>;
}

// File backend

pub struct FileStore {
    working_dir: PathBuf,
    cache: RefCell<Table>,
}

impl FileStore {
    pub fn new(working_dir: PathBuf, cache: Table) -> Self {
        Self {
            working_dir,
            cache: RefCell::new(cache),
        }
    }

    /// The cache as it stands after any `set` calls this run — the caller
    /// uses this to update `Context::user_variables` in place.
    pub fn into_table(self) -> Table {
        self.cache.into_inner()
    }
}

impl PromptStore for FileStore {
    fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        Ok(self
            .cache
            .borrow()
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }

    fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.cache
            .borrow_mut()
            .insert(key.to_string(), toml::Value::String(value.to_string()));
        let toml_string = toml::to_string(&*self.cache.borrow())?;
        std::fs::write(self.working_dir.join(".uservariables.toml"), toml_string)?;
        Ok(())
    }
}

// Keychain backend

pub struct KeychainStore {
    service: String,
}

impl KeychainStore {
    pub fn new(working_dir: &Path) -> Self {
        // No real "spec" for entry naming here — `DOTR`/`DOTR_` prefixes
        // plus the repo path just keep entries identifiable and scoped
        // per-repo.
        Self {
            service: format!("DOTR:{}", working_dir.display()),
        }
    }

    fn entry(&self, key: &str) -> anyhow::Result<keyring::Entry> {
        keyring::Entry::new(&self.service, &format!("DOTR_{key}"))
            .map_err(|e| anyhow::anyhow!("Failed to access the OS keychain for '{key}': {e}"))
    }
}

impl PromptStore for KeychainStore {
    fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        match self.entry(key)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow::anyhow!(
                "Failed to read '{key}' from the OS keychain: {e}"
            )),
        }
    }

    fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.entry(key)?
            .set_password(value)
            .map_err(|e| anyhow::anyhow!("Failed to save '{key}' to the OS keychain: {e}"))
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
}

pub struct BitwardenStore {
    note: String,
    state: RefCell<Option<BitwardenState>>,
    session: RefCell<Option<String>>,
}

impl BitwardenStore {
    pub fn new(note: String) -> Self {
        Self {
            note,
            state: RefCell::new(None),
            session: RefCell::new(None),
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
        if let Some(session) = self.session.borrow().as_ref() {
            cmd.args(["--session", session]);
        }
        cmd.stdin(Stdio::null())
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run `bw {}`: {e}", args.join(" ")))
    }

    /// Drives `bw login`/`bw unlock` interactively and caches the
    /// resulting session for subsequent `bw` calls.
    fn authenticate_interactively(&self) -> anyhow::Result<()> {
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
        *self.session.borrow_mut() = Some(
            String::from_utf8_lossy(&unlock_output.stdout)
                .trim()
                .to_string(),
        );
        Ok(())
    }

    fn ensure_loaded(&self) -> anyhow::Result<()> {
        if self.state.borrow().is_some() {
            return Ok(());
        }

        let mut output = self.bw(&["get", "item", &self.note])?;
        if !output.status.success() || output.stdout.is_empty() {
            self.authenticate_interactively()
                .map_err(|e| self.auth_failed_hint(&e.to_string()))?;
            output = self.bw(&["get", "item", &self.note])?;
        }
        if !output.status.success() || output.stdout.is_empty() {
            output = self.create_note()?;
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

        *self.state.borrow_mut() = Some(BitwardenState {
            id,
            envelope,
            values,
        });
        Ok(())
    }
}

impl PromptStore for BitwardenStore {
    fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        self.ensure_loaded()?;
        let state = self.state.borrow();
        let values = &state.as_ref().expect("just ensured loaded").values;
        Ok(values
            .get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()))
    }

    fn set(&self, key: &str, value: &str) -> anyhow::Result<()> {
        self.ensure_loaded()?;

        let (id, encoded_envelope) = {
            let mut state = self.state.borrow_mut();
            let state = state.as_mut().expect("just ensured loaded");
            state
                .values
                .insert(key.to_string(), toml::Value::String(value.to_string()));
            let notes_toml = toml::to_string(&state.values)?;
            state.envelope["notes"] = serde_json::Value::String(notes_toml);
            (state.id.clone(), serde_json::to_string(&state.envelope)?)
        };

        let encoded = run_bw_encode(&encoded_envelope)?;

        let mut edit_output = self.bw(&["edit", "item", &id, &encoded])?;
        if !edit_output.status.success() {
            self.authenticate_interactively()?;
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
        // pre-existing BW_SESSION from the user's own shell is left alone.
        if self.session.borrow().is_some() {
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
