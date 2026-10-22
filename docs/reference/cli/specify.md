# emery specify

Generate the specification, design, and build plan from the sources named on the invocation and stage them as one candidate for `emery apply` to seal.

## Synopsis

```bash
emery specify <adapter>... [--description <adapter>=<text>]
emery specify --config [<path>]
emery specify <adapter>... --config  # the command-line sources layered over emery.toml's
emery specify                       # discovers the project-root emery.toml
emery specify --config --apply      # stage and seal in one run
```

## Description

The one generate verb. Each run names its own sources: every positional `<adapter>` names a **workspace-backed** source (the adapter reads a read-only view rooted at the project directory; the source's name is the adapter's), and every `--description <adapter>=<text>` (repeatable, `-d`) names a **description-backed** source (the adapter extracts the inline text; no filesystem view is lent). Nothing about the source list persists between runs — repeat the sources on every invocation (Makefile, skill, CI), or keep them in an operator-owned `emery.toml` selected with `--config [<path>]` (`-c`; the omitted value selects the project-relative `emery.toml`). Command-line sources beside `--config` **layer** over the file's: the `[[source]]` entries are the base, a command-line source named as one of them is — by its adapter's package name — takes that entry's place whole (its `path`, `digest`, `rank`, and repository go with it), and any other is appended after the file's, in command-line order, so `emery specify emery:typescript@1.3.0-dev --config` tries a development build in place of the entry keyed `typescript` and leaves the rest as the file has them. A run naming no sources at all discovers the project-root `emery.toml` as a fallback; one naming sources without `--config` reads no file.

Each run resolves its adapters before extracting. An adapter is an exact package reference, `namespace:name@version` ([Adapters](../adapters.md)); every reference loads through the deployment's `omnia:plugins/loader` capability, which reads the release from the store, `~/.emery/adapters` — the file `namespace_name@version.wasm` there, read fresh on every run — and, when the store lacks it, fetches it from the registry the binary routes its namespace to (`emery` to `augentic.io`; nothing else is routed), writes it to the store, and loads it, so the next run reads the file. A package loads as its reference without the version (`emery:intent@1.0.0` is `emery:intent`), so two versions of one package in one run are refused by name before either loads. A `[[source]] digest` pins the bytes the release must resolve to: it rides the load, and the loader holds the bytes that answered — stored or fetched — to it before wasmtime sees them. Extract dispatches every source over the `Source` capability at once and waits for all of them, so a run takes as long as its slowest source — unless a source fails, which ends the run as soon as the failure lands; the engine then groups the requirement claims into requirements (byte-equal ids always merge; across two or more sources the model judges the rest, with authority withheld), ranks each requirement's agreeing classes by rank — each source's configured `rank` or its kind's default — into its status, drafts the scenarios and design content through schema-gated model answers checked against those requirements and the claim-derived section plan (each requirement's body is the winning claim's statement, rendered by the engine; the specification states behaviour by the role a thing plays — a message stream, a store, a secret store, an upstream service — and a preamble paragraph or scenario line naming the product, protocol, framework, or host behind it, or a package the source imports, is returned to the model to correct, since the design is where the technology lands: what the `call` claims spell as playing each role, and what a brief or decision record prescribes as a `decision`), places the accepted drafts beside its facts in the typed documents, slices the specification into a build plan (requirements sharing an id's first segment — the stem — are one slice at the least; a stem of more than 12 requirements is floored at its sub-stems, the first two segments, instead; the model may merge stems into one slice and never splits a floor, names each slice, assigns each design type to the one slice that owns it, and orders the slices by build dependency; a specification under one stem is one slice with no model turn), and **stages** the three as one content-addressed candidate, replacing any candidate staged before it. Nothing is sealed: the candidate waits in `.emery/storage` for review with [`emery show <artifact> --staged`](show.md) and for [`emery apply`](apply.md), which seals it as the current revision — the three documents under `spec/` at one commit labelled `emery/<id>` in the repository the `[target]` table names — and clears the slot. `--apply` does both in one run: the stage lands first, so a refused seal leaves the candidate staged for a later `emery apply`. Gaps stay `[unknown]`; disagreement surfaces inline as `[conflict]` / `[divergence]`. Re-running over identical sources is byte-stable and, where a revision is sealed, reports an empty re-mine diff in the success envelope — nothing is persisted for the diff.

### Every run starts from new

Nothing of the sealed revision reaches a run: the requirements are numbered `REQ-001` onward in the order of each group's earliest claim, the slices `SLICE-001` onward by each slice's lowest requirement, every scenario, design section, and slice is drafted again, and the staged candidate is a function of the sources and the drafts alone — each document's `parent` is `null`, since nothing was amended. The current revision is read only to report the re-mine diff, and, under `--apply`, as the commit the seal lands over and the `Parent:` its commit message names; one that is outdated or unreadable yields no diff and is sealed over. Amending an in-flight specification — continuing its ids, redrafting only what changed — is future work, not yet in the grammar.

### Where the revision lives

The specification lives in the **target repository**, the one the `[target]` table of `emery.toml` names and a build lands in ([`emery apply`](apply.md) has the three kinds: a URL cloned under `.emery/vcs/repos/<name>`, `"."` for the project's own repository, or none for a repository the engine creates there). A run that reads the file — `--config`, or the project-root `emery.toml` a run naming no sources discovers — reads that repository first, before any adapter loads, for the revision the candidate would displace: a URL is cloned, or fetched, on that read, and `repository = "."` needs the project to be a repository (`repository-required`, exit `1`, otherwise), with a commit under `--apply` (`base-not-sealed`, exit `1`). A `spec/` the repository's head holds that is not a sealed revision — the operator's own directory — refuses the run `spec-mismatch` (exit `1`) at that read, with no turn spent ([`emery apply`](apply.md) has the rule). A run naming its sources on the command line without `--config` reads no file at all, so it knows no target: it stages with no diff, and `--apply` on such a run is `build-target-required` (exit `1`), since the specification has no repository to live in.

`emery specify` without any source — and with no project-root `emery.toml` to discover — fails typed with `specify-source-required` (exit `1`); there is no interactive prompt mode, so every other input arrives as a flag. Naming the same `name` twice on the command line fails as `bad_request` (exit `1`); a `--description` entry without `<adapter>=` fails as `bad_request` (exit `1`).

A reference that is not an exact package reference — no version, no namespace, a bare name, a path, a URL — is `bad_request` (exit `1`) before anything loads, hinted with the grammar. A release the store lacks under a namespace the binary routes nowhere refuses `refused` (exit `1`) before any fetch, the message naming the namespace and the file the store holds no copy of, the hint the `wkg get <reference> -o ~/.emery/adapters/` that fills it; a registry that cannot supply the release refuses `unavailable` (exit `4`); a pre-compiled artifact, stored or fetched, and a pinned `digest` the bytes do not hash to refuse `refused` (exit `1`); a reference whose component exports no `emery:adapter/source` — a target adapter — is `bad_request` (exit `1`) before any dispatch. [`examples/emery.toml`](../../../examples/emery.toml) runs the built mock components under the shipped binary once each is copied into the store under a reference of its own. The first source to fail ends the run, without waiting for the sources still extracting, and its failure is returned as the adapter put it: a refusal of its input, the operator's to fix, is `bad_request` (exit `1`); an upstream failure is `bad_gateway` (exit `4`). Evidence the claim gate rejects is Emery's own finding and is reported as `server_error` (exit `3`).

This is the CLI command invoked by [`/emery:specify`](../../../plugins/emery/skills/specify/SKILL.md). The skill elicits any missing arguments conversationally and passes them as flags; the CLI itself has no interactive mode.

## The `emery.toml` config

`emery.toml` is operator-authored and operator-owned: the engine never writes it, and reads it when the `--config` flag names it or when a run naming no sources discovers it at the project root. `--config` without a value names the project-relative `emery.toml`; an explicit value names another project-relative file (a missing explicit file is a read error, exit `3`, never a discovery miss). Each `[[source]]` entry names one source, in declaration order; its `name` is the name the specification cites the source by, so one adapter may name several roots (the shared adapter loads once; each source still extracts over its own root). `name` may be omitted, in which case the entry is named as an argv source is — by the package's name (`documentation` for `emery:documentation@1.2.0`). `adapter` is an exact package reference, `namespace:name@version`, wherever the file sits. Exactly one content key per entry — `path` or `description`; omitted means the workspace lend at `.`. `path` resolves relative to the file containing it, as Cargo resolves `path` dependencies. `digest` pins the adapter's release to a full `sha256:` content hash, checked by the loader before the component is admitted. `rank` is the source's authority rank, an integer with `1` the highest: omitted, the adapter's kind decides it — `intent` `1`, `documentation` `2`, `behaviour` `3` — so one entry's `rank` is enough to place a source above or below a whole kind, and ranking every entry spells the order out. Equal ranks tie: two disagreeing sources at the top rank are a `[conflict]`, a source alone there wins a `[divergence]`, and each loser note records the kind and rank it lost under. `0`, a negative, or a non-integer `rank` is a parse error (`bad_request`, exit `1`). An argv source carries no `rank` and ranks by its kind. Duplicate names fail as `bad_request` (exit `1`), the same typed error argv raises.

A `[[source]]` may read a **repository** instead of the project: `repository` names its URL and `revision` the label, tag, or commit to read it at, and the two go together — one without the other is refused at the file (`bad_request`), as is a `repository` beside a `description`. After the adapters load, the run fetches the clone it keeps under `.emery/vcs/repos/` (or clones it on the first run; two sources naming one repository, however they spell its URL, share one clone and one fetch), resolves the revision, cuts a working copy under `.emery/vcs/sources/<name>`, lends it to the adapter — at the `path` within it when the entry names one, a plain relative path that may not climb out of the copy — and removes the copy once the source answered, whether or not it succeeded. A revision the repository lacks, or a URL that names no repository, is `revision-not-found` (exit `2`) before any source is asked; a remote that refuses or cannot be reached is `bad_gateway` (exit `4`). Credentials are the host's `git`'s. The success envelope names each repository source with the commit its revision resolved to.

The `[target]` table names the adapter [`emery build`](build.md) runs the plan through and, through its `name` and `repository`, the repository the specification lives in ([`emery apply`](apply.md)); `specify` reads the repository for its diff and loads no target adapter. A run naming its sources on the command line without `--config` reads no file at all: the project-root `emery.toml` is a fallback for a run naming none, and layering is `--config`'s alone. The file is read whole, so a table that does not parse — an unknown key, an `adapter` that is not an exact package reference — refuses every run that reads the file; a `[[source]]` path that escapes the project refuses only the run whose source it is.

Every source `path` is normalized within the project preopen `.`: absolute paths and relative paths that escape above the root fail as `bad_request` (exit `1`), and the engine never tries to infer a host path from the guest's ambient working directory. Nothing is reserved: an unknown key — `git`, `url`, a `[[source]] registry`, a `[registries]` table — is a parse error naming the key and its line (`bad_request`). The project names which package and, through `digest`, which bytes; where a release is fetched from is the binary's ([Adapters](../adapters.md)), so a rewritten `emery.toml` cannot redirect a fetch.

```toml
# Workspace lend of the invocation directory (the default: path = ".").
[[source]]
name = "docs"
adapter = "emery:documentation@1.2.0"

# Local path, resolved relative to this file.
[[source]]
name = "api-surface"
adapter = "emery:typescript@1.2.0"
path = "packages/api/src"

# Inline description instead of a filesystem view — the file form of
# `--description`.
[[source]]
name = "intent"
adapter = "emery:intent@1.0.0"
description = "Ship a location-independent spec generator."

# A wiki ranked beneath the code (behaviour defaults to 3), so where the
# two disagree the code wins and the wiki's statement is the loser note.
[[source]]
name = "wiki"
adapter = "emery:documentation@1.2.0"
path = "wiki"
rank = 4

# A developer's own build, copied into the store as
# `~/.emery/adapters/emery_typescript@1.3.0-dev.wasm`; without `name`,
# keyed `typescript`.
[[source]]
adapter = "emery:typescript@1.3.0-dev"

# Third-party package, fetched into the store by
# `wkg get acme:ledger@2.1.0 -o ~/.emery/adapters/`, pinned to its bytes,
# keyed `ledger`.
[[source]]
adapter = "acme:ledger@2.1.0"
digest = "sha256:0000000000000000000000000000000000000000000000000000000000000000"

# Another repository, read at a tag in a working copy the run cuts and
# removes; `path` is within the repository.
[[source]]
name = "upstream-api"
adapter = "emery:typescript@1.2.0"
repository = "https://github.com/acme/api.git"
revision = "v2.3.0"
path = "src"
```

## Where an adapter comes from

Every adapter a run names is read from the store, `~/.emery/adapters`, and a release the store lacks under the `emery` namespace is fetched from `augentic.io` into it on the first run that names it; a release under any other namespace is put there by hand, `wkg get <reference> -o ~/.emery/adapters/` or a `cp` of a built component under the name the store reads. [Adapters](../adapters.md) has the layout, the resolution order, and every refusal.

## Options

| Option                           | Description                                                                                                                                                                                                     |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `<adapter>...` (positional)      | Source adapter references, each an exact package reference `namespace:name@version` ([Adapters](../adapters.md)), named as a workspace-backed source; beside `--config`, one takes the place of the `[[source]]` of its name. |
| `--description <adapter>=<text>` | Inline description-backed source (repeatable); `-d` for short. Layers over `--config` as a positional adapter does.                                                                                              |
| `--config [<path>]`              | Operator-owned config; the omitted value selects `emery.toml` (`-c` for short). Its `[[source]]` entries are the base the command-line sources layer over, and its `[target]` names the repository the specification lives in. |
| `--apply`                        | Seal the staged candidate as the current revision in the same run, as [`emery apply`](apply.md) would; needs a `[target]`, so `--config` or a discovered `emery.toml`.                                         |
| `--format`                       | Global output format: `json` for structured automation output.                                                                                                                                                  |

## JSON output

When `--format json` is provided, returns:

- `revision` — the staged candidate's id
- `waves` — the candidate plan's slices grouped into the sets ready to build at once, a list of lists of slice ids in build order: the first wave every slice that depends on nothing, each wave after it every slice whose `depends-on` names only slices in the waves before, each wave in id order; their count is the plan's longest dependency chain and the widest is how many slices could build at once
- `diff` — the re-mine diff against the current revision the candidate would displace, read from the target repository: `from`, then a `{ added, removed, changed }` object each for `spec` (requirements matched by `id` — positional, so a requirement whose place moved reads as a change — as `{ id, subject }`, a `changed` entry naming the differing `fields`), `design` (section keys), and `plan` (slices matched by `id` as `{ id, name }`, a `changed` entry naming the differing `fields`); absent when no `[target]` is known or no revision is sealed, every list empty on a byte-stable re-run (see [CLI output shapes](../cli-output-shapes.md#emery-specify))
- `repositories` — every source read from a repository, in declaration order, each `{ source, repository, revision, commit }`: the source's name, the URL as the file spelled it, the revision asked for, and the commit it resolved to; omitted when no source names one
- `staged` — `true` while the candidate stands staged and nothing is sealed; `false` once `--apply` sealed it
- `applied` — under `--apply`, what the run sealed, in the shape [`emery apply`](apply.md#json-output) reports (`revision`, `parent`, `commit`, `label`, `repository`, `kind`, `diff`); omitted otherwise

Text mode prints `staged candidate <id>`, the plan's shape on one line — `  plan: 3 slices in 2 waves, widest 2` — one line per repository source — `  upstream-api read from https://github.com/acme/api.git at v2.3.0: 9f8e7d6…` — a one-line summary of the diff's counts where a revision is sealed, then `` nothing sealed; `emery apply` seals it `` or, under `--apply`, `  sealed revision <id> as emery/<id> at <commit8>`; the per-requirement and per-slice entries, and the waves themselves, ride the JSON envelope alone.

## See also

- [`emery show --staged`](show.md) renders the staged candidate, [`emery apply`](apply.md) seals it, and [`emery build`](build.md) builds the sealed plan; see the [CLI reference](index.md).
