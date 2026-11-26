//! Backing up a templated file: carrying what changed in its deployed copy back into the template.
//!
//! A three-way merge (diff3) with the template rendered as the common ancestor: `base` is the template compiled with
//! the current variables, `ours` the template itself, `theirs` the deployed file. A line with no tags is the same in
//! `base` and `ours`, so an edit to it in the deployed file merges cleanly; a line the template computes (a
//! `{{ variable }}`, a `{% %}` block, a loop's output) differs between the two, so an edit to it is a conflict.
//!
//! Lines taken from the deployed file go through reverse substitution first: where a variable's value appears in
//! them it is written back as `{{ name }}`, so a value (a secret above all) is not copied into the repository as text.
//! A conflict whose template side only holds bare `{{ variable }}` tags is settled the same way, when every one of
//! those variables can be put back. Any other conflict goes to the caller's `resolve`, which picks a `Resolution`.
//!
//! Nothing is accepted unless it round-trips: the merged template, compiled, must give the deployed file exactly,
//! except where a resolution kept the template's lines, where it must give what the template gave before. Every
//! substitution, every resolution and every part that could not be merged is returned for the caller to report.

use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    ops::Range,
    path::Path,
    sync::LazyLock,
};

use toml::Table;

use crate::utils::{as_text, compile_string};

#[cfg(test)]
mod tests;

/// The conflict markers of a draft opened in the editor, as git writes them.
const MARK_OURS: &str = "<<<<<<<";
const MARK_SPLIT: &str = "=======";
const MARK_THEIRS: &str = ">>>>>>>";

/// A variable's value shorter than this is too likely to occur by chance to be written back as a tag (a secret's is
/// written back whatever its length), or to stop a plain file's backup (`Package::backup`).
pub const MIN_VALUE_CHARS: usize = 4;

/// A bare variable tag: `{{ name }}` or `{{ a.b }}`, with Tera's whitespace control allowed, nothing else.
#[expect(
    clippy::expect_used,
    reason = "a fixed pattern: it compiles, and every merge test runs it"
)]
static BARE_TAG: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\{\{-?\s*([A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*)\s*-?\}\}")
        .expect("BARE_TAG is a valid pattern")
});

/// Anything that opens a Tera expression, statement or comment.
#[expect(
    clippy::expect_used,
    reason = "a fixed pattern: it compiles, and every merge test runs it"
)]
static ANY_TAG: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\{\{|\{%|\{#").expect("ANY_TAG is a valid pattern"));

/// What backing up one templated file came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The deployed file is what the template renders to: nothing to back up.
    Unchanged,
    /// The new template, and every way its content differs from what the deployed file holds.
    Merged {
        template: String,
        changes: Vec<Change>,
    },
    /// Parts of the deployed file the merge could not carry back; the template is left as it is.
    Conflicts(Vec<Conflict>),
    /// The merged template would not render to the deployed file, so it is not written. `error` is Tera's, when it
    /// did not compile at all.
    RoundTripFailed { error: Option<String> },
    /// The deployed file is not text, so it is not merged into a text template.
    NotText,
    /// A conflict was answered `Edit`: the merged file with the conflicts marked, for the user to finish. `changes`
    /// are those made outside the marked conflicts.
    Edit { draft: String, changes: Vec<Change> },
}

/// One change the merge made to deployed content on its way into the template. Lines are 1-based lines of the new
/// template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The value of `name` was written as `{{ name }}`.
    Substituted {
        line: usize,
        name: String,
        secret: bool,
    },
    /// A value that belongs to several variables was left as text: which one was meant can't be told.
    Ambiguous { line: usize, names: Vec<String> },
    /// Template lines the deployed file changed were rewritten from it, their variables put back as tags.
    Rewritten { lines: Range<usize> },
    /// A conflict resolved as ours: the template's `lines` were kept and the deployed file's `deployed_lines` left out.
    KeptTemplate {
        lines: Range<usize>,
        deployed_lines: Range<usize>,
    },
    /// A conflict resolved as theirs: the template's lines there (and whatever logic they held) were replaced by the
    /// deployed file's, now `lines`.
    TookDeployed { lines: Range<usize> },
    /// A conflict resolved as both: the template's `lines` were kept and the deployed file's added after them as
    /// `added`, so the template now renders both.
    Union {
        lines: Range<usize>,
        added: Range<usize>,
    },
}

/// What to do with one conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Resolution {
    /// Keep the template's lines; leave the deployed change out.
    Ours,
    /// Take the deployed lines (values put back as tags) in place of the template's.
    Theirs,
    /// Keep the template's lines and add the deployed lines after them, to tidy by hand.
    Both,
    /// Open the whole file in an editor, with this conflict and every later one marked, and use what is saved.
    Edit,
    /// Leave the whole file as it is.
    Skip,
}

/// Lines of the deployed file that change what the template computes. Lines are 1-based; `deployed` has every secret
/// value masked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conflict {
    pub template_lines: Range<usize>,
    pub template: Vec<String>,
    pub deployed_lines: Range<usize>,
    pub deployed: Vec<String>,
}

/// A variable whose value may be written back as a tag.
#[derive(Debug, Clone)]
struct Candidate {
    value: String,
    /// The variables holding this value, sorted.
    names: Vec<String>,
    secret: bool,
}

impl Candidate {
    /// The name to write back, or `None` when the value is ambiguous. A secret is always written back (by the first
    /// secret's name, since any of them renders the same), so its value never lands in the template as text.
    fn name(&self, secrets: &BTreeSet<String>) -> Option<&str> {
        match self.names.as_slice() {
            [one] => Some(one),
            _ if self.secret => self
                .names
                .iter()
                .find(|n| secrets.contains(*n))
                .map(String::as_str),
            _ => None,
        }
    }
}

/// Merges the deployed copy of a templated file back into the template. `vars` are the variables the template
/// renders with; `secrets` names the ones that must never be written out as text (the prompted variables).
/// `resolve` is asked about each conflict the merge can't settle itself; one answered `Skip` (and every one after it,
/// which is still asked so it can be reported) leaves the file as it is.
pub fn merge_template(
    template: &str,
    deployed: &[u8],
    vars: &Table,
    secrets: &BTreeSet<String>,
    resolve: &mut dyn FnMut(&Conflict) -> anyhow::Result<Resolution>,
) -> anyhow::Result<Outcome> {
    let Some(deployed) = as_text(deployed) else {
        return Ok(Outcome::NotText);
    };
    let base = compile_string(template, vars)?;
    if base == deployed {
        return Ok(Outcome::Unchanged);
    }

    let candidates = candidates(template, vars, secrets);
    let base_lines = split_lines(&base);
    let ours = split_lines(template);
    let theirs = split_lines(deployed);

    let mut m = Merging {
        merged: Vec::with_capacity(ours.len()),
        expected: Vec::with_capacity(theirs.len()),
        changes: vec![],
        unresolved: vec![],
        editing: false,
        candidates: &candidates,
        secrets,
    };
    for chunk in diff3(&base_lines, &ours, &theirs) {
        let b = slice(&base_lines, &chunk.base)?;
        let o = slice(&ours, &chunk.ours)?;
        let t = slice(&theirs, &chunk.theirs)?;
        if chunk.stable || o == t || t == b {
            m.keep(o);
            m.expected.extend(t);
        } else if o == b {
            m.take(t);
            m.expected.extend(t);
        } else if let Some((head, tail)) = tag_edges(o, b) {
            m.keep(head);
            m.take(t);
            m.keep(tail);
            m.expected.extend(t);
        } else if let Some(lines) =
            settle(o, t, m.candidates, secrets, m.merged.len(), &mut m.changes)
        {
            let start = m.next_line();
            m.merged.extend(lines);
            m.changes.push(Change::Rewritten {
                lines: start..m.next_line(),
            });
            m.expected.extend(t);
        } else {
            let conflict = Conflict {
                template_lines: chunk.ours.start + 1..chunk.ours.end + 1,
                template: o.iter().map(|l| trim_newline(l).to_string()).collect(),
                deployed_lines: chunk.theirs.start + 1..chunk.theirs.end + 1,
                deployed: t
                    .iter()
                    .map(|l| mask(trim_newline(l), &candidates))
                    .collect(),
            };
            let resolution = if !m.unresolved.is_empty() {
                Resolution::Skip
            } else if m.editing {
                Resolution::Edit
            } else {
                resolve(&conflict)?
            };
            m.resolve(resolution, conflict, b, o, t);
        }
    }
    let Merging {
        merged,
        expected,
        changes,
        unresolved,
        editing,
        ..
    } = m;
    if !unresolved.is_empty() {
        return Ok(Outcome::Conflicts(unresolved));
    }
    if editing {
        return Ok(Outcome::Edit {
            draft: merged.concat(),
            changes,
        });
    }

    let merged = merged.concat();
    if merged == template {
        return Ok(Outcome::Unchanged);
    }
    // A merge that compiles to something else (or not at all, say a `{{` the deployed file holds as text) is not a
    // backup of the deployed file: write nothing.
    match compile_string(&merged, vars) {
        Ok(rendered) if rendered == expected.concat() => Ok(Outcome::Merged {
            template: merged,
            changes,
        }),
        Ok(_) => Ok(Outcome::RoundTripFailed { error: None }),
        Err(e) => Ok(Outcome::RoundTripFailed {
            error: Some(format!("{e:#}")),
        }),
    }
}

/// A merge as it is built, chunk by chunk.
struct Merging<'a> {
    /// The template's lines so far.
    merged: Vec<String>,
    /// What `merged` must render to: the deployed lines, except the template's own output where it was kept.
    expected: Vec<&'a str>,
    changes: Vec<Change>,
    /// Conflicts left unsettled: once there is one, the file is not written, and every later one is only reported.
    unresolved: Vec<Conflict>,
    /// Whether a conflict went to the editor, so every later one goes there too.
    editing: bool,
    candidates: &'a [Candidate],
    secrets: &'a BTreeSet<String>,
}

impl<'a> Merging<'a> {
    /// The number the next merged line will have.
    fn next_line(&self) -> usize {
        self.merged.len() + 1
    }

    /// Keeps the template's lines as they are.
    fn keep(&mut self, lines: &[&str]) {
        self.merged.extend(lines.iter().map(|l| (*l).to_string()));
    }

    /// Takes deployed lines, their values written back as tags.
    fn take(&mut self, lines: &[&str]) {
        take(
            lines,
            &mut self.merged,
            self.candidates,
            self.secrets,
            &mut self.changes,
        );
    }

    /// Settles a conflict as `resolution` says: `ours` and `base` are the template's lines and their render,
    /// `theirs` the deployed lines.
    fn resolve(
        &mut self,
        resolution: Resolution,
        conflict: Conflict,
        base: &[&'a str],
        ours: &[&str],
        theirs: &[&'a str],
    ) {
        let start = self.next_line();
        match resolution {
            Resolution::Skip => self.unresolved.push(conflict),
            Resolution::Edit => {
                self.editing = true;
                mark(
                    &conflict,
                    ours,
                    theirs,
                    &mut self.merged,
                    self.candidates,
                    self.secrets,
                );
            }
            Resolution::Ours => {
                self.keep(ours);
                self.expected.extend(base);
                self.changes.push(Change::KeptTemplate {
                    lines: start..self.next_line(),
                    deployed_lines: conflict.deployed_lines,
                });
            }
            Resolution::Theirs => {
                self.take(theirs);
                self.expected.extend(theirs);
                self.changes.push(Change::TookDeployed {
                    lines: start..self.next_line(),
                });
            }
            Resolution::Both => {
                self.keep(ours);
                let added = self.next_line();
                self.expected.extend(base);
                // The kept lines must end in a newline, or the first added line would run on from the last; that
                // newline is text, so it renders too.
                if !ours.is_empty()
                    && let Some(last) = self.merged.last_mut().filter(|l| !l.ends_with('\n'))
                {
                    last.push('\n');
                    self.expected.push("\n");
                }
                self.take(theirs);
                self.expected.extend(theirs);
                self.changes.push(Change::Union {
                    lines: start..added,
                    added: added..self.next_line(),
                });
            }
        }
    }
}

/// Writes a conflict into a draft for the editor, git-style: the template's lines, then the deployed ones with their
/// values (secrets above all) written back as tags.
fn mark(
    conflict: &Conflict,
    ours: &[&str],
    theirs: &[&str],
    draft: &mut Vec<String>,
    candidates: &[Candidate],
    secrets: &BTreeSet<String>,
) {
    let ended = |lines: &[&str]| lines.last().is_none_or(|l| l.ends_with('\n'));
    draft.push(format!(
        "{MARK_OURS} template, {}\n",
        span(&conflict.template_lines)
    ));
    draft.extend(ours.iter().map(|l| (*l).to_string()));
    if !ended(ours) {
        draft.push("\n".to_string());
    }
    draft.push(format!("{MARK_SPLIT}\n"));
    take(theirs, draft, candidates, secrets, &mut vec![]);
    if !ended(theirs) {
        draft.push("\n".to_string());
    }
    draft.push(format!(
        "{MARK_THEIRS} deployed, {} (values written back as tags)\n",
        span(&conflict.deployed_lines)
    ));
}

/// Appends deployed lines to the template, each with its values written back as tags.
fn take(
    lines: &[&str],
    merged: &mut Vec<String>,
    candidates: &[Candidate],
    secrets: &BTreeSet<String>,
    changes: &mut Vec<Change>,
) {
    for line in lines {
        let n = merged.len() + 1;
        merged.push(substitute(line, n, candidates, secrets, changes));
    }
}

/// A conflict as console lines: both sides, numbered, the deployed one with its secrets masked.
pub fn describe(conflict: &Conflict) -> Vec<String> {
    let side = |title: &str, lines: &Range<usize>, text: &[String]| {
        let mut out = vec![format!("  {title} {}:", span(lines))];
        out.extend(
            text.iter()
                .zip(lines.clone())
                .map(|(line, n)| format!("  {n:>5} | {line}")),
        );
        out
    };
    let mut out = side("template", &conflict.template_lines, &conflict.template);
    out.extend(side(
        "deployed",
        &conflict.deployed_lines,
        &conflict.deployed,
    ));
    out
}

/// `line 3`, `lines 3-5`, or `(no lines)` for an empty range, from a 1-based half-open range.
pub fn span(lines: &Range<usize>) -> String {
    match lines.len() {
        0 => format!("(no lines, before line {})", lines.start),
        1 => format!("line {}", lines.start),
        _ => format!("lines {}-{}", lines.start, lines.end - 1),
    }
}

/// Asks what to do with a conflict in `file`, until the answer is one of o, t, b or s. The end of input skips.
pub fn ask<R: io::BufRead, W: io::Write>(
    file: &Path,
    conflict: &Conflict,
    reader: &mut R,
    writer: &mut W,
) -> anyhow::Result<Resolution> {
    writeln!(writer, "Conflict in {}:", file.display())?;
    for line in describe(conflict) {
        writeln!(writer, "{line}")?;
    }
    loop {
        write!(
            writer,
            "[o]urs: keep the template / [t]heirs: take the deployed lines / [b]oth / [e]dit the file / [s]kip it? "
        )?;
        writer.flush()?;
        let mut answer = String::new();
        if reader.read_line(&mut answer)? == 0 {
            return Ok(Resolution::Skip);
        }
        match answer.trim().to_ascii_lowercase().as_str() {
            "o" | "ours" => return Ok(Resolution::Ours),
            "t" | "theirs" => return Ok(Resolution::Theirs),
            "b" | "both" => return Ok(Resolution::Both),
            "e" | "edit" => return Ok(Resolution::Edit),
            "s" | "skip" => return Ok(Resolution::Skip),
            _ => {}
        }
    }
}

/// Asks whether to reopen an edit that can't be used. The end of input, or anything but r, gives up.
pub fn ask_reopen<R: io::BufRead, W: io::Write>(
    reader: &mut R,
    writer: &mut W,
) -> anyhow::Result<bool> {
    write!(writer, "[r]eopen the editor / [s]kip this file? ")?;
    writer.flush()?;
    let mut answer = String::new();
    reader.read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "r" | "reopen"
    ))
}

/// How an edited template renders, against the deployed file it was merged from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edited {
    /// Lines of the deployed file the render leaves out or changes.
    pub missing: usize,
    /// Lines the render has that the deployed file does not.
    pub added: usize,
}

impl Edited {
    pub fn renders_deployed(self) -> bool {
        self.missing == 0 && self.added == 0
    }
}

/// Checks a draft saved from the editor before it becomes the template: no conflict markers left, no secret's value
/// written out as text, and it compiles. `Err` is what to fix, for the user.
pub fn check_edit(
    edited: &str,
    deployed: &str,
    vars: &Table,
    secrets: &BTreeSet<String>,
) -> Result<Edited, String> {
    if let Some((n, _)) = edited.lines().enumerate().find(|(_, l)| {
        l.starts_with(MARK_OURS) || l.starts_with(MARK_THEIRS) || l.trim_end() == MARK_SPLIT
    }) {
        return Err(format!("line {} still has a conflict marker", n + 1));
    }
    for name in secrets {
        if lookup(vars, name).is_some_and(|value| !value.is_empty() && edited.contains(value)) {
            return Err(format!(
                "it holds the value of the secret `{name}` as text; write {{{{ {name} }}}} instead"
            ));
        }
    }
    let rendered =
        compile_string(edited, vars).map_err(|e| format!("it does not compile: {e:#}"))?;
    let (mut missing, mut added) = (0, 0);
    for step in diff::lines(deployed, &rendered) {
        match step {
            diff::Result::Left(_) => missing += 1,
            diff::Result::Right(_) => added += 1,
            diff::Result::Both(..) => {}
        }
    }
    Ok(Edited { missing, added })
}

/// The editor to finish a merge in: `DOTR_EDITOR`, then `VISUAL`, then `EDITOR`.
pub fn editor_command() -> Option<String> {
    ["DOTR_EDITOR", "VISUAL", "EDITOR"]
        .iter()
        .find_map(|key| std::env::var(key).ok().filter(|v| !v.trim().is_empty()))
}

/// Opens `draft` with `command` (split on spaces, so `code --wait` works) in a temporary file named after `name`, for
/// the editor's highlighting, and returns what was saved. The draft holds no secret values: they are tags in it.
pub fn edit_in_editor(command: &str, draft: &str, name: &Path) -> anyhow::Result<String> {
    let mut parts = command.split_whitespace();
    let program = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("The editor command is empty"))?;
    let file_name = name
        .file_name()
        .map_or_else(|| "template".into(), |n| n.to_string_lossy());
    let file =
        std::env::temp_dir().join(format!("dotr-merge-{}-{file_name}", uuid::Uuid::new_v4()));
    std::fs::write(&file, draft)?;
    let status = std::process::Command::new(program)
        .args(parts)
        .arg(&file)
        .status();
    let saved = std::fs::read_to_string(&file);
    // Best effort: what matters has been read, and a leftover file in the temporary directory holds no secret.
    std::fs::remove_file(&file).ok();
    let status =
        status.map_err(|e| anyhow::anyhow!("Failed to run the editor `{command}`: {e}"))?;
    if !status.success() {
        anyhow::bail!("The editor `{command}` exited with {status}");
    }
    Ok(saved?)
}

/// Lines with their line endings kept, so joining them gives the text back byte for byte.
fn split_lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

fn trim_newline(line: &str) -> &str {
    line.trim_end_matches(['\n', '\r'])
}

fn slice<'a>(lines: &'a [&'a str], range: &Range<usize>) -> anyhow::Result<&'a [&'a str]> {
    lines.get(range.clone()).ok_or_else(|| {
        anyhow::anyhow!(
            "merge chunk {range:?} is outside a {}-line file",
            lines.len()
        )
    })
}

/// The variables whose values may be written back: those the template already uses as bare tags (values of at least
/// `MIN_VALUE_CHARS`), and every secret. Longest value first, so a value containing another wins.
fn candidates(template: &str, vars: &Table, secrets: &BTreeSet<String>) -> Vec<Candidate> {
    let mut by_value: BTreeMap<String, Candidate> = BTreeMap::new();
    let mut add = |name: &str, secret: bool| {
        let Some(value) = lookup(vars, name) else {
            return;
        };
        if value.is_empty() || (!secret && value.chars().count() < MIN_VALUE_CHARS) {
            return;
        }
        let entry = by_value
            .entry(value.to_string())
            .or_insert_with(|| Candidate {
                value: value.to_string(),
                names: vec![],
                secret: false,
            });
        if !entry.names.iter().any(|n| n == name) {
            entry.names.push(name.to_string());
            entry.names.sort();
        }
        entry.secret |= secret;
    };
    for name in referenced(template) {
        add(&name, secrets.contains(&name));
    }
    for name in secrets {
        add(name, true);
    }
    let mut all: Vec<Candidate> = by_value.into_values().collect();
    all.sort_by(|a, b| {
        b.value
            .len()
            .cmp(&a.value.len())
            .then(a.value.cmp(&b.value))
    });
    all
}

/// The variables a template uses as bare tags.
fn referenced(text: &str) -> BTreeSet<String> {
    BARE_TAG
        .captures_iter(text)
        .filter_map(|c| c.get(1))
        .map(|m| m.as_str().to_string())
        .collect()
}

/// A string variable by dotted path (`git.email`).
fn lookup<'a>(vars: &'a Table, path: &str) -> Option<&'a str> {
    let mut parts = path.split('.');
    let mut value = vars.get(parts.next()?)?;
    for part in parts {
        value = value.as_table()?.get(part)?;
    }
    value.as_str()
}

/// `line` with each candidate value written back as its tag, scanning left to right and taking the longest value
/// that matches at each place. `n` is the line's number in the new template, for the changes it reports.
fn substitute(
    line: &str,
    n: usize,
    candidates: &[Candidate],
    secrets: &BTreeSet<String>,
    changes: &mut Vec<Change>,
) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(c) = rest.chars().next() {
        match candidates.iter().find(|cand| rest.starts_with(&cand.value)) {
            Some(cand) => {
                if let Some(name) = cand.name(secrets) {
                    out.push_str("{{ ");
                    out.push_str(name);
                    out.push_str(" }}");
                    changes.push(Change::Substituted {
                        line: n,
                        name: name.to_string(),
                        secret: cand.secret,
                    });
                } else {
                    out.push_str(&cand.value);
                    changes.push(Change::Ambiguous {
                        line: n,
                        names: cand.names.clone(),
                    });
                }
                rest = &rest[cand.value.len()..];
            }
            None => {
                out.push(c);
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    out
}

/// Settles a conflict whose template side holds only bare variable tags: the deployed lines, with every one of those
/// variables put back. `None` when the template side has any other tag, or a variable it uses can't be found again.
fn settle(
    ours: &[&str],
    theirs: &[&str],
    candidates: &[Candidate],
    secrets: &BTreeSet<String>,
    offset: usize,
    changes: &mut Vec<Change>,
) -> Option<Vec<String>> {
    if ours.is_empty() {
        return None;
    }
    let ours_text = ours.concat();
    if ANY_TAG.find_iter(&ours_text).count() != BARE_TAG.find_iter(&ours_text).count() {
        return None;
    }
    let mut found = vec![];
    let lines: Vec<String> = theirs
        .iter()
        .enumerate()
        .map(|(i, line)| substitute(line, offset + i + 1, candidates, secrets, &mut found))
        .collect();
    let put_back: BTreeSet<&str> = found
        .iter()
        .filter_map(|change| match change {
            Change::Substituted { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    let used = referenced(&ours_text);
    if !used.iter().all(|name| put_back.contains(name.as_str())) {
        return None;
    }
    changes.extend(found);
    Some(lines)
}

/// Whether a template line only holds `{% %}` or `{# #}` tags (an `{% if %}`, an `{% endfor %}`), and renders no text
/// of its own.
fn is_tag_only(line: &str) -> bool {
    !line.contains("{{")
        && LINE_TAG.is_match(line)
        && LINE_TAG.replace_all(line, "").trim().is_empty()
}

/// When the template side of a chunk is its rendered lines with tag-only lines at the edges (an edit just before an
/// `{% if %}` or just after an `{% endif %}`), those leading and trailing tag lines, to keep around the deployed lines.
fn tag_edges<'a>(ours: &'a [&'a str], base: &[&str]) -> Option<(&'a [&'a str], &'a [&'a str])> {
    let first = ours.iter().position(|l| !is_tag_only(l))?;
    let last = ours.iter().rposition(|l| !is_tag_only(l))?;
    let (head, rest) = ours.split_at_checked(first)?;
    let (text, tail) = rest.split_at_checked(last + 1 - first)?;
    (text == base).then_some((head, tail))
}

/// A deployed line for the console: every secret value shown as `***`.
fn mask(line: &str, candidates: &[Candidate]) -> String {
    candidates
        .iter()
        .filter(|c| c.secret)
        .fold(line.to_string(), |text, c| text.replace(&c.value, "***"))
}

/// A run of lines: `stable` where all three files agree, otherwise the lines each one has there.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Chunk {
    stable: bool,
    base: Range<usize>,
    ours: Range<usize>,
    theirs: Range<usize>,
}

/// Above this many line pairs (2,000 lines against 2,000), the template is lined up with its render by exact matches
/// only: the weighted match below keeps a byte per pair.
const MAX_PATTERN_PAIRS: usize = 4_000_000;

/// Tera tags within one line.
#[expect(
    clippy::expect_used,
    reason = "a fixed pattern: it compiles, and every merge test runs it"
)]
static LINE_TAG: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\{\{.*?\}\}|\{%.*?%\}|\{#.*?#\}").expect("LINE_TAG is a valid pattern")
});

/// What a template line can render to, as a pattern: its text exactly, each `{{ }}` any text, each `{% %}` and
/// `{# #}` nothing. `None` for a line with no tags (exact matching covers it) or with whitespace control (`{%-`),
/// whose render depends on its neighbours.
fn line_pattern(line: &str) -> Option<regex::Regex> {
    if !LINE_TAG.is_match(line) || line.contains("{{-") || line.contains("-}}") {
        return None;
    }
    if ["{%-", "-%}", "{#-", "-#}"]
        .iter()
        .any(|w| line.contains(w))
    {
        return None;
    }
    let mut pattern = String::from("^");
    let mut last = 0;
    for tag in LINE_TAG.find_iter(line) {
        pattern.push_str(&regex::escape(line.get(last..tag.start())?));
        if tag.as_str().starts_with("{{") {
            pattern.push_str("(?s:.*?)");
        }
        last = tag.end();
    }
    pattern.push_str(&regex::escape(line.get(last..)?));
    pattern.push('$');
    regex::Regex::new(&pattern).ok()
}

/// For each line of the render, the template line it came from, by the heaviest common subsequence: an equal line
/// weighs 2, a line the template line's pattern matches 1, so a real match wins where both would fit.
fn match_template(base: &[&str], ours: &[&str]) -> Vec<Option<usize>> {
    let (n, m) = (base.len(), ours.len());
    if n.saturating_mul(m) > MAX_PATTERN_PAIRS {
        return matches(base, ours);
    }
    let patterns: Vec<Option<regex::Regex>> = ours.iter().map(|l| line_pattern(l)).collect();
    let mut weight = vec![0u8; n * m];
    for (i, b) in base.iter().enumerate() {
        for (j, (o, pattern)) in ours.iter().zip(&patterns).enumerate() {
            weight[i * m + j] = if b == o {
                2
            } else if pattern.as_ref().is_some_and(|p| p.is_match(b)) {
                1
            } else {
                0
            };
        }
    }
    // best[i][j]: the heaviest match of base[i..] against ours[j..].
    let width = m + 1;
    let mut best = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            let w = u32::from(weight[i * m + j]);
            let skip = best[(i + 1) * width + j].max(best[i * width + j + 1]);
            let take = if w > 0 {
                w + best[(i + 1) * width + j + 1]
            } else {
                0
            };
            best[i * width + j] = skip.max(take);
        }
    }
    let mut matched = vec![None; n];
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        let w = u32::from(weight[i * m + j]);
        if w > 0 && best[i * width + j] == w + best[(i + 1) * width + j + 1] {
            matched[i] = Some(j);
            i += 1;
            j += 1;
        } else if best[(i + 1) * width + j] >= best[i * width + j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    matched
}

/// For each line of `a`, the line of `b` it is matched to by a longest common subsequence.
fn matches(a: &[&str], b: &[&str]) -> Vec<Option<usize>> {
    let mut matched = vec![None; a.len()];
    let (mut i, mut j) = (0, 0);
    for step in diff::slice(a, b) {
        match step {
            diff::Result::Left(_) => i += 1,
            diff::Result::Right(_) => j += 1,
            diff::Result::Both(..) => {
                if let Some(slot) = matched.get_mut(i) {
                    *slot = Some(j);
                }
                i += 1;
                j += 1;
            }
        }
    }
    matched
}

/// Splits three files into chunks: stable runs, where a line of `base` is matched in both others in step, and the
/// unstable runs between them.
fn diff3(base: &[&str], ours: &[&str], theirs: &[&str]) -> Vec<Chunk> {
    let in_ours = match_template(base, ours);
    let in_theirs = matches(base, theirs);
    let both = |i: usize| -> Option<(usize, usize)> {
        Some((in_ours.get(i).copied()??, in_theirs.get(i).copied()??))
    };
    let mut chunks = vec![];
    let (mut i, mut j, mut k) = (0, 0, 0);
    while i < base.len() || j < ours.len() || k < theirs.len() {
        let mut n = 0;
        while both(i + n) == Some((j + n, k + n)) {
            n += 1;
        }
        if n > 0 {
            chunks.push(Chunk {
                stable: true,
                base: i..i + n,
                ours: j..j + n,
                theirs: k..k + n,
            });
            (i, j, k) = (i + n, j + n, k + n);
            continue;
        }
        // The matches only move forward, so the next line matched in both is past (j, k) on at least one side, and
        // each pass moves on.
        let (ni, (nj, nk)) = (i..base.len())
            .find_map(|ii| both(ii).map(|at| (ii, at)))
            .unwrap_or((base.len(), (ours.len(), theirs.len())));
        chunks.push(Chunk {
            stable: false,
            base: i..ni,
            ours: j..nj,
            theirs: k..nk,
        });
        (i, j, k) = (ni, nj, nk);
    }
    chunks
}
