# emery specify

Generate the specification, design, and build plan from the sources named on the invocation and commit them as one revision.

## Synopsis

```bash
emery specify <adapter>... [--description <adapter>=<text>]
emery specify --config [<path>]
emery specify                       # discovers the project-root emery.toml
```

## Description

The one generate verb. Each run names its own sources: every positional `<adapter>` names a **workspace-backed** source (the adapter reads a read-only view rooted at the project directory; the source's name is the adapter's), and every `--description <adapter>=<text>` (repeatable, `-d`) names a **description-backed** source (the adapter extracts the inline text; no filesystem view is lent). Nothing about the source list persists between runs — repeat the sources on every invocation (Makefile, skill, CI), or keep them in an operator-owned `emery.toml` selected with `--config [<path>]` (`-c`; the omitted value selects the project-relative `emery.toml`). A run naming no sources at all discovers the project-root `emery.toml` as a fallback — discovery is a fallback, never merged with argv sources.

Each run resolves its adapters before extracting; every reference loads through the deployment's `omnia:plugins/loader` capability at the location it names, under the guest name it derives, and the deployment's grant bounds every load. A local `.wasm` component is a path beneath the adapters root, `~/.emery/adapters`, which the runtime mounts read-only and apart from the project tree so that a component is never loaded from a directory a run can write; it is read through that mount fresh on every run — nothing is mirrored, so deleting the file makes the next run fail `not_found` — and loads as its file's stem, so two components sharing a stem are refused by name before either loads; a package fetches from the registry the deployment routes its namespace to — `~/.emery/wasm-pkg.toml`, read natively at startup and never by a guest; `emery` is `augentic.io` unless a line there re-routes it; a namespace nothing routes refuses typed before any fetch — the engine naming no registry on the load, likewise read fresh on every run — the deployment keeps no project cache — and loads as its reference without the version (`emery:intent@1.0.0` is `emery:intent`), so two versions of one package are refused by name the same way; a bare name is a guest the deployment declares at boot, attested by the loader or refused typed. A `[[source]] digest` pins the bytes a component or package must resolve to: it rides the load, and the loader holds the resolved bytes to it before wasmtime sees them. Extract dispatches every source over the `Source` capability at once and waits for all of them, so a run takes as long as its slowest source — unless a source fails, which ends the run as soon as the failure lands; the engine then groups the requirement claims into requirements (byte-equal ids always merge; across two or more sources the model judges the rest, with authority withheld), ranks each requirement's agreeing classes under authority precedence (intent > documentation > behaviour) into its status, drafts the scenarios and design content through schema-gated model answers checked against those requirements and the claim-derived section plan (each requirement's body is the winning claim's statement, rendered by the engine), places the accepted drafts beside its facts in the typed documents, slices the specification into a build plan (requirements sharing an id's first segment — the stem — are one slice at the least; the model may merge stems into one slice and never splits one, names each slice, assigns each design type to the one slice that owns it, and orders the slices by build dependency; a specification under one stem is one slice with no model turn), and commits the three as one revision, atomically swapping the current revision id. Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]`. Re-running over identical sources is byte-stable and reports an empty re-mine diff in the success envelope — nothing is persisted for the diff. Review the committed set with [`emery show`](show.md).

### Every run starts from new

Nothing of the stored revision reaches a run: the requirements are numbered `REQ-001` onward in the order of each group's earliest claim, the slices `SLICE-001` onward by each slice's lowest requirement, every scenario, design section, and slice is drafted again, and the committed revision is a function of the sources and the drafts alone. The outgoing revision is read only to report the re-mine diff and to swap the current id; one that is outdated or unreadable yields no diff and is regenerated over. Amending an in-flight specification — continuing its ids, redrafting only what changed — is future work, not yet in the grammar.

`emery specify` without any source — and with no project-root `emery.toml` to discover — fails typed with `specify-source-required` (exit `1`); there is no interactive prompt mode, so every other input arrives as a flag. Naming the same `name` twice fails as `bad_request` (exit `1`); a `--description` entry without `<adapter>=` fails as `bad_request` (exit `1`); combining `--config` with positional adapters or `--description` fails as `bad_request` (exit `1`).

A local `.wasm` component loads dynamically through the deployment loader, by its path beneath the read-only adapters root ([`examples/emery.toml`](../../../examples/emery.toml) loads the built mock component that way under the shipped binary, once it is installed there), and a package reference (`emery:intent@1.0.0`, or the `intent@1.0.0` shorthand for the `emery` namespace) fetches from its registry and loads under the package reference itself. A namespace the deployment routes nowhere refuses `refused` (exit `1`) before any fetch, the hint naming the line to add to `~/.emery/wasm-pkg.toml`; registry and network failures refuse `unavailable` (exit `4`); a fetched artifact that fails host-side validation, or a pinned `digest` it does not resolve to, refuses `refused` (exit `1`). A bare name loads as a guest declared in the runtime invocation; one the invocation does not declare refuses `refused` (exit `1`) before any dispatch — the shipped binary declares no adapter, so under it every bare name refuses. GitHub URLs are refused (`bad_request`). The first source to fail ends the run, without waiting for the sources still extracting, and its failure is returned as the adapter put it: a refusal of its input, the operator's to fix, is `bad_request` (exit `1`); an upstream failure is `bad_gateway` (exit `4`). Evidence the claim gate rejects is Emery's own finding and is reported as `server_error` (exit `3`).

This is the CLI command invoked by [`/emery:specify`](../../../plugins/emery/skills/specify/SKILL.md). The skill elicits any missing arguments conversationally and passes them as flags; the CLI itself has no interactive mode.

## The `emery.toml` config

`emery.toml` is operator-authored and operator-owned: the engine never writes it, and reads it when the `--config` flag names it or when a run naming no sources discovers it at the project root. `--config` without a value names the project-relative `emery.toml`; an explicit value names another project-relative file (a missing explicit file is a read error, exit `3`, never a discovery miss). Each `[[source]]` entry names one source, in declaration order; its `name` is the name the specification cites the source by, so one adapter may name several roots (the shared adapter loads once; each source still extracts over its own root). `name` may be omitted, in which case the entry is named as an argv source is — by the adapter's name (`typescript`, `intent` for `emery:intent@1.0.0`, a component file's kebab stem). Exactly one content key per entry — `path` or `description`; omitted means the workspace lend at `.`. `path` resolves relative to the file containing it, as Cargo resolves `path` dependencies; a local component `adapter` is a path beneath the adapters root, `~/.emery/adapters`, wherever the file sits. `digest` pins the adapter's component to a full `sha256:` content hash, checked by the loader before the component is admitted; a declared guest takes none. Duplicate names fail as `bad_request` (exit `1`), the same typed error argv raises.

The `[target]` table names the adapter [`emery build`](build.md) runs the plan through; `specify` takes nothing from it. A run naming its sources on the command line reads no file at all: the project-root `emery.toml` is a fallback for a run naming none, never merged in. The file is read whole, so a table that does not parse — an unknown key, a malformed adapter reference — refuses every run that reads the file; a `[[source]]` path that escapes the project refuses only the run whose source it is.

Every filesystem input is normalized within its root: a source `path` within the project preopen `.`, a component `adapter` within the adapters root. Absolute paths and relative paths that escape above the root fail as `bad_request` (exit `1`); the engine never tries to infer a host path from the guest's ambient working directory. Nothing is reserved: an unknown key — `git`, `url`, a `[[source]] registry`, a `[registries]` table — is a parse error naming the key and its line (`bad_request`).

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

# Local component, `~/.emery/adapters/custom/custom.wasm`, loaded fresh each
# run; without `name`, keyed `custom`.
[[source]]
adapter = "./custom/custom.wasm"

# Third-party package: its `acme` namespace routed in
# `~/.emery/wasm-pkg.toml`, pinned to its bytes, keyed `ledger`.
[[source]]
adapter = "acme:ledger@2.1.0"
digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
```

## Registry routing

Where a package adapter is fetched from is the deployment's decision, never the project's: `~/.emery/wasm-pkg.toml`, in the schema of wkg's `config.toml`, read natively by the shipped binary at every start and never mounted, so no guest — the engine, an adapter, or a build turn writing the project tree — can reach it. The `emery` namespace routes to `augentic.io` unless a line there re-routes it; every other namespace must be routed there, or a run naming a package under it refuses `refused` (exit `1`) before any fetch. The project names which package and, through `digest`, which bytes; it cannot say where from, so a rewritten `emery.toml` cannot redirect a fetch. On first use the binary writes the file as a commented template, which it never touches again; a file that does not parse stops the binary at startup, naming the path.

```toml
# ~/.emery/wasm-pkg.toml

# A registry publishing /.well-known/wasm-pkg/registry.json.
[namespace_registries]
acme = "registry.acme.io"

# A plain OCI host, such as a GitHub Container Registry organisation.
# acme = { registry = "ghcr.io", metadata = { preferredProtocol = "oci", oci = { registry = "ghcr.io", namespacePrefix = "acme/" } } }

# Credentials for a private registry; pulls from augentic.io are anonymous.
[registry."registry.acme.io".oci]
auth = { username = "ci", password = "<token>" }
```

Leave `default_registry` unset: it would route every unrouted namespace somewhere. In CI, write the file before `emery` runs, as an `.npmrc` is provisioned.

## Options

| Option                           | Description                                                                                                                                                                                                     |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<adapter>...` (positional)      | Source adapter references: a `.wasm` component path beneath the adapters root, a package reference, or a bare name the deployment declares (the shipped binary declares none) — each named as a workspace-backed source. |
| `--description <adapter>=<text>` | Inline description-backed source (repeatable); `-d` for short.                                                                                                                                                  |
| `--config [<path>]`              | Operator-owned config; the omitted value selects `emery.toml` (`-c` for short). Mutually exclusive with positional adapters and `--description`.                                                                |
| `--format`                       | Global output format: `json` for structured automation output.                                                                                                                                                  |

## JSON output

When `--format json` is provided, returns:

- `revision` — the committed revision id, now current
- `diff` — the re-mine diff against the outgoing current revision: `from`, then a `{ added, removed, changed }` object each for `spec` (requirements matched by `id` — positional, so a requirement whose place moved reads as a change — as `{ id, subject }`, a `changed` entry naming the differing `fields`), `design` (section keys), and `plan` (slices matched by `id` as `{ id, name }`, a `changed` entry naming the differing `fields`); absent on a first run, every list empty on a byte-stable re-run (see [CLI output shapes](../cli-output-shapes.md#emery-specify))

Text mode prints only the revision and a one-line summary of those counts; the per-requirement and per-slice entries ride the JSON envelope alone.

## See also

- [`emery show`](show.md) renders the committed documents, and [`emery build`](build.md) builds the plan; see the [CLI reference](index.md).
