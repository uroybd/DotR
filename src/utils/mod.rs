use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use toml::Table;

#[cfg(test)]
mod tests;

pub const BACKUP_EXT: &str = "dotrbak";
pub const SYMLINK_FOLDER: &str = "deployed";

/// Resolve a path string to an absolute PathBuf
/// - If the path starts with '/', it's treated as an absolute path
/// - If the path starts with '~', it's treated as relative to the home directory
/// - Otherwise, it's treated as relative to the current working directory (cwd)
pub fn resolve_path(path: &str, cwd: &Path) -> anyhow::Result<PathBuf> {
    if path.starts_with('/') {
        Ok(PathBuf::from(path))
    } else if path.starts_with("~") {
        let home_dir = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        let p = path.splitn(2, '/').collect::<Vec<&str>>();
        Ok(home_dir.join(p[1..].join("/")))
    } else {
        let p = cwd.join(path);
        Ok(std::path::absolute(&p)?)
    }
}

/// Convert an absolute path to use ~ notation if it's in the home directory
/// - If the path is within the home directory, converts it to ~/...
/// - Otherwise, returns the original path as a string
pub fn normalize_home_path(path: &str) -> String {
    if path.starts_with('~') {
        return path.to_string();
    }

    if let Ok(home_dir) = std::env::var("HOME").map(PathBuf::from) {
        let home_str = home_dir.to_string_lossy();

        if path == home_str.as_ref() {
            return "~".to_string();
        }

        let home_with_slash = format!("{home_str}/");
        if path.starts_with(&home_with_slash) {
            let relative = &path[home_str.len()..];
            return format!("~{relative}");
        }
    }

    path.to_string()
}

pub const COLOR_WARNING: &str = "\x1b[33m"; // Yellow
pub const COLOR_ERROR: &str = "\x1b[31m"; // Red
pub const COLOR_INFO: &str = "\x1b[34m"; // Blue
pub const COLOR_FATAL: &str = "\x1b[35m"; // Magenta
pub const RESET_COLOR: &str = "\x1b[0m"; // Reset

pub enum LogLevel {
    Warning,
    Error,
    Info,
    Fatal,
}

impl LogLevel {
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
            LogLevel::Info => "INFO",
            LogLevel::Fatal => "FATAL",
        }
    }

    pub fn to_colorful_str(&self) -> String {
        match self {
            LogLevel::Warning => format!("{}[{}]{}", COLOR_WARNING, self.as_str(), RESET_COLOR),
            LogLevel::Error => format!("{}[{}]{}", COLOR_ERROR, self.as_str(), RESET_COLOR),
            LogLevel::Info => format!("{}[{}]{}", COLOR_INFO, self.as_str(), RESET_COLOR),
            LogLevel::Fatal => format!("{}[{}]{}", COLOR_FATAL, self.as_str(), RESET_COLOR),
        }
    }
}

/// Writes to standard output, as `print!` does. With [`outln!`] and [`cprintln`] it is the only way this crate
/// prints: the `print_stdout` lint (Cargo.toml) forbids `print!` and `println!` everywhere else.
#[macro_export]
macro_rules! out {
    ($($arg:tt)*) => {{
        ::std::print!($($arg)*);
    }};
}

/// Writes a line to standard output, as `println!` does. See [`out!`].
#[macro_export]
macro_rules! outln {
    ($($arg:tt)*) => {{
        ::std::println!($($arg)*);
    }};
}

#[expect(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the crate's one way to write a status line to the terminal"
)]
pub fn cprintln(message: &str, level: &LogLevel) {
    match level {
        LogLevel::Error | LogLevel::Fatal => {
            eprintln!("{} {}", level.to_colorful_str(), message);
        }
        LogLevel::Warning | LogLevel::Info => {
            println!("{} {}", level.to_colorful_str(), message);
        }
    }
}

pub fn get_string_from_value(v: Option<&toml::Value>, field_name: &str) -> anyhow::Result<String> {
    Ok(
        v.ok_or_else(|| anyhow::anyhow!("'{field_name}' is required"))?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'{field_name}' must be a string"))?
            .to_string(),
    )
}

pub fn get_string_hashmap_from_value(
    v: Option<&toml::Value>,
) -> anyhow::Result<HashMap<String, String>> {
    match v {
        Some(value) => value
            .as_table()
            .ok_or_else(|| anyhow::anyhow!("field must be a table"))?
            .iter()
            .map(|(key, value)| {
                let s = value
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("table values must be strings"))?;
                Ok((key.clone(), s.to_string()))
            })
            .collect::<Result<HashMap<_, _>, _>>(),
        None => Ok(HashMap::new()),
    }
}

pub fn get_vec_string_from_value(v: Option<&toml::Value>) -> anyhow::Result<Vec<String>> {
    match v {
        Some(block) => block
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("field must be an array"))?
            .iter()
            .map(|v| {
                v.as_str()
                    .ok_or_else(|| anyhow::anyhow!("array elements must be strings"))
                    .map(std::string::ToString::to_string)
            })
            .collect::<Result<Vec<_>, _>>(),
        None => Ok(Vec::new()),
    }
}

pub fn is_empty_table(t: &toml::Table) -> bool {
    t.is_empty()
}

pub fn execute_action(
    action: &str,
    variables: &Table,
    working_dir: &Path,
    dry_run: bool,
) -> anyhow::Result<()> {
    let compiled_action = compile_string(action, variables)?;
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    if dry_run {
        cprintln(
            &format!("(Dry Run) Would execute action: {compiled_action}"),
            &LogLevel::Info,
        );
        return Ok(());
    }
    let status = std::process::Command::new(shell)
        .arg("-c")
        .arg(compiled_action)
        .current_dir(working_dir)
        .status()?;
    if !status.success() {
        let msg = format!(
            "Action '{}' failed with exit code: {:?}",
            action,
            status.code()
        );
        cprintln(&msg, &LogLevel::Error);
        return Err(anyhow::anyhow!(msg));
    }
    Ok(())
}

/// Text as this crate means it: UTF-8 without a NUL byte. Anything else is binary, and is copied and compared
/// byte for byte, never diffed or compiled.
pub fn as_text(bytes: &[u8]) -> Option<&str> {
    std::str::from_utf8(bytes).ok().filter(|text| is_text(text))
}

/// Whether a string that decoded as UTF-8 is text, not binary that happens to (see [`as_text`]).
pub fn is_text(s: &str) -> bool {
    !s.contains('\0')
}

pub fn compile_string(template_str: &str, context: &Table) -> anyhow::Result<String> {
    let ctx = tera::Context::from_serialize(context)?;
    Ok(tera::Tera::one_off(template_str, &ctx, false)?)
}

/// An optional boolean field: `None` when absent, an error when it holds anything but a boolean (never read as if
/// it were absent: `skip = "true"` must not mean "don't skip").
pub fn get_opt_bool_from_value(
    v: Option<&toml::Value>,
    field_name: &str,
) -> anyhow::Result<Option<bool>> {
    v.map(|v| {
        v.as_bool()
            .ok_or_else(|| anyhow::anyhow!("'{field_name}' must be true or false"))
    })
    .transpose()
}

/// An optional string field: `None` when absent, an error when it holds anything but a string.
pub fn get_opt_string_from_value(
    v: Option<&toml::Value>,
    field_name: &str,
) -> anyhow::Result<Option<String>> {
    v.map(|v| {
        v.as_str()
            .map(ToString::to_string)
            .ok_or_else(|| anyhow::anyhow!("'{field_name}' must be a string"))
    })
    .transpose()
}

/// An optional table field: empty when absent, an error when it holds anything but a table.
pub fn get_table_from_value(v: Option<&toml::Value>, field_name: &str) -> anyhow::Result<Table> {
    v.map_or_else(
        || Ok(Table::new()),
        |v| {
            v.as_table()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("'{field_name}' must be a table"))
        },
    )
}
