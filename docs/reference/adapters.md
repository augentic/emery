# Adapters

How a run names its source and target adapters, where the `emery` binary finds them, and how an operator puts one there by hand.

## A reference

An adapter is an exact package reference, `namespace:name@version`: the wasm-pkg package the adapter is published as and the one release of it a run loads. Every adapter a run names — a positional `<adapter>` or a `--description <adapter>=<text>` on the command line, a `[[source]] adapter` or the `[target] adapter` of `emery.toml` — is spelled that way, and nothing else is accepted: a reference without a version (`emery:typescript`), a name without a namespace (`typescript@0.13.0`), a bare name (`typescript`), a path (`./typescript.wasm`), or a version that is not an exact semver (`emery:typescript@latest`) is `bad_request` (exit `1`) before anything loads, hinted `an adapter is an exact package reference, `namespace:name@version``. There is no default namespace, no latest, and no machine-wide active release: a project names the release it builds against, and two projects on one machine may name two.

First-party adapters are packages under the `emery` namespace, published to the train in [`augentic/emery-adapters`](https://github.com/augentic/emery-adapters): `emery:documentation@<version>`, `emery:intent@<version>`, `emery:typescript@<version>`, `emery:python@<version>`. A third-party adapter is a package under a namespace of its own.

A `digest` beside a reference in `emery.toml` is the project's optional pin: the `sha256:` hash the release's bytes must have, checked by the loader on whatever answered the reference — a stored file or a fetched release — before wasmtime sees them, and `refused` (exit `1`) when they hash to anything else.

## The store

The binary keeps its adapters in one directory, the **store**, `~/.emery/adapters`. A release is one file there, flat, named by its reference with the namespace joined to the name by `_`:

```text
~/.emery/adapters/
  emery_documentation@0.13.0.wasm
  emery_typescript@0.13.0.wasm
  emery_typescript@0.14.0-dev.wasm
  acme_ledger@2.1.0.wasm
```

That is the one spelling the store reads and the name `wkg get <reference> -o <dir>/` writes, so `ls ~/.emery/adapters` lists the releases on this machine as references, one file per release, and nothing in the store needs an index. The spelling is invertible — a wasm-pkg label holds neither `_` nor `@`, and a version holds no `_` — and nothing else is accepted as an alias: a file named any other way (`typescript.wasm`, a nested `emery/typescript/0.13.0.wasm`) is never read, and a run naming the release it holds reports the store as holding no such file.

A file in the store is that release on this machine, whoever wrote it. The binary's own fetch writes there; so does `cp`, and so does `wkg get`. The store never refreshes a file: a stored release is final until it is removed. A copy at a published version — `cp target.wasm ~/.emery/adapters/emery_typescript@0.13.0.wasm` — stands in for that release on this machine, for every project naming it, which is the operator's trust the store is, as `~/.cargo/bin` is; a developer's build is kept beside the release rather than in its place under a version of its own, by convention a `-dev` pre-release (`emery_typescript@0.14.0-dev.wasm`, named by a project as `emery:typescript@0.14.0-dev`), so removing it is targeted and a project naming the published version still fetches that.

The store holds raw wasm alone. A pre-compiled artifact (`.cwasm`) copied in is `refused` (exit `1`) however it hashes, since a pin says nothing about who built the bytes. A missing store is an empty one: the binary's first fetch creates it, while `cp` and `wkg get` need it there first (`mkdir -p ~/.emery/adapters`).

## Resolution

A reference resolves in two steps and no more:

1. **The store.** The file `<namespace>_<name>@<version>.wasm` under `~/.emery/adapters`, read fresh on every run, with no network.
2. **The registry.** When the store lacks the file, the release is fetched from the registry the binary routes its namespace to, hashed to the digest the registry declares for it, written to the store, and loaded. The next run reads the file.

The binary routes one namespace: `emery`, to `augentic.io`, compiled in and not redirected by any file on the machine — `~/.emery/wasm-pkg.toml` is not read. Any other namespace the binary routes nowhere: a run naming a release under it that the store lacks is `refused` (exit `1`) before any fetch, the message naming the namespace and the file the store holds no copy of, and the hint the command that fills it:

```bash
mkdir -p ~/.emery/adapters
wkg get acme:ledger@2.1.0 -o ~/.emery/adapters/
```

`wkg` routes the namespace under its own global configuration (`wkg config --edit`: `~/.config/wasm-pkg/config.toml`, its `[namespace_registries]` table) and writes `acme_ledger@2.1.0.wasm` where the store reads it; the same command, pointed at the store, is the mirror recipe for a machine that must not fetch — run it for each release a project names, and every run after reads the files. A `cp` of a built component is the same operation without a registry.

A registry that cannot supply the release — the network down, the exact version unpublished — is the loader's `unavailable` (exit `4`), and a copy at `~/.emery/adapters/<namespace>_<name>@<version>.wasm` stands in for it.

## The two axes

A **source adapter** exports the `emery:adapter/source` interface — `metadata`, and `extract`, which reads one source into a document of typed claims — and is what [`emery specify`](cli/specify.md) runs. A **target adapter** exports `emery:adapter/target`, three functions [`emery build`](cli/build.md) calls in turn:

- `metadata` names the adapter and its **merge rules**: each a glob of paths and the strategy a merge applies where both sides changed them — `union` keeps both sides' lines, each once (declaration and import lists); `ours` keeps the integrated side whole, for the adapter to regenerate (lockfiles); `theirs` the slice's. A path no rule covers that both sides changed is a conflict, which the build records and builds the slice again over; an adapter with no rule declares none.
- `build` is called once per slice, with the slice's plan entry, the specification cut to its requirements, the whole design, the commit its tree sits on, and the slice's working copy, and answers a report of the requirement ids it covered and the files it wrote. An adapter written with `emery-sdk` puts one model turn per slice under its `build.md`, the tree lent writable through the SDK's `write_files` tool.
- `verify` is called once per wave, with the integration working copy every slice of the wave merged into, and answers a verdict: `passed`, or each check that failed. An SDK adapter puts one turn under its `verify.md`, the tree lent with the shell and no write tool, so the checks the prompt names — a compiler, a test suite, a linter — run over the integrated tree, and a verdict whose `passed` disagrees with its failures is corrected before it answers.

A component exporting the wrong interface for the verb is `bad_request` (exit `1`) before any dispatch, naming the interface it lacks.

## The store and the project tree

The runtime mounts the invocation directory as `.`, writable: the tree `specify` lends its source adapters to read, and the repository `build` starts from, each slice's working copy cut beneath `.emery/vcs/` and lent to the target adapter to write, the integration working copy lent to it to verify. The store lies apart from it, under `$HOME`, so a component is never loaded from a tree a run can write — a build turn's `write_files` reaches the working copy and nothing above it. The runtime holds the rule at startup: a store beneath the writable mount is refused before any verb runs, so running `emery` from `$HOME` itself is refused — a project is a directory of its own. A project file can say which package and, through `digest`, which bytes; it cannot say where from, so a rewritten `emery.toml` cannot redirect a fetch or reach into the store.

## Refusals at a glance

| What a run names | Outcome |
| --- | --- |
| A reference the store holds | Loaded from the file; no network. |
| An `emery` release the store lacks | Fetched from `augentic.io`, written to the store, loaded. |
| A release under another namespace the store lacks | `refused` (exit `1`) before any fetch, hinted `wkg get <reference> -o ~/.emery/adapters/`. |
| A reference the registry cannot supply | `unavailable` (exit `4`); a copy in the store stands in. |
| A `digest` the bytes do not hash to | `refused` (exit `1`), naming the digest resolved and the one declared. |
| A pre-compiled artifact, stored or fetched | `refused` (exit `1`): raw wasm alone. |
| A reference without a version or a namespace, a path, a bare name | `bad_request` (exit `1`) before any load. |
| Two versions of one package in one run | `bad_request` (exit `1`) before any load: one guest per package per run. |
| One reference pinned to two digests | `bad_request` (exit `1`) before any load. |
| A source adapter under `build`, or a target under `specify` | `bad_request` (exit `1`) naming the interface it lacks, before any dispatch. |
| A release whose `emery-version` outruns the binary | `unsupported-version` (exit `1`), hinted the reinstall. |

## See also

- [`emery specify`](cli/specify.md) and [`emery build`](cli/build.md) name the adapters a run loads; [Deployment profiles](deployment-profiles.md) describes the mounts and the store rule from the runtime's side.
