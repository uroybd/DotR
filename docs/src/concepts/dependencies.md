# Dependencies

A package can declare other packages it depends on. Whenever it's selected
for an operation, its dependencies are pulled in automatically.

```toml
[packages.nvim]
src = "dotfiles/nvim"
dest = "~/.config/nvim/"
dependencies = ["fonts"]

[packages.fonts]
src = "dotfiles/fonts"
dest = "~/.local/share/fonts/"
```

Deploying just `nvim`...

```bash
dotr deploy --packages nvim
```

...also deploys `fonts`, since `nvim` depends on it. This applies to
`deploy`, `update`, and `diff` — anywhere packages are selected.

## Skipping dependency resolution

Pass `--ignore-dependencies` to select *only* the named (or profile-driven)
packages, without pulling in anything they depend on:

```bash
dotr deploy --packages nvim --ignore-dependencies
```

This is currently available on `deploy` (`dotr deploy` and
`dotr packages deploy`).

## Removing a package with dependents

`dotr packages remove` (and top-level `dotr remove`) refuses to remove a
package that another package's `dependencies` — or a profile's
`dependencies` — still references, to avoid leaving a dangling reference in
`config.toml`:

```
Package 'fonts' cannot be removed because it is depended on by profiles: [] and packages: ["nvim"]. Use --force to override.
```

Pass `--force` to remove it anyway. See also
[`--remove-orphans`](./packages.md#operating-on-packages-directly), which
does the reverse: clean up dependencies that are no longer referenced by
anything after a removal.
