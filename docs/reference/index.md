# Reference

This section provides structured lookup documentation for Emery's configuration file, command-line interface, JSON output envelopes, adapter package store, and runtime deployment profiles. It is for developers and operators who need exact syntax, field tables, and error codes.

## Sections

| Reference page | What it covers |
| --- | --- |
| [Configuration (`emery.toml`)](config.md) | Schema and field tables for `[[source]]` and `[target]`, file discovery rules, and how CLI flags layer over `emery.toml`. |
| [CLI reference](cli/index.md) | Synopsis, flags, behaviour, and error codes for [`emery specify`](cli/specify.md), [`emery amend`](cli/amend.md), [`emery apply`](cli/apply.md), [`emery build`](cli/build.md), [`emery show`](cli/show.md), and `emery completions`. |
| [CLI output shapes](cli-output-shapes.md) | Canonical `--format json` output schemas, text-mode conventions, exit codes, and recovery error codes for every CLI verb. |
| [Adapters](adapters.md) | Adapter package reference syntax (`namespace:name@version`), the `~/.emery/adapters` store layout, registry routing, and loader error codes. |
| [Deployment profiles](deployment-profiles.md) | Host capability bindings (`wasi:keyvalue`, `wasi:blobstore`, `omnia:vcs`), filesystem mounts, worker grants, and environment variables. |

> [!NOTE]
> The v1 delivery workflow reference pages are archived at Git tag `v1` alongside the v1 codebase.
