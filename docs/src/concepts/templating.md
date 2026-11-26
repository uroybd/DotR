# Templating

DotR compiles files through the [Tera](https://keats.github.io/tera/)
template engine at deploy time, using the resolved
[variables](./variables.md) for that package and profile.

```toml
# a config file with Tera templates
[user]
name = "{{ git.name }}"
email = "{{ git.email }}"

{% if HOME %}
[paths]
data = "{{ HOME }}/Data"
{% endif %}
```

- `{{ variable }}` — variable substitution
- `{% if condition %}...{% endif %}` — conditionals
- `{# comment #}` — comments, stripped from output

Any construct Tera supports (loops, filters, macros) works, since DotR
hands the file straight to Tera's renderer.

## Detection is automatic

A file is treated as templated if it contains **any** of `{{`, `}}`, `{%`,
`%}`, `{#`, or `#}` (including Tera's whitespace-trimming variants like
`{%-` and `-%}`) anywhere in its content — no extension, header, or config
flag required. Regular and templated files can coexist in the same package
or directory.

## Destination paths are templated too

Not just file contents — the `dest` path (and any per-profile
[`targets`](./packages.md#per-profile-destinations-targets) override) is
also run through Tera before use, so a destination can depend on variables:

```toml
[packages.ssh_config]
src = "dotfiles/ssh_config"
dest = "{{ HOME }}/.ssh/config"
```

## Backing up templated files

`dotr update` copies a deployed file's changes back into the repository. A
templated file can't simply be copied: that would replace the template with
its rendered output. So `update` *merges* instead, and the template stays
the source of truth.

It works like a three-way merge (diff3), with the template rendered with the
current variables as the common ancestor:

- **A changed line with no tags** (`mode = fast` → `mode = slow`) goes into
  the template as is.
- **A value is written back as its tag.** Where a deployed line holds the
  value of a variable the template already uses, it goes into the template
  as that variable: `cache = /home/you/.cache` becomes
  `cache = {{ HOME }}/.cache`. An edit to a line whose only tags are plain
  variables (`data = "{{ HOME }}/Data"` → `.../Documents`) is settled the
  same way, keeping the tag.
- **Secrets never land in the repository.** Any value answered at a
  [prompt](./prompts.md), or kept in `.uservariables.toml`, is always written
  back as its `{{ name }}`, whether the template used it before or not.
- **A change to a line the template computes** (inside `{% if %}` or a loop,
  or through a filter like `{{ name | upper }}`) is a *conflict*.

Nothing is written unless it round-trips: compiled again, the merged
template must give the deployed file exactly. A file that won't (say, the
deployed file now holds `{{` as text) is left alone, and you're told why.

### Conflicts

Run in a terminal, `update` shows each conflict, both sides numbered (secret
values masked as `***`), and asks what to do:

| Choice       | What happens                                                                                       |
| ------------ | -------------------------------------------------------------------------------------------------- |
| `o`, ours    | Keep the template's lines. The deployed change is left out; the next deploy puts the template's back. |
| `t`, theirs  | Take the deployed lines (values written back as tags). Any template logic in those lines is gone.  |
| `b`, both    | Keep the template's lines and add the deployed lines after them, to tidy by hand. Until you do, the file renders both. |
| `e`, edit    | Open the whole file in your editor, every remaining conflict marked git-style (`<<<<<<<`, `=======`, `>>>>>>>`). What you save becomes the template. |
| `s`, skip    | Leave this file as it is.                                                                          |

Without a terminal (or to answer every conflict at once), pass
`--conflict ours|theirs|both|edit|skip`. With neither, conflicts are skipped
and listed.

The editor is the first of `DOTR_EDITOR`, `VISUAL` and `EDITOR` that is set;
a command with arguments works (`DOTR_EDITOR="code --wait"`). The deployed
side of each marked conflict already has its values, secrets included,
written back as tags. A saved file is used only if no conflict markers are
left, no secret's value is in it as text, and it compiles; otherwise you can
reopen the editor or skip the file. If what you saved doesn't render to the
deployed file exactly, `update` says by how many lines, since the next
deploy will change the deployed file to match.

### Everything is reported

Whatever `update` writes into a template that differs from the deployed
bytes is reported, one line per change, with the file and line:

```
[INFO] Merged changes from '~/.config/app/app.conf' into template 'dotfiles/d_app/app.conf'
[INFO]   dotfiles/d_app/app.conf:3: wrote {{ HOME }} in place of its value
[INFO]   dotfiles/d_app/app.conf:7: wrote {{ API_TOKEN }} in place of a secret value
[WARNING]   dotfiles/d_app/app.conf: line 9: kept the template's; the deployed line 9 left out (the next deploy puts the template's back)
```

A value that belongs to more than one variable is left as text, with a
warning naming them: which one was meant can't be told. Values shorter than
four characters are left as text too (secrets aside), since they turn up by
chance.

### Binary files

Files that aren't text (not UTF-8, or holding a NUL byte) are never treated
as templates: they are copied back byte for byte. A templated file whose
deployed copy is no longer text is left alone, with a warning.

This is all per file: in a directory package, plain files are copied and
templated ones merged, side by side.
