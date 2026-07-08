# Ignoring Files

A package can exclude specific files within its `src` directory from both
deployment and [cleaning](./clean-mode.md), using glob patterns matched
against each file's path relative to the package root.

```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"
ignore = ["*.log", "cache/*", ".DS_Store"]
```

- Patterns are matched with [glob-match](https://docs.rs/glob-match)
  semantics (`*`, `**`, `?`, `[...]`, etc.) against the path relative to
  the package's `src` — not against absolute paths.
- A matched file is skipped entirely during `deploy`/`update`: it isn't
  copied, and if it already exists at the destination it isn't removed by
  [clean mode](./clean-mode.md) either — `ignore` means "hands off," not
  "delete this."
- `ignore` only applies to directory packages (it has no effect on a
  package whose `src` is a single file).

This is separate from your top-level `.gitignore` (which controls what git
tracks in the repository) — `ignore` controls what DotR itself touches at
deploy time.
