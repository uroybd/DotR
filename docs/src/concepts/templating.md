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

## Templated files are never backed up

`dotr update` normally copies a deployed file's changes back into the
repository. For a templated file, that would overwrite the template source
with rendered output — so `update` (and the underlying backup step) skips
templated files, leaving the template as the single source of truth. You'll
see a message like:

```
Skipping backup for templated file 'dotfiles/d_nvim/init.lua'
```

This is per-file, not per-package: in a directory package, only the
templated files are skipped, while regular files alongside them are still
backed up normally. For example, if `dotfiles/d_nushell/env.nu` contains
`{{ HOME }}` but the other files in `dotfiles/d_nushell/` don't have any
template markers, `update` backs up every file except `env.nu`.

If you need to change a templated file, edit the template in `dotfiles/`
directly and redeploy.
