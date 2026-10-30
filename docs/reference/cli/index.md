# CLI reference

This section documents every command, flag, and global option of the `emery` command-line interface. It is for developers, operators, and automation scripts invoking `emery`.

## Installation

```bash
brew tap augentic/tap
brew install emery

# Or install from source:
cargo install --git https://github.com/augentic/emery --locked
```

## Global options

Every `emery` subcommand accepts these global options:

| Flag | Environment variable | Default | Description |
| --- | --- | --- | --- |
| `--format <text\|json>` | `EMERY_FORMAT` | `text` | Output format on `stdout`. Use `json` for structured machine-readable envelopes (see [CLI output shapes](../cli-output-shapes.md)). |
| `-v`, `-vv`, ... | `RUST_LOG` | `info` | Increase log verbosity on `stderr` (`-v` for `debug`, `-vv` for `trace`). Cannot be combined with `-q`. |
| `-q`, `-qq`, ... | `RUST_LOG` | `info` | Decrease log verbosity on `stderr` (`-q` for `warn`, `-qq` for `error`). Cannot be combined with `-v`. |
| `-h`, `--help` | — | — | Print help text and exit `0`. |
| `-V`, `--version` | — | — | Print the binary version and exit `0`. |

## Commands

| Command | Summary |
| --- | --- |
| [`emery specify`](specify.md) | Extract claims from sources, synthesize `spec`, `design`, and `plan`, and stage a candidate revision (`--apply` seals it; `--fresh` starts a new history). |
| [`emery amend`](amend.md) | Match change sources against the existing requirements in the current revision and stage an incremental `Amendment` (`--apply` seals it). |
| [`emery apply`](apply.md) | Seal the currently staged candidate or amendment into the target Git repository under `spec/` on branch `emery/<id>`. |
| [`emery build`](build.md) | Build the sealed plan in dependency waves through a target adapter, merging and verifying each wave on branch `emery/<id>`. |
| [`emery show`](show.md) | Render `spec`, `design`, `plan`, or `amendment` from the current revision or staging area as Markdown or JSON. |
| `emery completions <shell>` | Print a shell-completion script (`bash`, `zsh`, `fish`, `elvish`, `powershell`) to `stdout`. |

## CLI conventions

- **Streams:** Semantic command output (text or JSON) is written to `stdout`. Error envelopes and live tracing logs are written to `stderr`.
- **Adapter references:** Every adapter is named by an exact package reference `namespace:name@version`, loaded from `~/.emery/adapters` and fetched automatically for the `emery` namespace on first use (see [Adapters](../adapters.md)).
- **Configuration:** Project sources and target settings live in [`emery.toml`](../config.md).
- **State integrity:** Never edit `.emery/storage/` or the root-level `spec/` directory in your target repository by hand. Always use `emery specify`, `emery amend`, and `emery apply` (see [Revision lifecycle](../../explanation/revision-lifecycle.md)).
