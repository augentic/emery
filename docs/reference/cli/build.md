# emery build

Build every slice of the current plan through a target adapter.

## Synopsis

```bash
emery build <adapter>
emery build --config [<path>]
emery build                         # discovers the project-root emery.toml
```

## Description

The one build verb. A run names one **target adapter** — a `.wasm` component beneath the adapters root, a package reference, or a bare name the deployment declares — on the command line, or in the `[target]` table of an operator-owned `emery.toml` selected with `--config [<path>]` (`-c`; the omitted value selects the project-relative `emery.toml`). A run naming no adapter discovers the project-root `emery.toml` as a fallback; one that finds no `[target]` there fails typed with `build-target-required` (exit `1`). Combining `--config` with a positional adapter fails as `bad_request` (exit `1`).

The run reads the current revision first: before any is committed it fails typed with `spec-not-generated` (exit `2`), and a stored revision written under an older grammar fails with `spec-outdated` (exit `1`), both before the adapter loads. The adapter then loads as a source adapter does under [`emery specify`](specify.md) — through the deployment's `omnia:plugins/loader` capability at the location its reference names, bounded by the deployment's grant, a `digest` riding the load as its pin — and its `emery-version` pin is gated the same way (`unsupported-version`, exit `1`).

Every slice of the plan is then built in turn, each after the slices its `Depends on:` line names, ties by id. One `build` dispatch per slice hands the adapter the slice's plan entry, the specification cut to the slice's requirements (the preamble and those requirements, rendered as `emery show spec` renders them), the whole design, and the project root as the tree to build into. The adapter returns a **report**: the requirement ids it covered and the files it wrote. The engine holds the report to the slice — a covered id must be one of the slice's requirements, named once; a written path must be a relative `/`-separated path beneath the project root, outside `.omnia/`, named once — and a report that breaks a rule is Emery's own finding, `server_error` (exit `3`). A requirement the adapter leaves out of `covered` is reported as uncovered, never invented.

The first slice that fails ends the run. Its failure is returned as the adapter put it — a refusal of the slice is `bad_request` (exit `1`), an upstream failure `bad_gateway` (exit `4`) — with the slice named in the message and, where earlier slices were built, the note that they stay written: the tree is the build's output, and the engine writes no state of its own. Re-running builds every slice again over whatever the tree now holds; what an adapter does with an existing tree is the adapter's prompt's to say.

### The tree a build writes

The runtime mounts the invocation directory as `.`, writable: the tree `specify` reads its sources from and `build` writes into. The adapters root, `~/.emery/adapters`, is mounted read-only as `adapters` and apart from the project, so a component is never loaded from a directory a run can write (see [Deployment profiles](../deployment-profiles.md)).

A build lends the whole project tree to the adapter's model turn. The model reads it through the host's workspace tools, which are read-only, and writes it through the SDK's `write_file` tool alone: one file per call, created or replaced whole, at a `/`-separated path relative to the project root. The tool refuses a path outside the root, one under `.omnia/`, and one naming `spec.md`, `design.md`, or `plan.md`, so the revision store and the projections are never written by a build; the mount beneath permits the write, the tool does not. Before the turn answers, the SDK holds the report to the tree: a `written` path the tree does not hold, or a file `write_file` wrote that `written` leaves out, is returned to the model as a correction, so a report of files never written does not reach the engine.

## The `[target]` table

`emery.toml` is the same operator-owned file [`emery specify`](specify.md) reads. Its `[target]` table names the one adapter a build runs through and, optionally, the `sha256:` digest its component must resolve to; a declared guest takes none. A run naming its adapter on the command line reads no file at all. The file is read whole, so an unknown key under any table is a parse error naming the key (`bad_request`), whichever verb reads the file. A package adapter fetches from the registry `~/.emery/wasm-pkg.toml` routes its namespace to, as under `specify` ([Registry routing](specify.md#registry-routing)); the project file names which package, never where from.

```toml
[target]
adapter = "./rust/target.wasm"       # beneath ~/.emery/adapters
digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"
```

## Options

| Option                      | Description                                                                                                                                                             |
| --------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<adapter>` (positional)    | The target adapter: a `.wasm` component path beneath the adapters root, a package reference, or a bare name the deployment declares (the shipped binary declares none). |
| `--config [<path>]`         | Operator-owned config; the omitted value selects `emery.toml` (`-c` for short). Mutually exclusive with the positional adapter.                                        |
| `--format`                  | Global output format: `json` for structured automation output.                                                                                                          |

## JSON output

When `--format json` is provided, returns:

- `revision` — the id of the revision whose plan was built
- `slices` — every slice in build order, each `{ id, name, covered, uncovered, written }`: the requirement ids the adapter reported implemented, those it did not, in id order, and the files it reported written relative to the project root (see [CLI output shapes](../cli-output-shapes.md#emery-build))

Text mode prints the revision and one line per slice — `  SLICE-001 authentication: covered 2/2, written 3 files` — naming any uncovered ids in parentheses.

## See also

- [`emery specify`](specify.md) commits the plan this verb builds; [`emery show plan`](show.md) renders it; see the [CLI reference](index.md).
