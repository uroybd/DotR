# Dry Run Mode

`--dry-run` previews a `deploy` or `update` without changing anything on
disk:

```bash
dotr deploy --dry-run
dotr update --dry-run

# Combine with other flags
dotr deploy --dry-run --profile work --packages nvim,bashrc
dotr deploy --dry-run --clean=false
```

Under `--dry-run`:

- **No files are written.** Deployments print what *would* be copied or
  symlinked instead of touching disk.
- **No backups are created.**
- **Pre/post actions are not executed** — each is printed as
  `(Dry Run) Would execute action: <command>` instead. See
  [Actions](./actions.md#dry-run).
- **[Clean mode](./clean-mode.md) still previews its removals** — it runs
  by default and reports `(Dry Run) Would remove file: <path>` for
  anything it would clean up, without actually deleting it. Pass
  `--clean=false` alongside `--dry-run` if you don't want to see those
  either.

Dry run is the safest way to check what a `deploy` or `update` will do
before committing to it — especially useful after changing `dest`,
`targets`, or `ignore` patterns, or before deploying to a machine for the
first time. For comparing file *contents* rather than previewing an
operation, see `dotr diff` in the [CLI Reference](../reference/cli.md).
