# emery specify

Generate the specification and design from the sources named on the invocation and commit them as one revision.

## Synopsis

```bash
emery specify <adapter>... [--description <adapter>=<text>]
emery specify --config [<path>]
emery specify                       # discovers the project-root emery.toml
```

## Description

The one generate verb. Each run names its own sources: every positional `<adapter>` names a **workspace-backed** source (the adapter reads a read-only view rooted at the project directory; the source key is the adapter name), and every `--description <adapter>=<text>` (repeatable, `-d`) names a **description-backed** source (the adapter extracts the inline text; no filesystem view is lent). Nothing about the source list persists between runs — repeat the sources on every invocation (Makefile, skill, CI), or keep them in an operator-owned `emery.toml` selected with `--config [<path>]` (`-c`; the omitted value selects the project-relative `emery.toml`). A run naming no sources at all discovers the project-root `emery.toml` as a fallback — discovery is a fallback, never merged with argv sources.

Each run resolves its adapters before extracting; every reference loads through the deployment's `omnia:plugins/loader` capability at the location it names, under the guest name it derives, and the deployment's grant bounds every load. A local `.wasm` component is read through the project's read-only mount fresh on every run — nothing is mirrored, so deleting the file makes the next run refuse typed — and loads as its file's stem, so two components sharing a stem are refused by name before either loads; a package fetches from the registry the project's `emery.toml` `[registries]` table routes its namespace to (`emery` is `augentic.io` unless a line re-routes it; a namespace no line routes refuses typed before any fetch), which the engine names on the load, likewise read fresh on every run — the deployment keeps no project cache; a bare name is a guest the deployment declares at boot, attested by the loader or refused typed. A `[[source]] digest` pins the bytes a component or package must resolve to: it rides the load, and the loader holds the resolved bytes to it before wasmtime sees them. Extract dispatches every source over the `Source` capability at once and waits for all of them, so a run takes as long as its slowest source — unless a source fails, which ends the run as soon as the failure lands; the engine then groups the requirement claims into requirements (byte-equal ids always merge; across two or more sources the model judges the rest, with authority withheld), ranks each requirement's agreeing classes under authority precedence (intent > documentation > behaviour) into its status, drafts the scenarios and design content through schema-gated model answers checked against those requirements and the claim-derived section plan (each requirement's body is the winning claim's statement, rendered by the engine), places the accepted drafts beside its facts in the typed documents, and commits them as one revision, atomically swapping the current revision id. Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]`. Re-running over identical sources is byte-stable and reports an empty re-mine diff in the success envelope — nothing is persisted for the diff. Review the committed set with [`emery show`](show.md).

### Every run starts from new

Nothing of the stored revision reaches a run: the requirements are numbered `REQ-001` onward in the order of each group's earliest claim, every scenario and design section is drafted again, and the committed revision is a function of the sources and the drafts alone. The outgoing revision is read only to report the re-mine diff and to swap the current id; one that is outdated or unreadable yields no diff and is regenerated over. Amending an in-flight specification — continuing its ids, redrafting only what changed — is future work, not yet in the grammar.

`emery specify` without any source — and with no project-root `emery.toml` to discover — fails typed with `specify-source-required` (exit `1`); there is no interactive prompt mode, so every other input arrives as a flag. Naming the same `name` twice fails as `bad_request` (exit `1`); a `--description` entry without `<adapter>=` fails as `bad_request` (exit `1`); combining `--config` with positional adapters or `--description` fails as `bad_request` (exit `1`).

A local `.wasm` component loads dynamically through the deployment loader, by its project-relative path through the read-only project mount ([`examples/emery.toml`](../../../examples/emery.toml) loads the built mock component that way under the shipped binary), and a package reference (`emery:intent@1.0.0`, or the `intent@1.0.0` shorthand for the `emery` namespace) fetches from its registry and loads under the package reference itself. Registry and network failures refuse `unavailable` (exit `4`); a fetched artifact that fails host-side validation, or a pinned `digest` it does not resolve to, refuses `refused` (exit `1`). A bare name loads as a guest declared in the runtime invocation; one the invocation does not declare refuses `refused` (exit `1`) before any dispatch — the shipped binary declares no adapter, so under it every bare name refuses. GitHub URLs are refused (`bad_request`). The first source to fail ends the run, without waiting for the sources still extracting, and its failure is returned as the adapter put it: a refusal of its input, the operator's to fix, is `bad_request` (exit `1`); an upstream failure is `bad_gateway` (exit `4`). Evidence the claim gate rejects is Emery's own finding and is reported as `server_error` (exit `3`).

This is the CLI command invoked by [`/emery:specify`](../../../plugins/emery/skills/specify/SKILL.md). The skill elicits any missing arguments conversationally and passes them as flags; the CLI itself has no interactive mode.

## The `emery.toml` config

`emery.toml` is operator-authored and operator-owned: the engine never writes it, and reads it when the `--config` flag names it or when a run naming no sources discovers it at the project root. `--config` without a value names the project-relative `emery.toml`; an explicit value names another project-relative file (a missing explicit file is a read error, exit `3`, never a discovery miss). Each `[[source]]` entry names one source, in declaration order; its `name` is the source key, so one adapter may name several roots (the shared adapter loads once; each source still extracts over its own root). `name` may be omitted, in which case the entry is keyed as an argv source is — by the adapter's name (`typescript`, `intent` for `emery:intent@1.0.0`, a component file's kebab stem). Exactly one content key per entry — `path` or `description`; omitted means the workspace lend at `.`. `path` (and a local component `adapter`) resolves relative to the file containing it, as Cargo resolves `path` dependencies. `digest` pins the adapter's component to a full `sha256:` content hash, checked by the loader before the component is admitted; a declared guest takes none. Duplicate names fail as `bad_request` (exit `1`), the same typed error argv raises.

The `[registries]` table routes each package namespace to the registry that serves it, `<namespace> = "<registry>"`. `emery` routes to `augentic.io` unless a line says otherwise; any other namespace is refused (`bad_request`, exit `1`) until a line routes it. A run naming its sources on the command line still reads the project-root `emery.toml` for this table alone — its `[[source]]` entries are neither merged in nor decoded, so an escaping path, an unknown key, or a malformed adapter among them refuses the run whose sources they are, never one that named its own.

Every filesystem input is normalized within the project preopen `.`. Absolute paths and relative paths that escape above it fail as `bad_request` (exit `1`); the engine never tries to infer a host path from the guest's ambient working directory. Nothing is reserved: an unknown key — `git`, `url`, a `[[source]] registry` — is a parse error naming the key and its line (`bad_request`).

```toml
# Workspace lend of the invocation directory (the default: path = ".").
[[source]]
name = "docs"
adapter = "emery:documentation@1.2.0"

# Local path, resolved relative to this file.
[[source]]
name = "api-surface"
adapter = "typescript"
path = "packages/api/src"

# Inline description instead of a filesystem view — the file form of
# `--description`.
[[source]]
name = "intent"
adapter = "intent@1.0.0"
description = "Ship a location-independent spec generator."

# Local component, loaded fresh each run; without `name`, keyed `custom`.
[[source]]
adapter = "./adapters/custom.wasm"

# Third-party package: routed by `[registries]` below, pinned to its
# bytes, keyed `ledger`.
[[source]]
adapter = "acme:ledger@2.1.0"
digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"

# The registry serving each package namespace; `emery` is augentic.io
# unless a line here re-routes it.
[registries]
acme = "registry.acme.io"
```

## Options

| Option                           | Description                                                                                                                                                                                                     |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<adapter>...` (positional)      | Source adapter references: a project-relative `.wasm` component path, a package reference, or a bare name the deployment declares (the shipped binary declares none) — each named as a workspace-backed source. |
| `--description <adapter>=<text>` | Inline description-backed source (repeatable); `-d` for short.                                                                                                                                                  |
| `--config [<path>]`              | Operator-owned config; the omitted value selects `emery.toml` (`-c` for short). Mutually exclusive with positional adapters and `--description`.                                                                |
| `--format`                       | Global output format: `json` for structured automation output.                                                                                                                                                  |

## JSON output

When `--format json` is provided, returns:

- `revision` — the committed revision id, now current
- `diff` — the re-mine diff against the outgoing current revision: `from`, then a `{ added, removed, changed }` object each for `spec` (requirements matched by `id` — positional, so a requirement whose place moved reads as a change — as `{ id, subject }`, a `changed` entry naming the differing `fields`) and `design` (section keys); absent on a first run, every list empty on a byte-stable re-run (see [CLI output shapes](../cli-output-shapes.md#emery-specify))

## See also

- [`emery show`](show.md) renders the committed documents; see the [CLI reference](index.md).
