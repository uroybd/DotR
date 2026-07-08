# Symlinks

By default, DotR **copies** files from the repository to their destination.
For configs you edit frequently and want to reflect immediately — no
`update` step — a package can instead be deployed as a **symlink**.

```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"
symlink = true
```

## Importing with a symlink

```bash
dotr import ~/.config/nvim/ --symlink
```

`--symlink` sets `symlink = true` on the new package and immediately
deploys it (a plain `dotr import` only copies into the repository and
registers the package, without deploying).

## Enabling symlinking globally

Setting `symlink = true` per package works well for a handful of them, but
if you want symlinking everywhere, set it once at the top level of
`config.toml` instead:

```toml
symlink = true

[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"
# no symlink field needed here - the global setting covers it
```

Every directory package deploys as a symlink, without needing
`symlink = true` written into each one individually.

### Opting a package out

A package's own `symlink` setting always wins over the global one, in
either direction — set `symlink = false` explicitly on a package to
exclude it from a global `symlink = true`:

```toml
symlink = true

[packages.dotfiles_only]
src = "dotfiles/dotfiles_only"
dest = "~/.dotfiles_only/"
symlink = false   # opts out; deployed as a plain copy despite the global flag
```

A package that doesn't mention `symlink` at all just follows whatever the
global flag says. Only an explicit `true` or `false` on the package
overrides it.

## How it works

1. `import` copies the source into `dotfiles/<name>/` in the repository, as
   normal.
2. `deploy` copies those files again into `deployed/<name>/` (a staging
   directory at the repository root — `dotr init` adds `deployed` to
   `.gitignore`, since it's derived, not source, content).
3. `deploy` then creates a symlink at the package's `dest` pointing at
   `deployed/<name>/`.

```
dotfiles/nvim/   (repository — source of truth, git-tracked)
      │  deploy copies files
      ▼
deployed/nvim/   (staging — gitignored)
      ▲  symlink
      │
~/.config/nvim/  (dest — symlinked to deployed/nvim/)
```

Editing files at `~/.config/nvim/` edits `deployed/nvim/` directly (it's a
symlink), so changes take effect immediately. Run `dotr update` to copy
those changes back into `dotfiles/nvim/` when you're ready to commit them.

This whole-directory symlink is called **folding** — the entire destination
becomes one symlink. It's the default for any directory package with
`symlink = true`.

## Unfolding: per-file symlinks

Folding means the destination directory doesn't really exist on disk
anymore — it's entirely replaced by a symlink. That's fine as long as
`dest` only ever holds files DotR manages, but it can't hold anything
else: there's nowhere for an untracked, local-only file to live alongside
the managed ones.

Setting `unfold_symlink = true` on a directory package switches to the
opposite granularity: `dest` stays a **real directory**, and DotR instead
creates an individual symlink for each file inside it.

```toml
[packages.nix]
src = "dotfiles/nix"
dest = "~/.config/nix/"
symlink = true
unfold_symlink = true
```

```
~/.config/nix/            (dest — a real directory)
├── nix.conf   -> ../../deployed/nix/nix.conf   (managed, symlinked)
└── local-only.conf                              (untouched, real file)
```

With this, you can drop `local-only.conf` straight into `~/.config/nix/`
and DotR will never touch it — it isn't part of `dotfiles/nix/`, so it's
simply never in scope for anything DotR does there.

**A non-empty `ignore` list implies unfolding**, even without setting
`unfold_symlink` explicitly. This is automatic because the two don't make
sense apart: an [ignored](./ignoring-files.md) file has nowhere to go if
the whole directory is one symlink to a fully-populated staging copy —
unfolding is what actually makes "leave this path alone" meaningful for a
symlinked package.

```toml
[packages.nix]
src = "dotfiles/nix"
dest = "~/.config/nix/"
symlink = true
ignore = ["secrets.conf"]   # implies unfold_symlink, even though it's unset
```

### Safety guarantees

- **Deploying** only ever creates or replaces symlinks for files that are
  actually part of the package's source tree. A path that isn't — a
  foreign file — is never inspected, never removed, never overwritten.
- **Cleaning** (when a source file is deleted and you redeploy) only
  removes symlinks it recognizes as its own — ones that resolve back into
  that package's `deployed/<name>/` staging directory. A real file, or a
  symlink pointing anywhere else, is left alone. A directory is only
  removed once it's completely empty, never forced.
- **`dotr update`** only pulls changes back from paths that are managed
  symlinks. A foreign file sitting in the same real directory is never
  copied into the tracked repository.

## Redeploying

If `dest` already exists (as a symlink, a directory, or a plain file) when
`deploy` runs, it's removed first and replaced with the fresh symlink — so
re-running `dotr deploy` after changing `symlink = true`/`false` correctly
switches a package between copy-mode and symlink-mode. The same applies to
switching `unfold_symlink` on: pre-existing content at a managed file's
path is replaced with a symlink the same way whole-directory folding
replaces it, since that's a one-time transition into symlink management —
after that, only foreign paths (paths outside the package entirely) are
ever left alone.
