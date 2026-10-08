# Deployment profiles

How the `emery` runtime binds engine storage and version control and mounts the filesystem, and how deployments other than the shipped local binary swap those bindings without touching engine code. A profile is host policy: one `omnia::runtime!` invocation choosing what backs the `wasi:keyvalue` / `wasi:blobstore` / `omnia:vcs` capability imports and which directories the guests see. The engine ships fixed key, container, mount, and repository-path formulas and never learns which backing or host directory it runs over.

## The storage boundary

Engine state — the revision store and its current revision id — is reachable only through the storage capabilities (`omnia_sdk::StateStore` / `BlobStore` on the guest side). The names the engine uses are flat, deployment-neutral formulas:

| Surface             | Kind                            | Name                                                    |
| ------------------- | ------------------------------- | ------------------------------------------------------- |
| Current revision id | keyvalue key                    | `current-revision`                                      |
| Revision documents  | blobstore container `revisions` | `<id>/spec.json`, `<id>/design.json`, `<id>/plan.json` |

The host side of the boundary is a backend type implementing `omnia::Backend` (connection options compiled into the `hosts:` row, or loaded from the environment when the row carries none) plus the host context traits `WasiKeyValueCtx` and `WasiBlobstoreCtx`. Bucket and container identifiers cross the boundary exactly once — on `open_bucket` and the container methods — which is where a profile may rewrite them.

The `spec.md` / `design.md` / `plan.md` projections a project keeps beside its code are not engine state either: they are `emery show` output the skill writes into the working tree for review, and nothing reads them back. Nor is what `emery build` commits: the labelled history is the repository's, and the engine keeps no record of it. Loaded components are not engine state: every adapter is an exact package reference, loaded through the deployment's `omnia:plugins/loader` capability inside the grant the shipped runtime fixes at compile time — the `plugins:` block of the `runtime!` invocation, a package store and the registries it routes. The store, `~/.emery/adapters`, is the authority for a release on this machine: a reference it holds is read from it, fresh on every run and with no network, and one it lacks is fetched from the registry the deployment routes its namespace to, written to the store once, and read from there after ([Adapters](adapters.md)). The routing is compiled in — `emery` to `augentic.io`, nothing else — and no file on the machine redirects it; a namespace the binary routes nowhere is filled by hand (`wkg get <reference> -o ~/.emery/adapters/`), which is also how an operator who must bound registry egress mirrors the releases a project names. `emery.toml` names which package and, through `digest`, which bytes, and a build that rewrites it cannot redirect a fetch. Earlier trees (`.omnia/`, the engine's root before `.emery/`, with its `cache/wasm-pkg` and `storage/plugins/`; `~/.emery/wasm-pkg.toml`) are orphaned — nothing reads them; delete them freely. Integrity binds the resolved sha256 digest to the exact bytes the host executes — a `[[source]] digest` pins it, checked on the bytes that answered, stored or fetched — never an engine-owned mutable sidecar.

## The shipped profile: local filesystem

The `hosts:` block of the `omnia::runtime!` invocation in [`src/main.rs`](../../src/main.rs) binds both storage hosts to `omnia_filesystem::Client` with the root compiled into the invocation: a durable, network-free store at `.emery/storage` under the invocation directory (`blobstore/` and `keyvalue/`). The root is deployment policy, not an environment tunable — retargeting it means shipping a different profile, never setting `FILESYSTEM_ROOT`. One invocation directory is one project; isolation between projects is the filesystem root itself. Revisions survive restart; what a build commits is the repository's, never the engine's.

The same block binds `omnia:vcs` to `omnia_git::Client`, one `git` process per operation over the `git` on the host's `PATH` (`GIT_BINARY` names another; 2.5 is the oldest that runs). The engine names every repository and working copy by a path beneath the project mount — the project itself as `.`, its clones under `.emery/vcs/repos/`, the working copies it cuts under `.emery/vcs/sources/<name>`, `.emery/vcs/worktrees/<slice>`, and `.emery/vcs/integration` — and the host resolves each against the mount before git sees it; a guest never holds a host path, a credential, or a git process. Credentials are git's own: whatever lets `git clone <url>` and `git push` run where `emery` runs — an SSH agent, a credential helper — serves a `[[source]] repository`, a `[target] repository`, and a `remote`. A deployment without git on the host refuses at startup, before any verb runs.

### The mount and the store

The `mounts:` block fixes one directory, and the `plugins:` block one more, after the W^X rule that what a run can write it never loads code from:

| Root       | Host directory                      | Access     | Role                                                                                                                                                      |
| ---------- | ----------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `.`        | the invocation directory            | writable   | The project, mounted: the tree `specify` lends its source adapters to read, and the repository `build` starts from. `.emery/storage` and `.emery/vcs/` sit beneath it. |
| the store  | `~/.emery/adapters`                 | host-only  | The package store, no mount at all: every adapter a reference names is a file beneath it, read by the host's loader and reachable by no guest.             |

The store is `$HOME`'s, apart from any project, so a release is fetched or copied once for every project on the machine and no run writes where the next one loads from. A missing store is an empty one, created on the first fetch. omnia refuses a store that lies beneath a writable mount — a guest could rewrite what the deployment loads — so running `emery` from `$HOME` itself is refused at startup, before any verb runs: a project is a directory of its own.

Beneath the mount, `.emery/vcs/` is the engine's: `repos/<key>` is one clone per repository URL a run names (the key a hash of the URL normalised), cloned on the first run that names it, fetched on every one after, and kept between runs; `sources/<name>` is the working copy a repository source is read in, `worktrees/<slice>` the one each slice of a wave is built in, and `integration` the one a build merges its slices into and verifies, each cut for the run and removed at its end — the integration copy, and the slice copies of a wave that failed, left in place when a run fails, for inspection, and removed by the next build. Nothing of the tree is state the engine reads back: delete `.emery/vcs/` and the next run clones again, and what a build resumes from is the label's history in the repository, not the tree. Add `.emery/` to the project's `.gitignore`; a build never counts it against the base, but git would list it.

A source adapter's extraction turn lends the project tree, or the working copy a repository source is read in, the same way it always has — the lend's path and the brief's `$SOURCE_DIR` are unchanged — but the mount beneath it is writable, so an extraction turn that wrote would not be stopped by the mount; the SDK's prompts ask for claims alone, the claim gate accepts nothing but them, and no extraction or survey turn declares a write tool. A build turn lends a slice's working copy writable and writes it through the SDK's `write_file` tool, which the guest serves through this mount: the tool refuses a path under `.emery/` or `.git/` or naming a projection, so neither the store nor the repository's own directory is written by a build, while the mount itself would permit it — the revision store still detects a document rewritten beneath it and fails closed on the next read. A verify turn lends the integration working copy with the shell and no `write_file` tool; what its checks leave behind is sealed as the wave's own commit. The project checkout is never lent to a build at all: what a build changes reaches the checkout only as the operator merges the label.

## Project-id-keyed shared backings

A multi-project deployment can scope every bucket and container under a project id. This requires a genuinely shared backing: a command-mode process over in-memory defaults creates one fresh store and one project id, so it cannot demonstrate cross-project isolation or persistence.

The `multi_project` scenario in [`tests/specify.rs`](../../tests/specify.rs) exercises the invariant directly: two project-scoped views over one shared scripted store commit and show independent revisions, and no unprefixed key is written. A concrete deployment supplies its shared backend clients and rewrites identifiers at the `WasiKeyValueCtx` and `WasiBlobstoreCtx` boundary; that host-specific configuration does not belong in the engine.

## Remote backings

`omnia-backends` ships host clients that drop into the same `hosts:` table:

| Backend            | keyvalue | blobstore | vcs | Environment configuration |
| ------------------ | -------- | --------- | --- | ------------------------- |
| `omnia-filesystem` | yes      | yes       | —   | `FILESYSTEM_ROOT`         |
| `omnia-redis`      | yes      | —         | —   | `REDIS_URL`               |
| `omnia-nats`       | yes      | yes       | —   | `NATS_ADDR`               |
| `omnia-mongodb`    | —        | yes       | —   | `MONGODB_URL`             |
| `omnia-azure-blob` | —        | yes       | —   | `AZURE_BLOB_ENDPOINT`     |
| `omnia-git`        | —        | —         | yes | `GIT_BINARY`              |

The environment variables apply to a bare `hosts:` row; a row carrying compiled-in connect options (`Backend(options)`, as the shipped profile does for its filesystem root) ignores them. Credentials and endpoints live in the host binding's environment, never in engine state or operator files. `omnia-git` is the one `omnia:vcs` backend; a profile that swaps it supplies another `WasiVcsCtx` over the same deployment-local paths.

> [!WARNING]
> Identifier grammar is backend policy. The filesystem backend rejects `/` inside a bucket or container name (path-traversal fencing), so a project-id prefix targeting it needs a single-segment delimiter (for example `<project>--revisions`) or per-project roots. The in-memory and remote backends accept `/`-separated identifiers.

Remote-binding performance is unmeasured: the numbers stay unconfirmed until a remote backing is deployed and wall-clocked.
