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

## Redeploying

If `dest` already exists (as a symlink, a directory, or a plain file) when
`deploy` runs, it's removed first and replaced with the fresh symlink — so
re-running `dotr deploy` after changing `symlink = true`/`false` correctly
switches a package between copy-mode and symlink-mode.
